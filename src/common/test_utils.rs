//! Test utilities shared across all modules for testing

#[cfg(test)]
#[allow(dead_code)] // These utilities will be used by other modules
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

    #[cfg(feature = "sync")]
    pub fn create_blocking_test_client() -> (crate::client::blocking::Client, Arc<MessageBusStub>) {
        create_blocking_test_client_with_version(server_versions::SIZE_RULES)
    }

    #[cfg(feature = "sync")]
    pub fn create_blocking_test_client_with_version(server_version: i32) -> (crate::client::blocking::Client, Arc<MessageBusStub>) {
        create_blocking_test_client_with_responses_and_version(vec![], server_version)
    }

    #[cfg(feature = "sync")]
    pub fn create_blocking_test_client_with_responses(responses: Vec<String>) -> (crate::client::blocking::Client, Arc<MessageBusStub>) {
        create_blocking_test_client_with_responses_and_version(responses, server_versions::SIZE_RULES)
    }

    #[cfg(feature = "sync")]
    pub fn create_blocking_test_client_with_responses_and_version(
        responses: Vec<String>,
        server_version: i32,
    ) -> (crate::client::blocking::Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: responses,
        });
        let client = crate::client::blocking::Client::stubbed(message_bus.clone(), server_version);
        (client, message_bus)
    }

    /// Asserts that the nth request message has the expected protobuf message ID
    pub fn assert_request_msg_id(message_bus: &MessageBusStub, index: usize, expected: crate::messages::OutgoingMessages) {
        let request_messages = message_bus.request_messages.read().unwrap();
        assert!(
            request_messages.len() > index,
            "Expected at least {} request messages, got {}",
            index + 1,
            request_messages.len()
        );
        assert_proto_msg_id(&request_messages[index], expected);
    }

    /// Gets request message count from the message bus
    pub fn request_message_count(message_bus: &MessageBusStub) -> usize {
        message_bus.request_messages.read().unwrap().len()
    }

    /// Decodes a protobuf request message (skips 4-byte msg_id header)
    pub fn decode_request_proto<T: prost::Message + Default>(message_bus: &MessageBusStub, index: usize) -> T {
        let request_messages = message_bus.request_messages.read().unwrap();
        T::decode(&request_messages[index][4..]).unwrap()
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

    /// Re-export constants at module level for easier access
    pub use constants::*;

    /// Asserts the first 4 bytes of a protobuf-encoded message match the expected OutgoingMessages variant + 200 offset.
    pub fn assert_proto_msg_id(bytes: &[u8], expected: crate::messages::OutgoingMessages) {
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, expected as i32 + 200);
    }

    /// Counts how many messages in `messages` carry the given protobuf message id (variant + 200 offset).
    pub fn count_proto_msgs(messages: &[Vec<u8>], expected: crate::messages::OutgoingMessages) -> usize {
        let target = expected as i32 + 200;
        messages
            .iter()
            .filter(|m| m.len() >= 4 && i32::from_be_bytes([m[0], m[1], m[2], m[3]]) == target)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use crate::server_versions;

    #[test]
    fn test_create_test_client() {
        let (client, message_bus) = create_test_client();
        assert_eq!(client.server_version(), server_versions::SIZE_RULES);
        assert!(message_bus.request_messages.read().unwrap().is_empty());
        assert!(message_bus.response_messages.is_empty());
    }

    #[test]
    fn test_create_test_client_with_version() {
        let custom_version = 150;
        let (client, message_bus) = create_test_client_with_version(custom_version);
        assert_eq!(client.server_version(), custom_version);
        assert!(message_bus.request_messages.read().unwrap().is_empty());
        assert!(message_bus.response_messages.is_empty());
    }

    #[test]
    fn test_create_test_client_with_responses() {
        let responses = vec!["1|2|123|".to_string(), "2|2|456|".to_string()];
        let (client, message_bus) = create_test_client_with_responses(responses.clone());
        assert_eq!(client.server_version(), server_versions::SIZE_RULES);
        assert_eq!(message_bus.response_messages, responses);
    }

    #[test]
    fn test_assert_request_msg_id() {
        let (_client, message_bus) = create_test_client();

        {
            let mut request_messages = message_bus.request_messages.write().unwrap();
            request_messages.push(encode_protobuf_message(OutgoingMessages::RequestAccountSummary as i32, &[]));
        }

        assert_request_msg_id(&message_bus, 0, OutgoingMessages::RequestAccountSummary);
    }

    #[test]
    fn test_request_message_count() {
        let (_client, message_bus) = create_test_client();

        assert_eq!(request_message_count(&message_bus), 0);

        {
            let mut request_messages = message_bus.request_messages.write().unwrap();
            request_messages.push(encode_protobuf_message(1, &[]));
            request_messages.push(encode_protobuf_message(2, &[]));
        }

        assert_eq!(request_message_count(&message_bus), 2);
    }

    #[test]
    fn test_constants() {
        assert_eq!(TEST_ACCOUNT, "DU1234567");
        assert_eq!(TEST_CONTRACT_ID, 1001);
        assert_eq!(TEST_ORDER_ID, 5001);
        assert_eq!(TEST_TICKER_ID, 100);
    }
}
