use prost::Message;

use super::*;
use crate::common::test_utils::helpers::proto_response;
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
    let message = proto_response(IncomingMessages::Error, raw_bytes);

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

    let message = proto_response(IncomingMessages::Error, raw_bytes);

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

    let message = proto_response(IncomingMessages::Error, raw_bytes);

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
    assert!(is_warning_error(2100, ""));
    assert!(is_warning_error(2169, ""));

    // Test some values in the middle
    assert!(is_warning_error(2119, ""));
    assert!(is_warning_error(2150, ""));

    // Test values outside the range
    assert!(!is_warning_error(2099, ""));
    assert!(!is_warning_error(2170, ""));
    assert!(!is_warning_error(200, ""));
    assert!(!is_warning_error(2200, ""));
}

#[test]
fn test_is_warning_error_data_advisory_codes() {
    // Delayed-data advisories: the request proceeds and data follows.
    for code in DATA_ADVISORY_CODES {
        assert!(is_warning_error(code, ""), "advisory code {code} should route as a warning");

        // Neighboring codes are real errors, not advisories (10089 and
        // 10090 are adjacent, so skip neighbors that are advisories too).
        for neighbor in [code - 1, code + 1] {
            if DATA_ADVISORY_CODES.contains(&neighbor) {
                continue;
            }
            assert!(!is_warning_error(neighbor, ""), "code {neighbor} should not route as a warning");
        }
    }
}

#[test]
fn test_is_warning_error_classifies_order_message_from_text() {
    assert!(is_warning_error(
        399,
        "Order Message:\nSELL 1 ES DEC'26\nWarning: Your order will not be placed at the exchange until 2026-08-17 08:30:00 US/Central.",
    ));
    assert!(!is_warning_error(399, "Order Message:\nOrder cannot be transmitted"));
}

#[test]
fn test_order_update_notice_gating() {
    let payload = DecodedError {
        request_id: 42,
        error_code: 201,
        error_message: "Order rejected".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };

    let notice = order_update_notice(&payload, false).expect("order-bound error should produce a notice");
    assert_eq!(notice.request_id, Some(42));
    assert_eq!(notice.code, 201);

    // Owned by a data-request subscription: nothing for the order stream.
    assert!(order_update_notice(&payload, true).is_none());

    // Request-less: nothing for the order stream regardless of ownership.
    let request_less = DecodedError {
        request_id: UNSPECIFIED_REQUEST_ID,
        ..payload
    };
    assert!(order_update_notice(&request_less, false).is_none());
}

#[test]
fn test_classify_error_unrouted_warning_is_notice_only() {
    let payload = DecodedError {
        error_code: 2104,
        error_message: "Market data farm OK".into(),
        ..Default::default()
    };

    match classify_error(payload) {
        ErrorDisposition::NoticeOnly(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm OK");
        }
        other => panic!("expected NoticeOnly, got {other:?}"),
    }
}

#[test]
fn test_classify_error_unrouted_hard_error_fails_one_shots() {
    let payload = DecodedError {
        error_code: 321,
        error_message: "Server error".into(),
        ..Default::default()
    };

    match classify_error(payload) {
        ErrorDisposition::NoticeAndFailOneShots(notice, error) => {
            assert_eq!(notice.code, 321);
            assert_eq!(notice.message, "Server error");
            match error {
                crate::Error::Notice(error_notice) => assert_eq!(error_notice, notice),
                other => panic!("expected Error::Notice, got {other:?}"),
            }
        }
        other => panic!("expected NoticeAndFailOneShots, got {other:?}"),
    }
}

#[test]
fn test_classify_error_routed_warning_is_notice() {
    let payload = DecodedError {
        request_id: 42,
        error_code: 2104,
        error_message: "Farm OK".into(),
        ..Default::default()
    };

    match classify_error(payload) {
        ErrorDisposition::Route(42, RoutedItem::Notice(notice)) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Farm OK");
        }
        other => panic!("expected routed Notice, got {other:?}"),
    }
}

#[test]
fn test_classify_error_routed_hard_error_is_error() {
    let payload = DecodedError {
        request_id: 7,
        error_code: 200,
        error_message: "No security".into(),
        ..Default::default()
    };

    match classify_error(payload) {
        ErrorDisposition::Route(7, RoutedItem::Error(crate::Error::Notice(notice))) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security");
        }
        other => panic!("expected routed Error, got {other:?}"),
    }
}

