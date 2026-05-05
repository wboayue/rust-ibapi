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

#[cfg(feature = "sync")]
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

// MessageBus trait - defines the interface for message handling
#[cfg(feature = "sync")]
pub(crate) trait MessageBus: Send + Sync {
    fn send_request(&self, request_id: i32, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn cancel_subscription(&self, request_id: i32, packet: &[u8]) -> Result<(), Error>;

    fn send_shared_request(&self, message_id: OutgoingMessages, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn cancel_shared_subscription(&self, message_id: OutgoingMessages, packet: &[u8]) -> Result<(), Error>;

    fn send_order_request(&self, request_id: i32, packet: &[u8]) -> Result<InternalSubscription, Error>;

    fn send_message(&self, packet: &[u8]) -> Result<(), Error>;

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error>;

    fn cancel_order_subscription(&self, request_id: i32, packet: &[u8]) -> Result<(), Error>;

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::sync_impl::NoticeStream;

    fn ensure_shutdown(&self);

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
    pub(crate) message_type: Option<OutgoingMessages>, // initiating message type
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
    pub(crate) fn try_next(&self) -> Option<Response> {
        Self::try_receive(self.pick_receiver()?)
    }

    /// Waits for next message until specified timeout.
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
        // TODO - shared sender
    }

    fn receive(receiver: &Receiver<RoutedItem>) -> Option<Response> {
        loop {
            if let Some(legacy) = receiver.recv().ok()?.into_legacy() {
                return Some(legacy);
            }
        }
    }

    fn try_receive(receiver: &Receiver<RoutedItem>) -> Option<Response> {
        loop {
            if let Some(legacy) = receiver.try_recv().ok()?.into_legacy() {
                return Some(legacy);
            }
        }
    }

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
        if let (Some(request_id), Some(signaler)) = (self.request_id, &self.signaler) {
            if let Err(e) = signaler.send(Signal::Request(request_id)) {
                log::warn!("error sending drop signal: {e}");
            }
        } else if let (Some(order_id), Some(signaler)) = (self.order_id, &self.signaler) {
            if let Err(e) = signaler.send(Signal::Order(order_id)) {
                log::warn!("error sending drop signal: {e}");
            }
        } else if let Some(signaler) = &self.signaler {
            // Currently is order update stream if no request or order id.
            if let Err(e) = signaler.send(Signal::OrderUpdateStream) {
                log::warn!("error sending drop signal: {e}");
            }
        }
    }
}

// Signals are used to notify the backend when a subscriber is dropped.
// This facilitates the cleanup of the SenderHashes.
#[cfg(feature = "sync")]
pub enum Signal {
    Request(i32),
    Order(i32),
    OrderUpdateStream,
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
    message_type: Option<OutgoingMessages>,
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
            message_type: None,
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

    pub(crate) fn message_type(mut self, message_type: OutgoingMessages) -> Self {
        self.message_type = Some(message_type);
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
                message_type: self.message_type,
            }
        } else if let Some(receiver) = self.shared_receiver {
            InternalSubscription {
                receiver: None,
                sender: None,
                shared_receiver: Some(receiver),
                signaler: None,
                request_id: self.request_id,
                order_id: self.order_id,
                message_type: self.message_type,
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
pub mod recorder;
pub mod routing;
