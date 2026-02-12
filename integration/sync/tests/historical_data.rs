use std::time::Duration as StdDuration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::TradingHours;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

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
        .historical_data(
            &contract,
            None,
            Duration::days(5),
            BarSize::Day,
            WhatToShow::Trades,
            TradingHours::Regular,
        )
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
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            WhatToShow::Trades,
            TradingHours::Regular,
        )
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
        .historical_data(
            &contract,
            None,
            Duration::seconds(1800),
            BarSize::Min,
            WhatToShow::Trades,
            TradingHours::Regular,
        )
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
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            WhatToShow::BidAsk,
            TradingHours::Regular,
        )
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
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            WhatToShow::MidPoint,
            TradingHours::Regular,
        )
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
        .historical_schedules_ending_now(&contract, Duration::months(1))
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
        .historical_ticks_trade(&contract, None, Some(end), 100, TradingHours::Regular)
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
        .historical_ticks_bid_ask(&contract, None, Some(end), 100, TradingHours::Regular, false)
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
        .historical_ticks_mid_point(&contract, None, Some(end), 100, TradingHours::Regular)
        .expect("historical_ticks_mid_point failed");

    let _tick = subscription.next_timeout(StdDuration::from_secs(10));
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
