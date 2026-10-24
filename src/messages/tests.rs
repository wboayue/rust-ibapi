use prost::Message;

use super::*;

// Table-driven test data structures
struct EncodeLengthTestCase {
    message: &'static str,
    expected_length: usize,
}

struct ResponseMessageParseTestCase {
    name: &'static str,
    input: &'static str,
    field_index: usize,
    parse_type: ParseType,
    expected: ParseResult,
}

enum ParseType {
    Int,
    Double,
    String,
}

enum ParseResult {
    Int(i32),
    Double(f64),
    String(String),
    Error,
}

// Test data that can be shared between sync and async tests
fn encode_length_test_cases() -> Vec<EncodeLengthTestCase> {
    vec![
        EncodeLengthTestCase {
            message: "hello",
            expected_length: 9, // 4 bytes for length + 5 bytes for "hello"
        },
        EncodeLengthTestCase {
            message: "",
            expected_length: 4, // 4 bytes for length + 0 bytes for empty string
        },
        EncodeLengthTestCase {
            message: "a\0b\0c",
            expected_length: 9, // 4 bytes for length + 5 bytes for "a\0b\0c"
        },
    ]
}

fn response_message_parse_test_cases() -> Vec<ResponseMessageParseTestCase> {
    vec![
        ResponseMessageParseTestCase {
            name: "parse_valid_int",
            input: "1\0123\0456\0",
            field_index: 1,
            parse_type: ParseType::Int,
            expected: ParseResult::Int(123),
        },
        ResponseMessageParseTestCase {
            name: "parse_invalid_int",
            input: "1\0abc\0456\0",
            field_index: 1,
            parse_type: ParseType::Int,
            expected: ParseResult::Error,
        },
        ResponseMessageParseTestCase {
            name: "parse_double",
            input: "1\03.14567\0456\0",
            field_index: 1,
            parse_type: ParseType::Double,
            expected: ParseResult::Double(3.14567),
        },
        ResponseMessageParseTestCase {
            name: "parse_double_zero",
            input: "1\00\0456\0",
            field_index: 1,
            parse_type: ParseType::Double,
            expected: ParseResult::Double(0.0),
        },
        ResponseMessageParseTestCase {
            name: "parse_string",
            input: "1\0hello world\0456\0",
            field_index: 1,
            parse_type: ParseType::String,
            expected: ParseResult::String("hello world".to_string()),
        },
    ]
}

