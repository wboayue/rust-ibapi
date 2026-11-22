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