/// Order-message routing for message types that lack an order_id at the proto
/// level. `CompletedOrdersEnd` and `CommissionsReport` are order-routed but
/// have no `order_id` field, so the dispatcher falls back to the sentinel `-1`.
/// (Cases with a real `order_id` are covered by the per-type proto tests below.)
#[test]
fn test_order_message_routing_without_order_id_returns_sentinel() {
    let completed_orders_end = proto_response(IncomingMessages::CompletedOrdersEnd, Vec::new());
    match determine_routing(&completed_orders_end) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
        routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
    }

    let commission_report = proto_response(
        IncomingMessages::CommissionsReport,
        crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec123".into()),
            ..Default::default()
        }
        .encode_to_vec(),
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
    let message = proto_response(IncomingMessages::OpenOrder, bytes);
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
    let message = proto_response(IncomingMessages::OrderStatus, bytes);
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
    let message = proto_response(IncomingMessages::ExecutionData, bytes);
    match determine_routing(&message) {
        RoutingDecision::ByOrderId(id) => assert_eq!(id, 58),
        routing => panic!("Expected ByOrderId(58), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_execution_data_end() {
    let bytes = crate::proto::ExecutionDetailsEnd { req_id: Some(7) }.encode_to_vec();
    let message = proto_response(IncomingMessages::ExecutionDataEnd, bytes);
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
    let message = proto_response(IncomingMessages::CommissionsReport, bytes);
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
    let message = proto_response(IncomingMessages::AccountSummary, bytes);
    match determine_routing(&message) {
        RoutingDecision::ByRequestId(id) => assert_eq!(id, 314),
        routing => panic!("Expected ByRequestId(314), got {routing:?}"),
    }
}

#[test]
fn test_determine_routing_protobuf_market_data_type() {
    // Regression for the silent drop: MarketDataType is declared by the
    // `TickTypes` decoder, so it has to reach a request_id-keyed subscription.
    // Before it was added to `text_request_id_field` it fell through to
    // ByMessageType and landed on a shared channel nobody subscribes to.
    let bytes = crate::proto::MarketDataType {
        req_id: Some(9001),
        market_data_type: Some(3),
    }
    .encode_to_vec();
    let message = proto_response(IncomingMessages::MarketDataType, bytes);
    match determine_routing(&message) {
        RoutingDecision::ByRequestId(id) => assert_eq!(id, 9001),
        routing => panic!("Expected ByRequestId(9001), got {routing:?}"),
    }
}

#[test]
fn test_first_unroutable_by_request_id_accepts_registered_types() {
    // The two ways in, one per arm of `routable_to_request_id_subscription`.
    let declared = &[
        IncomingMessages::TickPrice,         // text_request_id_field entry
        IncomingMessages::CommissionsReport, // order-scoped, arrives via the order-id channel
    ];
    assert_eq!(first_unroutable_by_request_id(declared), None);
    assert_eq!(first_unroutable_by_request_id(&[]), None);
}

#[test]
fn test_first_unroutable_by_request_id_rejects_error() {
    // `Error` used to be exempted here so that decoders declaring it would not
    // trip the guard — const declares, guard exempts, circular. It is
    // classified by `determine_routing` before the allow-list and reaches a
    // subscription as `RoutedItem::Error`/`Notice`, never as a `Response`, so
    // declaring it is the mistake the guard should report.
    assert_eq!(
        first_unroutable_by_request_id(&[IncomingMessages::TickPrice, IncomingMessages::Error]),
        Some(IncomingMessages::Error)
    );
}

#[test]
fn test_first_unroutable_by_request_id_reports_first_offender() {
    // FamilyCodes and MarketRule are shared-channel types with no request id.
    let declared = &[IncomingMessages::TickPrice, IncomingMessages::FamilyCodes, IncomingMessages::MarketRule];
    assert_eq!(first_unroutable_by_request_id(declared), Some(IncomingMessages::FamilyCodes));
}

/// The unknown-message-id alarm in `report_unroutable_frame` only fires for
/// frames that reach the end of routing unclaimed, so it depends on
/// `NotValid` landing in `ByMessageType` rather than any id-keyed arm. Nothing
/// else guards that: `is_order_message`/`is_shared_message` exclude `NotValid`
/// and `routes_by_request_id(NotValid)` is false, but if any of those shifted
/// the alarm would go quiet with no test failing.
#[test]
fn test_unknown_message_type_routes_by_message_type() {
    let message = ResponseMessage::from("9999\01\0");
    assert_eq!(message.message_type(), IncomingMessages::NotValid);
    assert_eq!(determine_routing(&message), RoutingDecision::ByMessageType(IncomingMessages::NotValid));
}
