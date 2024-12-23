use super::*;

#[test]
fn test_realtime_bars() {
    // Setup test message bus with mock responses
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "50|3|9001|1678323335|4028.75|4029.00|4028.25|4028.50|2|4026.75|1|".to_owned(),
            "50|3|9001|1678323340|4028.80|4029.10|4028.30|4028.55|3|4026.80|2|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = contract_samples::future_with_local_symbol();
    let bar_size = BarSize::Sec5;
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    // Test subscription creation
    let bars = client.realtime_bars(&contract, bar_size, what_to_show, use_rth);

    // Test receiving data
    let bars = bars.expect("Failed to create realtime bars subscription");
    let received_bars: Vec<Bar> = bars.iter().take(2).collect();

    assert_eq!(received_bars.len(), 2, "Should receive 2 bars");

    // Verify first bar
    assert_eq!(
        received_bars[0].date,
        OffsetDateTime::from_unix_timestamp(1678323335).unwrap(),
        "Wrong timestamp for first bar"
    );
    assert_eq!(received_bars[0].open, 4028.75, "Wrong open price for first bar");
    assert_eq!(received_bars[0].volume, 2.0, "Wrong volume for first bar");

    // Verify second bar
    assert_eq!(
        received_bars[1].date,
        OffsetDateTime::from_unix_timestamp(1678323340).unwrap(),
        "Wrong timestamp for second bar"
    );
    assert_eq!(received_bars[1].open, 4028.80, "Wrong open price for second bar");
    assert_eq!(received_bars[1].volume, 3.0, "Wrong volume for second bar");

    // Verify request messages
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    let request = &request_messages[0];
    assert_eq!(request[0], OutgoingMessages::RequestRealTimeBars.to_field(), "Wrong message type");
    assert_eq!(request[1], "8", "Wrong version");
    assert_eq!(request[16], what_to_show.to_field(), "Wrong what to show value");
    assert_eq!(request[17], use_rth.to_field(), "Wrong use RTH flag");
}

#[test]
fn test_tick_by_tick_all_last() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "99|9001|1|1678740829|3895.25|7|2|NASDAQ|Regular|".to_owned(),
            "99|9001|1|1678740830|3895.50|5|0|NYSE|Regular|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = contract_samples::simple_future();
    let number_of_ticks = 2;
    let ignore_size = false;

    // Test subscription creation
    let trades = client.tick_by_tick_all_last(&contract, number_of_ticks, ignore_size);
    let trades = trades.expect("Failed to create tick-by-tick subscription");

    // Test receiving data
    let received_trades: Vec<LastTicks> = trades.iter().take(2).collect();

    assert_eq!(received_trades.len(), 2, "Should receive 2 trades");

    // Verify first trade
    if let LastTicks::Trade(trade) = &received_trades[0] {
        assert_eq!(trade.price, 3895.25, "Wrong price for first trade");
        assert_eq!(trade.size, 7, "Wrong size for first trade");
        assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange for first trade");
    } else {
        panic!("Expected trade, got {:?}", received_trades[0]);
    }

    // Verify second trade
    if let LastTicks::Trade(trade) = &received_trades[1] {
        assert_eq!(trade.price, 3895.50, "Wrong price for second trade");
        assert_eq!(trade.size, 5, "Wrong size for second trade");
        assert_eq!(trade.exchange, "NYSE", "Wrong exchange for second trade");
    } else {
        panic!("Expected trade, got {:?}", received_trades[1]);
    }

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    let request = &request_messages[0];
    assert_eq!(request[0], OutgoingMessages::RequestTickByTickData.to_field(), "Wrong message type");
    assert_eq!(request[14], "AllLast", "Wrong tick type");
}
