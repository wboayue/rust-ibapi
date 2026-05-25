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
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_version);
        (client, message_bus)
    }

    /// Creates a test client with specified response messages
    pub fn create_test_client_with_responses(responses: Vec<String>) -> (Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: responses,
            ordered_responses: vec![],
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        (client, message_bus)
    }

    /// Creates a test client with specified response messages and server version
    pub fn create_test_client_with_responses_and_version(responses: Vec<String>, server_version: i32) -> (Client, Arc<MessageBusStub>) {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: responses,
            ordered_responses: vec![],
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
            ordered_responses: vec![],
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

    /// Asserts that the nth request matches the expected message id AND decodes to `expected`.
    /// Strict counterpart to `assert_request_msg_id`, which only checks the 4-byte header.
    pub fn assert_request_proto<T>(message_bus: &MessageBusStub, index: usize, expected_msg_id: crate::messages::OutgoingMessages, expected: &T)
    where
        T: prost::Message + Default + PartialEq + std::fmt::Debug,
    {
        assert_request_msg_id(message_bus, index, expected_msg_id);
        let actual: T = decode_request_proto(message_bus, index);
        assert_eq!(&actual, expected, "request {index} body mismatch");
    }

    /// Builder-aware variant of [`assert_request_proto`]: pulls the expected message id and
    /// proto body from the builder's `RequestEncoder` impl, so tests don't repeat the msg id.
    pub fn assert_request<B: crate::testdata::builders::RequestEncoder>(message_bus: &MessageBusStub, index: usize, expected: &B) {
        assert_request_proto(message_bus, index, B::MSG_ID, &expected.to_proto());
    }

    /// Build a text-format `ResponseMessage` for use with
    /// [`MessageBusStub::with_ordered_responses`]. Accepts pipe-delimited
    /// builder output (`encode_pipe()`) or raw NUL-delimited literals.
    pub fn text_response(s: impl Into<String>) -> crate::messages::ResponseMessage {
        crate::messages::ResponseMessage::from(&s.into().replace('|', "\0"))
    }

    /// Build a proto-framed `ResponseMessage` for use with
    /// [`MessageBusStub::with_ordered_responses`]. Pairs with
    /// `Builder::encode_proto()`.
    pub fn proto_response(msg_type: crate::messages::IncomingMessages, bytes: Vec<u8>) -> crate::messages::ResponseMessage {
        crate::messages::ResponseMessage::from_protobuf(msg_type as i32, bytes, server_versions::PROTOBUF_REST_MESSAGES_3)
    }

    /// Build a proto-framed wire payload (4-byte BE `msg_id + PROTOBUF_MSG_ID`
    /// followed by `proto.encode_to_vec()`). For `MemoryStream::push_inbound`
    /// and `spawn_handshake_listener` fixtures that need raw bytes, not a
    /// parsed `ResponseMessage`.
    pub fn binary_proto<M: prost::Message>(msg_id: i32, proto: &M) -> Vec<u8> {
        crate::messages::encode_protobuf_message(msg_id, &proto.encode_to_vec())
    }

    /// `NextValidId` proto-framed handshake frame.
    pub fn next_valid_id_frame(order_id: i32) -> Vec<u8> {
        binary_proto(
            crate::messages::IncomingMessages::NextValidId as i32,
            &crate::proto::NextValidId { order_id: Some(order_id) },
        )
    }

    /// `ManagedAccounts` proto-framed handshake frame.
    pub fn managed_accounts_frame(accounts: &str) -> Vec<u8> {
        binary_proto(
            crate::messages::IncomingMessages::ManagedAccounts as i32,
            &crate::proto::ManagedAccounts {
                accounts_list: Some(accounts.to_string()),
            },
        )
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

        /// First request_id assigned by `Client::next_request_id()`. Mirrors
        /// `client::id_generator::INITIAL_REQUEST_ID` for assertions in tests
        /// that don't have direct access to that private constant.
        pub const TEST_REQ_ID_FIRST: i32 = 9000;
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

    /// Builds an `Error::Notice` carrying a synthesized [`Notice`](crate::messages::Notice)
    /// — no wire timestamp, no advanced-order-reject JSON. Test-only sugar for the
    /// `Error::Notice(Notice::synthesized(code, msg))` shape used by Result-path tests
    /// (production code never builds these; the wire path goes through
    /// `From<ResponseMessage> for Error`).
    pub fn tws_error_notice(code: i32, message: impl Into<String>) -> crate::Error {
        crate::Error::Notice(crate::messages::Notice::synthesized(code, message.into()))
    }

    /// Asserts that `err` is `Error::Notice(notice)` where `notice.code == expected_code`
    /// and `notice.message` contains `expected_substring`.
    pub fn assert_tws_error_message(err: crate::Error, expected_code: i32, expected_substring: &str) {
        match err {
            crate::Error::Notice(notice) => {
                assert_eq!(notice.code, expected_code, "wrong error code");
                assert!(
                    notice.message.contains(expected_substring),
                    "error message {:?} does not contain {expected_substring:?}",
                    notice.message
                );
            }
            other => panic!("expected Error::Notice(code={expected_code}), got {other:?}"),
        }
    }
}

/// Generic round-trip / reject-unknown helpers for typed wire enums built with `impl_wire_enum!`.
#[cfg(test)]
#[allow(dead_code)] // Consumers grow as the typed-status sweep lands.
pub mod wire_enum {
    /// Assert `Display`, `FromStr`, and `ToField` agree on a hand-written
    /// `(variant, wire)` table. One helper covers every trait impl generated
    /// by `impl_wire_enum!` — independent verification (the table is not
    /// derived from `as_str()`, so a typo in either direction surfaces).
    pub fn check_wire_enum_round_trip<T>(table: &[(T, &'static str)])
    where
        T: Copy + std::fmt::Display + std::fmt::Debug + PartialEq + std::str::FromStr<Err = crate::Error> + crate::ToField,
    {
        for &(variant, wire) in table {
            assert_eq!(variant.to_string(), wire, "Display for {variant:?}");
            assert_eq!(T::from_str(wire).unwrap(), variant, "FromStr({wire})");
            assert_eq!(variant.to_field(), wire, "ToField for {variant:?}");
        }
    }

    /// Assert every input string in `unknowns` produces `Err(Error::Parse(..))`.
    pub fn check_wire_enum_rejects_unknown<T>(unknowns: &[&str])
    where
        T: std::str::FromStr<Err = crate::Error> + std::fmt::Debug,
    {
        for &s in unknowns {
            let err = T::from_str(s);
            assert!(
                matches!(err, Err(crate::Error::Parse(_, _, _))),
                "expected Parse error for {s:?}, got {err:?}",
            );
        }
    }
}

#[cfg(test)]
#[path = "test_utils_tests.rs"]
mod tests;
