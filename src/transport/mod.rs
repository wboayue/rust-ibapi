//! Transport layer for TWS communication with sync/async support

// Common utilities
pub(crate) mod common;

#[cfg(feature = "sync")]
use std::{sync::Mutex, time::Duration};

#[cfg(feature = "sync")]
use crossbeam::channel::{Receiver, Sender};

#[cfg(feature = "sync")]
use common::{Lease, LeaseRef};

#[cfg(feature = "sync")]
use crate::client::ids::{OrderId, RequestId};
use crate::errors::Error;
use crate::messages::ResponseMessage;

#[cfg(any(feature = "sync", feature = "async"))]
use crate::accounts::types::AccountId;
#[cfg(any(feature = "sync", feature = "async"))]
use crate::messages::OutgoingMessages;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Internal channel envelope shared across sync/async transports.
#[cfg(any(feature = "sync", feature = "async"))]
pub(crate) use crate::subscriptions::common::RoutedItem;

/// A request route's unread-item cap (`buffer_limit`), opened with
/// `send_request_bounded`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferBound {
    /// The most unread items the route queues.
    pub limit: usize,
    /// The request's end marker. It always gets through, like an error, so a
    /// result that fills the cap exactly still ends normally.
    pub end: crate::messages::IncomingMessages,
}

/// What a bounded route does with the next item.
#[derive(Debug, PartialEq)]
pub(crate) enum Admit {
    /// Queue the item.
    Deliver,
    /// Queue `Error::BufferLimitExceeded` instead; the route is now closed.
    Overflow,
    /// Drop the item: the route already delivered a terminal item.
    Discard,
}

/// The shared bookkeeping for a bounded route. Each transport supplies its
/// own count of unread items.
#[derive(Debug)]
pub(crate) struct BoundState {
    bound: BufferBound,
    /// Set once a terminal item (end marker, error, or the overflow error)
    /// was queued: the stream has ended, so later items are discarded and
    /// at most one item ever uses the slot past the cap.
    closed: std::sync::atomic::AtomicBool,
}

impl BoundState {
    pub(crate) fn new(bound: BufferBound) -> Self {
        Self {
            bound,
            closed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub(crate) fn closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Decide `item`'s fate with `unread` items queued. End markers and
    /// errors always get through; anything else past the cap overflows.
    pub(crate) fn admit(&self, item: &RoutedItem, unread: usize) -> Admit {
        use std::sync::atomic::Ordering;
        if self.closed() {
            return Admit::Discard;
        }
        let terminal = match item {
            RoutedItem::Error(_) => true,
            RoutedItem::Response(message) => message.message_type() == self.bound.end,
            RoutedItem::Notice(_) => false,
        };
        if terminal {
            self.closed.store(true, Ordering::Relaxed);
            Admit::Deliver
        } else if unread >= self.bound.limit {
            self.closed.store(true, Ordering::Relaxed);
            Admit::Overflow
        } else {
            Admit::Deliver
        }
    }

    pub(crate) fn overflow_error(&self) -> RoutedItem {
        Error::BufferLimitExceeded { limit: self.bound.limit }.into()
    }
}

// Result type for the connection read path (parse / I/O outcome).
#[allow(dead_code)]
pub(crate) type Response = Result<ResponseMessage, Error>;

/// One live shared-channel subscription: its request type and the session
/// generation it was made in. Handed back to `cancel_shared_subscription`
/// so a handle that outlived its session cannot touch the next one's count.
#[cfg(any(feature = "sync", feature = "async"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SharedTicket {
    pub(crate) message_type: OutgoingMessages,
    pub(crate) generation: u64,
}

/// Live streaming subscriptions per shared request type, scoped to a session.
///
/// TWS keeps one subscription per type (`CancelPositions` carries no id), so
/// the cancel goes out only when the last one ends. The bus holds the lock
/// on this across the request/cancel write so the count and the wire agree.
/// One-shot requests are not counted: they never cancel, and a count that is
/// never decremented would withhold every later cancel for that type.
///
/// A reconnect ends every subscription at once (each reads
/// `Error::ConnectionReset`), but the handles are dropped later, one by one,
/// possibly after the same type was resubscribed on the new session. Each
/// reset therefore starts a new generation with empty counts, and a ticket
/// from an earlier generation neither decrements nor cancels.
///
/// `RequestAccountData` also has one account: a request for another account
/// would switch every live subscription to it, so it is refused until the
/// last subscription of the first account ends.
#[cfg(any(feature = "sync", feature = "async"))]
#[derive(Debug, Default)]
pub(crate) struct SharedCounts {
    generation: u64,
    live: std::collections::HashMap<OutgoingMessages, usize>,
    // The account of the live `RequestAccountData` subscriptions; `Some`
    // exactly while any is live.
    account_updates: Option<AccountId>,
}

#[cfg(any(feature = "sync", feature = "async"))]
impl SharedCounts {
    /// Counts a new subscription of `message_type` and returns its ticket.
    /// `account` is an account-updates subscription's account, which
    /// `check_account_updates` admitted.
    pub(crate) fn subscribe(&mut self, message_type: OutgoingMessages, account: Option<&AccountId>) -> SharedTicket {
        if let Some(account) = account {
            self.account_updates = Some(account.clone());
        }
        if !crate::messages::shared_channel_configuration::is_one_shot_request(message_type) {
            *self.live.entry(message_type).or_insert(0) += 1;
        }
        SharedTicket {
            message_type,
            generation: self.generation,
        }
    }

