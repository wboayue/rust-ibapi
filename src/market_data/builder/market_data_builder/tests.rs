#[cfg(feature = "sync")]
mod sync_tests {
    use crate::client::sync::Client;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::Contract;
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_market_data_builder_default() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
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
    fn test_market_data_builder_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
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
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::Contract;
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn test_market_data_builder_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
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
    async fn test_market_data_builder_regulatory_snapshot_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
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
}
