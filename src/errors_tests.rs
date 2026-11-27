use super::*;
use crate::builder::ValidationError;
use crate::common::test_utils::helpers::tws_error_notice;
use crate::messages::ResponseMessage;
use crate::proto::decoders::DecodeProto;
use crate::transport::routing::DecodedError;
use std::error::Error as StdError;
use std::io;
use time::macros::format_description;
use time::Time;

fn parse_time_error() -> time::error::Parse {
    Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err()
}

fn protobuf_decode_error() -> Error {
    crate::proto::TickPrice::decode_proto(&[0xff, 0xff]).unwrap_err()
}

#[test]
fn error_debug() {
    let error = Error::InvalidArgument("test error".to_string());
    assert_eq!(format!("{error:?}"), "InvalidArgument(\"test error\")");
}

#[test]
fn error_display() {
    let cases = vec![
        (Error::Io(io::Error::new(io::ErrorKind::NotFound, "file not found")), "file not found"),
        (Error::ParseTime(parse_time_error()), "the 'month' component could not be parsed"),
        (
            Error::ServerVersion {
                required: 2,
                actual: 1,
                feature: "old version".to_string(),
            },
            "server version 2 required, got 1: old version",
        ),
        (Error::InvalidArgument("bad arg".to_string()), "InvalidArgument: bad arg"),
        (
            Error::Validation(ValidationError::InvalidQuantity(-1.0)),
            "validation error: Invalid quantity: -1",
        ),
        (Error::ConnectionFailed, "ConnectionFailed"),
        (Error::ConnectionReset, "ConnectionReset"),
        (Error::Cancelled, "Cancelled"),
        (Error::Shutdown, "Shutdown"),
        (Error::UnexpectedEndOfStream, "UnexpectedEndOfStream"),
        (Error::Timeout, "timed out before the request ended"),
        (
            Error::BufferLimitExceeded { limit: 5 },
            "subscription buffer limit exceeded (5 unread items)",
        ),
        (tws_error_notice(200, "No security found"), "[200] No security found"),
        (Error::AlreadySubscribed, "AlreadySubscribed"),
        (
            Error::OrderIdInRequestRange { order_id: 7 },
            "order id 7 is at or above 1500000000, which is reserved for request ids",
        ),
        (
            Error::AccountUpdatesInUse {
                active: AccountId("DU1".into()),
                requested: AccountId("DU2".into()),
            },
            "account updates already streaming DU1; cancel it before requesting DU2",
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
    assert!(rendered.contains("troubleshooting.md#gateway-time-zone"));
}

#[test]
fn unexpected_response_display_includes_message_debug() {
    let msg = ResponseMessage::from("4\02\0-1\0200\0boom\0");
    let error = Error::unexpected_response(&msg);
    assert!(error.to_string().starts_with("UnexpectedResponse:"));
}

#[test]
fn unexpected_wire_format_display_includes_message_debug() {
    let msg = ResponseMessage::from("50\0\09000\0");
    let error = Error::unexpected_wire_format(&msg);
    assert!(error.to_string().starts_with("UnexpectedWireFormat:"), "got {error}");
    assert!(error.to_string().contains("raw_bytes: None"), "got {error}");
}

#[test]
fn unexpected_wire_format_survives_clone() {
    // `clone_preserves_payloaded_variants` only compares Display, which a
    // collapse to another `String` variant would survive. This pins the discriminant,
    // which is what the dispatcher matches on.
    let msg = ResponseMessage::from("50\0\09000\0");
    assert!(matches!(Error::unexpected_wire_format(&msg).clone(), Error::UnexpectedWireFormat(_)));
}

#[test]
fn error_source_returns_none_for_invalid_argument() {
    let error = Error::InvalidArgument("test error".to_string());
    assert!(error.source().is_none());
}

#[test]
fn from_io_error() {
    let error: Error = io::Error::other("io error").into();
    assert!(matches!(error, Error::Io(_)));
}

#[test]
fn from_parse_time_error() {
    let error: Error = parse_time_error().into();
    assert!(matches!(error, Error::ParseTime(_)));
}

#[test]
fn parse_display_shows_field_and_value_when_present() {
    let cases = [
        (Error::invalid_value("XYZ", "unknown Action"), r#"parse error: "XYZ": unknown Action"#),
        (Error::invalid_field("action", "missing"), "parse error: action: missing"),
        (
            Error::Parse {
                field: Some("action"),
                value: "XYZ".into(),
                reason: "unknown Action".into(),
            },
            r#"parse error: action "XYZ": unknown Action"#,
        ),
        (Error::invalid_value("", "empty Action"), "parse error: empty Action"),
        (Error::invalid_text_field(3, "x", "bad int"), r#"parse error: "x": bad int (field 3)"#),
        (Error::eof_at(2, "int"), "parse error: expected int and found end of message (field 2)"),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn parse_constructors_fill_the_named_fields() {
    match Error::invalid_value("bogus", "unknown BarSize") {
        Error::Parse { field, value, reason } => {
            assert_eq!(field, None);
            assert_eq!(value, "bogus");
            assert_eq!(reason, "unknown BarSize");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
    match Error::invalid_field("fa_data_type", "missing in ReceiveFa") {
        Error::Parse { field, value, reason } => {
            assert_eq!(field, Some("fa_data_type"));
            assert!(value.is_empty());
            assert_eq!(reason, "missing in ReceiveFa");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn decode_proto_failure_builds_protobuf_decode() {
    let error = protobuf_decode_error();
    assert!(matches!(error, Error::ProtobufDecode(_)));
    assert!(error.to_string().contains("protobuf decode error"));
}

#[test]
fn protobuf_decode_has_no_source() {
    // The decode detail is already in Display; returning it from source() too
    // would print it twice in error-chain reports.
    let error = protobuf_decode_error();
    let Error::ProtobufDecode(ref inner) = error else {
        panic!("expected ProtobufDecode, got {error:?}");
    };
    assert!(error.source().is_none());
    assert!(inner.source().is_none());
    assert!(error.to_string().ends_with(&inner.to_string()));
}

#[test]
fn error_types_are_send_sync_static() {
    crate::tests::assert_send_and_sync::<Error>();
    crate::tests::assert_send_and_sync::<ProtobufDecodeError>();
    let _: Box<dyn StdError + Send + Sync + 'static> = Box::new(protobuf_decode_error());
}

#[test]
fn from_decoded_error_moves_into_notice_variant() {
    let decoded = DecodedError {
        request_id: 42,
        error_code: 321,
        error_message: "rejected".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let error: Error = decoded.into();
    assert!(matches!(error, Error::Notice(ref n) if n.code == 321 && n.message == "rejected"));
}

#[test]
fn from_validation_error_keeps_the_variant() {
    let cases = [
        ValidationError::InvalidQuantity(-1.0),
        ValidationError::InvalidPrice(f64::INFINITY),
        ValidationError::InvalidCombination("opposing legs".to_string()),
        ValidationError::InvalidStopPrice { stop: 99.0, current: 100.0 },
        ValidationError::InvalidLimitPrice {
            limit: 101.0,
            current: 100.0,
        },
        ValidationError::InvalidBracketOrder("missing parent".to_string()),
        ValidationError::InvalidPercentage {
            field: "max_pct_vol",
            value: 0.05,
            min: 0.1,
            max: 0.5,
        },
    ];

    for validation in cases {
        match Error::from(validation.clone()) {
            Error::Validation(inner) => assert_eq!(inner, validation),
            other => panic!("expected Validation({validation:?}), got {other:?}"),
        }
    }
}

#[test]
fn clone_preserves_unit_variants() {
    for variant in [
        Error::ConnectionFailed,
        Error::ConnectionReset,
        Error::Cancelled,
        Error::Shutdown,
        Error::UnexpectedEndOfStream,
        Error::Timeout,
        Error::AlreadySubscribed,
    ] {
        let cloned = variant.clone();
        assert_eq!(variant.to_string(), cloned.to_string());
    }
}

#[test]
fn clone_preserves_payloaded_variants() {
    let response = ResponseMessage::from("4\02\0-1\0200\0boom\0");
    let originals = vec![
        Error::Io(io::Error::other("io")),
        Error::Parse {
            field: Some("f"),
            value: "v".into(),
            reason: "m".into(),
        },
        Error::ServerVersion {
            required: 10,
            actual: 5,
            feature: "feat".into(),
        },
        Error::InvalidArgument("a".into()),
        Error::Validation(ValidationError::InvalidQuantity(0.0)),
        Error::UnsupportedTimeZone("US/Foo".into()),
        Error::unexpected_response(&response),
        Error::unexpected_wire_format(&response),
        Error::InvalidFrame("frame length 0 is shorter than the 4-byte message id".into()),
        tws_error_notice(404, "nope"),
        protobuf_decode_error(),
        Error::ParseTime(parse_time_error()),
        Error::AccountUpdatesInUse {
            active: AccountId("DU1".into()),
            requested: AccountId("DU2".into()),
        },
        Error::BufferLimitExceeded { limit: 5 },
        Error::OrderIdInRequestRange { order_id: 7 },
        partially_sent(Error::ConnectionReset),
    ];

    for original in originals {
        let cloned = original.clone();
        assert_eq!(original.to_string(), cloned.to_string());
    }
}

#[test]
fn clone_preserves_parse_time() {
    let original = Error::ParseTime(parse_time_error());
    assert!(matches!(original.clone(), Error::ParseTime(e) if e == parse_time_error()));
}

fn partially_sent(source: Error) -> Error {
    Error::orders_partially_sent([7, 8, 9], 1, source)
}

#[test]
fn orders_partially_sent_splits_after_the_failed_order() {
    let cases = [
        ("first send fails", 0, vec![7], vec![8, 9]),
        ("middle send fails", 1, vec![7, 8], vec![9]),
        ("last send fails", 2, vec![7, 8, 9], vec![]),
    ];
    for (label, failed, expected_sent, expected_not_sent) in cases {
        match Error::orders_partially_sent([7, 8, 9], failed, Error::ConnectionReset) {
            Error::OrdersPartiallySent { sent, not_sent, .. } => {
                assert_eq!(sent, expected_sent, "{label}");
                assert_eq!(not_sent, expected_not_sent, "{label}");
            }
            other => panic!("{label}: {other:?}"),
        }
    }
}

#[test]
fn orders_partially_sent_counts_the_orders_and_chains_the_send_failure() {
    let err = partially_sent(Error::ConnectionReset);

    assert_eq!(
        err.to_string(),
        format!("2 of 3 orders may have been sent before the submit stopped: {}", Error::ConnectionReset)
    );
    let source = StdError::source(&err).expect("the send failure is the source");
    assert_eq!(source.to_string(), Error::ConnectionReset.to_string());
}

#[test]
fn orders_partially_sent_is_connection_lost_when_its_send_failure_is() {
    assert!(partially_sent(Error::ConnectionReset).is_connection_lost());
    assert!(!partially_sent(Error::Shutdown).is_connection_lost());
}

#[test]
fn error_is_non_exhaustive() {
    fn assert_non_exhaustive<T: StdError>() {}
    assert_non_exhaustive::<Error>();
}

#[test]
fn is_connection_lost_true_for_connection_variants() {
    assert!(Error::ConnectionReset.is_connection_lost());

    // The socket is still open, but the framing has desynchronized and there is
    // no delimiter to re-anchor on — reconnecting is the only recovery, so the
    // dispatchers must take their reconnect branch rather than shutting down.
    assert!(Error::InvalidFrame("bad length".to_string()).is_connection_lost());

    for kind in [
        io::ErrorKind::ConnectionReset,
        io::ErrorKind::ConnectionAborted,
        io::ErrorKind::UnexpectedEof,
        io::ErrorKind::BrokenPipe,
    ] {
        let error = Error::Io(io::Error::new(kind, "disconnected"));
        assert!(error.is_connection_lost(), "{kind:?} should count as connection-lost");
    }
}

#[test]
fn is_connection_lost_false_for_non_connection() {
    let cases = [
        Error::Shutdown,
        // Reconnection exhausted — terminal, not a recoverable mid-stream loss.
        Error::ConnectionFailed,
        Error::ConnectionRejected("allow-list mismatch".to_string()),
        Error::Cancelled,
        tws_error_notice(200, "No security found"),
        Error::Io(io::Error::new(io::ErrorKind::NotFound, "missing")),
        Error::InvalidArgument("boom".to_string()),
        Error::invalid_value("v", "m"),
    ];

    for error in cases {
        assert!(!error.is_connection_lost(), "{error:?} should not count as connection-lost");
    }
}

#[test]
fn is_unsent_only_for_errors_raised_before_a_write() {
    let response = ResponseMessage::from("4\02\0-1\0200\0boom\0");
    let cases = vec![
        (Error::Validation(ValidationError::InvalidQuantity(-1.0)), true),
        (Error::InvalidArgument("buffer_limit must be above zero".into()), true),
        (Error::OrderIdInRequestRange { order_id: 1_500_000_000 }, true),
        (
            Error::ServerVersion {
                required: 223,
                actual: 213,
                feature: "hedge max size".into(),
            },
            true,
        ),
        (Error::AlreadySubscribed, true),
        (
            Error::AccountUpdatesInUse {
                active: AccountId("DU1".into()),
                requested: AccountId("DU2".into()),
            },
            true,
        ),
        (Error::Io(io::Error::new(io::ErrorKind::BrokenPipe, "write failed")), false),
        (Error::ParseTime(parse_time_error()), false),
        (
            Error::Parse {
                field: Some("action"),
                value: "v".into(),
                reason: "m".into(),
            },
            false,
        ),
        (Error::ConnectionFailed, false),
        (Error::ConnectionRejected("allow-list".into()), false),
        (Error::ClientIdInUse { client_id: 7 }, false),
        (Error::UnsupportedTimeZone("US/Foo".into()), false),
        (Error::ConnectionReset, false),
        (Error::Cancelled, false),
        (Error::Shutdown, false),
        (Error::unexpected_response(&response), false),
        (Error::unexpected_wire_format(&response), false),
        (Error::UnexpectedEndOfStream, false),
        (Error::Timeout, false),
        (Error::BufferLimitExceeded { limit: 5 }, false),
        (Error::InvalidFrame("short".into()), false),
        (tws_error_notice(201, "Order rejected"), false),
        (protobuf_decode_error(), false),
        (partially_sent(Error::ConnectionReset), false),
    ];

    for (error, unsent) in cases {
        assert_eq!(error.is_unsent(), unsent, "{error:?}");
    }
}

#[test]
fn is_read_timeout_only_for_no_data_yet() {
    // Both dispatchers poll on this: a timed-out read means "nothing arrived",
    // and must not be confused with the socket being gone.
    for kind in [io::ErrorKind::WouldBlock, io::ErrorKind::TimedOut] {
        let error = Error::Io(io::Error::new(kind, "no data"));
        assert!(error.is_read_timeout(), "{kind:?} should count as a read timeout");
        assert!(!error.is_connection_lost(), "{kind:?} must not also read as connection-lost");
    }

    for error in [
        Error::Io(io::Error::new(io::ErrorKind::BrokenPipe, "gone")),
        Error::Io(io::Error::other("other")),
        Error::ConnectionReset,
        Error::Cancelled,
    ] {
        assert!(!error.is_read_timeout(), "{error:?} should not count as a read timeout");
    }
}

#[test]
fn in_field_names_only_an_unnamed_parse_error() {
    // A `FromStr` error gains the decoder's field name.
    let named = Error::invalid_value("XYZ", "unknown Action").in_field("action");
    assert!(
        matches!(&named, Error::Parse { field: Some("action"), value, .. } if value == "XYZ"),
        "{named:?}"
    );
    // A field already named is kept.
    let kept = Error::invalid_field("contract", "missing in OpenOrder").in_field("action");
    assert!(matches!(kept, Error::Parse { field: Some("contract"), .. }), "{kept:?}");
    // Any other error passes through.
    assert!(matches!(Error::Timeout.in_field("action"), Error::Timeout));
}