#[test]
fn incoming_message_from_i32() {
    assert_eq!(IncomingMessages::from(1), IncomingMessages::TickPrice);
    assert_eq!(IncomingMessages::from(2), IncomingMessages::TickSize);
    assert_eq!(IncomingMessages::from(3), IncomingMessages::OrderStatus);
    assert_eq!(IncomingMessages::from(4), IncomingMessages::Error);
    assert_eq!(IncomingMessages::from(5), IncomingMessages::OpenOrder);
    assert_eq!(IncomingMessages::from(6), IncomingMessages::AccountValue);
    assert_eq!(IncomingMessages::from(7), IncomingMessages::PortfolioValue);
    assert_eq!(IncomingMessages::from(8), IncomingMessages::AccountUpdateTime);
    assert_eq!(IncomingMessages::from(9), IncomingMessages::NextValidId);
    assert_eq!(IncomingMessages::from(10), IncomingMessages::ContractData);
    assert_eq!(IncomingMessages::from(11), IncomingMessages::ExecutionData);
    assert_eq!(IncomingMessages::from(12), IncomingMessages::MarketDepth);
    assert_eq!(IncomingMessages::from(13), IncomingMessages::MarketDepthL2);
    assert_eq!(IncomingMessages::from(14), IncomingMessages::NewsBulletins);
    assert_eq!(IncomingMessages::from(15), IncomingMessages::ManagedAccounts);
    assert_eq!(IncomingMessages::from(16), IncomingMessages::ReceiveFA);
    assert_eq!(IncomingMessages::from(17), IncomingMessages::HistoricalData);
    assert_eq!(IncomingMessages::from(18), IncomingMessages::BondContractData);
    assert_eq!(IncomingMessages::from(19), IncomingMessages::ScannerParameters);
    assert_eq!(IncomingMessages::from(20), IncomingMessages::ScannerData);
    assert_eq!(IncomingMessages::from(21), IncomingMessages::TickOptionComputation);
    assert_eq!(IncomingMessages::from(45), IncomingMessages::TickGeneric);
    assert_eq!(IncomingMessages::from(46), IncomingMessages::TickString);
    assert_eq!(IncomingMessages::from(47), IncomingMessages::TickEFP);
    assert_eq!(IncomingMessages::from(49), IncomingMessages::CurrentTime);
    assert_eq!(IncomingMessages::from(50), IncomingMessages::RealTimeBars);
    assert_eq!(IncomingMessages::from(51), IncomingMessages::FundamentalData);
    assert_eq!(IncomingMessages::from(52), IncomingMessages::ContractDataEnd);
    assert_eq!(IncomingMessages::from(53), IncomingMessages::OpenOrderEnd);
    assert_eq!(IncomingMessages::from(54), IncomingMessages::AccountDownloadEnd);
    assert_eq!(IncomingMessages::from(55), IncomingMessages::ExecutionDataEnd);
    assert_eq!(IncomingMessages::from(56), IncomingMessages::DeltaNeutralValidation);
    assert_eq!(IncomingMessages::from(57), IncomingMessages::TickSnapshotEnd);
    assert_eq!(IncomingMessages::from(58), IncomingMessages::MarketDataType);
    assert_eq!(IncomingMessages::from(59), IncomingMessages::CommissionsReport);
    assert_eq!(IncomingMessages::from(61), IncomingMessages::Position);
    assert_eq!(IncomingMessages::from(62), IncomingMessages::PositionEnd);
    assert_eq!(IncomingMessages::from(63), IncomingMessages::AccountSummary);
    assert_eq!(IncomingMessages::from(64), IncomingMessages::AccountSummaryEnd);
    assert_eq!(IncomingMessages::from(65), IncomingMessages::VerifyMessageApi);
    assert_eq!(IncomingMessages::from(66), IncomingMessages::VerifyCompleted);
    assert_eq!(IncomingMessages::from(67), IncomingMessages::DisplayGroupList);
    assert_eq!(IncomingMessages::from(68), IncomingMessages::DisplayGroupUpdated);
    assert_eq!(IncomingMessages::from(69), IncomingMessages::VerifyAndAuthMessageApi);
    assert_eq!(IncomingMessages::from(70), IncomingMessages::VerifyAndAuthCompleted);
    assert_eq!(IncomingMessages::from(71), IncomingMessages::PositionMulti);
    assert_eq!(IncomingMessages::from(72), IncomingMessages::PositionMultiEnd);
    assert_eq!(IncomingMessages::from(73), IncomingMessages::AccountUpdateMulti);
    assert_eq!(IncomingMessages::from(74), IncomingMessages::AccountUpdateMultiEnd);
    assert_eq!(IncomingMessages::from(75), IncomingMessages::SecurityDefinitionOptionParameter);
    assert_eq!(IncomingMessages::from(76), IncomingMessages::SecurityDefinitionOptionParameterEnd);
    assert_eq!(IncomingMessages::from(77), IncomingMessages::SoftDollarTier);
    assert_eq!(IncomingMessages::from(78), IncomingMessages::FamilyCodes);
    assert_eq!(IncomingMessages::from(79), IncomingMessages::SymbolSamples);
    assert_eq!(IncomingMessages::from(80), IncomingMessages::MktDepthExchanges);
    assert_eq!(IncomingMessages::from(81), IncomingMessages::TickReqParams);
    assert_eq!(IncomingMessages::from(82), IncomingMessages::SmartComponents);
    assert_eq!(IncomingMessages::from(83), IncomingMessages::NewsArticle);
    assert_eq!(IncomingMessages::from(84), IncomingMessages::TickNews);
    assert_eq!(IncomingMessages::from(85), IncomingMessages::NewsProviders);
    assert_eq!(IncomingMessages::from(86), IncomingMessages::HistoricalNews);
    assert_eq!(IncomingMessages::from(87), IncomingMessages::HistoricalNewsEnd);
    assert_eq!(IncomingMessages::from(88), IncomingMessages::HeadTimestamp);
    assert_eq!(IncomingMessages::from(89), IncomingMessages::HistogramData);
    assert_eq!(IncomingMessages::from(90), IncomingMessages::HistoricalDataUpdate);
    assert_eq!(IncomingMessages::from(91), IncomingMessages::RerouteMktDataReq);
    assert_eq!(IncomingMessages::from(92), IncomingMessages::RerouteMktDepthReq);
    assert_eq!(IncomingMessages::from(93), IncomingMessages::MarketRule);
    assert_eq!(IncomingMessages::from(94), IncomingMessages::PnL);
    assert_eq!(IncomingMessages::from(95), IncomingMessages::PnLSingle);
    assert_eq!(IncomingMessages::from(96), IncomingMessages::HistoricalTick);
    assert_eq!(IncomingMessages::from(97), IncomingMessages::HistoricalTickBidAsk);
    assert_eq!(IncomingMessages::from(98), IncomingMessages::HistoricalTickLast);
    assert_eq!(IncomingMessages::from(99), IncomingMessages::TickByTick);
    assert_eq!(IncomingMessages::from(100), IncomingMessages::OrderBound);
    assert_eq!(IncomingMessages::from(101), IncomingMessages::CompletedOrder);
    assert_eq!(IncomingMessages::from(102), IncomingMessages::CompletedOrdersEnd);
    assert_eq!(IncomingMessages::from(103), IncomingMessages::ReplaceFAEnd);
    assert_eq!(IncomingMessages::from(104), IncomingMessages::WshMetaData);
    assert_eq!(IncomingMessages::from(105), IncomingMessages::WshEventData);
    assert_eq!(IncomingMessages::from(106), IncomingMessages::HistoricalSchedule);
    assert_eq!(IncomingMessages::from(107), IncomingMessages::UserInfo);
    assert_eq!(IncomingMessages::from(108), IncomingMessages::HistoricalDataEnd);
    assert_eq!(IncomingMessages::from(109), IncomingMessages::CurrentTimeInMillis);
    assert_eq!(IncomingMessages::from(110), IncomingMessages::ConfigResponse);
    assert_eq!(IncomingMessages::from(111), IncomingMessages::UpdateConfigResponse);
    assert_eq!(IncomingMessages::from(112), IncomingMessages::NotValid);
}

#[test]
fn routes_by_request_id_works() {
    // Sample of the allow-list — full registration check happens via
    // request_id() round-trips, not exhaustive enumeration here.
    assert!(routes_by_request_id(IncomingMessages::ContractData));
    // Bond queries answer with BondContractData; unrouted, every row was dropped.
    assert!(routes_by_request_id(IncomingMessages::BondContractData));
    assert!(routes_by_request_id(IncomingMessages::TickByTick));
    assert!(routes_by_request_id(IncomingMessages::SymbolSamples));
    assert!(routes_by_request_id(IncomingMessages::ExecutionData));
    assert!(routes_by_request_id(IncomingMessages::HeadTimestamp));
    assert!(routes_by_request_id(IncomingMessages::HistoricalData));
    assert!(routes_by_request_id(IncomingMessages::HistoricalSchedule));
    assert!(routes_by_request_id(IncomingMessages::ContractDataEnd));
    assert!(routes_by_request_id(IncomingMessages::RealTimeBars));
    assert!(routes_by_request_id(IncomingMessages::ExecutionDataEnd));

    // Error has its own envelope; shared messages route by message type.
    assert!(!routes_by_request_id(IncomingMessages::Error));
    // FundamentalData is retained as a known variant but no longer routes:
    // the fundamental-data feature was removed in TWS 10.47.
    assert!(!routes_by_request_id(IncomingMessages::FundamentalData));
    assert!(!routes_by_request_id(IncomingMessages::ManagedAccounts));
    assert!(!routes_by_request_id(IncomingMessages::NextValidId));
    assert!(!routes_by_request_id(IncomingMessages::CurrentTime));
    assert!(!routes_by_request_id(IncomingMessages::NotValid));
}

