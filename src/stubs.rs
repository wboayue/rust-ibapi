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
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();
        let (s1, r1) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(ResponseIterator::new(receiver, s1, None, None, Duration::from_secs(5)))
    }

    fn send_order_message(&mut self, request_id: i32, message: &RequestMessage) -> Result<ResponseIterator, Error> {
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();
        let (s1, r1) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(ResponseIterator::new(receiver, s1, None, None, Duration::from_secs(5)))
    }

    fn request_next_order_id(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(GlobalResponseIterator::new(Arc::new(receiver)))
    }

    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(GlobalResponseIterator::new(Arc::new(receiver)))
    }

    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(GlobalResponseIterator::new(Arc::new(receiver)))
    }

    fn request_positions(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.request_messages.borrow_mut().push(message.clone());

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace('|', "\0"))).unwrap();
        }

        Ok(GlobalResponseIterator::new(Arc::new(receiver)))
    }

    fn write(&mut self, packet: &str) -> Result<(), Error> {
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> Result<(), Error> {
        Ok(())
    }
}

fn encode_message(message: &RequestMessage) -> String {
    message.encode().replace('\0', "|")
}
