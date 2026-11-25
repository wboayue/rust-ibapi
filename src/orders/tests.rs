use super::*;
use crate::common::test_utils::wire_enum::{check_wire_code_round_trip, check_wire_enum_rejects_unknown, check_wire_enum_round_trip};
use crate::orders::common::test_tables::ALL_ORDER_TYPES;
use crate::Error;

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
fn wire_enum_serde_reads_5x_variant_names_as_documented() {
    // 5.x derived serde wrote variant names. A closed enum rejects them; an open one keeps the
    // name as an unrecognized code, as #1220 accepted for `OrderType` (migration-6.0).
    assert!(serde_json::from_str::<Action>("\"Buy\"").is_err());
    assert_eq!(
        serde_json::from_str::<Rule80A>("\"Individual\"").unwrap(),
        Rule80A::Unknown("Individual".into())
    );
    let stock: crate::contracts::SecurityType = serde_json::from_str("\"Stock\"").unwrap();
    assert_eq!(stock, crate::contracts::SecurityType::Other("Stock".into()));
    assert_eq!(stock.as_str(), "Stock", "sent to TWS as the stored name");
    // Integer-coded: the 5.x tagged unknown and variant name both fail.
    assert!(serde_json::from_str::<OcaType>("{\"Unknown\":7}").is_err());
    assert!(serde_json::from_str::<OcaType>("\"ReduceWithBlock\"").is_err());
}

#[test]
fn wire_enum_distinguishes_empty_from_unknown() {
    // Both macro arms report the two failures apart: no value at all vs. a
    // value this crate does not model. `parse_required` reaches the first only
    // when a caller parses directly — it intercepts empty with "missing
    // {label}", which names the field instead of the type.
    for (input, reason) in [("", "empty Action"), ("NOTASIDE", "unknown Action")] {
        match input.parse::<Action>() {
            Err(Error::Parse(_, raw, got)) => {
                assert_eq!(raw, input);
                assert_eq!(got, reason);
            }
            other => panic!("expected Error::Parse for {input:?}, got {other:?}"),
        }
    }

    // The `fallback` arm only ever rejects empty; anything else is Unknown.
    match "".parse::<Rule80A>() {
        Err(Error::Parse(_, _, reason)) => assert_eq!(reason, "empty Rule80A"),
        other => panic!("expected Error::Parse for an empty Rule80A, got {other:?}"),
    }
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
    check_wire_code_round_trip(
        &[
            (OcaType::None, 0),
            (OcaType::CancelWithBlock, 1),
            (OcaType::ReduceWithBlock, 2),
            (OcaType::ReduceWithoutBlock, 3),
            (OcaType::Unknown(4), 4),
            (OcaType::Unknown(-1), -1),
        ],
        OcaType::Unknown,
    );
}

#[test]
fn adjustable_trailing_unit_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (AdjustableTrailingUnit::Amount, 0),
            (AdjustableTrailingUnit::Percent, 100),
            (AdjustableTrailingUnit::Unknown(1), 1),
            (AdjustableTrailingUnit::Unknown(-1), -1),
        ],
        AdjustableTrailingUnit::Unknown,
    );
}

#[test]
fn order_origin_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (OrderOrigin::Customer, 0),
            (OrderOrigin::Firm, 1),
            (OrderOrigin::Unknown(2), 2),
            (OrderOrigin::Unknown(-1), -1),
        ],
        OrderOrigin::Unknown,
    );
}

#[test]
fn short_sale_slot_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (ShortSaleSlot::None, 0),
            (ShortSaleSlot::Broker, 1),
            (ShortSaleSlot::ThirdParty, 2),
            (ShortSaleSlot::Unknown(3), 3),
            (ShortSaleSlot::Unknown(-1), -1),
        ],
        ShortSaleSlot::Unknown,
    );
}

#[test]
fn volatility_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (VolatilityType::Daily, 1),
            (VolatilityType::Annual, 2),
            (VolatilityType::Unknown(0), 0),
            (VolatilityType::Unknown(3), 3),
        ],
        VolatilityType::Unknown,
    );
}

