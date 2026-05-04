use super::*;
use crate::messages::ResponseMessage;

#[test]
fn test_decoded_error_default() {
    // Manual Default impl: request_id falls back to UNSPECIFIED_REQUEST_ID,
    // not i32::default (0). Guards the silent regression that swapped these.
    let d = DecodedError::default();
    assert_eq!(d.request_id, UNSPECIFIED_REQUEST_ID);
    assert_eq!(d.error_code, 0);
    assert_eq!(d.error_message, "");
    assert_eq!(d.error_time, None);
    assert_eq!(d.advanced_order_reject_json, "");
}

#[test]
fn test_notice_from_decoded_preserves_rich_payload() {
    use crate::messages::Notice;
    use time::OffsetDateTime;

    let payload = DecodedError {
        request_id: 42,
        error_code: 2104,
        error_message: "Market data farm OK".into(),
        error_time: Some(1_700_000_000_000),
        advanced_order_reject_json: "{\"reject\":1}".into(),
    };
    let notice = Notice::from(payload);

    assert_eq!(notice.code, 2104);
    assert_eq!(notice.message, "Market data farm OK");
    assert_eq!(notice.advanced_order_reject_json, "{\"reject\":1}");
    let expected = OffsetDateTime::from_unix_timestamp_nanos(1_700_000_000_000_i128 * 1_000_000).unwrap();
    assert_eq!(notice.error_time, Some(expected));
}