    /// `Err` when `account` is given and account updates for another account
    /// are live; call under the lock, before writing the request.
    pub(crate) fn check_account_updates(&self, account: Option<&AccountId>) -> Result<(), Error> {
        match (&self.account_updates, account) {
            (Some(active), Some(account)) if active != account => Err(Error::AccountUpdatesInUse {
                active: active.clone(),
                requested: account.clone(),
            }),
            _ => Ok(()),
        }
    }

    /// Uncounts `ticket`'s subscription. `true` when the cancel should be
    /// written: the ticket is from this session and no other subscription of
    /// the type is live. A ticket with no counted subscription (one
    /// fabricated in tests) still writes.
    pub(crate) fn unsubscribe(&mut self, ticket: SharedTicket) -> bool {
        if ticket.generation != self.generation {
            log::debug!("shared subscription {:?} outlived its session: cancel skipped", ticket.message_type);
            return false;
        }
        let count = self.live.entry(ticket.message_type).or_insert(0);
        *count = count.saturating_sub(1);
        if *count > 0 {
            log::debug!("shared subscription {:?} ended, {count} still live: cancel withheld", ticket.message_type);
            return false;
        }
        if ticket.message_type == OutgoingMessages::RequestAccountData {
            self.account_updates = None;
        }
        true
    }

    /// Starts a new session. Every subscription counted so far is dead, so
    /// zero is the truth; see the type docs for why their drops must not
    /// decrement.
    pub(crate) fn reset(&mut self) {
        self.generation += 1;
        self.live.clear();
        self.account_updates = None;
    }

    #[cfg(test)]
    pub(crate) fn live(&self, message_type: OutgoingMessages) -> usize {
        self.live.get(&message_type).copied().unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn account_updates(&self) -> Option<&AccountId> {
        self.account_updates.as_ref()
    }
}

// MessageBus trait - defines the interface for message handling
#[cfg(feature = "sync")]
pub(crate) trait MessageBus: Send + Sync {
    fn send_request(&self, request_id: RequestId, packet: &[u8]) -> Result<InternalSubscription, Error>;

    /// [`send_request`](Self::send_request) with a cap on unread items: see
    /// [`BoundState::admit`]. Past the cap the route queues
    /// `Error::BufferLimitExceeded` and discards later frames.
    fn send_request_bounded(&self, request_id: RequestId, packet: &[u8], bound: BufferBound) -> Result<InternalSubscription, Error>;

    fn send_shared_request(&self, message_id: OutgoingMessages, packet: &[u8]) -> Result<InternalSubscription, Error>;

    /// `send_shared_request` for `RequestAccountData`, refused with
    /// `Error::AccountUpdatesInUse` while another account is live.
    fn send_account_updates_request(&self, account: &AccountId, packet: &[u8]) -> Result<InternalSubscription, Error>;

    /// Ends one subscription of `message_id`. `packet` is the cancel to write
    /// for the last one; `None` for a stream TWS never cancels, which still
    /// releases the count.
    fn cancel_shared_subscription(&self, ticket: SharedTicket, packet: Option<&[u8]>) -> Result<(), Error>;

