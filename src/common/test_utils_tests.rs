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

#[test]
fn assert_request_proto_matches_expected_body() {
    use crate::proto::ManagedAccountsRequest;
    use prost::Message;

    let (_client, message_bus) = create_test_client();

    let expected = ManagedAccountsRequest::default();
    let mut body = Vec::new();
    expected.encode(&mut body).unwrap();

    {
        let mut request_messages = message_bus.request_messages.write().unwrap();
        request_messages.push(encode_protobuf_message(OutgoingMessages::RequestManagedAccounts as i32, &body));
    }

    assert_request_proto(&message_bus, 0, OutgoingMessages::RequestManagedAccounts, &expected);
}

#[test]
fn assert_request_helper_resolves_msg_id_from_builder() {
    use crate::testdata::builders::{positions::request_positions_multi, RequestEncoder};

    let (_client, message_bus) = create_test_client();

    let builder = request_positions_multi().account("DU9999999").model_code("TARGET2024");

    {
        let mut request_messages = message_bus.request_messages.write().unwrap();
        request_messages.push(builder.encode_request());
    }

    assert_request(&message_bus, 0, &builder);
}

#[test]
#[should_panic(expected = "request 0 body mismatch")]
fn assert_request_helper_panics_on_body_mismatch() {
    use crate::testdata::builders::{positions::request_positions_multi, RequestEncoder};

    let (_client, message_bus) = create_test_client();

    let on_wire = request_positions_multi().account("DU0000001");
    {
        let mut request_messages = message_bus.request_messages.write().unwrap();
        request_messages.push(on_wire.encode_request());
    }

    let expected = request_positions_multi().account("DU0000002");
    assert_request(&message_bus, 0, &expected);
}

#[test]
#[should_panic(expected = "request 0 body mismatch")]
fn assert_request_proto_panics_on_body_mismatch() {
    use crate::proto::AccountSummaryRequest;
    use prost::Message;

    let (_client, message_bus) = create_test_client();

    let on_wire = AccountSummaryRequest {
        req_id: Some(7),
        ..Default::default()
    };
    let mut body = Vec::new();
    on_wire.encode(&mut body).unwrap();

    {
        let mut request_messages = message_bus.request_messages.write().unwrap();
        request_messages.push(encode_protobuf_message(OutgoingMessages::RequestAccountSummary as i32, &body));
    }

    let expected = AccountSummaryRequest {
        req_id: Some(99),
        ..Default::default()
    };
    assert_request_proto(&message_bus, 0, OutgoingMessages::RequestAccountSummary, &expected);
}
