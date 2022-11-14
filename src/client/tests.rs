use std::collections::VecDeque;

use anyhow::{anyhow, Result};

use super::*;
use crate::client::Client;

#[derive(Default, Debug, PartialEq)]
pub struct ClientStub {
    pub request_packets: Vec<RequestPacket>,
    pub response_packets: VecDeque<ResponsePacket>,
}

impl Client for ClientStub {
    fn next_request_id(&self) -> i32 {
        1
    }

    fn server_version(&self) -> i32 {
        1
    }

    fn send_packet(&mut self, packet: RequestPacket) -> Result<()> {
        self.request_packets.push(packet);
        Ok(())
    }

    fn receive_packet(&mut self, _request_id: i32) -> Result<ResponsePacket> {
        match self.response_packets.pop_front() {
            Some(packet) => Ok(packet),
            None => Err(anyhow!("ClientStub::receive_packet no packet")),
        }
    }

    fn receive_packets(&self, request_id: i32) -> ResponsePacketIterator {
        ResponsePacketIterator {}
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        Ok(())
    }
}

#[test]
fn request_packet_from_fields() {
    // let mut packet = RequestPacket::default();
    // packet.add_field(32);

    let packet = || -> RequestPacket {
        let mut packet = RequestPacket::default();
        packet.add_field(32);
        packet
    }();

    let result = 2 + 2;
    assert_eq!(result, 4);
}
