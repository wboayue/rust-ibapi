use super::*;
use crate::Error;
use std::str::FromStr;

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
fn order_status_kind_from_str_round_trips_for_all_variants() {
    for &(kind, text) in ALL_KINDS {
        let parsed = OrderStatusKind::from_str(text).unwrap_or_else(|e| panic!("FromStr failed for {text}: {e}"));
        assert_eq!(parsed, kind, "FromStr({text}) mapped to wrong variant");
        assert_eq!(kind.to_string(), text, "Display for {kind:?} did not produce {text}");
        // Display → FromStr must round-trip.
        assert_eq!(OrderStatusKind::from_str(&kind.to_string()).unwrap(), kind);
    }
}

#[test]
fn order_status_kind_from_str_rejects_unknown_status() {
    let err = OrderStatusKind::from_str("NotARealStatus").expect_err("unknown string should not parse");
    match err {
        Error::Parse(_, value, _) => assert_eq!(value, "NotARealStatus"),
        other => panic!("expected Error::Parse, got {other:?}"),
    }
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