#[test]
fn text_request_id_field_works() {
    // Field-1 messages (request_id immediately after message-type tag).
    assert_eq!(text_request_id_field(IncomingMessages::ContractData), Some(1));
    assert_eq!(text_request_id_field(IncomingMessages::BondContractData), Some(1));
    assert_eq!(text_request_id_field(IncomingMessages::TickByTick), Some(1));

    // Field-2 messages (request_id after a version field).
    assert_eq!(text_request_id_field(IncomingMessages::TickPrice), Some(2));
    assert_eq!(text_request_id_field(IncomingMessages::ContractDataEnd), Some(2));
    // MarketDataType is declared by the `TickTypes` decoder — without an entry
    // the tick never reaches the subscription that asked for it.
    assert_eq!(text_request_id_field(IncomingMessages::MarketDataType), Some(2));

    // Shared and unrouted messages get None.
    assert_eq!(text_request_id_field(IncomingMessages::ManagedAccounts), None);
    assert_eq!(text_request_id_field(IncomingMessages::Error), None);
    assert_eq!(text_request_id_field(IncomingMessages::NotValid), None);
    // FundamentalData no longer routes (feature removed in TWS 10.47).
    assert_eq!(text_request_id_field(IncomingMessages::FundamentalData), None);
}

#[test]
fn encode_length_works() {
    for test_case in encode_length_test_cases() {
        let encoded = encode_length(test_case.message);
        assert_eq!(encoded.len(), test_case.expected_length, "Failed for message: {:?}", test_case.message);

        // Verify the length bytes are correct
        let length_bytes = &encoded[0..4];
        let length = u32::from_be_bytes([length_bytes[0], length_bytes[1], length_bytes[2], length_bytes[3]]);
        assert_eq!(
            length as usize,
            test_case.message.len(),
            "Incorrect length encoding for message: {:?}",
            test_case.message
        );
    }
}

#[test]
fn response_message_parsing() {
    for test_case in response_message_parse_test_cases() {
        let mut message = ResponseMessage::from(test_case.input);
        message.i = test_case.field_index;

        match (&test_case.parse_type, &test_case.expected) {
            (ParseType::Int, ParseResult::Int(expected)) => match message.next_int() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::Int, ParseResult::Error) => {
                assert!(message.next_int().is_err(), "Test '{}' failed: expected error", test_case.name);
            }
            (ParseType::Double, ParseResult::Double(expected)) => match message.next_double() {
                Ok(val) => assert!(
                    (val - expected).abs() < f64::EPSILON,
                    "Test '{}' failed: expected {:?}, got {:?}",
                    test_case.name,
                    expected,
                    val
                ),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::String, ParseResult::String(expected)) => match message.next_string() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            _ => panic!("Test case type mismatch"),
        }
    }
}

#[test]
fn response_message_boundary_conditions() {
    // Test reading past end of message
    let mut message = ResponseMessage::from("1\02\0");
    message.i = 3; // Beyond the last field

    assert!(message.next_int().is_err());
    assert!(message.next_double().is_err());
    assert!(message.next_string().is_err());
}

#[test]
fn response_message_peek_operations() {
    let message = ResponseMessage::from("1\0123\0abc\0456\0");

    // Test peek_int
    assert_eq!(message.peek_int(1).unwrap(), 123);
    assert_eq!(message.peek_int(3).unwrap(), 456);
    assert!(message.peek_int(2).is_err()); // "abc" is not an int
    assert!(message.peek_int(4).is_err()); // Out of bounds (only 4 fields, indices 0-3)
}

#[test]
fn peek_int_rejects_proto_framed_message() {
    // Per docs/rules/wire/proto-aware-accessors.md, text-index accessors must be
    // proto-aware. peek_int defensively returns Err(UnexpectedWireFormat) on a
    // proto-framed message even if the caller passes a valid text-field index —
    // production callers already gate on raw_bytes().is_none() before invoking, so
    // the guard is unreachable in correct paths but catches misuse. Same variant
    // as require_proto's opposite-direction guard: a framing mismatch is never
    // the skippable UnexpectedResponse.
    let proto_msg = ResponseMessage::from_protobuf(5, vec![0x08, 0x7B]);
    assert!(matches!(proto_msg.peek_int(0), Err(crate::Error::UnexpectedWireFormat(_))));
    assert!(matches!(proto_msg.peek_int(1), Err(crate::Error::UnexpectedWireFormat(_))));
}

#[test]
fn proto_decodes_the_payload_as_the_requested_type() {
    let message = ResponseMessage::from_protobuf(9, crate::proto::NextValidId { order_id: Some(7) }.encode_to_vec());
    let p: crate::proto::NextValidId = message.proto().unwrap();
    assert_eq!(p.order_id, Some(7));

    let garbage = ResponseMessage::from_protobuf(9, vec![0xff, 0xff, 0xff]);
    assert!(matches!(
        garbage.proto::<crate::proto::NextValidId>(),
        Err(crate::Error::ProtobufDecode(_))
    ));

    let text = ResponseMessage::from("9\01\07\0");
    assert!(matches!(
        text.proto::<crate::proto::NextValidId>(),
        Err(crate::Error::UnexpectedWireFormat(_))
    ));
}

#[test]
fn text_framed_message_has_no_order_id_or_execution_id() {
    // order_id() and execution_id() require proto-framed payloads; a
    // text-framed message (raw_bytes = None) returns None regardless of the
    // historical text-layout field positions.
    let open_order = ResponseMessage::from("5\0123\0field2\0field3\0");
    assert_eq!(open_order.order_id(), None);
    assert_eq!(open_order.execution_id(), None);

    let commission_message = ResponseMessage::from("59\0field1\0exec123\0");
    assert_eq!(commission_message.execution_id(), None);
}

