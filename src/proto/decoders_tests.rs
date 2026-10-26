use super::*;
use crate::common::test_utils::helpers::assert_decimal_parse_error;
use crate::orders::{ExecutionSide, OrderStatusKind};

// === parse_required ===

#[test]
fn parse_required_none_errors_with_label() {
    let err = parse_required::<OrderStatusKind>(None, "OrderStatus").unwrap_err();
    match err {
        Error::Parse(_, _, msg) => assert!(msg.contains("OrderStatus"), "expected label in message, got: {msg}"),
        other => panic!("expected Error::Parse, got {other:?}"),
    }
}

#[test]
fn parse_required_empty_errors_with_label() {
    let err = parse_required::<OrderStatusKind>(Some(""), "OrderStatus").unwrap_err();
    match err {
        Error::Parse(_, _, msg) => assert!(msg.contains("OrderStatus"), "expected label in message, got: {msg}"),
        other => panic!("expected Error::Parse, got {other:?}"),
    }
}

#[test]
fn parse_required_valid_round_trips() {
    let v: OrderStatusKind = parse_required(Some("Submitted"), "OrderStatus").unwrap();
    assert_eq!(v, OrderStatusKind::Submitted);
}

#[test]
fn parse_required_defers_unknown_to_fromstr() {
    // The helper adds no vocabulary policy of its own: a closed enum's
    // unknown value propagates as the FromStr error. (Open-enum fallback
    // behavior is owned by the enum and tested beside it — see
    // order_status_kind_preserves_unknown_wire_status.)
    assert!(matches!(
        parse_required::<ExecutionSide>(Some("Garbage"), "side"),
        Err(Error::Parse(_, _, _))
    ));
}

// === parse_optional ===

#[test]
fn parse_optional_none_is_ok_none() {
    let v: Option<OrderStatusKind> = parse_optional(None).unwrap();
    assert_eq!(v, None);
}

#[test]
fn parse_optional_empty_is_ok_none() {
    let v: Option<OrderStatusKind> = parse_optional(Some("")).unwrap();
    assert_eq!(v, None);
}

#[test]
fn parse_optional_valid_round_trips() {
    let v: Option<OrderStatusKind> = parse_optional(Some("Filled")).unwrap();
    assert_eq!(v, Some(OrderStatusKind::Filled));
}

#[test]
fn parse_optional_defers_unknown_to_fromstr() {
    // Mirrors parse_required_defers_unknown_to_fromstr.
    assert!(matches!(parse_optional::<ExecutionSide>(Some("Garbage")), Err(Error::Parse(_, _, _))));
}

// === parse_optional_decimal ===
//
// Tier 1: the helper's semantics, exhaustively, in one place. Decoder tests
// elsewhere assert only that each decoder is *wired* to this helper.

/// Every input class that must decode to "no value". The literal-sentinel rows are
/// derived from the constant under test rather than re-typed (docs/rules/testing/derive-from-constants.md).
fn unset_inputs() -> impl Iterator<Item = Option<&'static str>> {
    [
        None,     // field absent
        Some(""), // present but empty
        Some("inf"),
        Some("-inf"),
        Some("NaN"),
        Some("1e309"),                   // overflows to inf
        Some("1.7976931348623157E308"),  // f64::MAX, C# "R" spelling
        Some("1.7976931348623157e308"),  // ...lowercase
        Some("1.7976931348623157E+308"), // ...explicit sign
    ]
    .into_iter()
    .chain(UNSET_DECIMAL_WIRE.map(Some))
}

#[test]
fn parse_optional_decimal_unset_inputs_are_none() {
    for input in unset_inputs() {
        assert_eq!(parse_optional_decimal(input).unwrap(), None, "expected None for {input:?}");
    }
}

#[test]
fn parse_optional_decimal_preserves_fractional_values() {
    // The regression this whole change exists for: these all decoded to 0
    // through the old integer parse (issue #716).
    for (input, expected) in [("0.5", 0.5), ("0.001", 0.001), ("-0.25", -0.25), ("1.5", 1.5)] {
        assert_eq!(parse_optional_decimal(Some(input)).unwrap(), Some(expected), "input {input}");
    }
}

#[test]
fn parse_optional_decimal_zero_is_a_value_not_unset() {
    assert_eq!(parse_optional_decimal(Some("0")).unwrap(), Some(0.0));
    assert_eq!(parse_optional_decimal(Some("100")).unwrap(), Some(100.0));
}

#[test]
fn parse_optional_decimal_sentinel_near_misses_survive() {
    // Proves the literal/numeric split: sentinels are matched as strings, so a
    // real size that merely resembles one must not be swallowed.
    for (input, expected) in [
        ("2147483647.0", 2147483647.0),
        ("9223372036854775806", 9223372036854775806.0),
        ("21474836470", 21474836470.0),
        ("214748364", 214748364.0),
    ] {
        assert_eq!(parse_optional_decimal(Some(input)).unwrap(), Some(expected), "input {input}");
    }
}

#[test]
fn parse_optional_decimal_accepts_more_digits_than_f64_round_trips() {
    // f64 cannot hold these exactly. Accepted for now — this precision loss is
    // the motivation for the follow-up decimal quantity type; the contract here
    // is only that the value is not rejected and lands on the nearest f64.
    assert_eq!(parse_optional_decimal(Some("0.1234567890123456789")).unwrap(), Some(0.12345678901234568));
    assert_eq!(
        parse_optional_decimal(Some("123456789012345678901234567890")).unwrap(),
        Some(1.2345678901234568e29)
    );
}

