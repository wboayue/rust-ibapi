use std::sync::Arc;

use crossbeam::channel::{self, Receiver, Sender};

use anyhow::{anyhow, Result};

use super::*;

#[derive(Default, Debug)]
pub struct ClientStub {
    pub request_messages: Vec<String>,
    pub response_messages: Vec<String>,
    pub next_request_id: i32,
    pub server_version: i32,
    pub order_id: i32,
}

impl ClientStub {
    pub fn new(server_version: i32) -> Self {
        Self {
            server_version: server_version,
            next_request_id: 3000,
            ..Default::default()
        }
    }
}

impl Client for ClientStub {
    fn next_request_id(&mut self) -> i32 {
        let tmp = self.next_request_id;
        self.next_request_id += 1;
        tmp
    }

    fn next_order_id(&mut self) -> i32 {
        self.order_id += 1;
        self.order_id
    }

    fn set_next_order_id(&mut self, order_id: i32) -> i32 {
        self.order_id = order_id;
        self.order_id
    }

    fn server_version(&self) -> i32 {
        self.server_version
    }

    /// Returns the server version.
    fn server_time(&self) -> String {
        "200".to_owned()
    }

    /// Returns the managed accounts.
    fn managed_accounts(&self) -> String {
        "XYZ".to_owned()
    }

    fn send_message(&mut self, message: RequestMessage) -> Result<()> {
        self.request_messages.push(encode_message(&message));
        Ok(())
    }

    fn send_request(&mut self, _request_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = channel::unbounded();
        let (s1, r1) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(ResponsePacketPromise::new(receiver, s1, None, None))
    }

    fn send_order(&mut self, _order_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = channel::unbounded();
        let (s1, r1) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(ResponsePacketPromise::new(receiver, s1, None, None))
    }

    /// Sends request for the next valid order id.
    fn request_next_order_id(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(GlobalResponsePacketPromise::new(Arc::new(receiver)))
    }

    fn request_open_orders(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(GlobalResponsePacketPromise::new(Arc::new(receiver)))
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(anyhow!("server version {} required, got {}: {}", version, self.server_version, message))
        }
    }
}

fn encode_message(message: &RequestMessage) -> String {
    message.encode().replace("\0", "|")
}
