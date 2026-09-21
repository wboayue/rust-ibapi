use std::time::Duration as StdDuration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{Bar, BarSize, BarTimestamp, Duration, WhatToShow};
use ibapi::market_data::IgnoreSize;
use ibapi::market_data::TradingHours;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;
use time::macros::datetime;
use time::OffsetDateTime;

#[test]
#[serial(historical)]
fn head_timestamp_stock() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let ts = client
        .head_timestamp(&contract, WhatToShow::Trades, TradingHours::Regular)
        .expect("head_timestamp failed");

    let now = time::OffsetDateTime::now_utc();
    assert!(ts.year() <= now.year(), "head timestamp should be in the past");
}

#[test]
#[serial(historical)]
fn head_timestamp_forex() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::forex("EUR", "USD").build();
    let ts = client
        .head_timestamp(&contract, WhatToShow::MidPoint, TradingHours::Extended)
        .expect("head_timestamp failed");

    assert!(ts.year() > 2000, "head timestamp year should be valid");
}

#[test]
#[serial(historical)]
fn historical_data_daily() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Day)
        .duration(Duration::days(5))
        .fetch()
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
    assert!(data.bars[0].volume >= 0.0, "volume should be non-negative");
}

#[test]
#[serial(historical)]
fn historical_data_hourly() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("MSFT").build();
    let data = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[test]
#[serial(historical)]
fn historical_data_minute() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Min)
        .duration(Duration::seconds(1800))
        .fetch()
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[test]
#[serial(historical)]
fn historical_data_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Hour)
        .what_to_show(WhatToShow::BidAsk)
        .duration(Duration::days(1))
        .fetch()
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[test]
#[serial(historical)]
fn historical_data_midpoint() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Hour)
        .what_to_show(WhatToShow::MidPoint)
        .duration(Duration::days(1))
        .fetch()
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[test]
#[serial(historical)]
fn historical_schedule() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let schedule = client
        .historical_schedules(&contract, Duration::months(1))
        .fetch()
        .expect("historical_schedule failed");

    assert!(!schedule.sessions.is_empty(), "expected at least one session");
}

#[test]
#[serial(historical)]
fn historical_ticks_trade() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let subscription = client
        .historical_ticks(&contract, 100)
        .ending(end)
        .trade()
        .expect("historical_ticks_trade failed");

    let _tick = subscription.next_timeout(StdDuration::from_secs(10));
}

#[test]
#[serial(historical)]
fn historical_ticks_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let subscription = client
        .historical_ticks(&contract, 100)
        .ending(end)
        .bid_ask(IgnoreSize::No)
        .expect("historical_ticks_bid_ask failed");

    let _tick = subscription.next_timeout(StdDuration::from_secs(10));
}

#[test]
#[serial(historical)]
fn historical_ticks_mid_point() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let subscription = client
        .historical_ticks(&contract, 100)
        .ending(end)
        .mid_point()
        .expect("historical_ticks_mid_point failed");

    let _tick = subscription.next_timeout(StdDuration::from_secs(10));
}

#[test]
#[serial(historical)]
fn cancel_historical_ticks_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // Cancel with an arbitrary request_id - should not error even if no request is pending
    rate_limit();
    let result = client.cancel_historical_ticks(99999);
    assert!(result.is_ok(), "cancel_historical_ticks failed: {:?}", result.err());
}

#[test]
#[serial(historical)]
fn histogram_data_weekly() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .histogram_data(&contract, TradingHours::Regular, BarSize::Week)
        .expect("histogram_data failed");

    assert!(!data.is_empty(), "expected non-empty histogram data");
    assert!(data[0].price > 0.0, "price should be positive");
}

// Issue #835: `.between` sent the wall-clock span as `N S`, which IBKR counts in
// trading time, so a 1-day range returned ~3.7 RTH sessions ending at `end`.
#[test]
#[serial(historical)]
fn historical_data_between_one_day_stays_in_range() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    let start = datetime!(2026-09-17 00:00 UTC);
    let end = datetime!(2026-09-18 00:00 UTC);

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Hour)
        .between(start, end)
        .fetch()
        .expect("historical_data failed");

    assert_bars_within(&data.bars, start, end);
}

// Issue #835: a multi-day range exceeds IBKR's 86400 S ceiling for the seconds unit.
#[test]
#[serial(historical)]
fn historical_data_between_multi_day_stays_in_range() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    let start = datetime!(2026-09-15 00:00 UTC);
    let end = datetime!(2026-09-18 00:00 UTC);

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(&contract, BarSize::Hour)
        .between(start, end)
        .fetch()
        .expect("historical_data failed");

    assert_bars_within(&data.bars, start, end);
    assert!(
        data.bars
            .first()
            .is_some_and(|bar| bar.date < BarTimestamp::from(datetime!(2026-09-16 00:00 UTC))),
        "expected bars from the first day of the range, first bar: {:?}",
        data.bars.first().map(|bar| bar.date)
    );
}

fn assert_bars_within(bars: &[Bar], start: OffsetDateTime, end: OffsetDateTime) {
    assert!(!bars.is_empty(), "expected non-empty bars");
    let (start, end) = (BarTimestamp::from(start), BarTimestamp::from(end));
    let outside: Vec<_> = bars.iter().map(|bar| bar.date).filter(|date| *date < start || *date >= end).collect();
    assert!(outside.is_empty(), "bars outside [{start:?}, {end:?}): {outside:?}");
}
