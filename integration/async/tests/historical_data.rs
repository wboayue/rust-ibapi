use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::TradingHours;
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

#[tokio::test]
#[serial(historical)]
async fn head_timestamp_stock() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let ts = client
        .head_timestamp(&contract, WhatToShow::Trades, TradingHours::Regular)
        .await
        .expect("head_timestamp failed");

    assert!(ts.year() < 2026, "head timestamp should be in the past");
}

#[tokio::test]
#[serial(historical)]
async fn head_timestamp_forex() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::forex("EUR", "USD").build();
    let ts = client
        .head_timestamp(&contract, WhatToShow::MidPoint, TradingHours::Extended)
        .await
        .expect("head_timestamp failed");

    assert!(ts.year() > 2000, "head timestamp year should be valid");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_daily() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(
            &contract,
            None,
            Duration::days(5),
            BarSize::Day,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
        )
        .await
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
    assert!(data.bars[0].volume >= 0.0, "volume should be non-negative");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_hourly() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("MSFT").build();
    let data = client
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
        )
        .await
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_minute() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(
            &contract,
            None,
            Duration::seconds(1800),
            BarSize::Min,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
        )
        .await
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::BidAsk),
            TradingHours::Regular,
        )
        .await
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_midpoint() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .historical_data(
            &contract,
            None,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::MidPoint),
            TradingHours::Regular,
        )
        .await
        .expect("historical_data failed");

    assert!(!data.bars.is_empty(), "expected non-empty bars");
}

#[tokio::test]
#[serial(historical)]
async fn historical_schedule() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let schedule = client
        .historical_schedule(&contract, None, Duration::months(1))
        .await
        .expect("historical_schedule failed");

    assert!(!schedule.sessions.is_empty(), "expected at least one session");
}

#[tokio::test]
#[serial(historical)]
async fn historical_ticks_trade() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let mut subscription = client
        .historical_ticks_trade(&contract, None, Some(end), 100, TradingHours::Regular)
        .await
        .expect("historical_ticks_trade failed");

    let _tick = tokio::time::timeout(std::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(historical)]
async fn historical_ticks_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let mut subscription = client
        .historical_ticks_bid_ask(&contract, None, Some(end), 100, TradingHours::Regular, false)
        .await
        .expect("historical_ticks_bid_ask failed");

    let _tick = tokio::time::timeout(std::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(historical)]
async fn historical_ticks_mid_point() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let end = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let mut subscription = client
        .historical_ticks_mid_point(&contract, None, Some(end), 100, TradingHours::Regular)
        .await
        .expect("historical_ticks_mid_point failed");

    let _tick = tokio::time::timeout(std::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(historical)]
async fn histogram_data_weekly() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let data = client
        .histogram_data(&contract, TradingHours::Regular, BarSize::Week)
        .await
        .expect("histogram_data failed");

    assert!(!data.is_empty(), "expected non-empty histogram data");
    assert!(data[0].price > 0.0, "price should be positive");
}

#[tokio::test]
#[serial(historical)]
async fn historical_data_streaming() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("SPY").build();
    let mut subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Min15,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            false,
        )
        .await
        .expect("historical_data_streaming failed");

    // Should receive initial historical bars
    let item = subscription.next().await;
    assert!(item.is_some(), "expected initial historical data");
}
