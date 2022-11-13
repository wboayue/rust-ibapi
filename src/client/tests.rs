use anyhow::Result;

use crate::client::Client;

use super::{RequestPacket, ResponsePacket, ResponsePacketIterator};

#[derive(Default, Debug, PartialEq)]
pub struct ClientStub {
    outbound_packets: Vec<RequestPacket>,
}

impl Client for ClientStub {
    fn next_request_id(&self) -> i32 {
        1
    }

    fn server_version(&self) -> i32 {
        1
    }

    fn send_packet(&mut self, packet: RequestPacket) -> Result<()> {
        self.outbound_packets.push(packet);
        Ok(())
    }

    fn receive_packet(&self, request_id: i32) -> ResponsePacket {
        ResponsePacket::default()
    }

    fn receive_packets(&self, request_id: i32) -> ResponsePacketIterator {
        ResponsePacketIterator {}
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        Ok(())
    }
}

#[test]
fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}
