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
    use crate::orders::conditions::*;

    let mut message = RequestMessage::new();

    message.push_field(&OrderCondition::Price(PriceCondition::default()));
    message.push_field(&OrderCondition::Time(TimeCondition::default()));
    message.push_field(&OrderCondition::Margin(MarginCondition::default()));
    message.push_field(&OrderCondition::Execution(ExecutionCondition::default()));
    message.push_field(&OrderCondition::Volume(VolumeCondition::default()));
    message.push_field(&OrderCondition::PercentChange(PercentChangeCondition::default()));

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
    let original = ResponseMessage::from("1\0test\0123\03.456\0");
    let encoded = original.encode();
    let decoded = ResponseMessage::from(&encoded);

    assert_eq!(original.fields, decoded.fields);
}

// Table-driven tests for parser_registry module
#[test]
fn test_field_based_parser() {
    use super::parser_registry::{FieldBasedParser, FieldDef, MessageParser};

    struct TestCase {
        name: &'static str,
        fields: Vec<FieldDef>,
        input: Vec<&'static str>,
        expected_count: usize,
        expected_values: Vec<(&'static str, &'static str)>,
    }

    let test_cases = vec![
        TestCase {
            name: "basic_parsing",
            fields: vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
            ],
            input: vec!["49", "1", "12345"],
            expected_count: 3,
            expected_values: vec![("message_type", "49"), ("version", "1"), ("request_id", "12345")],
        },
        TestCase {
            name: "missing_fields",
            fields: vec![FieldDef::new(0, "field1"), FieldDef::new(1, "field2"), FieldDef::new(5, "field6")],
            input: vec!["val1", "val2"],
            expected_count: 2,
            expected_values: vec![("field1", "val1"), ("field2", "val2")],
        },
        TestCase {
            name: "empty_input",
            fields: vec![FieldDef::new(0, "field1")],
            input: vec![],
            expected_count: 0,
            expected_values: vec![],
        },
        TestCase {
            name: "with_transform",
            fields: vec![FieldDef::new(0, "upper").with_transform(|s| s.to_uppercase())],
            input: vec!["hello"],
            expected_count: 1,
            expected_values: vec![("upper", "HELLO")],
        },
    ];

    for test_case in test_cases {
        let parser = FieldBasedParser::new(test_case.fields);
        let result = parser.parse(&test_case.input);

        assert_eq!(
            result.len(),
            test_case.expected_count,
            "Test '{}' failed: wrong field count",
            test_case.name
        );

        for (field, (expected_name, expected_value)) in result.iter().zip(test_case.expected_values.iter()) {
            assert_eq!(field.name, *expected_name, "Test '{}' failed: wrong field name", test_case.name);
            assert_eq!(field.value, *expected_value, "Test '{}' failed: wrong field value", test_case.name);
        }
    }
}

#[test]
fn test_timestamp_parser() {
    use super::parser_registry::{FieldBasedParser, FieldDef, MessageParser, TimestampParser};

    struct TestCase {
        name: &'static str,
        base_fields: Vec<FieldDef>,
        timestamp_index: usize,
        input: Vec<&'static str>,
        expect_parsed_timestamp: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "valid_timestamp",
            base_fields: vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "timestamp")],
            timestamp_index: 1,
            input: vec!["49", "1609459200"], // 2021-01-01 00:00:00 UTC
            expect_parsed_timestamp: true,
        },
        TestCase {
            name: "invalid_timestamp",
            base_fields: vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "timestamp")],
            timestamp_index: 1,
            input: vec!["49", "not_a_timestamp"],
            expect_parsed_timestamp: false,
        },
        TestCase {
            name: "missing_timestamp_field",
            base_fields: vec![FieldDef::new(0, "message_type")],
            timestamp_index: 3,
            input: vec!["49"],
            expect_parsed_timestamp: false,
        },
    ];

    for test_case in test_cases {
        let base_parser = FieldBasedParser::new(test_case.base_fields);
        let parser = TimestampParser::new(base_parser, test_case.timestamp_index);
        let result = parser.parse(&test_case.input);

        let has_parsed_timestamp = result.iter().any(|f| f.name == "timestamp_parsed");
        assert_eq!(
            has_parsed_timestamp, test_case.expect_parsed_timestamp,
            "Test '{}' failed: timestamp parsing expectation mismatch",
            test_case.name
        );
    }
}