#[test]
fn execution_id_protobuf_commissions_report() {
    let bytes = crate::proto::CommissionAndFeesReport {
        exec_id: Some("exec-proto-1".into()),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::CommissionsReport as i32, bytes);
    assert_eq!(message.execution_id(), Some("exec-proto-1".to_string()));
}

#[test]
fn execution_id_protobuf_execution_data() {
    let bytes = crate::proto::ExecutionDetails {
        req_id: Some(7),
        contract: None,
        execution: Some(crate::proto::Execution {
            exec_id: Some("exec-proto-2".into()),
            ..Default::default()
        }),
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ExecutionData as i32, bytes);
    assert_eq!(message.execution_id(), Some("exec-proto-2".to_string()));
}

#[test]
fn execution_id_protobuf_unknown_message_type_returns_none() {
    let message = ResponseMessage::from_protobuf(IncomingMessages::OpenOrder as i32, Vec::new());
    assert_eq!(message.execution_id(), None);
}

#[test]
fn order_id_protobuf_open_order() {
    let bytes = crate::proto::OpenOrder {
        order_id: Some(42),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::OpenOrder as i32, bytes);
    assert_eq!(message.order_id(), Some(42));
}

#[test]
fn order_id_protobuf_order_status() {
    let bytes = crate::proto::OrderStatus {
        order_id: Some(99),
        status: Some("Filled".into()),
        ..Default::default()
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::OrderStatus as i32, bytes);
    assert_eq!(message.order_id(), Some(99));
}

/// Each order frame's client id, read from where its proto carries it.
#[test]
fn client_id_of_each_order_frame() {
    use crate::proto;
    let full_order = proto::OpenOrder {
        order_id: Some(99),
        contract: Some(proto::Contract {
            symbol: Some("AAPL".into()),
            ..Default::default()
        }),
        order: Some(proto::Order {
            client_id: Some(7),
            order_id: Some(99),
            ..Default::default()
        }),
        order_state: Some(proto::OrderState {
            status: Some("Submitted".into()),
            ..Default::default()
        }),
    };
    let execution = proto::ExecutionDetails {
        req_id: Some(-1),
        execution: Some(proto::Execution {
            order_id: Some(99),
            client_id: Some(7),
            ..Default::default()
        }),
        ..Default::default()
    };
    let cases: Vec<(&str, IncomingMessages, Vec<u8>, Option<i32>)> = vec![
        (
            "order status",
            IncomingMessages::OrderStatus,
            proto::OrderStatus {
                order_id: Some(99),
                client_id: Some(7),
                ..Default::default()
            }
            .encode_to_vec(),
            Some(7),
        ),
        ("open order", IncomingMessages::OpenOrder, full_order.encode_to_vec(), Some(7)),
        ("execution", IncomingMessages::ExecutionData, execution.encode_to_vec(), Some(7)),
        (
            "order bound",
            IncomingMessages::OrderBound,
            proto::OrderBound {
                perm_id: Some(1),
                client_id: Some(7),
                order_id: Some(99),
            }
            .encode_to_vec(),
            Some(7),
        ),
        (
            "absent",
            IncomingMessages::OrderStatus,
            proto::OrderStatus {
                order_id: Some(99),
                ..Default::default()
            }
            .encode_to_vec(),
            None,
        ),
        (
            "open order without order",
            IncomingMessages::OpenOrder,
            proto::OpenOrder::default().encode_to_vec(),
            None,
        ),
        (
            "other type",
            IncomingMessages::CommissionsReport,
            proto::CommissionAndFeesReport {
                exec_id: Some("e1".into()),
                ..Default::default()
            }
            .encode_to_vec(),
            None,
        ),
    ];
    for (name, message_type, bytes, expected) in cases {
        let message = ResponseMessage::from_protobuf(message_type as i32, bytes);
        assert_eq!(message.client_id(), expected, "{name}");
    }
}

#[test]
fn order_id_protobuf_execution_data_nested() {
    // ExecutionData carries order_id nested under `execution.order_id`,
    // not at proto tag 1 (which is req_id).
    let bytes = crate::proto::ExecutionDetails {
        req_id: Some(7),
        contract: None,
        execution: Some(crate::proto::Execution {
            order_id: Some(123),
            ..Default::default()
        }),
    }
    .encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ExecutionData as i32, bytes);
    assert_eq!(message.order_id(), Some(123));
    assert_eq!(message.request_id(), Some(7));
}

#[test]
fn order_id_protobuf_execution_data_end() {
    // ExecutionDetailsEnd carries `req_id` at proto tag 1; order_id() returns
    // that value because routing keys this message family by order_id and the
    // proto schema overloads the field.
    let bytes = crate::proto::ExecutionDetailsEnd { req_id: Some(55) }.encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ExecutionDataEnd as i32, bytes);
    assert_eq!(message.order_id(), Some(55));
}

#[test]
fn request_id_protobuf_tag1() {
    // HistoricalDataEnd-shaped envelope: just req_id at tag 1.
    let bytes = ProtoIdEnvelope { id: Some(314) }.encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::HistoricalDataEnd as i32, bytes);
    assert_eq!(message.request_id(), Some(314));
}

#[test]
fn request_id_protobuf_message_type_without_request_id_returns_none() {
    // ManagedAccounts is a shared message — not in routes_by_request_id, so
    // request_id() returns None even if the proto payload carried an int @ tag 1.
    let bytes = ProtoIdEnvelope { id: Some(42) }.encode_to_vec();
    let message = ResponseMessage::from_protobuf(IncomingMessages::ManagedAccounts as i32, bytes);
    assert_eq!(message.request_id(), None);
}

