use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use crossbeam::channel;

use crate::client::transport::{GlobalResponseIterator, MessageBus, ResponseIterator};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::Error;

pub(crate) struct MessageBusStub {
    pub request_messages: RefCell<Vec<RequestMessage>>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

impl MessageBus for MessageBusStub {
    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.borrow().clone()
    }

    fn read_message(&mut self) -> Result<ResponseMessage, Error> {
        Ok(ResponseMessage::default())
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<(), Error> {
        self.request_messages.borrow_mut().push(message.clone());
        Ok(())
    }

    fn send_generic_message(&mut self, request_id: i32, message: &RequestMessage) -> Result<ResponseIterator, Error> {
        mock_request(self, request_id, message)
    }

    fn send_order_message(&mut self, request_id: i32, message: &RequestMessage) -> Result<ResponseIterator, Error> {
        mock_request(self, request_id, message)
    }

    fn request_next_order_id(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        mock_global_request(self, message)
    }

    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        mock_global_request(self, message)
    }

    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        mock_global_request(self, message)
    }

    fn request_positions(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        mock_global_request(self, message)
    }

    fn request_family_codes(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        mock_global_request(self, message)
    }

    fn write(&mut self, _packet: &str) -> Result<(), Error> {
        Ok(())
    }

    fn process_messages(&mut self, _server_version: i32) -> Result<(), Error> {
        Ok(())
    }
}

fn mock_request(stub: &mut MessageBusStub, _request_id: i32, message: &RequestMessage) -> Result<ResponseIterator, Error> {
    stub.request_messages.borrow_mut().push(message.clone());

    let (sender, receiver) = channel::unbounded();
    let (s1, _r1) = channel::unbounded();

    for message in &stub.response_messages {
        sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
    }

    Ok(ResponseIterator::new(receiver, s1, None, None, Duration::from_secs(5)))
}

fn mock_global_request(stub: &mut MessageBusStub, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
    stub.request_messages.borrow_mut().push(message.clone());

    let (sender, receiver) = channel::unbounded();

    for message in &stub.response_messages {
        sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
    }

    Ok(GlobalResponseIterator::new(Arc::new(receiver)))
}