#[test]
fn test_message_parser_registry() {
    use super::parser_registry::MessageParserRegistry;

    struct TestCase {
        name: &'static str,
        msg_type: OutgoingMessages,
        input: Vec<&'static str>,
        expected_fields: Vec<(&'static str, &'static str)>,
    }

    let test_cases = vec![
        TestCase {
            name: "request_current_time",
            msg_type: OutgoingMessages::RequestCurrentTime,
            input: vec!["49", "1"],
            expected_fields: vec![("message_type", "49"), ("version", "1")],
        },
        TestCase {
            name: "request_account_summary",
            msg_type: OutgoingMessages::RequestAccountSummary,
            input: vec!["62", "1", "123", "All", "NetLiquidation"],
            expected_fields: vec![
                ("message_type", "62"),
                ("version", "1"),
                ("request_id", "123"),
                ("group", "All"),
                ("tags", "NetLiquidation"),
            ],
        },
        TestCase {
            name: "request_pnl",
            msg_type: OutgoingMessages::RequestPnL,
            input: vec!["92", "456", "DU12345", ""],
            expected_fields: vec![("message_type", "92"), ("request_id", "456"), ("account", "DU12345"), ("model_code", "")],
        },
    ];

    let registry = MessageParserRegistry::new();

    for test_case in test_cases {
        let result = registry.parse_request(test_case.msg_type, &test_case.input);

        assert_eq!(
            result.len(),
            test_case.expected_fields.len(),
            "Test '{}' failed: wrong field count",
            test_case.name
        );

        for (field, (expected_name, expected_value)) in result.iter().zip(test_case.expected_fields.iter()) {
            assert_eq!(field.name, *expected_name, "Test '{}' failed: wrong field name", test_case.name);
            assert_eq!(field.value, *expected_value, "Test '{}' failed: wrong field value", test_case.name);
        }
    }
}

#[test]
fn test_response_parser_registry() {
    use super::parser_registry::MessageParserRegistry;

    struct TestCase {
        name: &'static str,
        msg_type: IncomingMessages,
        input: Vec<&'static str>,
        min_expected_fields: usize,
    }

    let test_cases = vec![
        TestCase {
            name: "error_message",
            msg_type: IncomingMessages::Error,
            input: vec!["4", "2", "-1", "2107", "HMDS data farm connection is inactive."],
            min_expected_fields: 5,
        },
        TestCase {
            name: "managed_accounts",
            msg_type: IncomingMessages::ManagedAccounts,
            input: vec!["15", "1", "DU12345,DU67890"],
            min_expected_fields: 3,
        },
        TestCase {
            name: "position",
            msg_type: IncomingMessages::Position,
            input: vec![
                "61", "3", "DU12345", "12345", "AAPL", "STK", "", "0", "", "", "NASDAQ", "USD", "AAPL", "NMS", "100", "150.50",
            ],
            min_expected_fields: 16,
        },
        TestCase {
            name: "pnl_single",
            msg_type: IncomingMessages::PnLSingle,
            input: vec!["95", "123", "100", "50.25", "75.50", "125.75", "10000"],
            min_expected_fields: 7,
        },
    ];

    let registry = MessageParserRegistry::new();

    for test_case in test_cases {
        let result = registry.parse_response(test_case.msg_type, &test_case.input);

        assert!(
            result.len() >= test_case.min_expected_fields,
            "Test '{}' failed: expected at least {} fields, got {}",
            test_case.name,
            test_case.min_expected_fields,
            result.len()
        );
    }
}

#[test]
fn test_parse_generic_message() {
    use super::parser_registry::parse_generic_message;

    struct TestCase {
        name: &'static str,
        input: Vec<&'static str>,
        expected_fields: Vec<(&'static str, &'static str)>,
    }

    let test_cases = vec![
        TestCase {
            name: "simple_message",
            input: vec!["100", "field1", "field2", "field3"],
            expected_fields: vec![
                ("message_type", "100"),
                ("field_2", "field1"),
                ("field_3", "field2"),
                ("field_4", "field3"),
            ],
        },
        TestCase {
            name: "message_with_trailing_empty",
            input: vec!["200", "value", ""],
            expected_fields: vec![("message_type", "200"), ("field_2", "value")],
        },
        TestCase {
            name: "single_field",
            input: vec!["300"],
            expected_fields: vec![("message_type", "300")],
        },
    ];

    for test_case in test_cases {
        let result = parse_generic_message(&test_case.input);

        assert_eq!(
            result.len(),
            test_case.expected_fields.len(),
            "Test '{}' failed: wrong field count",
            test_case.name
        );

        for (field, (expected_name, expected_value)) in result.iter().zip(test_case.expected_fields.iter()) {
            assert_eq!(field.name, *expected_name, "Test '{}' failed: wrong field name", test_case.name);
            assert_eq!(field.value, *expected_value, "Test '{}' failed: wrong field value", test_case.name);
        }
    }
}