#[test]
fn reference_price_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (ReferencePriceType::AverageOfNBBO, 1),
            (ReferencePriceType::NBBO, 2),
            (ReferencePriceType::Unknown(0), 0),
            (ReferencePriceType::Unknown(3), 3),
        ],
        ReferencePriceType::Unknown,
    );
}

#[test]
fn what_if_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (WhatIfType::FullOrderCheck, 0),
            (WhatIfType::MarginRequirements, 1),
            (WhatIfType::Unknown(2), 2),
            (WhatIfType::Unknown(-1), -1),
        ],
        WhatIfType::Unknown,
    );
}

#[test]
fn liquidity_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (Liquidity::None, 0),
            (Liquidity::AddedLiquidity, 1),
            (Liquidity::RemovedLiquidity, 2),
            (Liquidity::LiquidityRoutedOut, 3),
            (Liquidity::Unknown(4), 4),
            (Liquidity::Unknown(-1), -1),
        ],
        Liquidity::Unknown,
    );
}

/// Table from Execution.cs `COptionExerciseType`.
#[test]
fn option_exercise_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (OptionExerciseType::None, -1),
            (OptionExerciseType::Exercise, 1),
            (OptionExerciseType::Lapse, 2),
            (OptionExerciseType::DoNothing, 3),
            (OptionExerciseType::Assigned, 100),
            (OptionExerciseType::AutoexerciseClearing, 101),
            (OptionExerciseType::Expired, 102),
            (OptionExerciseType::AutoexerciseTrading, 200),
            (OptionExerciseType::Netting, 203),
            (OptionExerciseType::Unknown(0), 0),
            (OptionExerciseType::Unknown(4), 4),
        ],
        OptionExerciseType::Unknown,
    );
}

/// Compile-time guard that `ALL_TIFS` lists every modeled variant. A new
/// variant has no arm in `modeled_index` and fails to compile; the arm it
/// forces then indexes past `seen`, so the table has to grow before the
/// workspace goes green. `GoodTillCrossing` reached TWS as `DAY` for three
/// releases because nothing made a missing variant loud — see
/// `docs/migration-4.0.md` §13.
///
/// `src/orders/builder/order_builder_tests.rs` drives the same variant list
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

#[test]
fn order_type_round_trip() {
    check_wire_enum_round_trip(ALL_ORDER_TYPES);
}

/// Compile-time guard that `ALL_ORDER_TYPES` (`src/orders/common/test_tables.rs`)
/// lists every modeled variant, the shape of `all_tifs_covers_every_variant`.
/// `src/orders/builder/order_builder_tests.rs` drives the same table through
/// `build()`.
#[test]
fn all_order_types_covers_every_variant() {
    fn modeled_index(order_type: &OrderType) -> Option<usize> {
        match order_type {
            OrderType::Market => Some(0),
            OrderType::Limit => Some(1),
            OrderType::Stop => Some(2),
            OrderType::StopLimit => Some(3),
            OrderType::TrailingStop => Some(4),
            OrderType::TrailingStopLimit => Some(5),
            OrderType::MarketOnClose => Some(6),
            OrderType::LimitOnClose => Some(7),
            OrderType::MarketIfTouched => Some(8),
            OrderType::LimitIfTouched => Some(9),
            OrderType::MarketWithProtection => Some(10),
            OrderType::StopWithProtection => Some(11),
            OrderType::MarketToLimit => Some(12),
            OrderType::Midprice => Some(13),
            OrderType::PeggedToMarket => Some(14),
            OrderType::PeggedToStock => Some(15),
            OrderType::PeggedToMidpoint => Some(16),
            OrderType::PeggedToBenchmark => Some(17),
            OrderType::PegBest => Some(18),
            OrderType::Relative => Some(19),
            OrderType::PassiveRelative => Some(20),
            OrderType::Volatility => Some(21),
            OrderType::BoxTop => Some(22),
            OrderType::RelativeLimitCombo => Some(23),
            OrderType::RelativeMarketCombo => Some(24),
            // Unknown carries a raw string, so it has no place in a table of
            // modeled wire values; `order_type_preserves_unknown_wire_value`
            // covers it.
            OrderType::Unknown(_) => None,
        }
    }

    let mut seen = [false; 25];
    for (variant, _) in ALL_ORDER_TYPES {
        if let Some(index) = modeled_index(variant) {
            seen[index] = true;
        }
    }
    assert!(seen.iter().all(|&s| s), "ALL_ORDER_TYPES is missing an OrderType variant");
}

