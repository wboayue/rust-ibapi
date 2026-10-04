//! The async bus's channel registries, the counterparts of the sync
//! `SenderHash` and `SharedChannels`. Each owns its map and lock, so routing
//! and teardown are one call per registry instead of open-coded per map.

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use log::{debug, trace};
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::accounts::types::AccountId;
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::transport::{Admit, BoundState, BufferBound, RoutedItem, SharedCounts, SharedTicket};
use crate::Error;

pub(super) type BroadcastSender = broadcast::Sender<RoutedItem>;

/// A registration: the channel, plus an unread-item cap when the request was
/// opened with `send_request_bounded`.
#[derive(Debug)]
pub(super) struct Route {
    pub(super) sender: BroadcastSender,
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
    pub(super) fn unbounded(sender: BroadcastSender) -> Self {
        Self { sender, bound: None }
    }

    /// A bounded route's channel has one slot more than `bound.limit`, for the
    /// one terminal item (end marker, error, or overflow error) that may
    /// arrive with the cap full, so sending it never evicts a queued item.
    /// `reads` is the subscription's read counter.
    pub(super) fn bounded(sender: BroadcastSender, bound: BufferBound, reads: Arc<AtomicUsize>) -> Self {
        let bound = RouteBound {
            state: BoundState::new(bound),
            sent: AtomicUsize::new(0),
            reads,
        };
        Self { sender, bound: Some(bound) }
    }

    fn closed(&self) -> bool {
        self.bound.as_ref().is_some_and(|bound| bound.state.closed())
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

    /// Registers `route` under `id`, replacing any earlier registration.
    pub(super) async fn insert(&self, id: K, route: Route) {
        self.routes.write().await.insert(id, route);
    }

    /// Run `f` on `id`'s route while holding the read lock, so no removal can
    /// land between the lookup and `f`.
    pub(super) async fn with_route<R>(&self, id: &K, f: impl FnOnce(&Route) -> R) -> Option<R> {
        self.routes.read().await.get(id).map(f)
    }

    /// Deliver `item` to `id`'s route; hands it back when nothing is
    /// registered under `id`.
    pub(super) async fn deliver(&self, id: &K, item: RoutedItem) -> Result<(), RoutedItem> {
        match self.routes.read().await.get(id) {
            Some(route) => {
                route.deliver(id, item);
                Ok(())
            }
            None => Err(item),
        }
    }

    /// Remove `id`'s registration only if it is on `sender`'s channel, so a
    /// newer registration under the same id survives.
    pub(super) async fn remove_if_same(&self, id: &K, sender: &BroadcastSender) {
        let mut routes = self.routes.write().await;
        if routes.get(id).is_some_and(|route| route.sender.same_channel(sender)) {
            routes.remove(id);
        }
    }

    /// Remove `id`'s registration only if its channel has no receivers left —
    /// i.e. every subscription and clone feeding off it is gone. A stale drop
    /// signal that finds a live replacement under the same key is a no-op; the
    /// replacement's own drop signal performs the eventual removal. The count
    /// is authoritative because a dropping subscription detaches its receivers
    /// before signalling (`AsyncInternalSubscription::detach_receivers`).
    pub(super) async fn remove_if_dead(&self, id: K, kind: &str) {
        let mut routes = self.routes.write().await;
        let removed = routes.get(&id).is_some_and(|route| route.sender.receiver_count() == 0);
        if removed {
            routes.remove(&id);
        }
        debug!("cleanup {kind} channel {id}: removed={removed}");
    }

    /// Drop every route whose channel has no receivers left, so a dropped
    /// subscription's sender (and anything buffered in it) is released. Same
    /// liveness rule as [`remove_if_dead`](Self::remove_if_dead).
    pub(super) async fn prune_dead(&self) {
        if self.routes.read().await.is_empty() {
            return;
        }
        let mut routes = self.routes.write().await;
        let before = routes.len();
        routes.retain(|_, route| route.sender.receiver_count() > 0);
        debug!("pruned {} dead routes", before - routes.len());
    }

    /// Send `item()` to every route, then clear them all, under one write lock
    /// so a route registered meanwhile can't be cleared unnotified. A closed
    /// bounded route is skipped: its stream already ended.
    pub(super) async fn fail_all(&self, item: impl Fn() -> RoutedItem) {
        let mut routes = self.routes.write().await;
        for route in routes.values().filter(|route| !route.closed()) {
            let _ = route.sender.send(item());
        }
        routes.clear();
    }

    /// Clear every route without notifying. For aliases, whose owners are
    /// failed through their own registry.
    pub(super) async fn clear(&self) {
        self.routes.write().await.clear();
    }

    #[cfg(test)]
    pub(super) async fn sender(&self, id: &K) -> Option<BroadcastSender> {
        self.with_route(id, |route| route.sender.clone()).await
    }

    #[cfg(test)]
    pub(super) async fn contains(&self, id: &K) -> bool {
        self.routes.read().await.contains_key(id)
    }

    #[cfg(test)]
    pub(super) async fn len(&self) -> usize {
        self.routes.read().await.len()
    }

    #[cfg(test)]
    pub(super) async fn is_empty(&self) -> bool {
        self.routes.read().await.is_empty()
    }

    /// Hold the write lock, to stall the cleanup task in tests.
    #[cfg(test)]
    pub(super) async fn lock(&self) -> tokio::sync::RwLockWriteGuard<'_, HashMap<K, Route>> {
        self.routes.write().await
    }
}

