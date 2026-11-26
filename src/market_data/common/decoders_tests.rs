//! Size handling for the converters both `tick_by_tick` and `historical_ticks` decode through;
//! each path's own tests cover only its envelope (tick type, missing oneof, `is_done`).

use super::*;
use crate::common::test_utils::helpers::assert_decimal_parse_error;

/// An absent size or the unset sentinel decodes as `0.0`; a sent value, fractional included
/// (#716), decodes as itself.
const SIZE_CASES: [(&str, Option<&str>, f64); 5] = [
    ("absent", None, 0.0),
    ("unset sentinel", Some("9223372036854775807"), 0.0),
    ("zero", Some("0"), 0.0),
    ("whole", Some("7"), 7.0),
    ("fractional", Some("0.25"), 0.25),
];

fn last(size: Option<&str>) -> proto::HistoricalTickLast {
    proto::HistoricalTickLast {
        size: size.map(str::to_string),
        ..Default::default()
    }
}

fn bid_ask(size_bid: Option<&str>, size_ask: Option<&str>) -> proto::HistoricalTickBidAsk {
    proto::HistoricalTickBidAsk {
        size_bid: size_bid.map(str::to_string),
        size_ask: size_ask.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn tick_last_size() {
    for (label, wire, expected) in SIZE_CASES {
        assert_eq!(decode_tick_last(last(wire)).expect(label).size, expected, "{label}");
    }
}

#[test]
fn tick_bid_ask_sizes() {
    for (label, wire, expected) in SIZE_CASES {
        let bid_only = decode_tick_bid_ask(bid_ask(wire, Some("1"))).expect(label);
        assert_eq!((bid_only.size_bid, bid_only.size_ask), (expected, 1.0), "bid {label}");
        let ask_only = decode_tick_bid_ask(bid_ask(Some("1"), wire)).expect(label);
        assert_eq!((ask_only.size_bid, ask_only.size_ask), (1.0, expected), "ask {label}");
    }
}

#[test]
fn malformed_sizes_are_parse_errors() {
    assert_decimal_parse_error(decode_tick_last(last(Some("abc"))), "abc");
    assert_decimal_parse_error(decode_tick_bid_ask(bid_ask(Some("abc"), Some("1"))), "abc");
    assert_decimal_parse_error(decode_tick_bid_ask(bid_ask(Some("1"), Some("abc"))), "abc");
}

/// A midpoint has no size, so whatever the wire carries in the field, malformed included,
/// is not read.
#[test]
fn tick_midpoint_ignores_size() {
    for wire in [None, Some("0"), Some("abc")] {
        let mid = decode_tick_midpoint(proto::HistoricalTick {
            time: Some(1_681_133_400),
            price: Some(150.5),
            size: wire.map(str::to_string),
        });
        assert_eq!(
            mid,
            TickMidpoint {
                time: ts(1_681_133_400),
                price: 150.5
            },
            "{wire:?}"
        );
    }
}

/// Bar trade fields by wire value (live, servers 223 and 226; #1247). `(label, volume, wap, count)` in,
/// `BarTradeFields` out.
#[test]
fn bar_trade_fields_by_value() {
    type Case = (
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        Option<i32>,
        (Option<f64>, Option<f64>, Option<i32>),
    );
    let cases: [Case; 9] = [
        ("TRADES", Some("1500"), Some("185.7"), Some(42), (Some(1500.0), Some(185.7), Some(42))),
        ("TRADES, quiet bar", Some("0"), Some("185.7"), Some(0), (Some(0.0), Some(185.7), Some(0))),
        ("historical non-TRADES: -1 sentinel", Some("-1"), Some("-1"), Some(-1), (None, None, None)),
        ("realtime non-TRADES: absent, count -1", None, None, Some(-1), (None, None, None)),
        ("volatility bar", Some("1"), Some("0.23"), Some(0), (Some(1.0), Some(0.23), Some(0))),
        ("WAP tied to volume", Some("-1"), Some("185.7"), Some(-1), (None, None, None)),
        ("magnified WAP sentinel", Some("-1"), Some("-100"), Some(-1), (None, None, None)),
        ("count absent", Some("10"), Some("185.7"), None, (Some(10.0), Some(185.7), None)),
        ("WAP absent, volume present", Some("10"), None, Some(5), (Some(10.0), None, Some(5))),
    ];
    for (label, volume, wap, count, (want_volume, want_wap, want_count)) in cases {
        let fields = decode_bar_trade_fields(volume, wap, count).expect(label);
        assert_eq!(
            fields,
            BarTradeFields {
                volume: want_volume,
                wap: want_wap,
                count: want_count
            },
            "{label}"
        );
    }
}

#[test]
fn bar_trade_fields_malformed_is_parse_error() {
    assert_decimal_parse_error(decode_bar_trade_fields(Some("abc"), Some("1"), Some(1)), "abc");
    assert_decimal_parse_error(decode_bar_trade_fields(Some("1"), Some("abc"), Some(1)), "abc");
}
