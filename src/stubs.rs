use std::time::Duration;

use crossbeam::channel::{self, Receiver, Sender};

use crate::client::{transport::{MessageBus, ResponsePacketPromise}, ResponseMessage, RequestMessage};

pub struct MessageBusStub {
    pub request_messages: Vec<RequestMessage>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

impl MessageBus for MessageBusStub {

    fn request_messages(&self) -> Vec<RequestMessage> {
        self.request_messages.clone()
    }

    fn read_message(&mut self) -> anyhow::Result<crate::client::ResponseMessage> {
        Ok(ResponseMessage::default())
    }

    fn write_message(&mut self, message: &crate::client::RequestMessage) -> anyhow::Result<()> {
        self.request_messages.push(message.clone());
        Ok(())
    }

    fn send_generic_message(
        &mut self,
        request_id: i32,
        message: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::ResponsePacketPromise> {
        self.request_messages.push(message.clone());

        let (sender, receiver) = channel::unbounded();
        let (s1, r1) = channel::unbounded();

        for message in &self.response_messages {
            sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
        }

        Ok(ResponsePacketPromise::new(receiver, s1, None, None, Duration::from_secs(5)))
    }

    fn send_order_message(
        &mut self,
        request_id: i32,
        packet: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::ResponsePacketPromise> {
        todo!()
    }

    fn request_next_order_id(
        &mut self,
        message: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::GlobalResponsePacketPromise> {
        todo!()
    }

    fn request_open_orders(
        &mut self,
        message: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::GlobalResponsePacketPromise> {
        todo!()
    }

    fn request_market_rule(
        &mut self,
        message: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::GlobalResponsePacketPromise> {
        todo!()
    }

    fn write(&mut self, packet: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> anyhow::Result<()> {
        Ok(())
    }
}

// fn send_message(&mut self, message: RequestMessage) -> Result<()> {
//     Ok(())
// }

// fn send_request(&mut self, _request_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
//     self.request_messages.push(encode_message(&message));

//     let (sender, receiver) = channel::unbounded();
//     let (s1, r1) = channel::unbounded();

//     for message in &self.response_messages {
//         sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
//     }

//     Ok(ResponsePacketPromise::new(receiver, s1, None, None, Duration::from_secs(5)))
// }

// fn send_order(&mut self, _order_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
//     self.request_messages.push(encode_message(&message));

//     let (sender, receiver) = channel::unbounded();
//     let (s1, r1) = channel::unbounded();

//     for message in &self.response_messages {
//         sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
//     }

//     Ok(ResponsePacketPromise::new(receiver, s1, None, None, Duration::from_secs(5)))
// }

// /// Sends request for the next valid order id.
// fn request_next_order_id(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
//     self.request_messages.push(encode_message(&message));

//     let (sender, receiver) = channel::unbounded();

//     for message in &self.response_messages {
//         sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
//     }

//     Ok(GlobalResponsePacketPromise::new(Arc::new(receiver)))
// }

// fn request_order_data(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
//     self.request_messages.push(encode_message(&message));

//     let (sender, receiver) = channel::unbounded();

//     for message in &self.response_messages {
//         sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
//     }

//     Ok(GlobalResponsePacketPromise::new(Arc::new(receiver)))
// }

// fn request_market_rule(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
//     self.request_messages.push(encode_message(&message));

//     let (sender, receiver) = channel::unbounded();

//     for message in &self.response_messages {
//         sender.send(ResponseMessage::from(&message.replace("|", "\0"))).unwrap();
//     }

//     Ok(GlobalResponsePacketPromise::new(Arc::new(receiver)))
// }

// fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
//     if version <= self.server_version {
//         Ok(())
//     } else {
//         Err(anyhow!("server version {} required, got {}: {}", version, self.server_version, message))
//     }
// }
// }

fn encode_message(message: &RequestMessage) -> String {
    message.encode().replace("\0", "|")
}