#[test]
fn test_notice_from_decoded_missing_optionals() {
    use crate::messages::Notice;

    // Old format: error_time absent, JSON empty. Conversion preserves both.
    let payload = DecodedError {
        request_id: -1,
        error_code: 200,
        error_message: "no security".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let notice = Notice::from(payload);

    assert_eq!(notice.code, 200);
    assert_eq!(notice.error_time, None);
    assert_eq!(notice.advanced_order_reject_json, "");
}

#[test]
fn test_error_from_decoded_projects_to_message() {
    // `From<DecodedError> for Error` projects code+message to Error::Message,
    // mirroring the existing `From<ResponseMessage>` projection.
    let payload = DecodedError {
        request_id: 42,
        error_code: 200,
        error_message: "no security".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let err = crate::Error::from(payload);

    match err {
        crate::Error::Message(code, msg) => {
            assert_eq!(code, 200);
            assert_eq!(msg, "no security");
        }
        other => panic!("expected Error::Message, got {other:?}"),
    }
}

#[test]
fn test_determine_routing_error_protobuf_malformed() {
    // Garbage bytes that aren't a valid ErrorMessage proto fall back to Default,
    // which sets request_id = UNSPECIFIED_REQUEST_ID (not 0).
    let raw_bytes = vec![0xFFu8; 16];
    let message = ResponseMessage::from_protobuf(IncomingMessages::Error as i32, raw_bytes, crate::server_versions::PROTOBUF);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, UNSPECIFIED_REQUEST_ID);
            assert_eq!(payload.error_code, 0);
            assert_eq!(payload.error_message, "");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_by_request_id() {
    // Create a mock message with request ID (AccountSummary = 63)
    let message_str = "63\01\0123\0DU123456\0AccountType\0ADVISOR\0USD\0";
    let message = ResponseMessage::from(message_str);

    match determine_routing(&message) {
        RoutingDecision::ByRequestId(id) => assert_eq!(id, 123),
        routing => panic!("Expected ByRequestId routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_error_old_format() {
    // Old format (server_version < ERROR_TIME): message_type|version|request_id|error_code|error_msg
    let message_str = "4\02\0123\0200\0No security definition found\0";
    let message = ResponseMessage::from(message_str);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, 123);
            assert_eq!(payload.error_code, 200);
            assert_eq!(payload.error_message, "No security definition found");
            assert_eq!(payload.error_time, None, "old format has no error_time field");
            assert_eq!(payload.advanced_order_reject_json, "");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_error_new_format() {
    // New format (server_version >= ERROR_TIME): msg_type|request_id|error_code|error_msg|advanced|error_time
    let message_str = "4\0123\0200\0No security definition found\0{\"reject\":1}\01700000000000\0";
    let message = ResponseMessage::from(message_str).with_server_version(crate::server_versions::ERROR_TIME);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, 123);
            assert_eq!(payload.error_code, 200);
            assert_eq!(payload.error_message, "No security definition found");
            assert_eq!(payload.error_time, Some(1700000000000));
            assert_eq!(payload.advanced_order_reject_json, "{\"reject\":1}");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_warning_text_format() {
    // Text-format warning (code 2104, in WARNING_CODE_RANGE) — message text is captured.
    let message_str = "4\02\042\02104\0Market data farm connection is OK:usfarm\0";
    let message = ResponseMessage::from(message_str);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, 42);
            assert_eq!(payload.error_code, 2104);
            assert_eq!(payload.error_message, "Market data farm connection is OK:usfarm");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_error_protobuf() {
    // Protobuf Error with id=42 and error_code=2100 — full decode populates all five fields.
    let envelope = crate::proto::ErrorMessage {
        id: Some(42),
        error_time: Some(1700000000000),
        error_code: Some(2100),
        error_msg: Some("Market data farm connection is OK".to_string()),
        advanced_order_reject_json: Some("{\"hint\":\"check filters\"}".to_string()),
    };
    let mut raw_bytes = Vec::new();
    prost::Message::encode(&envelope, &mut raw_bytes).expect("encode error envelope");

    let message = ResponseMessage::from_protobuf(IncomingMessages::Error as i32, raw_bytes, crate::server_versions::PROTOBUF);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, 42);
            assert_eq!(payload.error_code, 2100);
            assert_eq!(payload.error_message, "Market data farm connection is OK");
            assert_eq!(payload.error_time, Some(1700000000000));
            assert_eq!(payload.advanced_order_reject_json, "{\"hint\":\"check filters\"}");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_error_protobuf_unspecified_id() {
    // Protobuf Error with no id (global notice) decodes to UNSPECIFIED_REQUEST_ID.
    let envelope = crate::proto::ErrorMessage {
        id: None,
        error_time: None,
        error_code: Some(2104),
        error_msg: Some("Market data farm connection is OK".to_string()),
        advanced_order_reject_json: None,
    };
    let mut raw_bytes = Vec::new();
    prost::Message::encode(&envelope, &mut raw_bytes).expect("encode error envelope");

    let message = ResponseMessage::from_protobuf(IncomingMessages::Error as i32, raw_bytes, crate::server_versions::PROTOBUF);

    match determine_routing(&message) {
        RoutingDecision::Error(payload) => {
            assert_eq!(payload.request_id, UNSPECIFIED_REQUEST_ID);
            assert_eq!(payload.error_code, 2104);
            assert_eq!(payload.error_message, "Market data farm connection is OK");
            assert_eq!(payload.error_time, None);
            assert_eq!(payload.advanced_order_reject_json, "");
        }
        routing => panic!("Expected Error routing, got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_shared_message() {
    // ManagedAccounts message (type 15)
    let message_str = "15\01\0DU123456,DU234567\0";
    let message = ResponseMessage::from(message_str);

    match determine_routing(&message) {
        RoutingDecision::SharedMessage(msg_type) => {
            assert_eq!(msg_type, IncomingMessages::ManagedAccounts);
        }
        routing => panic!("Expected SharedMessage routing, got {routing:?}"),
    }
}

#[test]
fn test_is_warning_error() {
    // Test range boundaries
    assert!(is_warning_error(2100));
    assert!(is_warning_error(2169));

    // Test some values in the middle
    assert!(is_warning_error(2119));
    assert!(is_warning_error(2150));

    // Test values outside the range
    assert!(!is_warning_error(2099));
    assert!(!is_warning_error(2170));
    assert!(!is_warning_error(200));
    assert!(!is_warning_error(2200));
}

#[test]
fn test_order_message_routing() {
    // Test OpenOrder with order ID at position 1
    let message_str = "5\0123\0AAPL\0STK\0"; // OpenOrder with order_id=123
    let message = ResponseMessage::from(message_str);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 123),
        routing => panic!("Expected ByOrderId routing, got {routing:?}"),
    }

    // Test CompletedOrdersEnd (no order ID)
    let message_str = "102\01\0"; // CompletedOrdersEnd
    let message = ResponseMessage::from(message_str);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
    }

    // Test ExecutionData with order ID at position 2
    let message_str = "11\01\0123\0456\0"; // ExecutionData with request_id=1, order_id=123
    let message = ResponseMessage::from(message_str);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 123),
        routing => panic!("Expected ByOrderId routing, got {routing:?}"),
    }

    // Test CommissionsReport (no order ID but still an order message)
    let message_str = "59\01\0exec123\0100.0\0USD\0"; // CommissionsReport
    let message = ResponseMessage::from(message_str);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
    }
}