    fn send_order_request(&self, order_id: OrderId, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn send_message(&self, packet: &[u8]) -> Result<(), Error>;

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error>;

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::sync_impl::NoticeStream;

    fn ensure_shutdown(&self);

    /// Block until the session is connected again, returning
    /// [`Error::Shutdown`] if the session will never reconnect.
    fn wait_connected(&self) -> Result<(), Error>;

    fn is_connected(&self) -> bool;
}

// InternalSubscription - handles receiving messages for sync subscriptions
#[cfg(feature = "sync")]
#[derive(Debug)]
pub(crate) struct InternalSubscription {
    receiver: Option<Receiver<RoutedItem>>,   // this subscription's own queue
    sender: Option<Sender<RoutedItem>>,       // feeds `receiver`, for the cancel notification
    signaler: Sender<Signal>,                 // for client to signal termination
    lease: Mutex<Option<Lease>>,              // the registration's liveness; released at cancel or drop
    pub(crate) request_id: Option<RequestId>, // initiating request id
    pub(crate) order_id: Option<OrderId>,     // initiating order id
    pub(crate) shared: Option<SharedTicket>,  // shared-channel identity, when routed by message type
}

#[cfg(feature = "sync")]
impl InternalSubscription {
    fn pick_receiver(&self) -> Option<&Receiver<RoutedItem>> {
        self.receiver.as_ref()
    }

    /// Blocks until next message become available.
    pub(crate) fn next(&self) -> Option<Response> {
        Self::receive(self.pick_receiver()?)
    }

    /// Returns message if available or immediately returns None.
    ///
    /// Test-only since the `SubscriptionItem` migration retired the last
    /// production caller; transport tests still drive the bus through the
    /// legacy projection. Mirrors the `#[cfg(test)]` gating on
    /// [`try_next_routed`](Self::try_next_routed).
    #[cfg(test)]
    pub(crate) fn try_next(&self) -> Option<Response> {
        Self::try_receive(self.pick_receiver()?)
    }

    /// Waits for next message until specified timeout. Test-only — see
    /// [`try_next`](Self::try_next).
    #[cfg(test)]
    pub(crate) fn next_timeout(&self, timeout: Duration) -> Option<Response> {
        Self::timeout_receive(self.pick_receiver()?, timeout)
    }

    /// Blocks until the next RoutedItem is available, exposing the typed
    /// dispatcher envelope (Response / Notice / Error) without the legacy
    /// ResponseMessage/Error projection.
    pub(crate) fn next_routed(&self) -> Option<RoutedItem> {
        self.pick_receiver()?.recv().ok()
    }

    /// Non-blocking variant of [`next_routed`](Self::next_routed). Returns
    /// `None` if no `RoutedItem` is queued right now.
    pub(crate) fn try_next_routed(&self) -> Option<RoutedItem> {
        self.pick_receiver()?.try_recv().ok()
    }

    /// Bounded-wait variant of [`next_routed`](Self::next_routed). Returns
    /// `None` if no `RoutedItem` arrives within `timeout`.
    pub(crate) fn next_timeout_routed(&self, timeout: Duration) -> Option<RoutedItem> {
        self.pick_receiver()?.recv_timeout(timeout).ok()
    }

    pub(crate) fn cancel(&self) {
        if let Some(sender) = &self.sender {
            if let Err(e) = sender.send(Error::Cancelled.into()) {
                log::warn!("error sending cancel notification: {e}")
            }
        }
        // A cancelled subscription is unregistered by the cleanup thread once
        // it processes this signal, rather than at the handle's drop: a handle
        // kept after `cancel()` must not go on collecting frames. Frames
        // dispatched before the signal is processed still land on the queue,
        // behind `Cancelled`. The signal carries this lease, so cleanup
        // removes only this subscription's registration, never a newer one
        // under the same id. The lease is released here, so the later drop
        // sends nothing.
        self.release("cancel");
    }

    /// Release the lease and send the cleanup signal, once per subscription:
    /// whichever of cancel and drop comes first. Released before the signal is
    /// sent, so the registration reads as dead from here on:
    /// `create_order_update_subscription` can replace it before the cleanup
    /// thread runs. Cleanup itself matches identity only (see `Signal`).
    fn release(&self, cause: &str) {
        let lease = self.lease.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        let Some(lease) = lease else {
            return;
        };
        let lease_ref = lease.downgrade();
        drop(lease);
        if let Err(e) = self.signaler.send(self.signal(lease_ref)) {
            log::warn!("error sending {cause} signal: {e}");
        }
    }

