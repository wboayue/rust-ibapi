use super::*;

// The `Default` tests destructure every field without `..`, so a new field fails to compile
// here until its default is chosen.

#[test]
fn tick_last_default_is_zero_at_epoch_with_empty_strings() {
    let TickLast {
        time,
        price,
        size,
        attributes,
        exchange,
        special_conditions,
    } = TickLast::default();
    assert_eq!(time, OffsetDateTime::UNIX_EPOCH);
    assert_eq!((price, size), (0.0, 0.0));
    let TickAttributeLast { past_limit, unreported } = attributes;
    assert!(!past_limit && !unreported);
    assert_eq!((exchange.as_str(), special_conditions.as_str()), ("", ""));
}

#[test]
fn tick_bid_ask_default_is_zero_at_epoch() {
    let TickBidAsk {
        time,
        price_bid,
        price_ask,
        size_bid,
        size_ask,
        attributes,
    } = TickBidAsk::default();
    assert_eq!(time, OffsetDateTime::UNIX_EPOCH);
    assert_eq!((price_bid, price_ask, size_bid, size_ask), (0.0, 0.0, 0.0, 0.0));
    let TickAttributeBidAsk { bid_past_low, ask_past_high } = attributes;
    assert!(!bid_past_low && !ask_past_high);
}

#[test]
fn tick_midpoint_default_is_zero_at_epoch() {
    let TickMidpoint { time, price } = TickMidpoint::default();
    assert_eq!((time, price), (OffsetDateTime::UNIX_EPOCH, 0.0));
}

#[test]
fn quote_side_needs_positive_size() {
    let cases = [
        ("quote", 1.09, 2e6, Some(QuoteSide { price: 1.09, size: 2e6 })),
        ("negative combo price", -71.0, 1.0, Some(QuoteSide { price: -71.0, size: 1.0 })),
        ("market data placeholder", -1.0, 0.0, None),
        ("magnified placeholder", -100.0, 0.0, None),
        ("historical tick placeholder", 0.0, 0.0, None),
        ("real price, zero size", 5.0, 0.0, None),
    ];
    for (label, price, size, expected) in cases {
        assert_eq!(QuoteSide::new(price, size), expected, "{label}");
    }
}

/// A far out-of-the-money option's historical bid/ask tick: no bid, an offer.
#[test]
fn tick_bid_ask_sides() {
    let tick = TickBidAsk {
        price_bid: 0.0,
        size_bid: 0.0,
        price_ask: 0.01,
        size_ask: 8.0,
        ..TickBidAsk::default()
    };

    assert_eq!((tick.bid(), tick.ask()), (None, Some(QuoteSide { price: 0.01, size: 8.0 })));
}
