use super::*;
use crate::common::test_utils::helpers::assert_decimal_parse_error;
use crate::orders::{ExecutionSide, OrderStatusKind, TimeInForce};

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
        ..Default::default()
    };
    assert!(matches!(decode_contract(&proto_contract), Err(Error::Parse(_, _, _))));
}

// === decode_order ===

/// The smallest `proto::Order` `decode_order` accepts: `action` is the one
/// required wire field.
fn proto_order() -> proto::Order {
    proto::Order {
        action: Some("BUY".into()),
        ..Default::default()
    }
}

#[test]
fn decode_order_maps_hedge_max_size() {
    let proto_order = proto::Order {
        hedge_max_size: Some(500),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.hedge_max_size, Some(500));
}

#[test]
fn decode_order_hedge_max_size_absent_is_none() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(order.hedge_max_size.is_none());
}

// === decode_order deactivate ===

#[test]
fn decode_order_maps_deactivate() {
    let proto_order = proto::Order {
        deactivate: Some(true),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert!(order.deactivate);
}

#[test]
fn decode_order_deactivate_absent_is_false() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(!order.deactivate);
}

// === decode_order tif ===

#[test]
fn decode_order_preserves_unknown_tif() {
    // An unmodeled TIF keeps its raw value instead of decoding as DAY — the
    // bug GoodTillCrossing hit before it was a variant (#822).
    let proto_order = proto::Order {
        tif: Some("GTZ".to_string()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().tif, TimeInForce::Unknown("GTZ".to_string()));
}

#[test]
fn decode_order_maps_known_tif() {
    let proto_order = proto::Order {
        tif: Some("GTX".to_string()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().tif, TimeInForce::GoodTillCrossing);
}

#[test]
fn decode_order_tif_absent_or_empty_is_day() {
    // Upstream omits an empty Tif rather than sending one, so neither form is
    // an unknown value to preserve (unlike action, which is required).
    assert_eq!(decode_order(&proto_order()).unwrap().tif, TimeInForce::Day);

    let empty_tif = proto::Order {
        tif: Some(String::new()),
        ..proto_order()
    };
    assert_eq!(decode_order(&empty_tif).unwrap().tif, TimeInForce::Day);
}

// === decimal wire fields are routed through parse_optional_decimal (issue #716) ===

#[test]
fn decode_order_rejects_malformed_total_quantity() {
    let proto_order = proto::Order {
        total_quantity: Some("abc".into()),
        ..proto_order()
    };
    assert_decimal_parse_error(decode_order(&proto_order), "abc");
}

#[test]
fn decode_order_preserves_fractional_total_quantity() {
    let proto_order = proto::Order {
        total_quantity: Some("0.5".into()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().total_quantity, 0.5);
}

// === decode_order enum fields go through parse_required / parse_optional ===

#[test]
fn decode_order_rejects_missing_or_empty_action() {
    // Unlike tif, action has no unset state upstream: an order without a
    // side is a malformed frame, not a default.
    for proto_order in [
        proto::Order {
            action: None,
            ..proto_order()
        },
        proto::Order {
            action: Some(String::new()),
            ..proto_order()
        },
    ] {
        assert!(
            matches!(decode_order(&proto_order), Err(Error::Parse(_, _, _))),
            "expected Error::Parse for {proto_order:?}"
        );
    }
}

#[test]
fn decode_order_rejects_unknown_action() {
    // Action is closed: an unrecognized side fails the decode rather than
    // reading as Buy.
    let proto_order = proto::Order {
        action: Some("NOTASIDE".into()),
        ..proto_order()
    };
    assert!(matches!(decode_order(&proto_order), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_order_preserves_unknown_string_codes() {
    // rule80_a and open_close are open: an unrecognized value survives the
    // decode as Unknown(raw) instead of failing the order stream (tif is
    // covered by decode_order_preserves_unknown_tif above).
    let proto_order = proto::Order {
        rule80_a: Some("Z".into()),
        open_close: Some("X".into()),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.rule_80_a, Some(Rule80A::Unknown("Z".into())));
    assert_eq!(order.open_close, Some(OrderOpenClose::Unknown("X".into())));
}

#[test]
fn decode_order_empty_optional_strings_are_none() {
    let proto_order = proto::Order {
        rule80_a: Some(String::new()),
        open_close: Some(String::new()),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.rule_80_a, None);
    assert_eq!(order.open_close, None);
}

#[test]
fn decode_order_preserves_unknown_integer_codes() {
    let proto_order = proto::Order {
        oca_type: Some(9),
        trigger_method: Some(99),
        volatility_type: Some(9),
        reference_price_type: Some(9),
        origin: Some(9),
        short_sale_slot: Some(9),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.oca_type, OcaType::Unknown(9));
    assert_eq!(order.trigger_method, TriggerMethod::Unknown(99));
    assert_eq!(order.volatility_type, Some(VolatilityType::Unknown(9)));
    assert_eq!(order.reference_price_type, Some(ReferencePriceType::Unknown(9)));
    assert_eq!(order.origin, OrderOrigin::Unknown(9));
    assert_eq!(order.short_sale_slot, ShortSaleSlot::Unknown(9));
}

#[test]
fn decode_order_condition_preserves_unknown_trigger_method() {
    let proto_order = proto::Order {
        conditions: vec![proto::OrderCondition {
            r#type: Some(1),
            trigger_method: Some(99),
            ..Default::default()
        }],
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    match &order.conditions[..] {
        [OrderCondition::Price(price)] => assert_eq!(price.trigger_method, TriggerMethod::Unknown(99)),
        other => panic!("expected one price condition, got {other:?}"),
    }
}

#[test]
fn decode_execution_rejects_malformed_shares() {
    let proto_exec = proto::Execution {
        side: Some("BOT".into()),
        shares: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_execution(&proto_exec), "abc");
}

#[test]
fn decode_execution_rejects_malformed_cumulative_quantity() {
    let proto_exec = proto::Execution {
        side: Some("BOT".into()),
        shares: Some("10".into()),
        cum_qty: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_execution(&proto_exec), "abc");
}

#[test]
fn decode_contract_details_rejects_malformed_min_size() {
    let details = proto::ContractDetails {
        min_size: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_contract_details(&proto::Contract::default(), &details), "abc");
}

#[test]
fn decode_contract_details_rejects_malformed_min_tick() {
    let details = proto::ContractDetails {
        min_tick: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_contract_details(&proto::Contract::default(), &details), "abc");
}

#[test]
fn decode_order_state_sentinel_suggested_size_is_none() {
    // Regression guard: `optional_string_f64`'s unset semantics had to survive
    // the swap to `parse_optional_decimal`.
    let state = proto::OrderState {
        status: Some("Submitted".into()),
        suggested_size: Some("2147483647".into()),
        ..Default::default()
    };
    assert_eq!(decode_order_state(&state).unwrap().suggested_size, None);
}

#[test]
fn decode_order_state_rejects_malformed_allocation_position() {
    let state = proto::OrderState {
        status: Some("Submitted".into()),
        order_allocations: vec![proto::OrderAllocation {
            account: Some("DU1234".into()),
            position: Some("abc".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert_decimal_parse_error(decode_order_state(&state), "abc");
}

// === ContractDetails size rules are optional (issue #716) ===

#[test]
fn decode_contract_details_absent_size_rules_are_none() {
    // Contracts without size rules omit these entirely; `0.0` would have been a
    // nonsense size increment.
    let details = decode_contract_details(&proto::Contract::default(), &proto::ContractDetails::default()).unwrap();

    assert_eq!(details.min_size, None);
    assert_eq!(details.size_increment, None);
    assert_eq!(details.suggested_size_increment, None);
}

#[test]
fn decode_contract_details_preserves_fractional_size_increment() {
    let details = proto::ContractDetails {
        size_increment: Some("0.0001".into()),
        ..Default::default()
    };
    let decoded = decode_contract_details(&proto::Contract::default(), &details).unwrap();

    assert_eq!(decoded.size_increment, Some(0.0001));
}
