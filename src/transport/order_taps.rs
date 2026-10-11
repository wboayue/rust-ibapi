//! Side channels on the order frames the dispatcher routes: the client-wide
//! order-update stream, and per-order `OrderStatus` streams (#951).
//!
//! Both are copies taken *in addition to* normal routing, so neither takes a
//! frame from a `place_order` subscription or a shared channel. One type owns
//! both, so each bus has one field to publish to, reset, and close.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard};

use log::warn;

use super::common::{lock, Lease, LeaseRef};
use crate::client::ids::OrderId;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::transport::RoutedItem;
use crate::Error;

/// How many orders' latest statuses are kept for new status streams, most
/// recently first seen. Older ones are dropped, so a stream opened on an
/// order that finished long ago starts empty.
pub(crate) const LATEST_STATUS_CAPACITY: usize = 10_000;

/// The sending half of a tap's channel; one impl per runtime.
pub(crate) trait TapSender: Clone + Send {
    type Receiver;

    /// A new channel. The sync channel is unbounded and ignores `capacity`.
    fn channel(capacity: usize) -> (Self, Self::Receiver);

    /// Queue `item`; `false` when the receiver is gone. `label` names the
    /// stream in a backlog warning.
    fn send_item(&self, item: RoutedItem, label: &str) -> bool;
}

#[cfg(feature = "sync")]
impl TapSender for crossbeam::channel::Sender<RoutedItem> {
    type Receiver = crossbeam::channel::Receiver<RoutedItem>;

    fn channel(_capacity: usize) -> (Self, Self::Receiver) {
        crossbeam::channel::unbounded()
    }

    fn send_item(&self, item: RoutedItem, label: &str) -> bool {
        let sent = self.send(item).is_ok();
        if sent {
            super::sync::warn_if_backlogged(format_args!("{label}"), self.len());
        }
        sent
    }
}

#[cfg(feature = "async")]
impl TapSender for tokio::sync::broadcast::Sender<RoutedItem> {
    type Receiver = tokio::sync::broadcast::Receiver<RoutedItem>;

    fn channel(capacity: usize) -> (Self, Self::Receiver) {
        tokio::sync::broadcast::channel(capacity)
    }

    fn send_item(&self, item: RoutedItem, _label: &str) -> bool {
        self.send(item).is_ok()
    }
}

const UPDATES: &str = "order update stream";
const STATUS: &str = "order status stream";

/// A new tap, for the subscription to hold.
pub(crate) struct NewTap<S: TapSender> {
    /// A sender clone; sync uses it for the subscription's cancel notification.
    #[cfg_attr(not(feature = "sync"), allow(dead_code))]
    pub(crate) sender: S,
    pub(crate) receiver: S::Receiver,
    /// Identifies the subscription to its cleanup signal.
    pub(crate) lease: Lease,
}

struct Tap<S> {
    sender: S,
    lease: LeaseRef,
}

impl<S: TapSender> Tap<S> {
    fn open(capacity: usize) -> (Self, NewTap<S>) {
        let (sender, receiver) = S::channel(capacity);
        let lease = Lease::new();
        let tap = Tap {
            sender: sender.clone(),
            lease: lease.downgrade(),
        };
        (tap, NewTap { sender, receiver, lease })
    }

    /// Whether `lease` names this tap's subscription and it has gone.
    fn released(&self, lease: &LeaseRef) -> bool {
        self.lease.is(lease) && !self.lease.is_live()
    }
}

struct StatusSlot<S> {
    latest: Option<ResponseMessage>,
    streams: Vec<Tap<S>>,
}

impl<S> Default for StatusSlot<S> {
    fn default() -> Self {
        Self {
            latest: None,
            streams: Vec::new(),
        }
    }
}

struct State<S> {
    updates: Option<Tap<S>>,
    statuses: HashMap<OrderId, StatusSlot<S>>,
    /// The orders holding a latest status, oldest first; at most
    /// [`LATEST_STATUS_CAPACITY`].
    recent: VecDeque<OrderId>,
    /// Set at shutdown; nothing subscribes after it.
    closed: bool,
}

/// The order-update stream and the per-order status streams of one bus.
pub(crate) struct OrderTaps<S> {
    state: Mutex<State<S>>,
}

impl<S> std::fmt::Debug for OrderTaps<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderTaps").finish_non_exhaustive()
    }
}

impl<S> Default for OrderTaps<S> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                updates: None,
                statuses: HashMap::new(),
                recent: VecDeque::new(),
                closed: false,
            }),
        }
    }
}

