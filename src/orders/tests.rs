use super::*;
use crate::common::test_utils::wire_enum::{check_wire_code_round_trip, check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

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

const ALL_TIFS: &[(TimeInForce, &str)] = &[
    (TimeInForce::Day, "DAY"),
    (TimeInForce::GoodTillCanceled, "GTC"),
    (TimeInForce::ImmediateOrCancel, "IOC"),
    (TimeInForce::GoodTillDate, "GTD"),
    (TimeInForce::OnOpen, "OPG"),
    (TimeInForce::FillOrKill, "FOK"),
    (TimeInForce::DayTillCanceled, "DTC"),
    (TimeInForce::Auction, "AUC"),
    (TimeInForce::GoodTillCrossing, "GTX"),
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

#[test]
fn time_in_force_round_trips_every_wire_value() {
    check_wire_enum_round_trip(ALL_TIFS);

    // `From` is the seam the proto decoder uses; it must agree with `FromStr`
    // on every known value.
    for (variant, wire) in ALL_TIFS {
        assert_eq!(&TimeInForce::from(*wire), variant, "From(&str {wire})");
        assert_eq!(&TimeInForce::from(wire.to_string()), variant, "From(String {wire})");
    }
}

#[test]
fn action_round_trip() {
    check_wire_enum_round_trip(&[
        (Action::Buy, "BUY"),
        (Action::Sell, "SELL"),
        (Action::SellShort, "SSHORT"),
        (Action::SellLong, "SLONG"),
    ]);
}

#[test]
fn action_from_str_rejects_unknown() {
    // Closed enum: empty, arbitrary, case-variants, and the Execution.side
    // vocabulary (BOT/SLD) are all errors, never coerced to Buy.
    check_wire_enum_rejects_unknown::<Action>(&["", "INVALID", "buy", "sell", "BOT", "SLD"]);
}

#[test]
fn rule_80_a_round_trip() {
    check_wire_enum_round_trip(&[
        (Rule80A::Individual, "I"),
        (Rule80A::Agency, "A"),
        (Rule80A::AgentOtherMember, "W"),
        (Rule80A::IndividualPTIA, "J"),
        (Rule80A::AgencyPTIA, "U"),
        (Rule80A::AgentOtherMemberPTIA, "M"),
        (Rule80A::IndividualPT, "K"),
        (Rule80A::AgencyPT, "Y"),
        (Rule80A::AgentOtherMemberPT, "N"),
    ]);
}

#[test]
fn rule_80_a_preserves_unknown_wire_value() {
    check_wire_enum_round_trip(&[(Rule80A::Unknown("Z".into()), "Z"), (Rule80A::Unknown("i".into()), "i")]);
    check_wire_enum_rejects_unknown::<Rule80A>(&[""]);
}

#[test]
fn order_open_close_round_trip() {
    check_wire_enum_round_trip(&[(OrderOpenClose::Open, "O"), (OrderOpenClose::Close, "C")]);
}

#[test]
fn order_open_close_preserves_unknown_wire_value() {
    check_wire_enum_round_trip(&[(OrderOpenClose::Unknown("X".into()), "X"), (OrderOpenClose::Unknown("o".into()), "o")]);
    check_wire_enum_rejects_unknown::<OrderOpenClose>(&[""]);
}

#[test]
fn oca_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (OcaType::None, 0),
        (OcaType::CancelWithBlock, 1),
        (OcaType::ReduceWithBlock, 2),
        (OcaType::ReduceWithoutBlock, 3),
        (OcaType::Unknown(4), 4),
        (OcaType::Unknown(-1), -1),
    ]);
}

#[test]
fn order_origin_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (OrderOrigin::Customer, 0),
        (OrderOrigin::Firm, 1),
        (OrderOrigin::Unknown(2), 2),
        (OrderOrigin::Unknown(-1), -1),
    ]);
}

#[test]
fn short_sale_slot_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (ShortSaleSlot::None, 0),
        (ShortSaleSlot::Broker, 1),
        (ShortSaleSlot::ThirdParty, 2),
        (ShortSaleSlot::Unknown(3), 3),
        (ShortSaleSlot::Unknown(-1), -1),
    ]);
}

#[test]
fn volatility_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (VolatilityType::Daily, 1),
        (VolatilityType::Annual, 2),
        (VolatilityType::Unknown(0), 0),
        (VolatilityType::Unknown(3), 3),
    ]);
}

