//! Transport layer for TWS communication with sync/async support

// Common utilities
pub(crate) mod common;

#[cfg(feature = "sync")]
use std::sync::Arc;
#[cfg(feature = "sync")]
use std::time::Duration;

#[cfg(feature = "sync")]
use crossbeam::channel::{Receiver, Sender};

use crate::errors::Error;
use crate::messages::ResponseMessage;

#[cfg(any(feature = "sync", feature = "async"))]
use crate::messages::OutgoingMessages;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Internal channel envelope shared across sync/async transports.
#[cfg(any(feature = "sync", feature = "async"))]
pub(crate) use crate::subscriptions::common::RoutedItem;

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
#[cfg(any(feature = "sync", feature = "async"))]
#[derive(Debug, Default)]
pub(crate) struct SharedCounts {
    generation: u64,
    live: std::collections::HashMap<OutgoingMessages, usize>,
}

#[cfg(any(feature = "sync", feature = "async"))]
impl SharedCounts {
    /// Counts a new subscription of `message_type` and returns its ticket.
    pub(crate) fn subscribe(&mut self, message_type: OutgoingMessages) -> SharedTicket {
        if !crate::messages::shared_channel_configuration::is_one_shot_request(message_type) {
            *self.live.entry(message_type).or_insert(0) += 1;
        }
        SharedTicket {
            message_type,
            generation: self.generation,
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
        true
    }

    /// Starts a new session. Every subscription counted so far is dead, so
    /// zero is the truth; see the type docs for why their drops must not
    /// decrement.
    pub(crate) fn reset(&mut self) {
        self.generation += 1;
        self.live.clear();
    }

    #[cfg(test)]
    pub(crate) fn live(&self, message_type: OutgoingMessages) -> usize {
        self.live.get(&message_type).copied().unwrap_or(0)
    }
}

// MessageBus trait - defines the interface for message handling
#[cfg(feature = "sync")]
pub(crate) trait MessageBus: Send + Sync {
    fn send_request(&self, request_id: i32, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn cancel_subscription(&self, request_id: i32, packet: &[u8]) -> Result<(), Error>;

    fn send_shared_request(&self, message_id: OutgoingMessages, packet: &[u8]) -> Result<InternalSubscription, Error>;

    /// Ends one subscription of `message_id`. `packet` is the cancel to write
    /// for the last one; `None` for a stream TWS never cancels, which still
    /// releases the count.
    fn cancel_shared_subscription(&self, ticket: SharedTicket, packet: Option<&[u8]>) -> Result<(), Error>;

    fn send_order_request(&self, request_id: i32, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn send_message(&self, packet: &[u8]) -> Result<(), Error>;

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error>;

    fn cancel_order_subscription(&self, request_id: i32, packet: &[u8]) -> Result<(), Error>;

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::sync_impl::NoticeStream;

    fn ensure_shutdown(&self);

    /// Block until the session is connected again, returning
    /// [`Error::Shutdown`] if the session will never reconnect.
    fn wait_connected(&self) -> Result<(), Error>;

    fn is_connected(&self) -> bool;
}

// InternalSubscription - handles receiving messages for sync subscriptions
#[cfg(feature = "sync")]
#[derive(Debug, Default)]
pub(crate) struct InternalSubscription {
    receiver: Option<Receiver<RoutedItem>>, // requests with request ids receive responses via this channel
    sender: Option<Sender<RoutedItem>>,     // requests with request ids receive responses via this channel
    shared_receiver: Option<Arc<Receiver<RoutedItem>>>, // this channel is for responses that share channel based on message type
    signaler: Option<Sender<Signal>>,       // for client to signal termination
    pub(crate) request_id: Option<i32>,     // initiating request id
    pub(crate) order_id: Option<i32>,       // initiating order id
    pub(crate) shared: Option<SharedTicket>, // shared-channel identity, when routed by message type
}

#[cfg(feature = "sync")]
impl InternalSubscription {
    /// The underlying receiver — either the per-subscription one or the
    /// shared-channel one. Both deliver `RoutedItem`.
    fn pick_receiver(&self) -> Option<&Receiver<RoutedItem>> {
        self.receiver.as_ref().or(self.shared_receiver.as_deref())
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
        // No shared-channel signal. On drop nobody is left reading this
        // handle, and other live subscriptions of the type keep reading the
        // queue. An explicit `cancel()` on a handle another thread is blocked
        // in `next()` on leaves that thread blocked; not addressed here.
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
        // The sender is the drop signal's identity (see `Signal`); without
        // one there is nothing safe to send — better a leaked registration
        // than removing a live successor.
        let (Some(signaler), Some(sender)) = (&self.signaler, self.sender.clone()) else {
            return;
        };
        let signal = match (self.request_id, self.order_id) {
            (Some(request_id), _) => Signal::Request(request_id, sender),
            (_, Some(order_id)) => Signal::Order(order_id, sender),
            // No request or order id: the order update stream.
            _ => Signal::OrderUpdateStream(sender),
        };
        if let Err(e) = signaler.send(signal) {
            log::warn!("error sending drop signal: {e}");
        }
    }
}

// Signals are used to notify the backend when a subscriber is dropped.
// This facilitates the cleanup of the SenderHashes. Each signal carries the
// dropped subscription's data sender; cleanup removes a registration only
// when it is `same_channel` with it, so a stale signal cannot remove a newer
// registration under the same key.
#[cfg(feature = "sync")]
pub enum Signal {
    Request(i32, Sender<RoutedItem>),
    Order(i32, Sender<RoutedItem>),
    OrderUpdateStream(Sender<RoutedItem>),
}

// SubscriptionBuilder for creating InternalSubscription instances
#[cfg(feature = "sync")]
pub(crate) struct SubscriptionBuilder {
    receiver: Option<Receiver<RoutedItem>>,
    sender: Option<Sender<RoutedItem>>,
    shared_receiver: Option<Arc<Receiver<RoutedItem>>>,
    signaler: Option<Sender<Signal>>,
    order_id: Option<i32>,
    request_id: Option<i32>,
    shared: Option<SharedTicket>,
}

#[cfg(feature = "sync")]
impl SubscriptionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            shared_receiver: None,
            signaler: None,
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

    pub(crate) fn shared_receiver(mut self, receiver: Arc<Receiver<RoutedItem>>) -> Self {
        self.shared_receiver = Some(receiver);
        self
    }

    pub(crate) fn signaler(mut self, signaler: Sender<Signal>) -> Self {
        self.signaler = Some(signaler);
        self
    }

    pub(crate) fn order_id(mut self, order_id: i32) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub(crate) fn request_id(mut self, request_id: i32) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub(crate) fn shared(mut self, ticket: SharedTicket) -> Self {
        self.shared = Some(ticket);
        self
    }

    pub(crate) fn build(self) -> InternalSubscription {
        if let (Some(receiver), Some(signaler)) = (self.receiver, self.signaler) {
            InternalSubscription {
                receiver: Some(receiver),
                sender: self.sender,
                shared_receiver: None,
                signaler: Some(signaler),
                request_id: self.request_id,
                order_id: self.order_id,
                shared: self.shared,
            }
        } else if let Some(receiver) = self.shared_receiver {
            InternalSubscription {
                receiver: None,
                sender: None,
                shared_receiver: Some(receiver),
                signaler: None,
                request_id: self.request_id,
                order_id: self.order_id,
                shared: self.shared,
            }
        } else {
            panic!("bad configuration");
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
