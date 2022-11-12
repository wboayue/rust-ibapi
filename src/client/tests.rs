use anyhow::Result;

use crate::client::Client;

use super::{ResponsePacket, RequestPacket, ResponsePacketIterator};

pub struct ClientStub{

}

impl Client for ClientStub {
    fn next_request_id(&self) -> i32 {
        1
    }

    fn server_version(&self) -> i32 {
        1
    }

    fn send_packet(&self, packet: &RequestPacket) -> i32 {
        1
    }

    fn receive_packet(&self, request_id: i32) -> ResponsePacket {
        ResponsePacket{}
    }

    fn receive_packets(&self, request_id: i32) -> ResponsePacketIterator {
        ResponsePacketIterator{}
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