#[test]
fn request_message_index() {
    let message = RequestMessage {
        fields: vec!["field0".to_string(), "field1".to_string(), "field2".to_string()],
    };

    assert_eq!(message[0], "field0");
    assert_eq!(message[1], "field1");
    assert_eq!(message[2], "field2");
}

#[test]
#[should_panic]
fn request_message_index_out_of_bounds() {
    let message = RequestMessage {
        fields: vec!["field0".to_string()],
    };

    let _ = &message[1]; // Should panic
}

#[test]
fn response_message_is_shutdown() {
    let shutdown_message = ResponseMessage::from("-2\0");
    assert!(shutdown_message.is_shutdown());

    let normal_message = ResponseMessage::from("1\0");
    assert!(!normal_message.is_shutdown());
}

#[test]
fn response_message_encode_decode_roundtrip() {
    let original = ResponseMessage::from("1\0test\0123\03.456\0");
    let encoded = original.encode();
    let decoded = ResponseMessage::from(&encoded);

    assert_eq!(original.fields, decoded.fields);
}

// Tests for error conditions and edge cases
#[test]
fn response_message_next_methods_edge_cases() {
    struct TestCase {
        name: &'static str,
        input: &'static str,
        test_fn: fn(&mut ResponseMessage) -> bool,
    }

    let test_cases = vec![
        TestCase {
            name: "empty_message",
            input: "",
            test_fn: |msg| msg.next_int().is_err(),
        },
        TestCase {
            name: "single_null_terminator",
            input: "\0",
            test_fn: |msg| {
                msg.i = 0;
                let result = msg.next_string();
                result.is_ok() && result.unwrap().is_empty()
            },
        },
        TestCase {
            name: "multiple_null_terminators",
            input: "\0\0\0",
            test_fn: |msg| {
                msg.i = 0;
                let result = msg.next_string();
                result.is_ok() && result.unwrap().is_empty()
            },
        },
        TestCase {
            name: "malformed_int",
            input: "not_an_int\0",
            test_fn: |msg| {
                msg.i = 0;
                msg.next_int().is_err()
            },
        },
        TestCase {
            name: "malformed_double",
            input: "not_a_double\0",
            test_fn: |msg| {
                msg.i = 0;
                msg.next_double().is_err()
            },
        },
        TestCase {
            name: "overflow_int",
            input: "99999999999999999999\0",
            test_fn: |msg| {
                msg.i = 0;
                msg.next_int().is_err()
            },
        },
    ];

    for test_case in test_cases {
        let mut message = ResponseMessage::from(test_case.input);
        assert!((test_case.test_fn)(&mut message), "Test '{}' failed", test_case.name);
    }
}

#[test]
fn channel_mappings_completeness() {
    use super::shared_channel_configuration::CHANNEL_MAPPINGS;

    // Verify that each mapping has at least one response
    for mapping in CHANNEL_MAPPINGS {
        assert!(
            !mapping.responses.is_empty(),
            "Channel mapping for {:?} has no responses",
            mapping.request
        );
    }

    // Test specific known mappings
    let mappings = CHANNEL_MAPPINGS;

    // Find RequestPositions mapping
    let positions_mapping = mappings
        .iter()
        .find(|m| matches!(m.request, OutgoingMessages::RequestPositions))
        .expect("RequestPositions mapping should exist");

    assert_eq!(positions_mapping.responses.len(), 2);
    assert!(positions_mapping.responses.contains(&IncomingMessages::Position));
    assert!(positions_mapping.responses.contains(&IncomingMessages::PositionEnd));

    // Find RequestAccountData mapping
    let account_data_mapping = mappings
        .iter()
        .find(|m| matches!(m.request, OutgoingMessages::RequestAccountData))
        .expect("RequestAccountData mapping should exist");

    assert_eq!(account_data_mapping.responses.len(), 4);
    assert!(account_data_mapping.responses.contains(&IncomingMessages::AccountValue));
    assert!(account_data_mapping.responses.contains(&IncomingMessages::PortfolioValue));
    assert!(account_data_mapping.responses.contains(&IncomingMessages::AccountDownloadEnd));
    assert!(account_data_mapping.responses.contains(&IncomingMessages::AccountUpdateTime));
}

#[test]
fn exclusive_one_shot_response_types_works() {
    use super::shared_channel_configuration::exclusive_one_shot_response_types;

    let one_shot = exclusive_one_shot_response_types();

    // Genuine one-shots (single terminating response) are eligible for fail-fast.
    for included in [
        IncomingMessages::NextValidId,
        IncomingMessages::MarketRule,
        IncomingMessages::ManagedAccounts,
        IncomingMessages::FamilyCodes,
    ] {
        assert!(one_shot.contains(&included), "{included:?} should be a one-shot error response type");
    }

    // Streaming channels are excluded so an unrelated error never terminates a
    // live subscription — including NewsBulletins, which streams without an
    // `*End` marker. MarketDataType and WshEventData are not in the table at all
    // (their callers use request-id-keyed channels).
    for excluded in [
        IncomingMessages::OpenOrder,
        IncomingMessages::OrderStatus,
        IncomingMessages::Position,
        IncomingMessages::PositionMulti,
        IncomingMessages::AccountValue,
        IncomingMessages::NewsBulletins,
        IncomingMessages::MarketDataType,
        IncomingMessages::WshEventData,
    ] {
        assert!(!one_shot.contains(&excluded), "{excluded:?} is streaming and must not be fail-fast");
    }
}

#[test]
fn is_one_shot_request_works() {
    use super::shared_channel_configuration::{is_one_shot_request, CHANNEL_MAPPINGS};

    // Every mapping's flag round-trips through the request-side lookup.
    for mapping in CHANNEL_MAPPINGS {
        assert_eq!(
            is_one_shot_request(mapping.request),
            mapping.one_shot,
            "{:?} lookup should match its mapping's one_shot flag",
            mapping.request
        );
    }

    // A request without a shared-channel mapping is not one-shot.
    assert!(!is_one_shot_request(OutgoingMessages::RequestMarketData));
}

