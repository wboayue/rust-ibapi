use std::sync::{Arc, Mutex, RwLock};

use crossbeam::channel;

use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
use crate::transport::{InternalSubscription, MessageBus, SubscriptionBuilder};
use crate::Error;

pub(crate) struct MessageBusStub {
    pub request_messages: RwLock<Vec<RequestMessage>>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

// Separate tracking for order update subscriptions to maintain backward compatibility
static ORDER_UPDATE_SUBSCRIPTION_TRACKER: Mutex<Option<usize>> = Mutex::new(None);

impl Default for MessageBusStub {
    fn default() -> Self {
        Self {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }
    }
}

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
        if let Some(existing_id) = *tracker {
            if existing_id == stub_id {
                return Err(Error::AlreadySubscribed);
            }
        }
        *tracker = Some(stub_id);
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

    // fn process_messages(&mut self, _server_version: i32) -> Result<(), Error> {
    //     Ok(())
    // }
}

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