#[test]
fn parse_optional_decimal_malformed_errors_with_offending_value() {
    for input in ["abc", " ", "1,000", "1.2.3", "--1", "0x10"] {
        assert_decimal_parse_error(parse_optional_decimal(Some(input)), input);
    }
}

// === parse_decimal_or_zero ===
//
// A composition of `parse_optional_decimal`, so it only needs to prove the
// delegation — re-running the full class table here would just test `unwrap_or`.

#[test]
fn parse_decimal_or_zero_collapses_unset_to_zero() {
    for input in unset_inputs() {
        assert_eq!(parse_decimal_or_zero(input).unwrap(), 0.0, "expected 0.0 for {input:?}");
    }
}

#[test]
fn parse_decimal_or_zero_passes_values_through() {
    assert_eq!(parse_decimal_or_zero(Some("0.5")).unwrap(), 0.5);
}

#[test]
fn parse_decimal_or_zero_propagates_parse_error() {
    assert!(matches!(parse_decimal_or_zero(Some("abc")), Err(Error::Parse(_, _, _))));
}

// === decode_combo_leg end-to-end (docs/rules/testing/exercise-production-code.md) ===

fn proto_leg(action: Option<&str>) -> proto::ComboLeg {
    proto::ComboLeg {
        con_id: Some(1),
        ratio: Some(1),
        action: action.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn decode_combo_leg_rejects_missing_action() {
    assert!(matches!(decode_combo_leg(&proto_leg(None)), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_rejects_empty_action() {
    assert!(matches!(decode_combo_leg(&proto_leg(Some(""))), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_rejects_unknown_action() {
    // SLONG is the variant LegAction deliberately excludes — guards against
    // a future "let's just reuse Action after all" regression.
    assert!(matches!(decode_combo_leg(&proto_leg(Some("SLONG"))), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_accepts_buy() {
    let leg = decode_combo_leg(&proto_leg(Some("BUY"))).unwrap();
    assert_eq!(leg.action, LegAction::Buy);
}

#[test]
fn decode_combo_leg_accepts_sell() {
    let leg = decode_combo_leg(&proto_leg(Some("SELL"))).unwrap();
    assert_eq!(leg.action, LegAction::Sell);
}

#[test]
fn decode_combo_leg_accepts_sshort() {
    let leg = decode_combo_leg(&proto_leg(Some("SSHORT"))).unwrap();
    assert_eq!(leg.action, LegAction::SellShort);
}

// === decode_contract surfaces combo-leg errors ===

#[test]
fn decode_contract_propagates_bad_combo_leg() {
    let proto_contract = proto::Contract {
        combo_legs: vec![proto_leg(Some("NOTAVARIANT"))],
        ..stk_contract()
    };
    assert!(matches!(decode_contract(&proto_contract), Err(Error::Parse(_, _, _))));
}

// === decode_contract sec_type ===

#[test]
fn decode_contract_reads_missing_or_empty_sec_type_as_other_empty() {
    // Upstream sets SecType only `if (HasSecType)` and still delivers the
    // frame, so absence is an unset field, not a malformed frame. Requiring it
    // would end every stream carrying a contract (see #840).
    for sec_type in [None, Some(String::new())] {
        let proto_contract = proto::Contract {
            sec_type,
            ..Default::default()
        };
        let contract = decode_contract(&proto_contract).unwrap();
        assert_eq!(contract.security_type, crate::contracts::SecurityType::Other(String::new()));
    }
}

#[test]
fn decode_contract_preserves_unknown_sec_type() {
    let proto_contract = proto::Contract {
        sec_type: Some("NOTASECTYPE".into()),
        ..Default::default()
    };
    let contract = decode_contract(&proto_contract).unwrap();
    assert_eq!(contract.security_type, crate::contracts::SecurityType::Other("NOTASECTYPE".into()));
}

/// A `proto::Contract` carrying a `sec_type`, for tests that are not about the
/// `Other("")` fallback above.
fn stk_contract() -> proto::Contract {
    proto::Contract {
        sec_type: Some("STK".into()),
        ..Default::default()
    }
}

// === decode_contract last_trade_date (#1087) ===

#[test]
fn decode_contract_parses_last_trade_date() {
    use time::macros::date;
    // Live: OPT/FUT/FOP carry plain YYYYMMDD; STK/BOND omit the tag.
    let cases = [
        (None, None),
        (Some(""), None),
        (Some("20261218"), Some(date!(2026 - 12 - 18))),
        // Strict: time/zone belong to tag 4, never seen on tag 21.
        (Some("20261218 08:30:00 US/Central"), None),
        (Some("202612"), None),
        (Some("garbage!"), None),
    ];
    for (raw, expected) in cases {
        let proto_contract = proto::Contract {
            sec_type: Some("FUT".into()),
            last_trade_date: raw.map(String::from),
            ..Default::default()
        };
        let contract = decode_contract(&proto_contract).unwrap();
        assert_eq!(contract.last_trade_date, expected, "last_trade_date for {raw:?}");
    }
}
