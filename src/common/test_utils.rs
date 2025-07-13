//! Test utilities shared across all modules for testing

#[cfg(test)]
pub mod helpers {
    use crate::stubs::MessageBusStub;
    use crate::{server_versions, Client};
    use std::sync::{Arc, RwLock};

    /// Creates a test client with an empty message bus
    pub fn create_test_client() -> (Client, Arc<MessageBusStub>) {
        create_test_client_with_version(server_versions::SIZE_RULES)
    }

    /// Creates a test client with a specific server version
    pub fn create_test_client_with_version(server_version: i32) -> (Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_version);
        (client, message_bus)
    }

    /// Creates a test client with specified response messages
    pub fn create_test_client_with_responses(responses: Vec<String>) -> (Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: responses,
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        (client, message_bus)
    }

    /// Creates a test client with specified response messages and server version
    pub fn create_test_client_with_responses_and_version(responses: Vec<String>, server_version: i32) -> (Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: responses,
        });
        let client = Client::stubbed(message_bus.clone(), server_version);
        (client, message_bus)
    }

    /// Asserts that the request messages match expected values
    pub fn assert_request_messages(message_bus: &MessageBusStub, expected: &[&str]) {
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(
            request_messages.len(),
            expected.len(),
            "Expected {} request messages, got {}",
            expected.len(),
            request_messages.len()
        );

        for (i, expected_msg) in expected.iter().enumerate() {
            assert_eq!(request_messages[i].encode_simple(), *expected_msg, "Request message {} mismatch", i);
        }
    }

    /// Gets request messages from the message bus
    pub fn get_request_messages(message_bus: &MessageBusStub) -> Vec<String> {
        message_bus
            .request_messages
            .read()
            .unwrap()
            .iter()
            .map(|msg| msg.encode_simple())
            .collect()
    }

    /// Asserts that a request message contains a specific substring
    pub fn assert_request_contains(message_bus: &MessageBusStub, index: usize, substring: &str) {
        let request_messages = get_request_messages(message_bus);
        assert!(
            request_messages.len() > index,
            "Expected at least {} request messages, got {}",
            index + 1,
            request_messages.len()
        );
        assert!(
            request_messages[index].contains(substring),
            "Request message {} does not contain '{}'. Actual: '{}'",
            index,
            substring,
            request_messages[index]
        );
    }

    /// Common test constants that can be used across modules
    pub mod constants {
        /// Test account identifiers
        pub const TEST_ACCOUNT: &str = "DU1234567";
        pub const TEST_ACCOUNT_2: &str = "DU7654321";
        pub const TEST_ACCOUNT_3: &str = "DU9876543";

        /// Test model codes
        pub const TEST_MODEL_CODE: &str = "TARGET2024";
        pub const TEST_MODEL_CODE_2: &str = "GROWTH2024";

        /// Test contract IDs
        pub const TEST_CONTRACT_ID: i32 = 1001;
        pub const TEST_CONTRACT_ID_2: i32 = 2002;

        /// Test order IDs
        pub const TEST_ORDER_ID: i32 = 5001;
        pub const TEST_ORDER_ID_2: i32 = 5002;

        /// Test ticker IDs
        pub const TEST_TICKER_ID: i32 = 100;
        pub const TEST_TICKER_ID_2: i32 = 200;
    }
}

/// Re-export constants at module level for easier access
#[cfg(test)]
pub use helpers::constants::*;
