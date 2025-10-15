#[cfg(feature = "sync")]
mod sync_tests {
    use crate::client::sync::Client;
    use crate::contracts::Contract;
    use crate::market_data::realtime::common::encoders::test_constants::*;
    use crate::stubs::MessageBusStub;
    use crate::{server_versions, ToField};
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

        let request = &request_messages[0];
        assert_eq!(
            request[MARKET_DATA_MSG_TYPE_IDX],
            crate::messages::OutgoingMessages::RequestMarketData.to_field()
        );

        // Check that generic_ticks is empty
        assert_eq!(request[MARKET_DATA_GENERIC_TICKS_IDX], "", "Generic ticks should be empty by default");

        // Check snapshot is false
        assert_eq!(request[MARKET_DATA_SNAPSHOT_IDX], "0", "Snapshot should be false by default");
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
        let request = &request_messages[0];

        // Check that generic_ticks contains our values
        assert_eq!(
            request[MARKET_DATA_GENERIC_TICKS_IDX], "233,236",
            "Generic ticks should contain specified values"
        );
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
        let request = &request_messages[0];

        // Check snapshot is true
        assert_eq!(request[MARKET_DATA_SNAPSHOT_IDX], "1", "Snapshot should be true");
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
        let request = &request_messages[0];

        // Check regulatory snapshot is true
        assert_eq!(request[MARKET_DATA_REGULATORY_SNAPSHOT_IDX], "1", "Regulatory snapshot should be true");
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
        let request = &request_messages[0];

        // Check snapshot is false (streaming mode)
        assert_eq!(request[MARKET_DATA_SNAPSHOT_IDX], "0", "Should be in streaming mode");
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
        let request = &request_messages[0];

        // Check all parameters
        assert_eq!(request[MARKET_DATA_GENERIC_TICKS_IDX], "100,101,104,106", "Generic ticks should be set");
        assert_eq!(request[MARKET_DATA_SNAPSHOT_IDX], "1", "Snapshot should be true");
        assert_eq!(request[MARKET_DATA_REGULATORY_SNAPSHOT_IDX], "1", "Regulatory snapshot should be true");
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::client::r#async::Client;
    use crate::contracts::Contract;
    use crate::market_data::realtime::common::encoders::test_constants::*;
    use crate::stubs::MessageBusStub;
    use crate::{server_versions, ToField};
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

        let request = &request_messages[0];
        assert_eq!(
            request[MARKET_DATA_MSG_TYPE_IDX],
            crate::messages::OutgoingMessages::RequestMarketData.to_field()
        );

        // Check parameters
        assert_eq!(request[MARKET_DATA_GENERIC_TICKS_IDX], "233,236", "Generic ticks should be set");
        assert_eq!(request[MARKET_DATA_SNAPSHOT_IDX], "1", "Snapshot should be true");
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
        let request = &request_messages[0];

        // Check regulatory snapshot is true
        assert_eq!(request[MARKET_DATA_REGULATORY_SNAPSHOT_IDX], "1", "Regulatory snapshot should be true");
    }
}
