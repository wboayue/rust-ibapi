//! Transport layer for TWS communication with sync/async support

#[cfg(all(feature = "sync", not(feature = "async")))]
use std::sync::Arc;
#[cfg(all(feature = "sync", not(feature = "async")))]
use std::time::Duration;

#[cfg(all(feature = "sync", not(feature = "async")))]
use crossbeam::channel::{Receiver, Sender};

use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};

#[cfg(all(feature = "sync", not(feature = "async")))]
use crate::messages::OutgoingMessages;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Common types
#[allow(dead_code)]
pub(crate) type Response = Result<ResponseMessage, Error>;

// MessageBus trait - defines the interface for message handling
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) trait MessageBus: Send + Sync {
    // Sends formatted message to TWS and creates a reply channel by request id.
    fn send_request(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    // Sends formatted message to TWS and creates a reply channel by request id.
    fn cancel_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error>;

    // Sends formatted message to TWS and creates a reply channel by message type.
    fn send_shared_request(&self, message_id: OutgoingMessages, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    // Sends formatted message to TWS and creates a reply channel by message type.
    fn cancel_shared_subscription(&self, message_id: OutgoingMessages, packet: &RequestMessage) -> Result<(), Error>;

    // Sends formatted order specific message to TWS and creates a reply channel by order id.
    fn send_order_request(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    /// Sends a message to TWS without creating a unique reply channel.
    ///
    /// This method is used for fire-and-forget messages that don't require
    /// tracking responses by request ID. The message is sent directly to TWS
    /// without establishing a dedicated response channel.
    ///
    /// # Arguments
    /// * `packet` - The formatted request message to send
    ///
    /// # Returns
    /// * `Ok(())` if the message was successfully sent
    /// * `Err(Error)` if sending failed
    fn send_message(&self, packet: &RequestMessage) -> Result<(), Error>;

    /// Creates a subscription to the order update stream.
    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error>;

    fn cancel_order_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error>;

    fn ensure_shutdown(&self);

    // Testing interface. Tracks requests sent messages when Bus is stubbed.
    #[cfg(test)]
    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
    }
}

// InternalSubscription - handles receiving messages for sync subscriptions
#[cfg(all(feature = "sync", not(feature = "async")))]
#[derive(Debug, Default)]
pub(crate) struct InternalSubscription {
    receiver: Option<Receiver<Response>>,              // requests with request ids receive responses via this channel
    sender: Option<Sender<Response>>,                  // requests with request ids receive responses via this channel
    shared_receiver: Option<Arc<Receiver<Response>>>,  // this channel is for responses that share channel based on message type
    signaler: Option<Sender<Signal>>,                  // for client to signal termination
    pub(crate) request_id: Option<i32>,                // initiating request id
    pub(crate) order_id: Option<i32>,                  // initiating order id
    pub(crate) message_type: Option<OutgoingMessages>, // initiating message type
}

#[cfg(all(feature = "sync", not(feature = "async")))]
impl InternalSubscription {
    // Blocks until next message become available.
    pub(crate) fn next(&self) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::receive(receiver)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::receive(receiver)
        } else {
            None
        }
    }

    // Returns message if available or immediately returns None.
    pub(crate) fn try_next(&self) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::try_receive(receiver)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::try_receive(receiver)
        } else {
            None
        }
    }

    // Waits for next message until specified timeout.
    pub(crate) fn next_timeout(&self, timeout: Duration) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::timeout_receive(receiver, timeout)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::timeout_receive(receiver, timeout)
        } else {
            None
        }
    }

    pub(crate) fn cancel(&self) {
        if let Some(sender) = &self.sender {
            if let Err(e) = sender.send(Err(Error::Cancelled)) {
                log::warn!("error sending cancel notification: {e}")
            }
        }
        // TODO - shared sender
    }

    fn receive(receiver: &Receiver<Response>) -> Option<Response> {
        receiver.recv().ok()
    }

    fn try_receive(receiver: &Receiver<Response>) -> Option<Response> {
        receiver.try_recv().ok()
    }

    fn timeout_receive(receiver: &Receiver<Response>, timeout: Duration) -> Option<Response> {
        receiver.recv_timeout(timeout).ok()
    }
}

#[cfg(all(feature = "sync", not(feature = "async")))]
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
#[cfg(all(feature = "sync", not(feature = "async")))]
pub enum Signal {
    Request(i32),
    Order(i32),
    OrderUpdateStream,
}

// SubscriptionBuilder for creating InternalSubscription instances
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) struct SubscriptionBuilder {
    receiver: Option<Receiver<Response>>,
    sender: Option<Sender<Response>>,
    shared_receiver: Option<Arc<Receiver<Response>>>,
    signaler: Option<Sender<Signal>>,
    order_id: Option<i32>,
    request_id: Option<i32>,
    message_type: Option<OutgoingMessages>,
}

#[cfg(all(feature = "sync", not(feature = "async")))]
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

    pub(crate) fn receiver(mut self, receiver: Receiver<Response>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub(crate) fn sender(mut self, sender: Sender<Response>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub(crate) fn shared_receiver(mut self, receiver: Arc<Receiver<Response>>) -> Self {
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
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::TcpMessageBus;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) use sync::TcpSocket;

// Async exports (placeholder for now)
#[cfg(feature = "async")]
pub use r#async::{AsyncInternalSubscription, AsyncMessageBus};

pub mod connection;
pub mod recorder;
pub mod routing;
