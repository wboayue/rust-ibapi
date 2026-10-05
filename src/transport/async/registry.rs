//! The async bus's channel registries, the counterparts of the sync
//! `SenderHash` and `SharedChannels`. Each owns its map and lock, so routing
//! and teardown are one call per registry instead of open-coded per map.
//!
//! The maps sit behind std locks, which no caller holds across an `.await`,
//! so teardown needs no runtime: `Client::drop` runs the whole shutdown.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use log::{debug, trace};
use tokio::sync::{broadcast, Mutex};

use crate::accounts::types::AccountId;
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::transport::common::LeaseRef;
use crate::transport::{Admit, BoundState, BufferBound, RoutedItem, SharedCounts, SharedTicket};
use crate::Error;

pub(super) type BroadcastSender = broadcast::Sender<RoutedItem>;

/// A registration: the channel, its subscription's lease, plus an unread-item
/// cap when the request was opened with `send_request_bounded`.
#[derive(Debug)]
pub(super) struct Route {
    pub(super) sender: BroadcastSender,
    pub(super) lease: LeaseRef,
    bound: Option<RouteBound>,
}

#[derive(Debug)]
struct RouteBound {
    state: BoundState,
    /// Items sent to the channel, and items the subscription has read (its
    /// `AsyncInternalSubscription::reads`): the difference is unread.
    /// `Sender::len()` can't stand in, since it counts values the
    /// never-reading `template_receiver` hasn't seen.
    sent: AtomicUsize,
    reads: Arc<AtomicUsize>,
}

impl RouteBound {
    fn unread(&self) -> usize {
        self.sent.load(Ordering::Relaxed).saturating_sub(self.reads.load(Ordering::Relaxed))
    }
}

impl Route {
    pub(super) fn unbounded(sender: BroadcastSender, lease: LeaseRef) -> Self {
        Self { sender, lease, bound: None }
    }

    /// A bounded route's channel has one slot more than `bound.limit`, for the
    /// one terminal item (end marker, error, or overflow error) that may
    /// arrive with the cap full, so sending it never evicts a queued item.
    /// `reads` is the subscription's read counter.
    pub(super) fn bounded(sender: BroadcastSender, lease: LeaseRef, bound: BufferBound, reads: Arc<AtomicUsize>) -> Self {
        let bound = RouteBound {
            state: BoundState::new(bound),
            sent: AtomicUsize::new(0),
            reads,
        };
        Self {
            sender,
            lease,
            bound: Some(bound),
        }
    }

    fn closed(&self) -> bool {
        self.bound.as_ref().is_some_and(|bound| bound.state.closed())
    }

    /// An unbounded route sharing this one's channel and lease.
    fn alias(&self) -> Self {
        Self::unbounded(self.sender.clone(), self.lease.clone())
    }

    /// Whether this route holds `lease` and every holder of it is gone.
    pub(super) fn released(&self, lease: &LeaseRef) -> bool {
        self.lease.is(lease) && !self.lease.is_live()
    }

    /// Send `item`, subject to a bounded route's cap ([`BoundState::admit`]).
    /// The dispatcher is the only producer, so the unread check cannot race
    /// another send. Never blocks.
    pub(super) fn deliver(&self, id: impl Display, item: RoutedItem) {
        let item = match &self.bound {
            Some(bound) => match bound.state.admit(&item, bound.unread()) {
                Admit::Deliver => {
                    bound.sent.fetch_add(1, Ordering::Relaxed);
                    item
                }
                Admit::Overflow => bound.state.overflow_error(),
                Admit::Discard => {
                    trace!("discarding item for closed route {id}");
                    return;
                }
            },
            None => item,
        };
        let _ = self.sender.send(item);
    }
}

/// Routes keyed by request id, order id, or execution id.
#[derive(Debug)]
pub(super) struct SenderHash<K> {
    routes: RwLock<HashMap<K, Route>>,
}

impl<K: Hash + Eq + Display + Debug> SenderHash<K> {
    pub(super) fn new() -> Self {
        Self {
            routes: RwLock::new(HashMap::new()),
        }
    }

