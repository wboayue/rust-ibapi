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
