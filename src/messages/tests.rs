use crate::contracts::{ComboLegOpenClose, SecurityType};
#[cfg(feature = "sync")]
use crate::orders::{Action, OrderCondition, OrderOpenClose, Rule80A};
use time::macros::datetime;

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
    OptionalInt,
    Long,
    OptionalLong,
    Double,
    OptionalDouble,
    String,
    Bool,
    DateTime,
}

enum ParseResult {
    Int(i32),
    OptionalInt(Option<i32>),
    Long(i64),
    OptionalLong(Option<i64>),
    Double(f64),
    OptionalDouble(Option<f64>),
    String(String),
    Bool(bool),
    DateTime(Result<time::OffsetDateTime, ()>),
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
            name: "parse_optional_int_present",
            input: "1\0123\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalInt,
            expected: ParseResult::OptionalInt(Some(123)),
        },
        ResponseMessageParseTestCase {
            name: "parse_optional_int_empty",
            input: "1\0\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalInt,
            expected: ParseResult::OptionalInt(None),
        },
        ResponseMessageParseTestCase {
            name: "parse_optional_int_unset",
            input: "1\02147483647\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalInt,
            expected: ParseResult::OptionalInt(None),
        },
        ResponseMessageParseTestCase {
            name: "parse_long",
            input: "1\09876543210\0456\0",
            field_index: 1,
            parse_type: ParseType::Long,
            expected: ParseResult::Long(9876543210),
        },
        ResponseMessageParseTestCase {
            name: "parse_optional_long_unset",
            input: "1\09223372036854775807\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalLong,
            expected: ParseResult::OptionalLong(None),
        },
        ResponseMessageParseTestCase {
            name: "parse_double",
            input: "1\03.14159\0456\0",
            field_index: 1,
            parse_type: ParseType::Double,
            expected: ParseResult::Double(3.14159),
        },
        ResponseMessageParseTestCase {
            name: "parse_double_zero",
            input: "1\00\0456\0",
            field_index: 1,
            parse_type: ParseType::Double,
            expected: ParseResult::Double(0.0),
        },
        ResponseMessageParseTestCase {
            name: "parse_optional_double_infinity",
            input: "1\0Infinity\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalDouble,
            expected: ParseResult::OptionalDouble(Some(f64::INFINITY)),
        },
        ResponseMessageParseTestCase {
            name: "parse_optional_double_unset",
            input: "1\01.7976931348623157E308\0456\0",
            field_index: 1,
            parse_type: ParseType::OptionalDouble,
            expected: ParseResult::OptionalDouble(None),
        },
        ResponseMessageParseTestCase {
            name: "parse_string",
            input: "1\0hello world\0456\0",
            field_index: 1,
            parse_type: ParseType::String,
            expected: ParseResult::String("hello world".to_string()),
        },
        ResponseMessageParseTestCase {
            name: "parse_bool_true",
            input: "1\01\0456\0",
            field_index: 1,
            parse_type: ParseType::Bool,
            expected: ParseResult::Bool(true),
        },
        ResponseMessageParseTestCase {
            name: "parse_bool_false",
            input: "1\00\0456\0",
            field_index: 1,
            parse_type: ParseType::Bool,
            expected: ParseResult::Bool(false),
        },
        ResponseMessageParseTestCase {
            name: "parse_datetime",
            input: "1\01609459200\0456\0", // 2021-01-01 00:00:00 UTC
            field_index: 1,
            parse_type: ParseType::DateTime,
            expected: ParseResult::DateTime(Ok(datetime!(2021-01-01 00:00:00 UTC))),
        },
    ]
}

#[test]
fn test_message_encodes_bool() {
    let mut message = RequestMessage::new();

    message.push_field(&false);
    message.push_field(&true);

    assert_eq!(2, message.fields.len());
    assert_eq!("0\01\0", message.encode());
}

#[test]
fn test_message_encodes_i32() {
    let mut message = RequestMessage::new();

    message.push_field(&1);
    message.push_field(&Some(2));
    message.push_field(&Option::<i32>::None);

    assert_eq!(3, message.fields.len());
    assert_eq!("1\02\0\0", message.encode());
}

#[test]
fn test_message_encodes_f64() {
    let mut message = RequestMessage::new();

    message.push_field(&2.0);
    message.push_field(&Some(3.0));
    message.push_field(&Option::<f64>::None);

    assert_eq!(3, message.fields.len());
    // assert_eq!("2.0\03.0\0\0", message.encode());
}