/// One persistent broadcast channel per shared request type, created up front
/// from `CHANNEL_MAPPINGS`. Every live subscription of a type reads the same
/// channel from its own receiver.
#[derive(Debug)]
pub(super) struct SharedChannels {
    /// One entry per request type; emptied at shutdown, which closes them.
    channels: RwLock<HashMap<OutgoingMessages, SharedChannel>>,
    /// Live subscriptions per request type; see [`SharedCounts`].
    counts: Mutex<SharedCounts>,
}

#[derive(Debug)]
struct SharedChannel {
    responses: &'static [IncomingMessages],
    sender: BroadcastSender,
}

impl SharedChannels {
    pub(super) fn new(capacity: usize) -> Self {
        let channels = shared_channel_configuration::CHANNEL_MAPPINGS
            .iter()
            .map(|mapping| {
                let channel = SharedChannel {
                    responses: mapping.responses,
                    sender: broadcast::channel(capacity).0,
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
            .channels
            .read()
            .await
            .get(&message_type)
            .map(|channel| channel.sender.subscribe())
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

    /// Every live shared subscription has just been failed: start a new
    /// generation so their later drops cannot touch the next session's counts.
    pub(super) async fn reset_counts(&self) {
        self.counts.lock().await.reset();
    }

    /// Sends `item()` once to every channel selected by `filter`; returns how
    /// many were selected.
    async fn send_to(&self, filter: impl Fn(&SharedChannel) -> bool, item: impl Fn() -> RoutedItem) -> usize {
        let channels = self.channels.read().await;
        let mut selected = 0;
        for (request, channel) in channels.iter().filter(|(_, channel)| filter(channel)) {
            selected += 1;
            // Fails only with no receivers: nobody is subscribed.
            if channel.sender.send(item()).is_err() {
                trace!("no shared subscription for {request:?}");
            }
        }
        selected
    }

    /// Deliver `message` to every channel whose request maps `message_type`.
    /// Returns `false` when no channel maps it (or after shutdown).
    pub(super) async fn send_message(&self, message_type: IncomingMessages, message: &ResponseMessage) -> bool {
        self.send_to(|channel| channel.responses.contains(&message_type), || message.clone().into())
            .await
            > 0
    }

    /// Deliver `item()` once to every channel.
    pub(super) async fn notify_all(&self, item: impl Fn() -> RoutedItem) {
        self.send_to(|_| true, item).await;
    }

    /// Fail in-flight one-shot requests fast by delivering an error to the
    /// one-shot channels only. Used for request-less errors, which carry no
    /// id to correlate. Streaming channels are excluded so an unrelated error
    /// can't terminate a live stream.
    pub(super) async fn fail_one_shot_channels(&self, item: impl Fn() -> RoutedItem) {
        let one_shot = shared_channel_configuration::exclusive_one_shot_response_types();
        self.send_to(|channel| channel.responses.iter().any(|r| one_shot.contains(r)), item).await;
    }

    /// Deliver `item()` once to every channel, then drop the senders so every
    /// subscription ends after it. Later sends are no-ops.
    pub(super) async fn close(&self, item: impl Fn() -> RoutedItem) {
        let mut channels = self.channels.write().await;
        for channel in channels.values() {
            let _ = channel.sender.send(item());
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
