use std::rc::Rc;

use time::OffsetDateTime;

use super::*;
use crate::client::stub::ClientStub;

use crate::contracts::contract_samples;
use crate::stubs::MessageBusStub;

#[test]
fn realtime_bars() {
    // let mut client: ClientStub = ClientStub::new(server_versions::SIZE_RULES);
    let mut stub = Box::new(MessageBusStub {
        request_messages: vec![],
        response_messages: vec![],
    });

    stub.response_messages = vec!["50|3|9001|1678323335|4028.75|4029.00|4028.25|4028.50|2|4026.75|1|".to_owned()];

    let mut client = IBClient::do_connect(stub).unwrap();

    let contract = contract_samples::future_with_local_symbol();
    let bar_size = BarSize::Secs5;
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    let bars = super::realtime_bars(&mut client, &contract, &bar_size, &what_to_show, use_rth);
    assert!(bars.is_ok(), "failed to request realtime bars: {}", bars.err().unwrap());

    // Verify Responses
    let mut bars = bars.unwrap();
    if let Some(bar) = bars.next() {
        let timestamp = OffsetDateTime::from_unix_timestamp(1678323335).unwrap();

        assert_eq!(bar.date, timestamp, "bar.date");
        assert_eq!(bar.open, 4028.75, "bar.open");
        assert_eq!(bar.high, 4029.00, "bar.high");
        assert_eq!(bar.low, 4028.25, "bar.low");
        assert_eq!(bar.close, 4028.50, "bar.close");
        assert_eq!(bar.volume, 2.0, "bar.volume");
        assert_eq!(bar.wap, 4026.75, "bar.wap");
        assert_eq!(bar.count, 1, "bar.count");
    } else {
        assert!(false, "expected a real time bar");
    }

    // Should trigger cancel realtime bars
    drop(bars);

    // Verify Requests
    // assert_eq!(
    //     client.request_messages[0],
    //     "50|8|3000|0||FUT|202303|0|||EUREX||EUR|FGBL MAR 23||0|TRADES|1||" // 50|8|9001|495512569|ES|FUT|20230616|0||50|CME||USD|ESM3|ES|0|TRADES|0||
    // );
    // assert_eq!(client.request_messages[1], "51|1|3000|");
}
