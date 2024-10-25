use super::*;

#[test]
fn test_market_depth() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["12|2|9001|0|1|1|185.50|100|".to_owned(), "12|2|9001|1|1|0|185.45|200|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SMART_DEPTH);
    let contract = Contract::stock("AAPL");
    let number_of_rows = 5;
    let is_smart_depth = true;

    // Test subscription creation
    let result = client.market_depth(&contract, number_of_rows, is_smart_depth);

    // Test receiving data
    let subscription = result.expect("Failed to create market depth subscription");
    let received_depth: Vec<MarketDepths> = subscription.iter().take(2).collect();
    if subscription.error().is_some() {
        panic!("Error received: {:?}", subscription.error());
    }

    assert_eq!(received_depth.len(), 2, "Should receive 2 market depth updates");

    // Verify first update
    if let MarketDepths::MarketDepth(update) = &received_depth[0] {
        assert_eq!(update.position, 0, "Wrong position for first update");
        assert_eq!(update.operation, 1, "Wrong operation for first update");
        assert_eq!(update.side, 1, "Wrong side for first update");
        assert_eq!(update.price, 185.50, "Wrong price for first update");
        assert_eq!(update.size, 100.0, "Wrong size for first update");
    } else {
        panic!("Expected MarketDepth variant");
    }

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    let request = &request_messages[0];
    assert_eq!(request[0], OutgoingMessages::RequestMarketDepth.to_field(), "Wrong message type");
}

#[test]
fn test_market_depth_exchanges() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["71|2|ISLAND|STK|NASDAQ|DEEP2|1|NYSE|STK|NYSE|DEEP|1|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SERVICE_DATA_TYPE);

    // Test request execution
    let exchanges = market_depth_exchanges(&client);
    assert!(exchanges.is_ok(), "Failed to request market depth exchanges");

    let exchanges = exchanges.unwrap();
    assert_eq!(exchanges.len(), 2, "Should receive 2 exchange descriptions");

    // Verify first exchange
    let first = &exchanges[0];
    assert_eq!(first.exchange_name, "ISLAND", "Wrong exchange name");
    assert_eq!(first.security_type, "STK", "Wrong security type");
    assert_eq!(first.listing_exchange, "NASDAQ", "Wrong listing exchange");
    assert_eq!(first.service_data_type, "DEEP2", "Wrong service data type");
    assert_eq!(first.aggregated_group, Some("1".to_string()), "Wrong aggregated group");

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    let request = &request_messages[0];
    assert_eq!(request[0], OutgoingMessages::RequestMktDepthExchanges.to_field(), "Wrong message type");
}