#[test]
fn all_incoming_message_conversions() {
    // Test boundary values and ensure all message types are covered
    let test_cases = vec![
        (0, IncomingMessages::NotValid),
        (1, IncomingMessages::TickPrice),
        (108, IncomingMessages::HistoricalDataEnd),
        (109, IncomingMessages::CurrentTimeInMillis),
        (110, IncomingMessages::ConfigResponse),
        (111, IncomingMessages::UpdateConfigResponse),
        (112, IncomingMessages::NotValid),
        (i32::MAX, IncomingMessages::NotValid),
        (i32::MIN, IncomingMessages::NotValid),
        (-1, IncomingMessages::NotValid),
    ];

    for (value, expected) in test_cases {
        assert_eq!(IncomingMessages::from(value), expected, "Failed for value {}", value);
    }
}

#[test]
fn outgoing_message_display() {
    // Test Display implementation for OutgoingMessages
    let test_cases = vec![
        (OutgoingMessages::RequestMarketData, "1"),
        (OutgoingMessages::CancelMarketData, "2"),
        (OutgoingMessages::PlaceOrder, "3"),
        (OutgoingMessages::CancelOrder, "4"),
        (OutgoingMessages::RequestOpenOrders, "5"),
        (OutgoingMessages::RequestIds, "8"),
        (OutgoingMessages::RequestCurrentTime, "49"),
        (OutgoingMessages::RequestAccountSummary, "62"),
        (OutgoingMessages::RequestPnL, "92"),
        (OutgoingMessages::RequestUserInfo, "104"),
    ];

    for (msg, expected) in test_cases {
        assert_eq!(format!("{}", msg), expected);
    }
}

#[test]
fn encode_length_edge_cases() {
    // Test with various sizes
    let x255 = "x".repeat(255);
    let x256 = "x".repeat(256);
    let x1000 = "x".repeat(1000);

    let test_cases = vec![
        ("", 4),                // Empty string
        ("x", 5),               // Single character
        (x255.as_str(), 259),   // 255 characters
        (x256.as_str(), 260),   // 256 characters
        (x1000.as_str(), 1004), // 1000 characters
    ];

    for (input, expected_len) in test_cases {
        let encoded = encode_length(input);
        assert_eq!(encoded.len(), expected_len);

        // Verify the encoded length is correct
        let length_bytes = &encoded[0..4];
        let decoded_length = u32::from_be_bytes([length_bytes[0], length_bytes[1], length_bytes[2], length_bytes[3]]);
        assert_eq!(decoded_length as usize, input.len());
    }
}

#[test]
fn response_message_access_patterns() {
    let message = ResponseMessage::from("5\0123\0field2\0field3\0field4\0");

    // Test message_type
    assert_eq!(message.message_type(), IncomingMessages::OpenOrder);

    // Test multiple peeks don't change state
    assert_eq!(message.peek_int(1).unwrap(), 123);
    assert_eq!(message.peek_int(1).unwrap(), 123);

    assert_eq!(message.fields.len(), 5);
}

#[test]
fn message_type_is_resolved_once_at_construction() {
    // Text framing keeps the discriminant in fields[0] — the handshake reader
    // walks the cursor from index 0 — and `kind` is derived from it.
    let text = ResponseMessage::from("5\0123\0");
    assert_eq!(text.message_type(), IncomingMessages::OpenOrder);
    assert_eq!(text.fields[0], "5", "text framing keeps the id as a readable field");

    // Proto framing carries the id out of band, so there are no text fields at
    // all and nothing re-parses a string to answer message_type().
    let proto = ResponseMessage::from_protobuf(IncomingMessages::OpenOrder as i32, vec![0x08, 0x2a]);
    assert_eq!(proto.message_type(), IncomingMessages::OpenOrder);
    assert!(proto.fields.is_empty(), "a proto frame allocates no text fields");

    // An unparseable or absent discriminant is NotValid with no id. It used to
    // be -1 here and 0 from Default - two encodings, and -1 is NotValid's own
    // discriminant.
    for message in [ResponseMessage::from("nonsense\0"), ResponseMessage::from(""), ResponseMessage::default()] {
        assert_eq!(message.message_type(), IncomingMessages::NotValid);
        assert_eq!(message.message_id(), None);
    }
    // An unrecognized id that did parse is kept.
    assert_eq!(ResponseMessage::from("-1\0").message_id(), Some(-1));
}

#[test]
fn response_message_fields_modification() {
    // Test that ResponseMessage handles field modification correctly
    let mut message = ResponseMessage::from("1\02\03\0");
    assert_eq!(message.fields.len(), 3);
    assert_eq!(message.fields[0], "1");
    assert_eq!(message.fields[1], "2");
    assert_eq!(message.fields[2], "3");

    // Test that we can read fields correctly after creation
    message.i = 0;
    assert_eq!(message.next_int().unwrap(), 1);
    assert_eq!(message.next_int().unwrap(), 2);
    assert_eq!(message.next_int().unwrap(), 3);
}

#[test]
fn incoming_messages_equality() {
    // Test that IncomingMessages enum variants are properly comparable
    assert_eq!(IncomingMessages::TickPrice, IncomingMessages::TickPrice);
    assert_ne!(IncomingMessages::TickPrice, IncomingMessages::TickSize);

    // Test with from conversion
    assert_eq!(IncomingMessages::from(1), IncomingMessages::TickPrice);
    assert_eq!(IncomingMessages::from(2), IncomingMessages::TickSize);
    assert_ne!(IncomingMessages::from(1), IncomingMessages::from(2));
}

