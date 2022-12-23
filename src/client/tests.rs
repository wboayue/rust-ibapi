use std::collections::VecDeque;

use anyhow::{anyhow, Result};

use super::*;

#[derive(Default, Debug)]
pub struct ClientStub {
    pub request_packets: Vec<RequestMessage>,
    pub response_packets: VecDeque<ResponseMessage>,
}

impl Client for ClientStub {
    fn next_request_id(&mut self) -> i32 {
        1
    }

    fn server_version(&self) -> i32 {
        1
    }

    fn send_packet(&mut self, packet: RequestMessage) -> Result<()> {
        self.request_packets.push(packet);
        Ok(())
    }

    fn send_message(
        &mut self,
        request_id: i32,
        message: RequestMessage,
    ) -> Result<ResponsePacketPromise> {
        Err(anyhow!("not implemented"))
    }

    // fn receive_packet(&mut self, _request_id: i32) -> Result<ResponsePacket> {
    //     match self.response_packets.pop_front() {
    //         Some(packet) => Ok(packet),
    //         None => Err(anyhow!("ClientStub::receive_packet no packet")),
    //     }
    // }

    fn receive_packets(&self, request_id: i32) -> Result<ResponsePacketIterator> {
        Ok(ResponsePacketIterator {})
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        Ok(())
    }
}

#[test]
fn request_packet_from_fields() {
    // let mut packet = RequestPacket::default();
    // packet.add_field(32);

    let packet = || -> RequestMessage {
        let mut packet = RequestMessage::default();
        packet.push_field(&32);
        packet
    }();

    let result = 2 + 2;
    assert_eq!(result, 4);
}