#[test]
fn test_message_encodes_string() {
    let mut message = RequestMessage::new();

    message.push_field(&"interactive");
    message.push_field(&"brokers");

    assert_eq!(2, message.fields.len());
    assert_eq!("interactive\0brokers\0", message.encode());
}

#[test]
#[cfg(feature = "sync")]
fn test_message_encodes_rule_80_a() {
    let mut message = RequestMessage::new();

    message.push_field(&Some(Rule80A::Individual));
    message.push_field(&Some(Rule80A::Agency));
    message.push_field(&Some(Rule80A::AgentOtherMember));
    message.push_field(&Some(Rule80A::IndividualPTIA));
    message.push_field(&Some(Rule80A::AgencyPTIA));
    message.push_field(&Some(Rule80A::AgentOtherMemberPTIA));
    message.push_field(&Some(Rule80A::IndividualPT));
    message.push_field(&Some(Rule80A::AgencyPT));
    message.push_field(&Some(Rule80A::AgentOtherMemberPT));
    message.push_field(&Option::<Rule80A>::None);

    assert_eq!(10, message.fields.len());
    assert_eq!("I\0A\0W\0J\0U\0M\0K\0Y\0N\0\0", message.encode());
}

#[test]
#[cfg(feature = "sync")]
fn test_message_encodes_order_condition() {
    let mut message = RequestMessage::new();

    message.push_field(&OrderCondition::Price);
    message.push_field(&OrderCondition::Time);
    message.push_field(&OrderCondition::Margin);
    message.push_field(&OrderCondition::Execution);
    message.push_field(&OrderCondition::Volume);
    message.push_field(&OrderCondition::PercentChange);

    assert_eq!(6, message.fields.len());
    assert_eq!("1\03\04\05\06\07\0", message.encode());
}

#[test]
#[cfg(feature = "sync")]
fn test_message_encodes_action() {
    let mut message = RequestMessage::new();

    message.push_field(&Action::Buy);
    message.push_field(&Action::Sell);
    message.push_field(&Action::SellShort);
    message.push_field(&Action::SellLong);

    assert_eq!(4, message.fields.len());
    assert_eq!("BUY\0SELL\0SSHORT\0SLONG\0", message.encode());
}

#[test]
fn test_message_encodes_security_type() {
    let mut message = RequestMessage::new();

    message.push_field(&SecurityType::Stock);
    message.push_field(&SecurityType::Option);
    message.push_field(&SecurityType::Future);
    message.push_field(&SecurityType::Index);
    message.push_field(&SecurityType::FuturesOption);
    message.push_field(&SecurityType::ForexPair);
    message.push_field(&SecurityType::Spread);
    message.push_field(&SecurityType::Warrant);
    message.push_field(&SecurityType::Bond);
    message.push_field(&SecurityType::Commodity);
    message.push_field(&SecurityType::News);
    message.push_field(&SecurityType::MutualFund);
    message.push_field(&SecurityType::Crypto);
    message.push_field(&SecurityType::CFD);
    message.push_field(&SecurityType::Other("??".to_owned()));

    assert_eq!(15, message.fields.len());
    assert_eq!(
        "STK\0OPT\0FUT\0IND\0FOP\0CASH\0BAG\0WAR\0BOND\0CMDTY\0NEWS\0FUND\0CRYPTO\0CFD\0??\0",
        message.encode()
    );
}

#[test]
fn test_message_encodes_outgoing_message() {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestMarketData);
    message.push_field(&OutgoingMessages::CancelMarketData);
    message.push_field(&OutgoingMessages::PlaceOrder);
    message.push_field(&OutgoingMessages::RequestUserInfo);

    assert_eq!(4, message.fields.len());
    assert_eq!("1\02\03\0104\0", message.encode());
}

#[test]
#[cfg(feature = "sync")]
fn test_message_encodes_order_open_close() {
    let mut message = RequestMessage::new();

    message.push_field(&Option::<OrderOpenClose>::None);
    message.push_field(&OrderOpenClose::Open);
    message.push_field(&OrderOpenClose::Close);

    assert_eq!(3, message.fields.len());
    assert_eq!("\0O\0C\0", message.encode());
}