    fn read(&self) -> RwLockReadGuard<'_, HashMap<K, Route>> {
        self.routes.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<K, Route>> {
        self.routes.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers `route` under `id`, replacing any earlier registration.
    pub(super) fn insert(&self, id: K, route: Route) {
        self.write().insert(id, route);
    }

    /// Run `f` on `id`'s route while holding the read lock, so no removal can
    /// land between the lookup and `f`.
    #[cfg(test)]
    pub(super) fn with_route<R>(&self, id: &K, f: impl FnOnce(&Route) -> R) -> Option<R> {
        self.read().get(id).map(f)
    }

    /// Deliver `item` to `id`'s route; hands it back when nothing is
    /// registered under `id`.
    pub(super) fn deliver(&self, id: &K, item: RoutedItem) -> Result<(), RoutedItem> {
        self.deliver_then(id, item, |_| {})
    }

    /// Deliver `item` to `id`'s route, then alias that route under `alias`
    /// in `aliases` while still holding `id`'s read lock, so a concurrent
    /// clear can't land between the delivery and the alias. Lock order:
    /// `self` (read), then `aliases` (write); nothing takes them the other
    /// way round. Hands the item back when nothing is registered under `id`.
    pub(super) fn deliver_aliased<A: Hash + Eq + Clone + Display + Debug>(
        &self,
        id: &K,
        item: RoutedItem,
        alias: Option<&A>,
        aliases: &SenderHash<A>,
    ) -> Result<(), RoutedItem> {
        self.deliver_then(id, item, |route| {
            if let Some(alias) = alias {
                aliases.insert(alias.clone(), route.alias());
            }
        })
    }

    /// Deliver `item` to `id`'s route and run `then` on it, both under the
    /// read lock.
    fn deliver_then(&self, id: &K, item: RoutedItem, then: impl FnOnce(&Route)) -> Result<(), RoutedItem> {
        let routes = self.read();
        let Some(route) = routes.get(id) else {
            return Err(item);
        };
        route.deliver(id, item);
        then(route);
        Ok(())
    }

    /// Whether a route is registered under `id`. For a caller whose miss
    /// path still needs the message `deliver` would consume.
    pub(super) fn contains(&self, id: &K) -> bool {
        self.read().contains_key(id)
    }

    /// Remove `id`'s registration if it matches `pred`; returns whether it
    /// was removed.
    fn remove_if(&self, id: K, pred: impl FnOnce(&Route) -> bool) -> bool {
        match self.write().entry(id) {
            Entry::Occupied(route) if pred(route.get()) => {
                route.remove();
                true
            }
            _ => false,
        }
    }

    /// Remove `id`'s registration only if it holds `lease`, so a newer
    /// registration under the same id survives.
    pub(super) fn remove_if_same(&self, id: K, lease: &LeaseRef) {
        self.remove_if(id, |route| route.lease.is(lease));
    }

    /// Remove `id`'s registration only if it holds `lease` and that lease is
    /// dead — every subscription and clone holding it is gone. A stale drop
    /// signal that finds a replacement under the same key, or a clone's drop
    /// while others live, is a no-op; the last holder's signal performs the
    /// removal. A dropping subscription releases its lease before signalling,
    /// so its own signal always sees it dead.
    pub(super) fn release(&self, id: K, lease: &LeaseRef, kind: &str) {
        let label = id.to_string();
        let removed = self.remove_if(id, |route| route.released(lease));
        debug!("cleanup {kind} channel {label}: removed={removed}");
    }

    /// Drop every route whose lease is dead, so a dropped subscription's
    /// sender (and anything buffered in it) is released. Same liveness rule
    /// as [`release`](Self::release).
    pub(super) fn prune_dead(&self) {
        if self.read().is_empty() {
            return;
        }
        let mut routes = self.write();
        let before = routes.len();
        routes.retain(|_, route| route.lease.is_live());
        debug!("pruned {} dead routes", before - routes.len());
    }

    /// Send `item()` to every route, then clear them all, under one write lock
    /// so a route registered meanwhile can't be cleared unnotified. A closed
    /// bounded route is skipped: its stream already ended.
    pub(super) fn fail_all(&self, item: impl Fn() -> RoutedItem) {
        let mut routes = self.write();
        for route in routes.values().filter(|route| !route.closed()) {
            let _ = route.sender.send(item());
        }
        routes.clear();
    }

    /// Clear every route without notifying. For aliases, whose owners are
    /// failed through their own registry.
    pub(super) fn clear(&self) {
        self.write().clear();
    }

    #[cfg(test)]
    pub(super) fn sender(&self, id: &K) -> Option<BroadcastSender> {
        self.with_route(id, |route| route.sender.clone())
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.read().len()
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.read().is_empty()
    }
}

/// One persistent broadcast channel per shared request type, from
/// `CHANNEL_MAPPINGS`. Each is allocated on its first subscription (tokio
/// preallocates every slot) and kept from then on; every live subscription
/// of a type reads the same channel from its own receiver.
#[derive(Debug)]
pub(super) struct SharedChannels {
    /// One entry per request type; emptied at shutdown, which closes them.
    /// A std lock, never held across an `.await`, so shutdown can run from
    /// `Drop`.
    channels: RwLock<HashMap<OutgoingMessages, SharedChannel>>,
    /// Live subscriptions per request type; see [`SharedCounts`].
    /// A tokio lock: it spans the request write.
    counts: Mutex<SharedCounts>,
}

#[derive(Debug)]
struct SharedChannel {
    responses: &'static [IncomingMessages],
    capacity: usize,
    /// Unset until the first subscription; a frame for a channel nobody has
    /// subscribed to has no receiver anyway.
    sender: OnceLock<BroadcastSender>,
}

impl SharedChannel {
    fn subscribe(&self) -> broadcast::Receiver<RoutedItem> {
        self.sender.get_or_init(|| broadcast::channel(self.capacity).0).subscribe()
    }
}

impl SharedChannels {
    /// `capacity` maps each request type to its channel's capacity.
    pub(super) fn new(capacity: impl Fn(OutgoingMessages) -> usize) -> Self {
        let channels = shared_channel_configuration::CHANNEL_MAPPINGS
            .iter()
            .map(|mapping| {
                let channel = SharedChannel {
                    responses: mapping.responses,
                    capacity: capacity(mapping.request),
                    sender: OnceLock::new(),
                };
                (mapping.request, channel)
            })
            .collect();
        Self {
            channels: RwLock::new(channels),
            counts: Mutex::new(SharedCounts::default()),
        }
    }

