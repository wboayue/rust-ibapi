use std::sync::{Arc, RwLock};

use crossbeam::channel;

use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
use crate::transport::{InternalSubscription, MessageBus, Response, SubscriptionBuilder};
use crate::Error;

pub(crate) struct MessageBusStub {
    pub request_messages: RwLock<Vec<RequestMessage>>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

impl MessageBus for MessageBusStub {
    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.read().unwrap().clone()
    }

    fn send_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_request(self, Some(request_id), None, message)
    }

    fn cancel_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, Some(request_id), None, packet);
        Ok(())
    }

    fn send_order_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_request(self, Some(request_id), None, message)
    }

    fn cancel_order_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, Some(request_id), None, packet);
        Ok(())
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_request(self, None, Some(message_type), message)
    }

    fn cancel_shared_subscription(&self, message_type: OutgoingMessages, packet: &RequestMessage) -> Result<(), Error> {
        mock_request(self, None, Some(message_type), packet)?;
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
) -> Result<InternalSubscription, Error> {
    stub.request_messages.write().unwrap().push(message.clone());

    let (sender, receiver) = channel::unbounded();
    let (s1, _r1) = channel::unbounded();

    for message in &stub.response_messages {
        let message = ResponseMessage::from(&message.replace('|', "\0"));
        sender.send(Response::from(message)).unwrap();
    }

    let mut subscription = SubscriptionBuilder::new().shared_receiver(Arc::new(receiver)).signaler(s1);
    if let Some(request_id) = request_id {
        subscription = subscription.request_id(request_id);
    }
    if let Some(message_type) = message_type {
        subscription = subscription.message_type(message_type);
    }

    Ok(subscription.build())
}

fn mock_global_request(stub: &mut MessageBusStub, message: &RequestMessage) -> Result<InternalSubscription, Error> {
    stub.request_messages.write().unwrap().push(message.clone());

    let (sender, receiver) = channel::unbounded();

    for message in &stub.response_messages {
        let message = ResponseMessage::from(&message.replace('|', "\0"));
        sender.send(Response::from(message)).unwrap();
    }

    let subscription = SubscriptionBuilder::new().shared_receiver(Arc::new(receiver)).build();

    Ok(subscription)
}