#[test]
fn test_message_encodes_combo_leg_open_close() {
    let mut message = RequestMessage::new();

    message.push_field(&ComboLegOpenClose::Same);
    message.push_field(&ComboLegOpenClose::Open);
    message.push_field(&ComboLegOpenClose::Close);
    message.push_field(&ComboLegOpenClose::Unknown);

    assert_eq!(4, message.fields.len());
    assert_eq!("0\01\02\03\0", message.encode());
}

#[test]
fn test_incoming_message_from_i32() {
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
    assert_eq!(IncomingMessages::from(108), IncomingMessages::NotValid);
}

#[test]
fn test_order_id_index() {
    assert_eq!(order_id_index(IncomingMessages::OpenOrder), Some(1));
    assert_eq!(order_id_index(IncomingMessages::OrderStatus), Some(1));

    assert_eq!(order_id_index(IncomingMessages::ExecutionData), Some(2));
    assert_eq!(order_id_index(IncomingMessages::ExecutionDataEnd), Some(2));

    assert_eq!(order_id_index(IncomingMessages::NotValid), None);
}

#[test]
fn test_request_id_index() {
    assert_eq!(request_id_index(IncomingMessages::ContractData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::TickByTick), Some(1));
    assert_eq!(request_id_index(IncomingMessages::SymbolSamples), Some(1));
    assert_eq!(request_id_index(IncomingMessages::OpenOrder), Some(1));
    assert_eq!(request_id_index(IncomingMessages::ExecutionData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HeadTimestamp), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HistoricalData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HistoricalSchedule), Some(1));

    assert_eq!(request_id_index(IncomingMessages::ContractDataEnd), Some(2));
    assert_eq!(request_id_index(IncomingMessages::RealTimeBars), Some(2));
    assert_eq!(request_id_index(IncomingMessages::Error), Some(2));
    assert_eq!(request_id_index(IncomingMessages::ExecutionDataEnd), Some(2));
}

#[test]
fn test_request_id_index_invalid() {
    assert_eq!(request_id_index(IncomingMessages::NotValid), None);
}

#[test]
fn test_notice() {
    let message = ResponseMessage::from("4\02\0-1\02107\0HMDS data farm connection is inactive.\0");

    let notice = Notice::from(&message);

    assert_eq!(notice.code, 2107);
    assert_eq!(notice.message, "HMDS data farm connection is inactive.");
    assert_eq!(format!("{notice}"), "[2107] HMDS data farm connection is inactive.");
}

#[test]
fn test_encode_length() {
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
fn test_response_message_parsing() {
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
            (ParseType::OptionalInt, ParseResult::OptionalInt(expected)) => match message.next_optional_int() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::Long, ParseResult::Long(expected)) => match message.next_long() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::OptionalLong, ParseResult::OptionalLong(expected)) => match message.next_optional_long() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
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
            (ParseType::OptionalDouble, ParseResult::OptionalDouble(expected)) => match (message.next_optional_double(), expected) {
                (Ok(Some(val)), Some(exp)) if exp.is_infinite() => {
                    assert!(
                        val.is_infinite() && val.is_sign_positive() == exp.is_sign_positive(),
                        "Test '{}' failed: expected {:?}, got {:?}",
                        test_case.name,
                        expected,
                        val
                    );
                }
                (Ok(Some(val)), Some(exp)) => {
                    assert!(
                        (val - exp).abs() < f64::EPSILON,
                        "Test '{}' failed: expected {:?}, got {:?}",
                        test_case.name,
                        expected,
                        val
                    );
                }
                (Ok(None), None) => {}
                (result, exp) => panic!("Test '{}' failed: expected {:?}, got {:?}", test_case.name, exp, result),
            },
            (ParseType::String, ParseResult::String(expected)) => match message.next_string() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::Bool, ParseResult::Bool(expected)) => match message.next_bool() {
                Ok(val) => assert_eq!(val, *expected, "Test '{}' failed", test_case.name),
                Err(e) => panic!("Test '{}' failed: expected {:?}, got error: {:?}", test_case.name, expected, e),
            },
            (ParseType::DateTime, ParseResult::DateTime(expected)) => match (message.next_date_time(), expected) {
                (Ok(val), Ok(exp)) => {
                    assert_eq!(val, *exp, "Test '{}' failed", test_case.name);
                }
                (Err(_), Err(_)) => {}
                (result, exp) => panic!("Test '{}' failed: expected {:?}, got {:?}", test_case.name, exp, result),
            },
            _ => panic!("Test case type mismatch"),
        }
    }
}

