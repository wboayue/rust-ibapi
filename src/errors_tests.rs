use super::*;
use crate::market_data::historical::HistoricalParseError;
use crate::messages::ResponseMessage;
use crate::orders::builder::ValidationError;
use crate::transport::routing::DecodedError;
use std::error::Error as StdError;
use std::io;
use std::sync::{Mutex, PoisonError};
use time::macros::format_description;
use time::Time;

#[test]
fn test_error_debug() {
    let error = Error::Simple("test error".to_string());
    assert_eq!(format!("{error:?}"), "Simple(\"test error\")");
}

#[test]
fn test_error_display() {
    let cases = vec![
        (Error::Io(io::Error::new(io::ErrorKind::NotFound, "file not found")), "file not found"),
        (Error::ParseInt("123x".parse::<i32>().unwrap_err()), "invalid digit found in string"),
        (
            Error::FromUtf8(String::from_utf8(vec![0, 159, 146, 150]).unwrap_err()),
            "invalid utf-8 sequence of 1 bytes from index 1",
        ),
        (
            Error::ParseTime(Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err()),
            "the 'month' component could not be parsed",
        ),
        (Error::Poison("test poison".to_string()), "test poison"),
        (Error::NotImplemented, "not implemented"),
        (
            Error::Parse(1, "value".to_string(), "message".to_string()),
            "parse error: 1 - value - message",
        ),
        (
            Error::ServerVersion(2, 1, "old version".to_string()),
            "server version 2 required, got 1: old version",
        ),
        (Error::Simple("simple error".to_string()), "error occurred: simple error"),
        (Error::InvalidArgument("bad arg".to_string()), "InvalidArgument: bad arg"),
        (Error::ConnectionFailed, "ConnectionFailed"),
        (Error::ConnectionReset, "ConnectionReset"),
        (Error::Cancelled, "Cancelled"),
        (Error::Shutdown, "Shutdown"),
        (Error::EndOfStream, "EndOfStream"),
        (Error::UnexpectedEndOfStream, "UnexpectedEndOfStream"),
        (Error::Message(200, "No security found".to_string()), "[200] No security found"),
        (Error::AlreadySubscribed, "AlreadySubscribed"),
        (
            Error::HistoricalParseError(HistoricalParseError::BarSize("bogus".to_string())),
            "HistoricalParseError: Invalid BarSize input 'bogus'",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn unsupported_timezone_display_contains_alias_and_helpers() {
    let error = Error::UnsupportedTimeZone("US/Foo".to_string());
    let rendered = error.to_string();
    assert!(rendered.contains("US/Foo"));
    assert!(rendered.contains("register_timezone_alias"));
    assert!(rendered.contains("IBAPI_TIMEZONE_ALIASES"));
}

#[test]
fn unexpected_response_display_includes_message_debug() {
    let msg = ResponseMessage::from("4\02\0-1\0200\0boom\0");
    let error = Error::UnexpectedResponse(msg);
    assert!(error.to_string().starts_with("UnexpectedResponse:"));
}

#[test]
fn test_error_is_error() {
    let error = Error::Simple("test error".to_string());
    assert!(error.source().is_none());
}

#[test]
fn test_from_io_error() {
    let io_error = io::Error::other("io error");
    let error: Error = io_error.into();
    assert!(matches!(error, Error::Io(_)));
}

#[test]
fn test_from_parse_int_error() {
    let parse_error = "abc".parse::<i32>().unwrap_err();
    let error: Error = parse_error.into();
    assert!(matches!(error, Error::ParseInt(_)));
}

#[test]
fn test_from_utf8_error() {
    let utf8_error = String::from_utf8(vec![0, 159, 146, 150]).unwrap_err();
    let error: Error = utf8_error.into();
    assert!(matches!(error, Error::FromUtf8(_)));
}

#[test]
fn test_from_parse_time_error() {
    let time_error = Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err();
    let error: Error = time_error.into();
    assert!(matches!(error, Error::ParseTime(_)));
}

#[test]
fn test_from_poison_error() {
    let mutex = Mutex::new(());
    let poison_error = PoisonError::new(mutex);
    let error: Error = poison_error.into();
    assert!(matches!(error, Error::Poison(_)));
}

#[test]
fn test_from_protobuf_decode_error() {
    let bad_bytes: &[u8] = &[0xff, 0xff];
    let decode_err = prost::Message::decode(bad_bytes).map(|_: crate::proto::TickPrice| ()).unwrap_err();
    let error: Error = decode_err.into();
    assert!(matches!(error, Error::ProtobufDecode(_)));
    assert!(error.to_string().contains("protobuf decode error"));
}

#[test]
fn from_text_response_message_extracts_code_and_message() {
    // Old-format text error: msg_type, version, request_id, error_code, error_message
    let msg = ResponseMessage::from("4\02\0-1\0200\0No security found\0");
    let error: Error = msg.into();
    assert!(matches!(error, Error::Message(200, ref m) if m == "No security found"));
}

#[test]
fn from_protobuf_response_message_decodes_envelope() {
    let envelope = crate::proto::ErrorMessage {
        id: Some(7),
        error_time: Some(0),
        error_code: Some(2104),
        error_msg: Some("Market data farm OK".to_string()),
        advanced_order_reject_json: None,
    };
    let raw = prost::Message::encode_to_vec(&envelope);
    // Message type 4 = Error.
    let msg = ResponseMessage::from_protobuf(4, raw, crate::server_versions::PROTOBUF);
    let error: Error = msg.into();
    assert!(matches!(error, Error::Message(2104, ref m) if m == "Market data farm OK"));
}

#[test]
fn from_protobuf_response_message_falls_back_when_decode_fails() {
    // raw_bytes won't parse as ErrorMessage — should fall back to the text accessors,
    // which return defaults for an empty proto-framed message.
    let msg = ResponseMessage::from_protobuf(4, vec![0xff, 0xff, 0xff, 0xff], crate::server_versions::PROTOBUF);
    let error: Error = msg.into();
    // Falls back to error_code() / error_message() text path: both default.
    assert!(matches!(error, Error::Message(0, _)));
}

#[test]
fn from_decoded_error_moves_into_message_variant() {
    let decoded = DecodedError {
        request_id: 42,
        error_code: 321,
        error_message: "rejected".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let error: Error = decoded.into();
    assert!(matches!(error, Error::Message(321, ref m) if m == "rejected"));
}

#[test]
fn from_validation_error_covers_every_variant() {
    let cases: Vec<(ValidationError, &str)> = vec![
        (ValidationError::InvalidQuantity(-1.0), "Invalid quantity: -1"),
        (ValidationError::InvalidPrice(f64::NAN), "Invalid price: NaN"),
        (ValidationError::MissingRequiredField("contract"), "Missing required field: contract"),
        (
            ValidationError::InvalidCombination("opposing legs".to_string()),
            "Invalid combination: opposing legs",
        ),
        (
            ValidationError::InvalidStopPrice { stop: 99.0, current: 100.0 },
            "Invalid stop price 99 for current price 100",
        ),
        (
            ValidationError::InvalidLimitPrice {
                limit: 101.0,
                current: 100.0,
            },
            "Invalid limit price 101 for current price 100",
        ),
        (
            ValidationError::InvalidBracketOrder("missing parent".to_string()),
            "Invalid bracket order: missing parent",
        ),
        (
            ValidationError::InvalidPercentage {
                field: "max_pct_vol",
                value: 0.05,
                min: 0.1,
                max: 0.5,
            },
            "Invalid max_pct_vol: 0.05 (must be between 0.1 and 0.5)",
        ),
    ];

    for (validation, expected_suffix) in cases {
        let error: Error = validation.into();
        match error {
            Error::InvalidArgument(msg) => assert_eq!(msg, expected_suffix),
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }
}

#[test]
fn clone_preserves_unit_variants() {
    for variant in [
        Error::NotImplemented,
        Error::ConnectionFailed,
        Error::ConnectionReset,
        Error::Cancelled,
        Error::Shutdown,
        Error::EndOfStream,
        Error::UnexpectedEndOfStream,
        Error::AlreadySubscribed,
    ] {
        let cloned = variant.clone();
        assert_eq!(variant.to_string(), cloned.to_string());
    }
}

#[test]
fn clone_preserves_payloaded_variants() {
    let bad_bytes: &[u8] = &[0xff, 0xff];
    let proto_err = prost::Message::decode(bad_bytes).map(|_: crate::proto::TickPrice| ()).unwrap_err();
    let response = ResponseMessage::from("4\02\0-1\0200\0boom\0");
    let originals = vec![
        Error::Io(io::Error::other("io")),
        Error::ParseInt("x".parse::<i32>().unwrap_err()),
        Error::FromUtf8(String::from_utf8(vec![0xff]).unwrap_err()),
        Error::Poison("p".into()),
        Error::Parse(3, "v".into(), "m".into()),
        Error::ServerVersion(10, 5, "feat".into()),
        Error::Simple("s".into()),
        Error::InvalidArgument("a".into()),
        Error::UnsupportedTimeZone("US/Foo".into()),
        Error::UnexpectedResponse(response),
        Error::Message(404, "nope".into()),
        Error::HistoricalParseError(HistoricalParseError::WhatToShow("Z".into())),
        Error::ProtobufDecode(proto_err),
    ];

    for original in originals {
        let cloned = original.clone();
        assert_eq!(original.to_string(), cloned.to_string());
    }
}

#[test]
fn clone_collapses_parse_time_to_simple() {
    let parse_time = Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err();
    let original = Error::ParseTime(parse_time);
    let display = original.to_string();
    let cloned = original.clone();
    assert!(matches!(cloned, Error::Simple(ref s) if *s == display));
}

#[test]
fn test_non_exhaustive() {
    fn assert_non_exhaustive<T: StdError>() {}
    assert_non_exhaustive::<Error>();
}
