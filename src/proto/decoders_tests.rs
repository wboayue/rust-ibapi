use super::*;
use crate::orders::OrderStatusKind;

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
fn parse_required_unknown_propagates_fromstr_err() {
    assert!(matches!(
        parse_required::<OrderStatusKind>(Some("Garbage"), "OrderStatus"),
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
fn parse_optional_unknown_propagates_fromstr_err() {
    assert!(matches!(parse_optional::<OrderStatusKind>(Some("Garbage")), Err(Error::Parse(_, _, _))));
}

// === decode_combo_leg end-to-end (CLAUDE.md rule 10) ===

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