#[test]
fn test_response_message_boundary_conditions() {
    // Test reading past end of message
    let mut message = ResponseMessage::from("1\02\0");
    message.i = 3; // Beyond the last field

    assert!(message.next_int().is_err());
    assert!(message.next_optional_int().is_err());
    assert!(message.next_long().is_err());
    assert!(message.next_optional_long().is_err());
    assert!(message.next_double().is_err());
    assert!(message.next_optional_double().is_err());
    assert!(message.next_string().is_err());
    assert!(message.next_bool().is_err());
    assert!(message.next_date_time().is_err());
}

#[test]
fn test_response_message_peek_operations() {
    let message = ResponseMessage::from("1\0123\0abc\0456\0");

    // Test peek_int
    assert_eq!(message.peek_int(1).unwrap(), 123);
    assert_eq!(message.peek_int(3).unwrap(), 456);
    assert!(message.peek_int(2).is_err()); // "abc" is not an int
    assert!(message.peek_int(4).is_err()); // Out of bounds (only 4 fields, indices 0-3)

    // Test peek_string
    assert_eq!(message.peek_string(2), "abc");
    assert_eq!(message.peek_string(0), "1");
}

#[test]
fn test_response_message_skip() {
    let mut message = ResponseMessage::from("1\02\03\04\0");

    assert_eq!(message.i, 0);
    message.skip();
    assert_eq!(message.i, 1);

    assert_eq!(message.next_int().unwrap(), 2);
    assert_eq!(message.i, 2);

    message.skip();
    assert_eq!(message.i, 3);

    assert_eq!(message.next_int().unwrap(), 4);
}

#[test]
fn test_response_message_special_fields() {
    // OpenOrder message (message type 5) - order_id is at index 1
    let open_order = ResponseMessage::from("5\0123\0field2\0field3\0");
    assert_eq!(open_order.order_id(), Some(123));
    assert_eq!(open_order.execution_id(), None);

    // OrderStatus message (message type 3) - order_id is at index 1
    let order_status = ResponseMessage::from("3\0456\0field2\0field3\0");
    assert_eq!(order_status.order_id(), Some(456));

    // ExecutionData message (message type 11) - order_id is at index 2, execution_id at index 14
    let mut exec_fields = vec!["11"];
    for i in 1..=14 {
        if i == 2 {
            exec_fields.push("789"); // order_id at index 2
        } else if i == 14 {
            exec_fields.push("exec789"); // execution_id at index 14
        } else {
            exec_fields.push("field");
        }
    }
    let exec_message = ResponseMessage::from(&exec_fields.join("\0"));
    assert_eq!(exec_message.order_id(), Some(789));
    assert_eq!(exec_message.execution_id(), Some("exec789".to_string()));

    // CommissionsReport message (message type 59) - execution_id at index 2
    let commission_message = ResponseMessage::from("59\0field1\0exec123\0");
    assert_eq!(commission_message.execution_id(), Some("exec123".to_string()));
}

#[test]
fn test_request_message_index() {
    let message = RequestMessage {
        fields: vec!["field0".to_string(), "field1".to_string(), "field2".to_string()],
    };

    assert_eq!(message[0], "field0");
    assert_eq!(message[1], "field1");
    assert_eq!(message[2], "field2");
}

#[test]
#[should_panic]
fn test_request_message_index_out_of_bounds() {
    let message = RequestMessage {
        fields: vec!["field0".to_string()],
    };

    let _ = &message[1]; // Should panic
}

#[test]
fn test_response_message_is_empty() {
    let empty_message = ResponseMessage::default();
    assert!(empty_message.is_empty());
    assert_eq!(empty_message.len(), 0);

    let non_empty_message = ResponseMessage::from("1\02\03\0");
    assert!(!non_empty_message.is_empty());
    assert_eq!(non_empty_message.len(), 3);
}

#[test]
fn test_response_message_is_shutdown() {
    let shutdown_message = ResponseMessage::from("-2\0");
    assert!(shutdown_message.is_shutdown());

    let normal_message = ResponseMessage::from("1\0");
    assert!(!normal_message.is_shutdown());
}

#[test]
fn test_response_message_encode_decode_roundtrip() {
    let original = ResponseMessage::from("1\0test\0123\03.14\0");
    let encoded = original.encode();
    let decoded = ResponseMessage::from(&encoded);

    assert_eq!(original.fields, decoded.fields);
}
