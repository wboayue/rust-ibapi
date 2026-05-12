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
fn order_status_kind_from_str_rejects_unknown() {
    check_wire_enum_rejects_unknown::<OrderStatusKind>(&["NotARealStatus", "", "submitted", "FILLED"]);
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
fn is_active_and_is_terminal_partition_eight_of_nine_variants() {
    // Exhaustive check: exactly one helper returns true for 8 variants;
    // ApiPending is the documented gap (neither active nor terminal).
    for &(kind, text) in ALL_KINDS {
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
        }
    }
}
