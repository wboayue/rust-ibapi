use super::*;

#[test]
fn test_validate_tick_by_tick_request() {
    // Test with old server version
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK - 1);
    let contract = contract_samples::simple_future();

    let result = validate_tick_by_tick_request(&client, &contract, 0, false);
    assert!(result.is_err(), "Should fail with old server version");

    // Test with new server version but old parameters
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK);

    let result = validate_tick_by_tick_request(&client, &contract, 1, true);
    assert!(result.is_err(), "Should fail with new server version but old parameters");

    // Test with new server version and new parameters
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK_IGNORE_SIZE);

    let result = validate_tick_by_tick_request(&client, &contract, 1, true);
    assert!(result.is_ok(), "Should succeed with new server version and parameters");
}

#[test]
fn test_what_to_show_display() {
    assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
    assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
    assert_eq!(WhatToShow::Bid.to_string(), "BID");
    assert_eq!(WhatToShow::Ask.to_string(), "ASK");
}