    /// Runs `write` for a new subscription of `message_type` and counts it on
    /// success; returns the receiver and the ticket. The count lock spans the
    /// write so the count and the wire agree. `account` is the account-updates
    /// account, checked before the write.
    pub(super) async fn subscribe<F: Future<Output = Result<(), Error>>>(
        &self,
        message_type: OutgoingMessages,
        account: Option<&AccountId>,
        write: impl FnOnce() -> F,
    ) -> Result<(broadcast::Receiver<RoutedItem>, SharedTicket), Error> {
        let receiver = self
            .channels()
            .get(&message_type)
            .map(SharedChannel::subscribe)
            .ok_or_else(|| Error::InvalidArgument(format!("No shared channel configured for message type: {message_type:?}")))?;

        let mut counts = self.counts.lock().await;
        counts.check_account_updates(account)?;
        write().await?;
        Ok((receiver, counts.subscribe(message_type, account)))
    }

    /// Uncounts `ticket`'s subscription; runs `write` (the cancel) only when
    /// `SharedCounts::unsubscribe` says so.
    pub(super) async fn unsubscribe<F: Future<Output = Result<(), Error>>>(
        &self,
        ticket: SharedTicket,
        write: impl FnOnce() -> F,
    ) -> Result<(), Error> {
        let mut counts = self.counts.lock().await;
        if counts.unsubscribe(ticket) {
            write().await
        } else {
            Ok(())
        }
    }

    fn channels(&self) -> RwLockReadGuard<'_, HashMap<OutgoingMessages, SharedChannel>> {
        self.channels.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn channels_mut(&self) -> RwLockWriteGuard<'_, HashMap<OutgoingMessages, SharedChannel>> {
        self.channels.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Every live shared subscription has just been failed: start a new
    /// generation so their later drops cannot touch the next session's counts.
    pub(super) async fn reset_counts(&self) {
        self.counts.lock().await.reset();
    }

    /// Sends `item()` once to every channel selected by `filter`; returns how
    /// many were selected.
    fn send_to(&self, filter: impl Fn(&SharedChannel) -> bool, item: impl Fn() -> RoutedItem) -> usize {
        let channels = self.channels();
        let mut selected = 0;
        for (request, channel) in channels.iter().filter(|(_, channel)| filter(channel)) {
            selected += 1;
            // Fails only with no receivers: nobody is subscribed.
            if channel.sender.get().is_none_or(|sender| sender.send(item()).is_err()) {
                trace!("no shared subscription for {request:?}");
            }
        }
        selected
    }

    /// Deliver `message` to every channel whose request maps `message_type`.
    /// Returns `false` when no channel maps it (or after shutdown).
    pub(super) fn send_message(&self, message_type: IncomingMessages, message: &ResponseMessage) -> bool {
        self.send_to(|channel| channel.responses.contains(&message_type), || message.clone().into()) > 0
    }

    /// Deliver `item()` once to every channel.
    pub(super) fn notify_all(&self, item: impl Fn() -> RoutedItem) {
        self.send_to(|_| true, item);
    }

    /// Fail in-flight one-shot requests fast by delivering an error to the
    /// one-shot channels only. Used for request-less errors, which carry no
    /// id to correlate. Streaming channels are excluded so an unrelated error
    /// can't terminate a live stream.
    pub(super) fn fail_one_shot_channels(&self, item: impl Fn() -> RoutedItem) {
        let one_shot = shared_channel_configuration::exclusive_one_shot_response_types();
        self.send_to(|channel| channel.responses.iter().any(|r| one_shot.contains(r)), item);
    }

    /// Deliver `item()` once to every channel, then drop the senders so every
    /// subscription ends after it. Later sends are no-ops.
    ///
    /// The counterpart of sync `SharedChannels::fail_all` (notify and clear
    /// under one lock); named `close` because async channels persist across
    /// subscriptions, and a closed registry refuses later subscribes.
    pub(super) fn close(&self, item: impl Fn() -> RoutedItem) {
        let mut channels = self.channels_mut();
        for sender in channels.values().filter_map(|channel| channel.sender.get()) {
            let _ = sender.send(item());
        }
        channels.clear();
    }

    #[cfg(test)]
    pub(super) async fn live(&self, message_type: OutgoingMessages) -> usize {
        self.counts.lock().await.live(message_type)
    }

    #[cfg(test)]
    pub(super) async fn account_updates(&self) -> Option<AccountId> {
        self.counts.lock().await.account_updates().cloned()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
