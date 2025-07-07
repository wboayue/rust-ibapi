use super::*;
use crate::{server_versions, Client};

#[test]
fn test_tick_by_tick_last() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["99|9001|1|1678740829|3895.25|7|2|NASDAQ|Regular|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = contract_samples::simple_future();
    let number_of_ticks = 1;
    let ignore_size = false;

    // Test subscription creation
    let result = client.tick_by_tick_last(&contract, number_of_ticks, ignore_size);

    // Test receiving data
    let trades = result.expect("Failed to receive tick-by-tick last data");
    let received_trades: Vec<Trade> = trades.iter().take(1).collect();

    assert_eq!(received_trades.len(), 1, "Should receive 1 trade");

    // Verify trade data
    let trade = &received_trades[0];
    assert_eq!(trade.price, 3895.25, "Wrong price");
    assert_eq!(trade.size, 7.0, "Wrong size");
    assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange");

    // Verify request message uses "Last" instead of "AllLast"
    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];
    assert_eq!(request[14], "Last", "Wrong tick type");
}
