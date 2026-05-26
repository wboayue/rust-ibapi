use prost::Message;

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
fn test_error_from_decoded_projects_to_notice() {
    // `From<DecodedError> for Error` projects to Error::Notice(Notice),
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
        crate::Error::Notice(notice) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "no security");
        }
        other => panic!("expected Error::Notice, got {other:?}"),
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

/// Order-message routing for message types that lack an order_id at the proto
/// level. `CompletedOrdersEnd` and `CommissionsReport` are order-routed but
/// have no `order_id` field, so the dispatcher falls back to the sentinel `-1`.
/// (Cases with a real `order_id` are covered by the per-type proto tests below.)
#[test]
fn test_order_message_routing_without_order_id_returns_sentinel() {
    let completed_orders_end =
        ResponseMessage::from_protobuf(IncomingMessages::CompletedOrdersEnd as i32, Vec::new(), crate::server_versions::PROTOBUF);
    match determine_routing(&completed_orders_end) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
    }

    let commission_report = ResponseMessage::from_protobuf(
        IncomingMessages::CommissionsReport as i32,
        crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec123".into()),
            ..Default::default()
        }
        .encode_to_vec(),
        crate::server_versions::PROTOBUF,
    );
    match determine_routing(&commission_report) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
    }
}

// Proto-form routing: exercises the `message.{order_id,request_id}` proto
// path through `determine_routing`.

#[test]
fn test_determine_routing_protobuf_open_order() {
    let bytes = crate::proto::OpenOrder {
        order_id: Some(58),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::OpenOrder as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 58),
        routing => panic!("Expected ByOrderId(58), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_order_status() {
    let bytes = crate::proto::OrderStatus {
        order_id: Some(58),
        status: Some("Filled".into()),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::OrderStatus as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 58),
        routing => panic!("Expected ByOrderId(58), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_execution_data_uses_nested_order_id() {
    // ExecutionData's tag 1 is req_id (-1 for unsolicited). The order_id is
    // nested under `execution.order_id`. Routing must pick the nested value.
    let bytes = crate::proto::ExecutionDetails {
        req_id: Some(-1),
        contract: None,
        execution: Some(crate::proto::Execution {
            order_id: Some(58),
            ..Default::default()
        }),
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ExecutionData as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 58),
        routing => panic!("Expected ByOrderId(58), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_execution_data_end() {
    let bytes = crate::proto::ExecutionDetailsEnd { req_id: Some(7) }.encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ExecutionDataEnd as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 7),
        routing => panic!("Expected ByOrderId(7), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_commissions_report_no_order_id() {
    // CommissionsReport has no order_id (in either proto or text); routing
    // falls back to ByOrderId(-1) and the dispatcher then reroutes via
    // execution_id.
    let bytes = crate::proto::CommissionAndFeesReport {
        exec_id: Some("0000e0d5.69fb6496.01.01".into()),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::CommissionsReport as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_request_id_message() {
    // AccountSummary uses ByRequestId and proto `req_id` lives at tag 1.
    let bytes = crate::proto::AccountSummary {
        req_id: Some(314),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::AccountSummary as i32, bytes, crate::server_versions::PROTOBUF);
    match determine_routing(&message) {
        RoutingDecision::ByRequestId(id) => assert_eq!(id, 314),
        routing => panic!("Expected ByRequestId(314), got {routing:?}"),
    }
}
