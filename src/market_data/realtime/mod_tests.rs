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
fn what_to_show_display() {
    assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
    assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
    assert_eq!(WhatToShow::Bid.to_string(), "BID");
    assert_eq!(WhatToShow::Ask.to_string(), "ASK");
    assert_eq!(WhatToShow::AggTrades.to_string(), "AGGTRADES");
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
