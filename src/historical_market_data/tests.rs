use std::collections::VecDeque;

use time::OffsetDateTime;

use crate::client::tests::ClientStub;
use crate::client::ResponsePacket;
use crate::contracts;
use crate::historical_market_data;

#[test]
fn test_head_timestamp() {
    let mut client = ClientStub::default();
    client.response_packets = VecDeque::from([ResponsePacket::from(vec![
        String::from("10"),
        String::from("0"),
        String::from("c"),
    ])]);

    let contract = contracts::stock("MSFT");
    let what_to_show = "trades";
    let use_rth = true;

    let result =
        historical_market_data::head_timestamp(&mut client, &contract, what_to_show, use_rth);

    match result {
        Err(error) => assert_eq!(error.to_string(), ""),
        Ok(head_timestamp) => assert_eq!(head_timestamp, OffsetDateTime::now_utc()),
    };

    assert_eq!(client.request_packets.len(), 1);

    let packet = &client.request_packets[0];

    assert_eq!(packet[0], "hh");
    assert_eq!(packet[1], "hh");
}

#[test]
fn histogram_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn historical_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}
