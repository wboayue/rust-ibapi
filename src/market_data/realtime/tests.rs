use std::sync::RwLock;
use std::sync::{Arc, Mutex};

use time::OffsetDateTime;

use crate::contracts::contract_samples;
use crate::messages::OutgoingMessages;
use crate::stubs::MessageBusStub;
use crate::ToField;

use super::*;

#[test]
fn realtime_bars() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["50|3|9001|1678323335|4028.75|4029.00|4028.25|4028.50|2|4026.75|1|".to_owned()],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = contract_samples::future_with_local_symbol();
    let bar_size = BarSize::Sec5;
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    let bars = client.realtime_bars(&contract, bar_size, what_to_show, use_rth);
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

    let request_messages = client.message_bus.lock().unwrap().request_messages();

    // Verify Requests
    let realtime_bars_request = &request_messages[0];
    assert_eq!(
        realtime_bars_request[0],
        OutgoingMessages::RequestRealTimeBars.to_field(),
        "message.message_type"
    );
    assert_eq!(realtime_bars_request[1], "8", "message.version");
    assert_eq!(realtime_bars_request[2], "9000", "message.request_id");
    assert_eq!(realtime_bars_request[3], contract.contract_id.to_field(), "message.contract_id");
    assert_eq!(realtime_bars_request[4], contract.symbol.to_field(), "message.symbol");
    assert_eq!(realtime_bars_request[5], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        realtime_bars_request[6],
        contract.last_trade_date_or_contract_month.to_field(),
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(realtime_bars_request[7], contract.strike.to_field(), "message.strike");
    assert_eq!(realtime_bars_request[8], contract.right.to_field(), "message.right");
    assert_eq!(realtime_bars_request[9], contract.multiplier.to_field(), "message.multiplier");
    assert_eq!(realtime_bars_request[10], contract.exchange.to_field(), "message.exchange");
    assert_eq!(
        realtime_bars_request[11],
        contract.primary_exchange.to_field(),
        "message.primary_exchange"
    );
    assert_eq!(realtime_bars_request[12], contract.currency.to_field(), "message.currency");
    assert_eq!(realtime_bars_request[13], contract.local_symbol.to_field(), "message.local_symbol");
    assert_eq!(realtime_bars_request[14], contract.trading_class.to_field(), "message.trading_class");
    assert_eq!(realtime_bars_request[15], "0", "message.bar_size");
    assert_eq!(realtime_bars_request[16], what_to_show.to_field(), "message.what_to_show");
    assert_eq!(realtime_bars_request[17], use_rth.to_field(), "message.use_rth");
    assert_eq!(realtime_bars_request[18], "", "message.options");

    let cancel_request = &request_messages[1];
    assert_eq!(cancel_request[0], OutgoingMessages::CancelRealTimeBars.to_field(), "message.message_type");
    assert_eq!(cancel_request[1], "1", "message.version");
    assert_eq!(cancel_request[2], "9000", "message.request_id");
}

#[test]
fn what_to_show() {
    assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
    assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
    assert_eq!(WhatToShow::Bid.to_string(), "BID");
    assert_eq!(WhatToShow::Ask.to_string(), "ASK");
}
