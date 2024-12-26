use super::*;

#[test]
fn test_tick_by_tick_bid_ask() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["99|9001|3|1678745793|3895.50|3896.00|9|11|3|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = contract_samples::simple_future();
    let number_of_ticks = 1;
    let ignore_size = false;

    // Test subscription creation
    let result = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size);

    // Test receiving data
    let subscription = result.expect("Failed to create bid/ask subscription");
    let received_ticks: Vec<BidAsk> = subscription.iter().take(1).collect();

    assert_eq!(received_ticks.len(), 1, "Should receive 1 bid/ask tick");

    // Verify tick data
    let tick = &received_ticks[0];
    assert_eq!(tick.bid_price, 3895.50, "Wrong bid price");
    assert_eq!(tick.ask_price, 3896.00, "Wrong ask price");
    assert_eq!(tick.bid_size, 9.0, "Wrong bid size");
    assert_eq!(tick.ask_size, 11.0, "Wrong ask size");

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];
    assert_eq!(request[14], "BidAsk", "Wrong tick type");
}

#[test]
fn test_tick_by_tick_midpoint() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["99|9001|4|1678746113|3896.875|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = contract_samples::simple_future();
    let number_of_ticks = 1;
    let ignore_size = false;

    // Test subscription creation
    let result = client.tick_by_tick_midpoint(&contract, number_of_ticks, ignore_size);

    // Test receiving data
    let subscription = result.expect("Failed to create midpoint subscription");
    let received_ticks: Vec<MidPoint> = subscription.iter().take(1).collect();

    assert_eq!(received_ticks.len(), 1, "Should receive 1 midpoint tick");

    // Verify tick data
    let tick = &received_ticks[0];
    assert_eq!(tick.mid_point, 3896.875, "Wrong midpoint price");

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];
    assert_eq!(request[14], "MidPoint", "Wrong tick type");
}