#[test]
fn order_type_preserves_unknown_wire_value() {
    // Open enum, same shape as TimeInForce: an unrecognized non-empty order
    // type keeps its raw value and puts it back on the wire unchanged, so an
    // order read from TWS round-trips. Matching is exact and case-sensitive.
    check_wire_enum_round_trip(&[
        (OrderType::Unknown("SNAP MID".into()), "SNAP MID"),
        (OrderType::Unknown("lmt".into()), "lmt"),
        (OrderType::Unknown("Mkt".into()), "Mkt"),
    ]);

    // `FromStr` rejects the absent value (docs/rules/wire/enum-typing.md); the
    // proto decoder maps it, and the string "None" TWS echoes for an unset adjusted
    // or delta-neutral type, to `None` before it gets here. "None" alone is Unknown.
    check_wire_enum_rejects_unknown::<OrderType>(&[""]);
    assert_eq!("None".parse::<OrderType>().unwrap(), OrderType::Unknown("None".into()));
    assert_eq!(Order::default().order_type, None);
}

#[test]
fn order_type_serde_round_trips_as_plain_string() {
    // Manual serde keeps the JSON the TWS wire string in both directions, so
    // Unknown("X") serializes as "X" rather than {"Unknown":"X"} and `Order`
    // JSON keeps "order_type":"LMT".
    let known = OrderType::StopLimit;
    assert_eq!(serde_json::to_string(&known).unwrap(), "\"STP LMT\"");
    assert_eq!(serde_json::from_str::<OrderType>("\"STP LMT\"").unwrap(), known);

    let unknown = OrderType::Unknown("SNAP MID".to_string());
    assert_eq!(serde_json::to_string(&unknown).unwrap(), "\"SNAP MID\"");
    assert_eq!(serde_json::from_str::<OrderType>("\"SNAP MID\"").unwrap(), unknown);

    // "" is rejected, as by FromStr; the `Order` fields handle it below.
    assert!(serde_json::from_str::<OrderType>("\"\"").is_err());
}

#[test]
fn order_type_fields_serde_unset_is_null_and_reads_legacy_strings() {
    // An unset order-type field serializes as null. 5.x JSON holds "" for an unset
    // order_type and the TWS echo "None" for an unset adjusted or delta-neutral type;
    // both read back as None.
    let json = serde_json::to_string(&Order::default()).unwrap();
    for field in ["order_type", "adjusted_order_type", "delta_neutral_order_type"] {
        assert!(json.contains(&format!("\"{field}\":null")), "{field} in {json}");
    }
    let legacy = json
        .replace("\"order_type\":null", "\"order_type\":\"\"")
        .replace("\"adjusted_order_type\":null", "\"adjusted_order_type\":\"None\"")
        .replace("\"delta_neutral_order_type\":null", "\"delta_neutral_order_type\":\"None\"");
    assert_ne!(legacy, json);
    assert_eq!(serde_json::from_str::<Order>(&legacy).unwrap(), Order::default());

    let set = json
        .replace("\"order_type\":null", "\"order_type\":\"STP LMT\"")
        .replace("\"adjusted_order_type\":null", "\"adjusted_order_type\":\"TRAIL\"");
    let order: Order = serde_json::from_str(&set).unwrap();
    assert_eq!(order.order_type, Some(OrderType::StopLimit));
    assert_eq!(order.adjusted_order_type, Some(OrderType::TrailingStop));
}
