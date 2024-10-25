use super::*;
use crate::contracts::{ComboLeg, DeltaNeutralContract, SecurityType};

#[test]
fn test_basic_market_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Tick Price message
            "1|2|9001|1|185.50|100|7|".to_owned(),
            // Tick Size message
            "2|2|9001|0|150|".to_owned(),
            // Tick String message
            "46|2|9001|45|2023-03-13 09:30:00|".to_owned(),
            // Tick Generic message
            "45|2|9001|23|20.5|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL");
    let generic_ticks = &["100", "101", "104", "106"]; // Option Volume, OI, Historical Vol, Implied Vol
    let snapshot = false;
    let regulatory_snapshot = false;

    // Test subscription creation
    let result = client.market_data(&contract, generic_ticks, snapshot, regulatory_snapshot);

    // Test receiving data
    let subscription = result.expect("Failed to create market data subscription");
    let received_ticks: Vec<TickTypes> = subscription.iter().take(4).collect();

    assert_eq!(received_ticks.len(), 4, "Should receive 4 market data updates");

    // Verify different tick types
    for tick in received_ticks {
        match tick {
            TickTypes::Price(tick) => {
                assert_eq!(tick.tick_type, TickType::Bid, "Wrong tick type");
                assert_eq!(tick.price, 185.50, "Wrong price");
                assert_eq!(tick.attributes.can_auto_execute, true, "Wrong can auto execute flag");
            }
            TickTypes::Size(tick) => {
                assert_eq!(tick.tick_type, TickType::BidSize, "Wrong tick type");
                assert_eq!(tick.size, 150.0, "Wrong size");
            }
            TickTypes::PriceSize(tick) => {
                assert_eq!(tick.price_tick_type, TickType::Bid, "Wrong tick type");
                assert_eq!(tick.price, 185.50, "Wrong price");
                assert_eq!(tick.attributes.can_auto_execute, false, "Wrong can auto execute flag");
                assert_eq!(tick.size_tick_type, TickType::BidSize, "Wrong tick type");
                assert_eq!(tick.size, 100.0, "Wrong size");
            }
            TickTypes::String(tick) => {
                assert_eq!(tick.tick_type, TickType::LastTimestamp, "Wrong tick type");
                assert_eq!(tick.value, "2023-03-13 09:30:00", "Wrong timestamp");
            }
            TickTypes::Generic(tick) => {
                assert_eq!(tick.tick_type, TickType::OptionHistoricalVol, "Wrong tick type");
                assert_eq!(tick.value, 20.5, "Wrong value");
            }
            tick => panic!("Unexpected tick type received: {:?}", tick),
        }
    }

    // Verify request message
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    let request = &request_messages[0];
    assert_eq!(request[0], OutgoingMessages::RequestMarketData.to_field(), "Wrong message type");
    assert_eq!(request[1], "11", "Wrong version");
    assert_eq!(request[16], "100,101,104,106", "Wrong generic ticks");
    assert_eq!(request[17], snapshot.to_field(), "Wrong snapshot flag");
}

#[test]
fn test_market_data_with_combo_legs() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["1|2|9001|1|185.50|100|7|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let mut contract = Contract::stock("AAPL");
    contract.security_type = SecurityType::Spread;
    contract.combo_legs = vec![
        ComboLeg {
            contract_id: 1,
            ratio: 1,
            action: "BUY".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
        ComboLeg {
            contract_id: 2,
            ratio: 1,
            action: "SELL".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
    ];

    let generic_ticks = &["100"];
    let snapshot = false;
    let regulatory_snapshot = false;

    let market_data = client
        .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
        .expect("Failed to create market data subscription");

    // Verify request message contains combo legs
    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];

    // Find combo legs section in the message
    let combo_legs_count_index = 15;
    assert_eq!(request[combo_legs_count_index], "2", "Wrong combo legs count");

    // Verify first leg
    assert_eq!(request[combo_legs_count_index + 1], "1", "Wrong first leg contract id");
    assert_eq!(request[combo_legs_count_index + 2], "1", "Wrong first leg ratio");
    assert_eq!(request[combo_legs_count_index + 3], "BUY", "Wrong first leg action");
    assert_eq!(request[combo_legs_count_index + 4], "SMART", "Wrong first leg exchange");

    // Verify second leg
    assert_eq!(request[combo_legs_count_index + 5], "2", "Wrong second leg contract id");
    assert_eq!(request[combo_legs_count_index + 6], "1", "Wrong second leg ratio");
    assert_eq!(request[combo_legs_count_index + 7], "SELL", "Wrong second leg action");
    assert_eq!(request[combo_legs_count_index + 8], "SMART", "Wrong second leg exchange");
}

#[test]
fn test_market_data_with_delta_neutral() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["1|2|9001|1|185.50|100|7|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let mut contract = Contract::stock("AAPL");
    contract.delta_neutral_contract = Some(DeltaNeutralContract {
        contract_id: 12345,
        delta: 0.5,
        price: 100.0,
    });

    let generic_ticks = &["100"];
    let snapshot = false;
    let regulatory_snapshot = false;

    let _ = client
        .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
        .expect("Failed to create market data subscription");

    // Verify request message contains delta neutral contract
    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];

    // Find delta neutral marker in message
    let delta_neutral_index = 15;

    assert_eq!(request[delta_neutral_index + 1], "12345", "Wrong delta neutral contract id");
    assert_eq!(request[delta_neutral_index + 2], "0.5", "Wrong delta");
    assert_eq!(request[delta_neutral_index + 3], "100", "Wrong price");
}

#[test]
fn test_market_data_regulatory_snapshot() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    // Test with server version that supports regulatory snapshots
    let client = Client::stubbed(message_bus, server_versions::REQ_SMART_COMPONENTS);
    let contract = Contract::stock("AAPL");

    let _ = client
        .market_data(
            &contract,
            &[],
            false,
            true, // regulatory snapshot
        )
        .expect("Failed to create market data subscription");

    let request_messages = client.message_bus.request_messages();
    let request = &request_messages[0];
    assert_eq!(request[18], "1", "Regulatory snapshot flag should be set");
}

#[test]
fn test_market_data_error_handling() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "4|2|9001|123|Error Message|".to_owned(), // Error message
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL");

    let subscription = client
        .market_data(&contract, &[], false, false)
        .expect("Failed to create market data subscription");

    let received_messages: Vec<TickTypes> = subscription.iter().take(1).collect();
    assert_eq!(received_messages.len(), 1, "Should receive error message");

    match &received_messages[0] {
        TickTypes::Notice(notice) => {
            assert_eq!(notice.code, 123, "Wrong error code");
            assert_eq!(notice.message, "Error Message", "Wrong error message");
        }
        _ => panic!("Expected error notice"),
    }
}
