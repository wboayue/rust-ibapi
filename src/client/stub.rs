use std::sync::mpsc;

use anyhow::{anyhow, Result};

use super::*;

#[derive(Default, Debug)]
pub struct ClientStub {
    pub request_messages: Vec<String>,
    pub response_messages: Vec<String>,
    pub next_request_id: i32,
    pub server_version: i32,
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

    fn server_version(&self) -> i32 {
        self.server_version
    }

    fn send_message(&mut self, message: RequestMessage) -> Result<()> {
        self.request_messages.push(encode_message(&message));
        Ok(())
    }

    fn send_request(&mut self, _request_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = mpsc::channel();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(ResponsePacketPromise::new(receiver))
    }

    fn send_order(&mut self, _order_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        self.request_messages.push(encode_message(&message));

        let (sender, receiver) = mpsc::channel();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(ResponsePacketPromise::new(receiver))
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