    /// The cleanup signal for this subscription, identified by `lease`.
    fn signal(&self, lease: LeaseRef) -> Signal {
        match (self.request_id, self.order_id, self.shared) {
            (Some(request_id), _, _) => Signal::Request(request_id, lease),
            (_, Some(order_id), _) => Signal::Order(order_id, lease),
            (_, _, Some(_)) => Signal::Shared(lease),
            // No request, order id or shared ticket: the order update stream.
            _ => Signal::OrderUpdateStream(lease),
        }
    }

    fn receive(receiver: &Receiver<RoutedItem>) -> Option<Response> {
        loop {
            if let Some(legacy) = receiver.recv().ok()?.into_legacy() {
                return Some(legacy);
            }
        }
    }

    #[cfg(test)]
    fn try_receive(receiver: &Receiver<RoutedItem>) -> Option<Response> {
        loop {
            if let Some(legacy) = receiver.try_recv().ok()?.into_legacy() {
                return Some(legacy);
            }
        }
    }

    #[cfg(test)]
    fn timeout_receive(receiver: &Receiver<RoutedItem>, timeout: Duration) -> Option<Response> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if let Some(legacy) = receiver.recv_timeout(remaining).ok()?.into_legacy() {
                return Some(legacy);
            }
        }
    }
}

#[cfg(feature = "sync")]
impl Drop for InternalSubscription {
    fn drop(&mut self) {
        self.release("drop");
    }
}

// Signals are used to notify the backend when a subscriber is cancelled or
// dropped. This facilitates the cleanup of the SenderHashes. Each signal
// carries the subscription's lease; cleanup removes a registration only when
// it holds that lease, so a stale signal cannot remove a newer registration
// under the same key.
#[cfg(feature = "sync")]
pub(crate) enum Signal {
    Request(RequestId, LeaseRef),
    Order(OrderId, LeaseRef),
    OrderUpdateStream(LeaseRef),
    Shared(LeaseRef),
}

// SubscriptionBuilder for creating InternalSubscription instances
#[cfg(feature = "sync")]
pub(crate) struct SubscriptionBuilder {
    receiver: Option<Receiver<RoutedItem>>,
    sender: Option<Sender<RoutedItem>>,
    signaler: Option<Sender<Signal>>,
    lease: Option<Lease>,
    order_id: Option<OrderId>,
    request_id: Option<RequestId>,
    shared: Option<SharedTicket>,
}

#[cfg(feature = "sync")]
impl SubscriptionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            signaler: None,
            lease: None,
            order_id: None,
            request_id: None,
            shared: None,
        }
    }

    pub(crate) fn receiver(mut self, receiver: Receiver<RoutedItem>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub(crate) fn sender(mut self, sender: Sender<RoutedItem>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub(crate) fn signaler(mut self, signaler: Sender<Signal>) -> Self {
        self.signaler = Some(signaler);
        self
    }

    pub(crate) fn lease(mut self, lease: Lease) -> Self {
        self.lease = Some(lease);
        self
    }

    pub(crate) fn order_id(mut self, order_id: OrderId) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub(crate) fn request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub(crate) fn shared(mut self, ticket: SharedTicket) -> Self {
        self.shared = Some(ticket);
        self
    }

    pub(crate) fn build(self) -> InternalSubscription {
        let (Some(receiver), Some(signaler), Some(lease)) = (self.receiver, self.signaler, self.lease) else {
            panic!("bad configuration");
        };
        InternalSubscription {
            receiver: Some(receiver),
            sender: self.sender,
            signaler,
            lease: Mutex::new(Some(lease)),
            request_id: self.request_id,
            order_id: self.order_id,
            shared: self.shared,
        }
    }
}

// Sync exports
#[cfg(feature = "sync")]
pub use sync::TcpMessageBus;

// Async exports (placeholder for now)
#[cfg(feature = "async")]
pub use r#async::{AsyncInternalSubscription, AsyncMessageBus};

pub mod connection;
pub(crate) mod raw_capture;
pub mod recorder;
pub mod routing;

#[cfg(all(test, any(feature = "sync", feature = "async")))]
mod tests;