#[test]
fn test_custom_parser_registration() {
    use super::parser_registry::{MessageParser, MessageParserRegistry};

    struct CustomParser;
    impl MessageParser for CustomParser {
        fn parse(&self, _parts: &[&str]) -> Vec<super::parser_registry::ParsedField> {
            vec![super::parser_registry::ParsedField {
                name: "custom".to_string(),
                value: "parser".to_string(),
            }]
        }
    }

    let mut registry = MessageParserRegistry::new();

    // Register custom parsers
    registry.register_request_parser(OutgoingMessages::RequestGlobalCancel, Box::new(CustomParser));
    registry.register_response_parser(IncomingMessages::NewsArticle, Box::new(CustomParser));

    // Test custom request parser
    let result = registry.parse_request(OutgoingMessages::RequestGlobalCancel, &["58"]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "custom");
    assert_eq!(result[0].value, "parser");

    // Test custom response parser
    let result = registry.parse_response(IncomingMessages::NewsArticle, &["83"]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "custom");
    assert_eq!(result[0].value, "parser");
}

// Tests for error conditions and edge cases
#[test]
fn test_response_message_next_methods_edge_cases() {
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
            name: "malformed_long",
            input: "not_a_long\0",
            test_fn: |msg| {
                msg.i = 0;
                msg.next_long().is_err()
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
            name: "malformed_bool",
            input: "2\0",
            test_fn: |msg| {
                msg.i = 0;
                // next_bool returns Ok(false) for any non-"1" value
                let result = msg.next_bool();
                result.is_ok() && !result.unwrap()
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
        TestCase {
            name: "negative_timestamp",
            input: "-1000\0",
            test_fn: |msg| {
                msg.i = 0;
                // Negative timestamps are valid (dates before 1970)
                msg.next_date_time().is_ok()
            },
        },
    ];

    for test_case in test_cases {
        let mut message = ResponseMessage::from(test_case.input);
        assert!((test_case.test_fn)(&mut message), "Test '{}' failed", test_case.name);
    }
}

#[test]
fn test_request_message_push_field_edge_cases() {
    // Test with very large numbers
    let mut message = RequestMessage::new();
    message.push_field(&i32::MAX);
    message.push_field(&i32::MIN);
    message.push_field(&format!("{}", i64::MAX).as_str());
    message.push_field(&format!("{}", i64::MIN).as_str());
    assert_eq!(message.fields.len(), 4);

    // Test with special floats
    let mut message = RequestMessage::new();
    message.push_field(&f64::INFINITY);
    message.push_field(&f64::NEG_INFINITY);
    message.push_field(&f64::NAN);
    assert_eq!(message.fields.len(), 3);

    // Test with empty strings
    let mut message = RequestMessage::new();
    message.push_field(&"");
    message.push_field(&Some(""));
    message.push_field(&Option::<&str>::None);
    assert_eq!(message.encode(), "\0\0\0");

    // Test with strings containing special characters
    let mut message = RequestMessage::new();
    message.push_field(&"hello\nworld");
    message.push_field(&"tab\there");
    message.push_field(&"null\0byte"); // This should only encode up to the null byte
    assert_eq!(message.fields.len(), 3);
}

#[test]
fn test_channel_mappings_completeness() {
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
fn test_notice_edge_cases() {
    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected_code: i32,
        expected_message: &'static str,
    }

    let test_cases = vec![
        TestCase {
            name: "normal_error",
            input: "4\02\0-1\02107\0HMDS data farm connection is inactive.\0",
            expected_code: 2107,
            expected_message: "HMDS data farm connection is inactive.",
        },
        TestCase {
            name: "empty_message",
            input: "4\02\0-1\01000\0\0",
            expected_code: 1000,
            expected_message: "",
        },
        TestCase {
            name: "negative_code",
            input: "4\02\0-1\0-500\0Negative error code\0",
            expected_code: -500,
            expected_message: "Negative error code",
        },
    ];

    for test_case in test_cases {
        let message = ResponseMessage::from(test_case.input);
        let notice = Notice::from(&message);

        assert_eq!(notice.code, test_case.expected_code, "Test '{}' failed: wrong error code", test_case.name);
        assert_eq!(
            notice.message, test_case.expected_message,
            "Test '{}' failed: wrong error message",
            test_case.name
        );
    }
}

#[test]
fn test_notice_is_cancellation() {
    // Code 202 = order cancelled
    let cancellation = Notice {
        code: 202,
        message: "Order Cancelled - reason:".to_string(),
    };
    assert!(cancellation.is_cancellation());
    assert!(!cancellation.is_warning());
    assert!(!cancellation.is_system_message());
    assert!(cancellation.is_informational());
    assert!(!cancellation.is_error());

    // Other codes are not cancellations
    let error = Notice {
        code: 200,
        message: "No security definition found".to_string(),
    };
    assert!(!error.is_cancellation());
}

#[test]
fn test_notice_is_warning() {
    // Codes 2100-2169 are warnings
    let warning_codes = [2100, 2107, 2119, 2150, 2169];
    for code in warning_codes {
        let notice = Notice {
            code,
            message: format!("Warning with code {}", code),
        };
        assert!(notice.is_warning(), "Code {} should be a warning", code);
        assert!(!notice.is_cancellation());
        assert!(!notice.is_system_message());
        assert!(notice.is_informational());
        assert!(!notice.is_error());
    }

    // Codes outside 2100-2169 are not warnings
    let non_warning_codes = [2099, 2170, 200, 202, 1000];
    for code in non_warning_codes {
        let notice = Notice {
            code,
            message: format!("Non-warning with code {}", code),
        };
        assert!(!notice.is_warning(), "Code {} should not be a warning", code);
    }
}

#[test]
fn test_notice_is_system_message() {
    // System message codes: 1100, 1101, 1102, 1300
    let system_codes = [
        (1100, "Connectivity between IB and TWS has been lost."),
        (1101, "Connectivity restored, data lost."),
        (1102, "Connectivity restored, data maintained."),
        (1300, "Socket port has been reset."),
    ];
    for (code, msg) in system_codes {
        let notice = Notice {
            code,
            message: msg.to_string(),
        };
        assert!(notice.is_system_message(), "Code {} should be a system message", code);
        assert!(!notice.is_cancellation());
        assert!(!notice.is_warning());
        assert!(notice.is_informational());
        assert!(!notice.is_error());
    }

    // Non-system codes
    let non_system_codes = [200, 202, 1099, 1103, 1299, 1301, 2100];
    for code in non_system_codes {
        let notice = Notice {
            code,
            message: format!("Non-system message with code {}", code),
        };
        assert!(!notice.is_system_message(), "Code {} should not be a system message", code);
    }
}

#[test]
fn test_notice_is_informational() {
    // Informational includes cancellations, warnings, and system messages
    let informational_codes = [202, 1100, 1101, 1102, 1300, 2100, 2107, 2169];
    for code in informational_codes {
        let notice = Notice {
            code,
            message: format!("Informational code {}", code),
        };
        assert!(notice.is_informational(), "Code {} should be informational", code);
        assert!(!notice.is_error(), "Code {} should not be an error", code);
    }

    // Non-informational (actual errors)
    let error_codes = [100, 200, 201, 321, 502, 10000];
    for code in error_codes {
        let notice = Notice {
            code,
            message: format!("Error code {}", code),
        };
        assert!(!notice.is_informational(), "Code {} should not be informational", code);
        assert!(notice.is_error(), "Code {} should be an error", code);
    }
}

#[test]
fn test_notice_is_error() {
    // Code 200 = actual error
    let error = Notice {
        code: 200,
        message: "No security definition found".to_string(),
    };
    assert!(error.is_error());
    assert!(!error.is_informational());

    // Code 202 = cancellation, not error
    let cancellation = Notice {
        code: 202,
        message: "Order Cancelled".to_string(),
    };
    assert!(!cancellation.is_error());
    assert!(cancellation.is_informational());

    // Code 1100 = system message, not error
    let system_msg = Notice {
        code: 1100,
        message: "Connectivity lost".to_string(),
    };
    assert!(!system_msg.is_error());
    assert!(system_msg.is_informational());

    // Code 2107 = warning, not error
    let warning = Notice {
        code: 2107,
        message: "HMDS data farm connection is inactive.".to_string(),
    };
    assert!(!warning.is_error());
    assert!(warning.is_informational());
}

#[test]
fn test_all_incoming_message_conversions() {
    // Test boundary values and ensure all message types are covered
    let test_cases = vec![
        (0, IncomingMessages::NotValid),
        (1, IncomingMessages::TickPrice),
        (108, IncomingMessages::NotValid),
        (109, IncomingMessages::NotValid),
        (i32::MAX, IncomingMessages::NotValid),
        (i32::MIN, IncomingMessages::NotValid),
        (-1, IncomingMessages::NotValid),
    ];

    for (value, expected) in test_cases {
        assert_eq!(IncomingMessages::from(value), expected, "Failed for value {}", value);
    }
}

#[test]
fn test_outgoing_message_display() {
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
fn test_encode_length_edge_cases() {
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
fn test_response_message_access_patterns() {
    let message = ResponseMessage::from("5\0123\0field2\0field3\0field4\0");

    // Test order_id for OpenOrder message
    assert_eq!(message.order_id(), Some(123));

    // Test message_type
    assert_eq!(message.message_type(), IncomingMessages::OpenOrder);

    // Test multiple peeks don't change state
    assert_eq!(message.peek_string(2), "field2");
    assert_eq!(message.peek_string(2), "field2");
    assert_eq!(message.peek_int(1).unwrap(), 123);
    assert_eq!(message.peek_int(1).unwrap(), 123);

    // Test len and is_empty
    assert_eq!(message.len(), 5);
    assert!(!message.is_empty());
}

#[test]
fn test_response_message_fields_modification() {
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
fn test_incoming_messages_equality() {
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
fn test_outgoing_messages_from_str_comprehensive() {
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
    ];

    for (input, expected) in test_cases {
        let result = OutgoingMessages::from_str(input).unwrap();
        assert_eq!(result, expected, "Failed to parse '{}' as {:?}", input, expected);
    }

    // Test invalid cases
    assert!(OutgoingMessages::from_str("105").is_err());
    assert!(OutgoingMessages::from_str("999").is_err());
    assert!(OutgoingMessages::from_str("-1").is_err());
    assert!(OutgoingMessages::from_str("abc").is_err());
    assert!(OutgoingMessages::from_str("").is_err());
}

#[test]
fn test_request_id_index_comprehensive() {
    // Test message types with request_id at different indices
    assert_eq!(request_id_index(IncomingMessages::MarketDepthL2), Some(2));
    assert_eq!(request_id_index(IncomingMessages::TickEFP), Some(2));
    assert_eq!(request_id_index(IncomingMessages::TickReqParams), Some(1));
    assert_eq!(request_id_index(IncomingMessages::TickSnapshotEnd), Some(2));

    // Test message types without request_id
    assert_eq!(request_id_index(IncomingMessages::ManagedAccounts), None);
    assert_eq!(request_id_index(IncomingMessages::NextValidId), None);
    assert_eq!(request_id_index(IncomingMessages::CurrentTime), None);
}

#[test]
fn test_response_message_error_paths() {
    // Test empty message type detection
    let empty_msg = ResponseMessage::default();
    assert_eq!(empty_msg.message_type(), IncomingMessages::NotValid);

    // Test parsing errors for optional types
    let mut msg = ResponseMessage::from("test\0not_a_number\0");
    msg.i = 1;
    assert!(msg.next_optional_int().is_err());

    msg.i = 1;
    assert!(msg.next_optional_long().is_err());

    msg.i = 1;
    assert!(msg.next_optional_double().is_err());

    // Test empty timestamp error
    let mut msg = ResponseMessage::from("test\0\0");
    msg.i = 1;
    assert!(msg.next_date_time().is_err());

    // Test invalid timestamp parsing
    let mut msg = ResponseMessage::from("test\0not_a_timestamp\0");
    msg.i = 1;
    assert!(msg.next_date_time().is_err());

    // Test timestamp conversion error (out of range)
    let mut msg = ResponseMessage::from("test\099999999999999999999\0");
    msg.i = 1;
    let result = msg.next_date_time();
    assert!(result.is_err());
}

#[test]
fn test_response_message_special_double_values() {
    // Test parsing Infinity
    let mut msg = ResponseMessage::from("test\0Infinity\0");
    msg.i = 1;
    let result = msg.next_optional_double().unwrap();
    assert_eq!(result, Some(f64::INFINITY));

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
