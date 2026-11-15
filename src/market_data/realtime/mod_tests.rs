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

#[test]
fn tick_by_tick_and_bar_defaults_are_zero_at_epoch() {
    let BidAsk {
        time,
        bid_price,
        ask_price,
        bid_size,
        ask_size,
        bid_ask_attribute,
    } = BidAsk::default();
    assert_eq!(time, OffsetDateTime::UNIX_EPOCH);
    assert_eq!((bid_price, ask_price, bid_size, ask_size), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(
        bid_ask_attribute,
        BidAskAttribute {
            bid_past_low: false,
            ask_past_high: false
        }
    );

    let MidPoint { time, mid_point } = MidPoint::default();
    assert_eq!((time, mid_point), (OffsetDateTime::UNIX_EPOCH, 0.0));

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
    assert_eq!((open, high, low, close, volume, wap, count), (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0));

    let Trade {
        time,
        price,
        size,
        trade_attribute,
        exchange,
        special_conditions,
    } = Trade::default();
    assert_eq!(time, OffsetDateTime::UNIX_EPOCH);
    assert_eq!((price, size), (0.0, 0.0));
    assert_eq!(
        trade_attribute,
        TradeAttribute {
            past_limit: false,
            unreported: false
        }
    );
    assert_eq!((exchange.as_str(), special_conditions.as_str()), ("", ""));
}
