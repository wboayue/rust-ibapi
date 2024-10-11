use std::sync::{Arc, RwLock};

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

impl MessageBus for MessageBusStub {
    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.read().unwrap().clone()
    }

    fn read_message(&mut self) -> Result<ResponseMessage, Error> {
        Ok(ResponseMessage::default())
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<(), Error> {
        self.request_messages.write().unwrap().push(message.clone());
        Ok(())
    }

    fn send_request(&mut self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_request(self, request_id, message)
    }

    fn send_order_request(&mut self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_request(self, request_id, message)
    }

    fn send_shared_request(&mut self, _message_id: OutgoingMessages, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        mock_global_request(self, message)
    }

    fn write(&mut self, _packet: &str) -> Result<(), Error> {
        Ok(())
    }

    fn process_messages(&mut self, _server_version: i32) -> Result<(), Error> {
        Ok(())
    }
}

fn mock_request(stub: &mut MessageBusStub, _request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
    stub.request_messages.write().unwrap().push(message.clone());

    let (sender, receiver) = channel::unbounded();
    let (s1, _r1) = channel::unbounded();

    for message in &stub.response_messages {
        sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
    }

    let subscription = SubscriptionBuilder::new().shared_receiver(Arc::new(receiver)).signaler(s1).build();

    Ok(subscription)
}

fn mock_global_request(stub: &mut MessageBusStub, message: &RequestMessage) -> Result<InternalSubscription, Error> {
    stub.request_messages.write().unwrap().push(message.clone());

    let (sender, receiver) = channel::unbounded();

    for message in &stub.response_messages {
        sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
    }

    let subscription = SubscriptionBuilder::new().shared_receiver(Arc::new(receiver)).build();

    Ok(subscription)
}
