use std::sync::Arc;

use crate::{
    stubs::MessageBusStub,
    transport::{ConnectionMetadata, MessageBus},
};

use super::Client;

#[test]
fn test_client_id() {
    let client_id = 500;
    let connection_metadata = ConnectionMetadata {
        client_id,
        ..ConnectionMetadata::default()
    };
    let message_bus = Arc::new(MessageBusStub::default());

    let client = Client::new(connection_metadata, message_bus).unwrap();

    assert_eq!(client.client_id(), client_id);
}

#[test]
fn test_subscription_cancel_only_sends_once() {
    // This test verifies that calling cancel() multiple times only sends one cancel message
    // This addresses issue #258 where explicit cancel() followed by Drop could send duplicate messages

    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), 100);

    // Create a subscription using realtime bars as an example
    let contract = crate::contracts::Contract::stock("AAPL");
    let subscription = client
        .realtime_bars(
            &contract,
            crate::market_data::realtime::BarSize::Sec5,
            crate::market_data::realtime::WhatToShow::Trades,
            false,
        )
        .expect("Failed to create subscription");

    // Get initial request count (should be 1 for the realtime bars request)
    let initial_count = message_bus.request_messages().len();
    assert_eq!(initial_count, 1, "Should have one request for realtime bars");

    // First cancel should add one more message
    subscription.cancel();
    let after_first_cancel = message_bus.request_messages().len();
    assert_eq!(after_first_cancel, 2, "Should have two messages after first cancel");

    // Second cancel should not send another message
    subscription.cancel();
    let after_second_cancel = message_bus.request_messages().len();
    assert_eq!(after_second_cancel, 2, "Should still have two messages after second cancel");

    // Drop should also not send another message (implicitly calls cancel)
    drop(subscription);
    let after_drop = message_bus.request_messages().len();
    assert_eq!(after_drop, 2, "Should still have two messages after drop");
}
