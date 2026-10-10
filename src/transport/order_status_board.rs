//! Per-order `OrderStatus` fan-out for `wait_for_fill` (#951).
//!
//! The dispatcher publishes every `OrderStatus` frame here *in addition to*
//! its normal routing, so a waiter never takes a frame from a `place_order`
//! subscription, the order-update stream or a shared channel. The board also
//! keeps each order's latest frame: a waiter registers and reads it under one
//! lock, so a fill that landed before the wait began is still seen.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::client::ids::OrderId;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::transport::RoutedItem;
use crate::Error;

/// The sending half of a waiter's channel; one impl per runtime.
pub(crate) trait WatchSender: Send + Sized {
    type Receiver;

    /// A new unbounded waiter channel.
    fn channel() -> (Self, Self::Receiver);

    /// Queue `item`; `false` when the waiter is gone.
    fn send_item(&self, item: RoutedItem) -> bool;
}

#[cfg(feature = "sync")]
impl WatchSender for crossbeam::channel::Sender<RoutedItem> {
    type Receiver = crossbeam::channel::Receiver<RoutedItem>;

    fn channel() -> (Self, Self::Receiver) {
        crossbeam::channel::unbounded()
    }

    fn send_item(&self, item: RoutedItem) -> bool {
        self.send(item).is_ok()
    }
}

#[cfg(feature = "async")]
impl WatchSender for tokio::sync::mpsc::UnboundedSender<RoutedItem> {
    type Receiver = tokio::sync::mpsc::UnboundedReceiver<RoutedItem>;

    fn channel() -> (Self, Self::Receiver) {
        tokio::sync::mpsc::unbounded_channel()
    }

    fn send_item(&self, item: RoutedItem) -> bool {
        self.send(item).is_ok()
    }
}

/// A sync waiter.
#[cfg(feature = "sync")]
pub(crate) type SyncOrderStatusWatch = OrderStatusWatch<crossbeam::channel::Sender<RoutedItem>>;

/// An async waiter.
#[cfg(feature = "async")]
pub(crate) type AsyncOrderStatusWatch = OrderStatusWatch<tokio::sync::mpsc::UnboundedSender<RoutedItem>>;

struct Slot<S> {
    latest: Option<ResponseMessage>,
    watchers: Vec<(u64, S)>,
}

impl<S> Default for Slot<S> {
    fn default() -> Self {
        Self {
            latest: None,
            watchers: Vec::new(),
        }
    }
}

struct State<S> {
    slots: HashMap<OrderId, Slot<S>>,
    next_watch: u64,
    /// Set at shutdown; no waiter registers after it.
    closed: bool,
}

/// Latest `OrderStatus` per order id, and the waiters on each.
///
/// The latest frames are kept for the session, one per order id, and survive
/// a reconnect: the order still lives in TWS.
pub(crate) struct OrderStatusBoard<S> {
    state: Mutex<State<S>>,
}

impl<S> std::fmt::Debug for OrderStatusBoard<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderStatusBoard").finish_non_exhaustive()
    }
}

impl<S: WatchSender> Default for OrderStatusBoard<S> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                slots: HashMap::new(),
                next_watch: 0,
                closed: false,
            }),
        }
    }
}

impl<S: WatchSender> OrderStatusBoard<S> {
    /// Lock the state, recovering from poisoning: every update leaves it whole.
    fn lock(&self) -> MutexGuard<'_, State<S>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Record an `OrderStatus` frame as its order's latest status and copy it
    /// to each waiter; any other frame is ignored. Waiters that have gone are
    /// dropped.
    pub(crate) fn publish(&self, message: &ResponseMessage) {
        if message.message_type() != IncomingMessages::OrderStatus {
            return;
        }
        let Some(order_id) = message.order_id().map(OrderId::from) else {
            return;
        };
        let mut state = self.lock();
        let slot = state.slots.entry(order_id).or_default();
        slot.watchers.retain(|(_, sender)| sender.send_item(message.clone().into()));
        slot.latest = Some(message.clone());
    }

    /// Register a waiter on `order_id`. The order's latest status, if any, is
    /// queued first, under the same lock as the registration, so no frame
    /// falls between the two. Refused with [`Error::Shutdown`] once the board is closed.
    pub(crate) fn watch(self: &Arc<Self>, order_id: OrderId) -> Result<OrderStatusWatch<S>, Error> {
        let mut state = self.lock();
        if state.closed {
            return Err(Error::Shutdown);
        }
        let id = state.next_watch;
        state.next_watch += 1;
        let (sender, receiver) = S::channel();
        let slot = state.slots.entry(order_id).or_default();
        if let Some(latest) = &slot.latest {
            sender.send_item(latest.clone().into());
        }
        slot.watchers.push((id, sender));
        Ok(OrderStatusWatch {
            receiver,
            _registration: Some(WatchRegistration {
                board: Arc::clone(self),
                order_id,
                id,
            }),
            _sender: None,
        })
    }

    fn unwatch(&self, order_id: OrderId, id: u64) {
        let mut state = self.lock();
        if let Some(slot) = state.slots.get_mut(&order_id) {
            slot.watchers.retain(|(watch, _)| *watch != id);
        }
    }

    /// End every waiter with [`Error::ConnectionReset`]. Latest statuses are
    /// kept, and new waiters may register.
    pub(crate) fn reset(&self) {
        Self::fail_watchers(&mut self.lock(), || Error::ConnectionReset);
    }

    /// [`reset`](Self::reset) with [`Error::Shutdown`], and refuse
    /// every later [`watch`](Self::watch). One lock, so no waiter registers
    /// between the two.
    pub(crate) fn close(&self) {
        let mut state = self.lock();
        state.closed = true;
        Self::fail_watchers(&mut state, || Error::Shutdown);
    }

    fn fail_watchers(state: &mut State<S>, error: impl Fn() -> Error) {
        for slot in state.slots.values_mut() {
            for (_, sender) in slot.watchers.drain(..) {
                sender.send_item(RoutedItem::Error(error()));
            }
        }
    }
}

/// One waiter: the channel its order's statuses arrive on, starting with the
/// latest one from before the wait. Dropping it deregisters the waiter.
pub(crate) struct OrderStatusWatch<S: WatchSender> {
    pub(crate) receiver: S::Receiver,
    _registration: Option<WatchRegistration<S>>,
    /// Keeps a detached watch's channel open.
    _sender: Option<S>,
}

impl<S: WatchSender> OrderStatusWatch<S> {
    /// A watch on no board, fed through `sender` by a test bus.
    #[cfg(test)]
    pub(crate) fn detached(sender: S, receiver: S::Receiver) -> Self {
        Self {
            receiver,
            _registration: None,
            _sender: Some(sender),
        }
    }
}

/// A waiter's place on the board; dropping it deregisters the waiter.
struct WatchRegistration<S: WatchSender> {
    board: Arc<OrderStatusBoard<S>>,
    order_id: OrderId,
    id: u64,
}

impl<S: WatchSender> Drop for WatchRegistration<S> {
    fn drop(&mut self) {
        self.board.unwatch(self.order_id, self.id);
    }
}

#[cfg(all(test, feature = "sync"))]
#[path = "order_status_board_tests.rs"]
mod tests;