// Additional tests for comprehensive FromStr coverage of OutgoingMessages
#[test]
fn outgoing_messages_from_str_comprehensive() {
    use std::str::FromStr;

    // Table-driven test for all OutgoingMessages variants
    let test_cases = vec![
        ("1", OutgoingMessages::RequestMarketData),
        ("2", OutgoingMessages::CancelMarketData),
        ("3", OutgoingMessages::PlaceOrder),
        ("4", OutgoingMessages::CancelOrder),
        ("5", OutgoingMessages::RequestOpenOrders),
        ("6", OutgoingMessages::RequestAccountData),
        ("7", OutgoingMessages::RequestExecutions),
        ("8", OutgoingMessages::RequestIds),
        ("9", OutgoingMessages::RequestContractData),
        ("10", OutgoingMessages::RequestMarketDepth),
        ("11", OutgoingMessages::CancelMarketDepth),
        ("12", OutgoingMessages::RequestNewsBulletins),
        ("13", OutgoingMessages::CancelNewsBulletin),
        ("14", OutgoingMessages::ChangeServerLog),
        ("15", OutgoingMessages::RequestAutoOpenOrders),
        ("16", OutgoingMessages::RequestAllOpenOrders),
        ("17", OutgoingMessages::RequestManagedAccounts),
        ("18", OutgoingMessages::RequestFA),
        ("19", OutgoingMessages::ReplaceFA),
        ("20", OutgoingMessages::RequestHistoricalData),
        ("21", OutgoingMessages::ExerciseOptions),
        ("22", OutgoingMessages::RequestScannerSubscription),
        ("23", OutgoingMessages::CancelScannerSubscription),
        ("24", OutgoingMessages::RequestScannerParameters),
        ("25", OutgoingMessages::CancelHistoricalData),
        ("49", OutgoingMessages::RequestCurrentTime),
        ("50", OutgoingMessages::RequestRealTimeBars),
        ("51", OutgoingMessages::CancelRealTimeBars),
        ("52", OutgoingMessages::RequestFundamentalData),
        ("53", OutgoingMessages::CancelFundamentalData),
        ("54", OutgoingMessages::ReqCalcImpliedVolat),
        ("55", OutgoingMessages::ReqCalcOptionPrice),
        ("56", OutgoingMessages::CancelImpliedVolatility),
        ("57", OutgoingMessages::CancelOptionPrice),
        ("58", OutgoingMessages::RequestGlobalCancel),
        ("59", OutgoingMessages::RequestMarketDataType),
        ("61", OutgoingMessages::RequestPositions),
        ("62", OutgoingMessages::RequestAccountSummary),
        ("63", OutgoingMessages::CancelAccountSummary),
        ("64", OutgoingMessages::CancelPositions),
        ("65", OutgoingMessages::VerifyRequest),
        ("66", OutgoingMessages::VerifyMessage),
        ("67", OutgoingMessages::QueryDisplayGroups),
        ("68", OutgoingMessages::SubscribeToGroupEvents),
        ("69", OutgoingMessages::UpdateDisplayGroup),
        ("70", OutgoingMessages::UnsubscribeFromGroupEvents),
        ("71", OutgoingMessages::StartApi),
        ("72", OutgoingMessages::VerifyAndAuthRequest),
        ("73", OutgoingMessages::VerifyAndAuthMessage),
        ("74", OutgoingMessages::RequestPositionsMulti),
        ("75", OutgoingMessages::CancelPositionsMulti),
        ("76", OutgoingMessages::RequestAccountUpdatesMulti),
        ("77", OutgoingMessages::CancelAccountUpdatesMulti),
        ("78", OutgoingMessages::RequestSecurityDefinitionOptionalParameters),
        ("79", OutgoingMessages::RequestSoftDollarTiers),
        ("80", OutgoingMessages::RequestFamilyCodes),
        ("81", OutgoingMessages::RequestMatchingSymbols),
        ("82", OutgoingMessages::RequestMktDepthExchanges),
        ("83", OutgoingMessages::RequestSmartComponents),
        ("84", OutgoingMessages::RequestNewsArticle),
        ("85", OutgoingMessages::RequestNewsProviders),
        ("86", OutgoingMessages::RequestHistoricalNews),
        ("87", OutgoingMessages::RequestHeadTimestamp),
        ("88", OutgoingMessages::RequestHistogramData),
        ("89", OutgoingMessages::CancelHistogramData),
        ("90", OutgoingMessages::CancelHeadTimestamp),
        ("91", OutgoingMessages::RequestMarketRule),
        ("92", OutgoingMessages::RequestPnL),
        ("93", OutgoingMessages::CancelPnL),
        ("94", OutgoingMessages::RequestPnLSingle),
        ("95", OutgoingMessages::CancelPnLSingle),
        ("96", OutgoingMessages::RequestHistoricalTicks),
        ("97", OutgoingMessages::RequestTickByTickData),
        ("98", OutgoingMessages::CancelTickByTickData),
        ("99", OutgoingMessages::RequestCompletedOrders),
        ("100", OutgoingMessages::RequestWshMetaData),
        ("101", OutgoingMessages::CancelWshMetaData),
        ("102", OutgoingMessages::RequestWshEventData),
        ("103", OutgoingMessages::CancelWshEventData),
        ("104", OutgoingMessages::RequestUserInfo),
        ("105", OutgoingMessages::RequestCurrentTimeInMillis),
        ("106", OutgoingMessages::CancelContractData),
        ("107", OutgoingMessages::CancelHistoricalTicks),
        ("108", OutgoingMessages::ReqConfig),
        ("109", OutgoingMessages::UpdateConfig),
    ];

    for (input, expected) in test_cases {
        let result = OutgoingMessages::from_str(input).unwrap();
        assert_eq!(result, expected, "Failed to parse '{}' as {:?}", input, expected);
    }

    // Test invalid cases
    assert!(OutgoingMessages::from_str("110").is_err());
    assert!(OutgoingMessages::from_str("999").is_err());
    assert!(OutgoingMessages::from_str("-1").is_err());
    assert!(OutgoingMessages::from_str("abc").is_err());
    assert!(OutgoingMessages::from_str("").is_err());
}

#[test]
fn outgoing_messages_try_from_round_trips() {
    for id in -1..=120 {
        if let Ok(message) = OutgoingMessages::try_from(id) {
            assert_eq!(message as i32, id);
        }
    }
}

