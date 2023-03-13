use crate::client::transport::MessageBus;

pub struct MessageBusStub {
    pub request_messages: Vec<String>,
    pub response_messages: Vec<String>,
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

impl MessageBus for MessageBusStub {
    fn read_message(&mut self) -> anyhow::Result<crate::client::ResponseMessage> {
        todo!()
    }

    fn write_message(&mut self, packet: &crate::client::RequestMessage) -> anyhow::Result<()> {
        todo!()
    }

    fn send_generic_message(
        &mut self,
        request_id: i32,
        packet: &crate::client::RequestMessage,
    ) -> anyhow::Result<crate::client::transport::ResponsePacketPromise> {
        todo!()
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
        todo!()
    }

    fn process_messages(&mut self, server_version: i32) -> anyhow::Result<()> {
        todo!()
    }
}
