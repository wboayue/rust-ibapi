use std::sync::RwLock;

#[cfg(feature = "sync")]
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock, Mutex},
};

#[cfg(feature = "sync")]
use crossbeam::channel;

use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
use crate::Error;

#[cfg(feature = "sync")]
use crate::transport::{InternalSubscription, MessageBus, SubscriptionBuilder};

#[cfg(feature = "async")]
use {
    crate::transport::{r#async::AsyncInternalSubscription, AsyncMessageBus},
    async_trait::async_trait,
    tokio::sync::broadcast,
};

#[cfg(feature = "async")]
const TEST_BROADCAST_CAPACITY: usize = 1024;

pub(crate) struct MessageBusStub {
    pub request_messages: RwLock<Vec<RequestMessage>>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

// Separate tracking for order update subscriptions to maintain backward compatibility
#[cfg(feature = "sync")]
static ORDER_UPDATE_SUBSCRIPTION_TRACKER: LazyLock<Mutex<HashSet<usize>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

impl Default for MessageBusStub {
    fn default() -> Self {
        Self {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }
    }
}

#[cfg(feature = "sync")]
impl Drop for MessageBusStub {
    fn drop(&mut self) {
        // Clean up the subscription tracker to prevent test isolation issues
        let stub_id = self as *const _ as usize;
        ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().remove(&stub_id);
    }
}

impl MessageBusStub {
    #[cfg(feature = "sync")]
    pub fn with_responses(response_messages: Vec<String>) -> Self {
        Self {
            request_messages: RwLock::new(vec![]),
            response_messages,
        }
    }

    pub fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.read().unwrap().clone()
    }
}

#[cfg(feature = "sync")]
impl MessageBus for MessageBusStub {
    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.read().unwrap().clone()
    }

    fn send_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, Some(request_id), None, message))
    }

    fn cancel_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, Some(request_id), None, packet);
        Ok(())
    }

    fn send_order_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, Some(request_id), None, message))
    }

    fn send_message(&self, message: &RequestMessage) -> Result<(), Error> {
        self.request_messages.write().unwrap().push(message.clone());
        Ok(())
    }

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error> {
        // Use pointer address as unique identifier for this stub instance
        let stub_id = self as *const _ as usize;

        let mut tracker = ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap();
        if !tracker.insert(stub_id) {
            return Err(Error::AlreadySubscribed);
        }
        drop(tracker); // Release lock early

        let (sender, receiver) = channel::unbounded();
        let (signaler, _) = channel::unbounded();

        // Send any pre-configured response messages
        for message in &self.response_messages {
            let message = ResponseMessage::from(&message.replace('|', "\0"));
            sender.send(Ok(message)).unwrap();
        }

        let subscription = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).build();

        Ok(subscription)
    }

    fn cancel_order_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, Some(request_id), None, packet);

        let stub_id = self as *const _ as usize;
        ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().remove(&stub_id);

        Ok(())
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, None, Some(message_type), message))
    }

    fn cancel_shared_subscription(&self, message_type: OutgoingMessages, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, None, Some(message_type), packet);
        Ok(())
    }

    fn ensure_shutdown(&self) {}

    fn is_connected(&self) -> bool {
        true // Stub always returns connected
    }

    // fn process_messages(&mut self, _server_version: i32) -> Result<(), Error> {
    //     Ok(())
    // }
}

#[cfg(feature = "sync")]
fn mock_request(
    stub: &MessageBusStub,
    request_id: Option<i32>,
    message_type: Option<OutgoingMessages>,
    message: &RequestMessage,
) -> InternalSubscription {
    stub.request_messages.write().unwrap().push(message.clone());

    let (sender, receiver) = channel::unbounded();
    let (s1, _r1) = channel::unbounded();

    for message in &stub.response_messages {
        let message = ResponseMessage::from(&message.replace('|', "\0"));
        sender.send(Ok(message)).unwrap();
    }

    let mut subscription = SubscriptionBuilder::new().signaler(s1);
    if let Some(request_id) = request_id {
        subscription = subscription.receiver(receiver).request_id(request_id);
    } else if let Some(message_type) = message_type {
        subscription = subscription.shared_receiver(Arc::new(receiver)).message_type(message_type);
    }

    subscription.build()
}

#[cfg(feature = "async")]
#[async_trait]
impl AsyncMessageBus for MessageBusStub {
    async fn send_request(&self, _request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.request_messages.write().unwrap().push(message);

        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);
        // Send pre-configured response messages
        for message in &self.response_messages {
            let message = ResponseMessage::from(&message.replace('|', "\0"));
            sender.send(message).unwrap();
        }

        Ok(AsyncInternalSubscription::new(receiver))
    }

    async fn send_order_request(&self, _order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.request_messages.write().unwrap().push(message);

        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);
        // Send pre-configured response messages
        for message in &self.response_messages {
            let message = ResponseMessage::from(&message.replace('|', "\0"));
            sender.send(message).unwrap();
        }

        Ok(AsyncInternalSubscription::new(receiver))
    }

    async fn send_shared_request(&self, _message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.request_messages.write().unwrap().push(message);

        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);
        // Send pre-configured response messages
        for message in &self.response_messages {
            let message = ResponseMessage::from(&message.replace('|', "\0"));
            sender.send(message).unwrap();
        }

        Ok(AsyncInternalSubscription::new(receiver))
    }

    async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
        self.request_messages.write().unwrap().push(message);
        Ok(())
    }

    async fn cancel_subscription(&self, _request_id: i32, _message: RequestMessage) -> Result<(), Error> {
        Ok(())
    }

    async fn cancel_order_subscription(&self, _order_id: i32, _message: RequestMessage) -> Result<(), Error> {
        Ok(())
    }

    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);

        // Send pre-configured response messages
        for message in &self.response_messages {
            let message = ResponseMessage::from(&message.replace('|', "\0"));
            sender.send(message).unwrap();
        }

        Ok(AsyncInternalSubscription::new(receiver))
    }

    async fn ensure_shutdown(&self) {
        // No-op for test stub
    }

    fn request_shutdown_sync(&self) {
        // No-op for test stub
    }

    fn is_connected(&self) -> bool {
        true // Stub always returns connected
    }

    #[cfg(test)]
    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.read().unwrap().clone()
    }
}
