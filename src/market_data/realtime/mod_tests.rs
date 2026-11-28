use super::*;
use crate::common::test_utils::wire_enum::check_wire_code_round_trip;

#[test]
fn depth_operation_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (DepthOperation::Insert, 0),
            (DepthOperation::Update, 1),
            (DepthOperation::Delete, 2),
            (DepthOperation::Unknown(3), 3),
            (DepthOperation::Unknown(-1), -1),
        ],
        DepthOperation::Unknown,
    );
}

#[test]
fn depth_side_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (DepthSide::Ask, 0),
            (DepthSide::Bid, 1),
            (DepthSide::Unknown(2), 2),
            (DepthSide::Unknown(-1), -1),
        ],
        DepthSide::Unknown,
    );
}

#[test]
fn what_to_show_round_trip() {
    use crate::builder::ValidationError;
    use crate::common::test_utils::wire_enum::check_wire_enum_round_trip;

    check_wire_enum_round_trip(&[
        (WhatToShow::Trades, "TRADES"),
        (WhatToShow::MidPoint, "MIDPOINT"),
        (WhatToShow::Bid, "BID"),
        (WhatToShow::Ask, "ASK"),
        (WhatToShow::AggTrades, "AGGTRADES"),
    ]);
    // Historical-only codes and the variant name are not realtime wire values, and matching is
    // case-sensitive (the historical `WhatToShow` is not). Caller input, so `ValidationError`.
    for (input, reason) in [
        ("", "empty"),
        ("BID_ASK", "unknown value"),
        ("MidPoint", "unknown value"),
        ("trades", "unknown value"),
    ] {
        let expected = ValidationError::InvalidField {
            field: "WhatToShow",
            value: input.to_owned(),
            reason: reason.to_owned(),
        };
        assert_eq!(input.parse::<WhatToShow>(), Err(expected), "{input:?}");
    }
}

/// Only 317 becomes `Reset`; other notices on a depth stream, including the
/// other data advisories, stay `Notice` (#899).
#[test]
fn market_depths_data_from_notice() {
    let notice = |code| Notice::synthesized(code, String::new());

    assert_eq!(
        <MarketDepths as StreamDecoder<MarketDepths>>::data_from_notice(&notice(317)),
        Some(MarketDepths::Reset)
    );
    for code in [316, 2104, 2188, 10167] {
        assert_eq!(
            <MarketDepths as StreamDecoder<MarketDepths>>::data_from_notice(&notice(code)),
            None,
            "code {code}"
        );
    }
}

// The `Default` tests destructure every field without `..`, so a new field fails to compile
// here until its default is chosen.

#[test]
fn bar_default_is_zero_at_epoch() {
    let Bar {
        date,
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    } = Bar::default();
    assert_eq!(date, OffsetDateTime::UNIX_EPOCH);
    assert_eq!((open, high, low, close), (0.0, 0.0, 0.0, 0.0));
    assert_eq!((volume, wap, count), (None, None, None));
}

#[test]
fn what_to_show_trades_for_security_type() {
    let cases = [
        (SecurityType::Crypto, Some(WhatToShow::AggTrades)),
        (SecurityType::ForexPair, None),
        (SecurityType::Commodity, None),
        (SecurityType::CFD, None),
        (SecurityType::Stock, Some(WhatToShow::Trades)),
        (SecurityType::Future, Some(WhatToShow::Trades)),
        (SecurityType::Index, Some(WhatToShow::Trades)),
        (SecurityType::Other("XYZ".into()), Some(WhatToShow::Trades)),
    ];
    for (security_type, expected) in cases {
        assert_eq!(WhatToShow::trades_for(&security_type), expected, "{security_type:?}");
    }
}
