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

#[test]
fn message_encodes_bool() {
    let mut message = RequestMessage::new();

    message.push_field(&false);
    message.push_field(&true);

    assert_eq!(2, message.fields.len());
    assert_eq!("0\01\0", message.encode());
}

#[test]
fn message_encodes_string() {
    let mut message = RequestMessage::new();

    message.push_field(&"interactive");
    message.push_field(&"brokers");

    assert_eq!(2, message.fields.len());
    assert_eq!("interactive\0brokers\0", message.encode());
}

#[test]
fn message_encodes_rule_80_a() {
    let mut message = RequestMessage::new();

    message.push_field(&Some(Rule80A::Individual));
    message.push_field(&Some(Rule80A::Agency));
    message.push_field(&Some(Rule80A::AgentOtherMember));
    message.push_field(&Some(Rule80A::IndividualPTIA));
    message.push_field(&Some(Rule80A::AgencyPTIA));
    message.push_field(&Some(Rule80A::AgentOtherMemberPTIA));
    message.push_field(&Some(Rule80A::IndividualPT));
    message.push_field(&Some(Rule80A::AgencyPT));
    message.push_field(&Some(Rule80A::AgentOtherMemberPT));
    message.push_field(&Option::<Rule80A>::None);

    assert_eq!(10, message.fields.len());
    assert_eq!("I\0A\0W\0J\0U\0M\0K\0Y\0N\0\0", message.encode());
}

#[test]
fn message_encodes_order_condition() {
    let mut message = RequestMessage::new();

    message.push_field(&OrderCondition::Price);
    message.push_field(&OrderCondition::Time);
    message.push_field(&OrderCondition::Margin);
    message.push_field(&OrderCondition::Execution);
    message.push_field(&OrderCondition::Volume);
    message.push_field(&OrderCondition::PercentChange);

    assert_eq!(6, message.fields.len());
    assert_eq!("1\03\04\05\06\07\0", message.encode());
}