impl<S: TapSender> OrderTaps<S> {
    /// Lock the state, recovering from poisoning: every update leaves it whole.
    fn lock(&self) -> MutexGuard<'_, State<S>> {
        lock(&self.state)
    }

    /// Open the order-update stream. Refused with [`Error::AlreadySubscribed`]
    /// while one is live, and [`Error::Shutdown`] once closed.
    ///
    /// A registration with a dead lease is a cancelled or dropped stream whose
    /// cleanup signal has not been processed yet; it is replaced rather than
    /// refused, so drop-then-recreate never races the cleanup. Its stale
    /// signal then finds another lease and leaves the replacement alone.
    pub(crate) fn subscribe_updates(&self, capacity: usize) -> Result<NewTap<S>, Error> {
        let mut state = self.lock();
        if state.closed {
            return Err(Error::Shutdown);
        }
        if state.updates.as_ref().is_some_and(|tap| tap.lease.is_live()) {
            return Err(Error::AlreadySubscribed);
        }
        let (tap, new) = Tap::<S>::open(capacity);
        state.updates = Some(tap);
        Ok(new)
    }

    /// Copy `item` to the order-update stream; `false` when there is none.
    pub(crate) fn send_update(&self, item: RoutedItem) -> bool {
        let state = self.lock();
        let Some(tap) = state.updates.as_ref() else {
            return false;
        };
        let sent = tap.sender.send_item(item, UPDATES);
        if !sent {
            warn!("order update stream closed before its cleanup");
        }
        sent
    }

    /// Remove the order-update stream if `lease` names it and it has gone.
    pub(crate) fn release_updates(&self, lease: &LeaseRef) -> bool {
        let mut state = self.lock();
        let released = state.updates.as_ref().is_some_and(|tap| tap.released(lease));
        if released {
            state.updates = None;
        }
        released
    }

    /// Open a status stream on `order_id`. The order's latest status, if any,
    /// is queued first, under the same lock as the registration, so no frame
    /// falls between the two. Refused with [`Error::Shutdown`] once closed.
    pub(crate) fn subscribe_status(&self, order_id: OrderId, capacity: usize) -> Result<NewTap<S>, Error> {
        let mut state = self.lock();
        if state.closed {
            return Err(Error::Shutdown);
        }
        let (tap, new) = Tap::<S>::open(capacity);
        let slot = state.statuses.entry(order_id).or_default();
        if let Some(latest) = &slot.latest {
            tap.sender.send_item(latest.clone().into(), STATUS);
        }
        slot.streams.push(tap);
        Ok(new)
    }

    /// Remove `order_id`'s status stream that `lease` names, once it has gone,
    /// and the order's entry when nothing is left in it.
    pub(crate) fn release_status(&self, order_id: OrderId, lease: &LeaseRef) {
        let mut state = self.lock();
        let Some(slot) = state.statuses.get_mut(&order_id) else {
            return;
        };
        slot.streams.retain(|tap| !tap.released(lease));
        if slot.streams.is_empty() && slot.latest.is_none() {
            state.statuses.remove(&order_id);
        }
    }

    /// Record an `OrderStatus` frame as its order's latest status and copy it
    /// to the order's status streams; any other frame is ignored. Streams
    /// whose receiver has gone are dropped.
    pub(crate) fn publish_status(&self, message: &ResponseMessage) {
        if message.message_type() != IncomingMessages::OrderStatus {
            return;
        }
        let Some(order_id) = message.order_id().map(OrderId::from) else {
            return;
        };
        let mut state = self.lock();
        let slot = state.statuses.entry(order_id).or_default();
        slot.streams.retain(|tap| tap.sender.send_item(message.clone().into(), STATUS));
        if slot.latest.replace(message.clone()).is_none() {
            state.recent.push_back(order_id);
            Self::evict_oldest(&mut state);
        }
    }

    /// Drop the oldest latest statuses past [`LATEST_STATUS_CAPACITY`].
    fn evict_oldest(state: &mut State<S>) {
        while state.recent.len() > LATEST_STATUS_CAPACITY {
            let Some(order_id) = state.recent.pop_front() else {
                return;
            };
            if let Some(slot) = state.statuses.get_mut(&order_id) {
                slot.latest = None;
                if slot.streams.is_empty() {
                    state.statuses.remove(&order_id);
                }
            }
        }
    }

    /// End every status stream with [`Error::ConnectionReset`]. The
    /// order-update stream and the latest statuses survive a reconnect: the
    /// orders still live in TWS.
    pub(crate) fn reset(&self) {
        Self::end_status_streams(&mut self.lock(), || Error::ConnectionReset);
    }

    /// End every stream with [`Error::Shutdown`] and refuse later subscribes.
    /// One lock, so nothing subscribes in between.
    pub(crate) fn close(&self) {
        let mut state = self.lock();
        state.closed = true;
        // The subscription holds a sender clone, so only a sent item ends it.
        if let Some(tap) = state.updates.take() {
            tap.sender.send_item(Error::Shutdown.into(), UPDATES);
        }
        Self::end_status_streams(&mut state, || Error::Shutdown);
    }

    fn end_status_streams(state: &mut State<S>, error: impl Fn() -> Error) {
        for slot in state.statuses.values_mut() {
            for tap in slot.streams.drain(..) {
                tap.sender.send_item(error().into(), STATUS);
            }
        }
        state.statuses.retain(|_, slot| slot.latest.is_some());
    }
}

#[cfg(test)]
impl<S: TapSender> OrderTaps<S> {
    /// The order-update stream's lease, if one is registered.
    pub(crate) fn updates_lease(&self) -> Option<LeaseRef> {
        self.lock().updates.as_ref().map(|tap| tap.lease.clone())
    }

    /// A clone of the order-update stream's sender, if one is registered.
    #[cfg(feature = "async")]
    pub(crate) fn updates_sender(&self) -> Option<S> {
        self.lock().updates.as_ref().map(|tap| tap.sender.clone())
    }

    /// How many status streams are open on `order_id`.
    #[cfg(feature = "sync")]
    pub(crate) fn status_streams(&self, order_id: OrderId) -> usize {
        self.lock().statuses.get(&order_id).map_or(0, |slot| slot.streams.len())
    }

    /// Whether `order_id` has an entry at all.
    pub(crate) fn has_status_entry(&self, order_id: OrderId) -> bool {
        self.lock().statuses.contains_key(&order_id)
    }

    /// Poison the state lock, as a panic while holding it would.
    #[cfg(feature = "sync")]
    pub(crate) fn poison(&self) {
        super::common::poison_with(|| self.state.lock().unwrap());
        assert!(self.state.is_poisoned());
    }
}

#[cfg(all(test, feature = "sync"))]
#[path = "order_taps_tests.rs"]
mod tests;