#[test]
fn reference_price_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (ReferencePriceType::AverageOfNBBO, 1),
        (ReferencePriceType::NBBO, 2),
        (ReferencePriceType::Unknown(0), 0),
        (ReferencePriceType::Unknown(3), 3),
    ]);
}

#[test]
fn auction_strategy_round_trips_every_wire_code() {
    check_wire_code_round_trip(&[
        (AuctionStrategy::Match, 1),
        (AuctionStrategy::Improvement, 2),
        (AuctionStrategy::Transparent, 3),
        (AuctionStrategy::Unknown(0), 0),
        (AuctionStrategy::Unknown(4), 4),
    ]);
}

/// Compile-time guard that `ALL_TIFS` lists every modeled variant. A new
/// variant has no arm in `modeled_index` and fails to compile; the arm it
/// forces then indexes past `seen`, so the table has to grow before the
/// workspace goes green. `GoodTillCrossing` reached TWS as `DAY` for three
/// releases because nothing made a missing variant loud — see
/// `docs/migration-4.0.md` §13.
///
/// `src/orders/builder/order_builder/tests.rs` drives the same variant list
/// through the builder and the proto encoder; this guard covers both tables.
#[test]
fn all_tifs_covers_every_variant() {
    fn modeled_index(tif: &TimeInForce) -> Option<usize> {
        match tif {
            TimeInForce::Day => Some(0),
            TimeInForce::GoodTillCanceled => Some(1),
            TimeInForce::ImmediateOrCancel => Some(2),
            TimeInForce::GoodTillDate => Some(3),
            TimeInForce::OnOpen => Some(4),
            TimeInForce::FillOrKill => Some(5),
            TimeInForce::DayTillCanceled => Some(6),
            TimeInForce::Auction => Some(7),
            TimeInForce::GoodTillCrossing => Some(8),
            // Unknown carries a raw string, so it has no place in a table of
            // modeled wire values; `time_in_force_preserves_unknown_wire_value`
            // covers it.
            TimeInForce::Unknown(_) => None,
        }
    }

    let mut seen = [false; 9];
    for (variant, _) in ALL_TIFS {
        if let Some(index) = modeled_index(variant) {
            seen[index] = true;
        }
    }
    assert!(seen.iter().all(|&s| s), "ALL_TIFS is missing a TimeInForce variant");
}

#[test]
fn time_in_force_preserves_unknown_wire_value() {
    // Open enum, same shape as OrderStatusKind: an unrecognized non-empty TIF
    // keeps its raw value instead of decoding as Day, and puts that value back
    // on the wire unchanged so an order read from TWS round-trips. Matching is
    // exact and case-sensitive.
    check_wire_enum_round_trip(&[
        (TimeInForce::Unknown("NotARealTif".into()), "NotARealTif"),
        (TimeInForce::Unknown("gtc".into()), "gtc"),
        (TimeInForce::Unknown("Gtd".into()), "Gtd"),
    ]);

    // `FromStr` rejects the absent value (docs/rules/wire/enum-typing.md);
    // infallible `From` cannot, so it yields Unknown("") — the proto decoder
    // substitutes "DAY" before it gets here.
    check_wire_enum_rejects_unknown::<TimeInForce>(&[""]);
    assert_eq!(TimeInForce::from(""), TimeInForce::Unknown(String::new()));
}

#[test]
fn time_in_force_serde_round_trips_as_plain_string() {
    // Manual serde keeps the JSON the TWS wire string in both directions, so
    // Unknown("X") serializes as "X" rather than {"Unknown":"X"}, and renaming
    // a variant on the Rust side never moves the JSON.
    let known = TimeInForce::GoodTillCanceled;
    assert_eq!(serde_json::to_string(&known).unwrap(), "\"GTC\"");
    assert_eq!(serde_json::from_str::<TimeInForce>("\"GTC\"").unwrap(), known);

    let unknown = TimeInForce::Unknown("GTZ".to_string());
    assert_eq!(serde_json::to_string(&unknown).unwrap(), "\"GTZ\"");
    assert_eq!(serde_json::from_str::<TimeInForce>("\"GTZ\"").unwrap(), unknown);

    assert!(serde_json::from_str::<TimeInForce>("\"\"").is_err(), "empty string must not deserialize");
}
