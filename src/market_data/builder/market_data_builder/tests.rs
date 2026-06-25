#[cfg(feature = "sync")]
mod sync_tests {
    use crate::client::sync::Client;
    use crate::common::test_utils::helpers::{assert_proto_msg_id, assert_request, proto_response, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::realtime::TickTypes;
    use crate::messages::{IncomingMessages, OutgoingMessages};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::{market_data_request, tick_price, tick_size, tick_snapshot_end};
    use crate::testdata::builders::ResponseProtoEncoder;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    #[test]
    fn test_market_data_builder_default() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client.market_data(&contract).subscribe().expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_market_data_builder_with_generic_ticks() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .generic_ticks(&["233", "236"])
            .subscribe()
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_market_data_builder_add_generic_tick_appends() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .add_generic_tick("233")
            .add_generic_tick("236")
            .subscribe()
            .expect("Failed to create subscription");

        assert_request(
            &message_bus,
            0,
            &market_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .generic_ticks(&["233", "236"]),
        );
    }

    #[test]
    fn test_market_data_builder_add_generic_tick_after_bulk() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .generic_ticks(&["233"])
            .add_generic_tick("236")
            .subscribe()
            .expect("Failed to create subscription");

        assert_request(
            &message_bus,
            0,
            &market_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .generic_ticks(&["233", "236"]),
        );
    }

    #[test]
    fn test_market_data_builder_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .snapshot()
            .subscribe()
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_market_data_builder_regulatory_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_SMART_COMPONENTS);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .regulatory_snapshot()
            .subscribe()
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_market_data_builder_streaming_after_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .snapshot() // First set to snapshot
            .streaming() // Then back to streaming
            .subscribe()
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_market_data_builder_full_configuration() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_SMART_COMPONENTS);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .generic_ticks(&["100", "101", "104", "106"])
            .snapshot()
            .regulatory_snapshot()
            .subscribe()
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_snapshot_once_collects_until_snapshot_end() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
            proto_response(IncomingMessages::TickPrice, tick_price().tick_type(4).price(185.50).encode_proto()),
            proto_response(IncomingMessages::TickSize, tick_size().tick_type(5).size(100.0).encode_proto()),
            proto_response(IncomingMessages::TickSnapshotEnd, tick_snapshot_end().encode_proto()),
        ]));
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let ticks = client
            .market_data(&contract)
            .snapshot_once(Duration::from_secs(30))
            .expect("snapshot_once failed");

        // Two data ticks collected; the SnapshotEnd sentinel is excluded.
        assert_eq!(ticks.len(), 2, "Should collect both ticks before the snapshot end");
        assert!(matches!(ticks[0], TickTypes::Price(_)));
        assert!(matches!(ticks[1], TickTypes::Size(_)));

        // snapshot_once forces snapshot mode regardless of prior builder state.
        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::{assert_proto_msg_id, assert_request, proto_response, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::realtime::TickTypes;
    use crate::messages::{IncomingMessages, OutgoingMessages};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::{market_data_request, tick_price, tick_size, tick_snapshot_end};
    use crate::testdata::builders::ResponseProtoEncoder;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    #[tokio::test]
    async fn test_market_data_builder_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .generic_ticks(&["233", "236"])
            .snapshot()
            .subscribe()
            .await
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[tokio::test]
    async fn test_market_data_builder_add_generic_tick_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .add_generic_tick("233")
            .add_generic_tick("236")
            .subscribe()
            .await
            .expect("Failed to create subscription");

        assert_request(
            &message_bus,
            0,
            &market_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .generic_ticks(&["233", "236"]),
        );
    }

    #[tokio::test]
    async fn test_market_data_builder_regulatory_snapshot_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_SMART_COMPONENTS);
        let contract = Contract::stock("AAPL").build();

        let _subscription = client
            .market_data(&contract)
            .regulatory_snapshot()
            .subscribe()
            .await
            .expect("Failed to create subscription");

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }

    #[tokio::test]
    async fn test_snapshot_once_collects_until_snapshot_end() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
            proto_response(IncomingMessages::TickPrice, tick_price().tick_type(4).price(185.50).encode_proto()),
            proto_response(IncomingMessages::TickSize, tick_size().tick_type(5).size(100.0).encode_proto()),
            proto_response(IncomingMessages::TickSnapshotEnd, tick_snapshot_end().encode_proto()),
        ]));
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let ticks = client
            .market_data(&contract)
            .snapshot_once(Duration::from_secs(30))
            .await
            .expect("snapshot_once failed");

        // Two data ticks collected; the SnapshotEnd sentinel is excluded.
        assert_eq!(ticks.len(), 2, "Should collect both ticks before the snapshot end");
        assert!(matches!(ticks[0], TickTypes::Price(_)));
        assert!(matches!(ticks[1], TickTypes::Size(_)));

        let request_messages = message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketData);
    }
}