#[test]
fn outgoing_messages_is_cancel() {
    // Oracle independent of the match: the variant name.
    for message in (0..=120).filter_map(|id| OutgoingMessages::try_from(id).ok()) {
        let name = format!("{message:?}");
        let expected = name.starts_with("Cancel") || matches!(name.as_str(), "RequestGlobalCancel" | "UnsubscribeFromGroupEvents");
        assert_eq!(message.is_cancel(), expected, "{name}");
    }
}

#[test]
fn outgoing_message_type_of_frame() {
    let proto = encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, b"payload");
    assert_eq!(outgoing_message_type(&proto), Some(OutgoingMessages::CancelMarketData));

    let text = (OutgoingMessages::RequestMarketData as i32).to_be_bytes();
    assert_eq!(outgoing_message_type(&text), Some(OutgoingMessages::RequestMarketData));

    assert_eq!(outgoing_message_type(&[0, 0, 0]), None, "short frame");
    assert_eq!(outgoing_message_type(&encode_protobuf_message(999, b"")), None, "unknown id");
}

#[test]
fn routes_by_request_id_comprehensive() {
    // Confirm the allow-list covers a representative slice across domains.
    assert!(routes_by_request_id(IncomingMessages::MarketDepthL2));
    assert!(routes_by_request_id(IncomingMessages::TickReqParams));
    assert!(routes_by_request_id(IncomingMessages::TickSnapshotEnd));

    // Shared/global messages are deliberately omitted.
    assert!(!routes_by_request_id(IncomingMessages::ManagedAccounts));
    assert!(!routes_by_request_id(IncomingMessages::NextValidId));
    assert!(!routes_by_request_id(IncomingMessages::CurrentTime));
}

#[test]
fn response_message_error_paths() {
    // Test empty message type detection
    let empty_msg = ResponseMessage::default();
    assert_eq!(empty_msg.message_type(), IncomingMessages::NotValid);
}

#[test]
fn response_message_special_double_values() {
    // Test parsing empty as 0.0
    let mut msg = ResponseMessage::from("test\0\0");
    msg.i = 1;
    let result = msg.next_double().unwrap();
    assert_eq!(result, 0.0);

    // Test parsing "0" as 0.0
    let mut msg = ResponseMessage::from("test\00\0");
    msg.i = 1;
    let result = msg.next_double().unwrap();
    assert_eq!(result, 0.0);

    // Test parsing "0.0" as 0.0
    let mut msg = ResponseMessage::from("test\00.0\0");
    msg.i = 1;
    let result = msg.next_double().unwrap();
    assert_eq!(result, 0.0);
}

#[cfg(test)]
mod from_str_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn outgoing_messages_from_str() {
        // Test some common message types
        assert_eq!(OutgoingMessages::from_str("1").unwrap(), OutgoingMessages::RequestMarketData);
        assert_eq!(OutgoingMessages::from_str("17").unwrap(), OutgoingMessages::RequestManagedAccounts);
        assert_eq!(OutgoingMessages::from_str("49").unwrap(), OutgoingMessages::RequestCurrentTime);
        assert_eq!(OutgoingMessages::from_str("61").unwrap(), OutgoingMessages::RequestPositions);

        // Test error cases
        assert!(OutgoingMessages::from_str("999").is_err());
        assert!(OutgoingMessages::from_str("abc").is_err());
        assert!(OutgoingMessages::from_str("").is_err());
    }

    #[test]
    fn outgoing_messages_roundtrip() {
        // Test that we can convert to string and back
        let msg = OutgoingMessages::RequestCurrentTime;
        let as_string = msg.to_string();
        let parsed = OutgoingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, OutgoingMessages::RequestCurrentTime);

        // Test with another message type
        let msg = OutgoingMessages::RequestManagedAccounts;
        let as_string = msg.to_string();
        let parsed = OutgoingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, OutgoingMessages::RequestManagedAccounts);
    }

    #[test]
    fn incoming_messages_from_str() {
        // Test some common message types
        assert_eq!(IncomingMessages::from_str("4").unwrap(), IncomingMessages::Error);
        assert_eq!(IncomingMessages::from_str("15").unwrap(), IncomingMessages::ManagedAccounts);
        assert_eq!(IncomingMessages::from_str("49").unwrap(), IncomingMessages::CurrentTime);
        assert_eq!(IncomingMessages::from_str("61").unwrap(), IncomingMessages::Position);

        // Test NotValid for unknown values
        assert_eq!(IncomingMessages::from_str("999").unwrap(), IncomingMessages::NotValid);
        assert_eq!(IncomingMessages::from_str("0").unwrap(), IncomingMessages::NotValid);
        assert_eq!(IncomingMessages::from_str("-1").unwrap(), IncomingMessages::NotValid);

        // Test error cases for non-numeric strings
        assert!(IncomingMessages::from_str("abc").is_err());
        assert!(IncomingMessages::from_str("").is_err());
        assert!(IncomingMessages::from_str("1.5").is_err());
    }

    #[test]
    fn incoming_messages_roundtrip() {
        // Test with CurrentTime message
        let n = 49;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);

        // Test with ManagedAccounts message
        let n = 15;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);

        // Test with NotValid (unknown value)
        let n = 999;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);
        assert_eq!(parsed, IncomingMessages::NotValid);
    }
}

#[test]
fn expect_type_narrows_or_rejects() {
    // Sibling of `require_proto`: narrows the type rather than the framing, and
    // rejects with the same terminating variant.
    let message = ResponseMessage::from(&format!("{}\0payload\0", IncomingMessages::FamilyCodes as i32));

    let matched = message.expect_type(IncomingMessages::FamilyCodes).expect("matching type passes through");
    assert_eq!(matched.message_type(), IncomingMessages::FamilyCodes);

    let err = message.expect_type(IncomingMessages::UserInfo).expect_err("mismatched type is rejected");
    assert!(matches!(err, Error::UnexpectedResponse(_)), "got {err:?}");
}
