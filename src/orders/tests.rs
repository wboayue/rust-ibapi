use super::*;
use crate::common::test_utils::wire_enum::{check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

const ALL_KINDS: &[(OrderStatusKind, &str)] = &[
    (OrderStatusKind::ApiPending, "ApiPending"),
    (OrderStatusKind::PendingSubmit, "PendingSubmit"),
    (OrderStatusKind::PendingCancel, "PendingCancel"),
    (OrderStatusKind::PreSubmitted, "PreSubmitted"),
    (OrderStatusKind::Submitted, "Submitted"),
    (OrderStatusKind::ApiCancelled, "ApiCancelled"),
    (OrderStatusKind::Cancelled, "Cancelled"),
    (OrderStatusKind::Filled, "Filled"),
    (OrderStatusKind::Inactive, "Inactive"),
];

#[test]
fn order_status_kind_round_trip() {
    check_wire_enum_round_trip(ALL_KINDS);
}

#[test]
fn order_status_kind_preserves_unknown_wire_status() {
    // OrderStatusKind is an open enum (#774): an unrecognized non-empty
    // status parses as Unknown(raw) instead of Error::Parse, so a status
    // string this crate does not model cannot terminate the order streams.
    // Matching stays exact and case-sensitive — case-variants land in
    // Unknown rather than being coerced to the nearest known variant.
    check_wire_enum_round_trip(&[
        (OrderStatusKind::Unknown("NotARealStatus".into()), "NotARealStatus"),
        (OrderStatusKind::Unknown("submitted".into()), "submitted"),
        (OrderStatusKind::Unknown("FILLED".into()), "FILLED"),
    ]);
    let unknown = OrderStatusKind::Unknown("NotARealStatus".into());
    assert!(!unknown.is_active(), "Unknown must not be active");
    assert!(!unknown.is_terminal(), "Unknown must not be terminal");

    // Absence of a value is still an error — only unrecognized values fall
    // back (docs/rules/wire/enum-typing.md).
    check_wire_enum_rejects_unknown::<OrderStatusKind>(&[""]);
}

#[test]
fn order_status_kind_serde_round_trips_as_plain_string() {
    // Manual serde keeps the JSON a plain string in both directions —
    // Unknown("X") serializes as "X", not {"Unknown":"X"}.
    let known = OrderStatusKind::Cancelled;
    assert_eq!(serde_json::to_string(&known).unwrap(), "\"Cancelled\"");
    assert_eq!(serde_json::from_str::<OrderStatusKind>("\"Cancelled\"").unwrap(), known);

    let unknown = OrderStatusKind::Unknown("PendingReplace".to_string());
    assert_eq!(serde_json::to_string(&unknown).unwrap(), "\"PendingReplace\"");
    assert_eq!(serde_json::from_str::<OrderStatusKind>("\"PendingReplace\"").unwrap(), unknown);

    assert!(
        serde_json::from_str::<OrderStatusKind>("\"\"").is_err(),
        "empty string must not deserialize"
    );
}

#[test]
fn execution_filter_side_round_trip() {
    check_wire_enum_round_trip(&[(ExecutionFilterSide::Buy, "BUY"), (ExecutionFilterSide::Sell, "SELL")]);
}

#[test]
fn execution_filter_side_from_str_rejects_unknown() {
    // Empty + arbitrary; case-sensitive (lowercase rejected); Action variants
    // (SSHORT/SLONG) not accepted on the filter; Execution.side wire (BOT/SLD)
    // also rejected — field-scoped vocabulary.
    check_wire_enum_rejects_unknown::<ExecutionFilterSide>(&["", "INVALID", "buy", "sell", "SSHORT", "SLONG", "BOT", "SLD"]);
}

#[test]
fn execution_side_round_trip() {
    check_wire_enum_round_trip(&[(ExecutionSide::Bought, "BOT"), (ExecutionSide::Sold, "SLD")]);
}

#[test]
fn execution_side_from_str_rejects_unknown() {
    // Empty + arbitrary; case-sensitive (lowercase rejected); ExecutionFilter
    // vocab (BUY/SELL) and Action vocab (SSHORT/SLONG) both rejected on the
    // execution-side field — field-scoped vocabulary per C# Execution.cs:83.
    check_wire_enum_rejects_unknown::<ExecutionSide>(&["", "INVALID", "bot", "sld", "BUY", "SELL", "SSHORT", "SLONG"]);
}

#[test]
fn is_active_and_is_terminal_agree_on_known_variants() {
    // ApiPending is the documented gap: neither active nor terminal.
    for (kind, text) in ALL_KINDS {
        let active = kind.is_active();
        let terminal = kind.is_terminal();
        match kind {
            OrderStatusKind::PreSubmitted | OrderStatusKind::PendingSubmit | OrderStatusKind::PendingCancel | OrderStatusKind::Submitted => {
                assert!(active, "{text} should be active");
                assert!(!terminal, "{text} should not be terminal");
            }
            OrderStatusKind::Filled | OrderStatusKind::Cancelled | OrderStatusKind::ApiCancelled | OrderStatusKind::Inactive => {
                assert!(!active, "{text} should not be active");
                assert!(terminal, "{text} should be terminal");
            }
            OrderStatusKind::ApiPending => {
                assert!(!active, "ApiPending should not be active");
                assert!(!terminal, "ApiPending should not be terminal");
            }
            OrderStatusKind::Unknown(_) => unreachable!("ALL_KINDS lists only the known variants"),
        }
    }
}

#[test]
fn liquidity_preserves_unknown_wire_code() {
    assert_eq!(Liquidity::from(0), Liquidity::None);
    assert_eq!(Liquidity::from(4), Liquidity::Unknown(4));
    assert_eq!(Liquidity::from(-1), Liquidity::Unknown(-1));
}
