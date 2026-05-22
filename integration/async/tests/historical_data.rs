use futures::StreamExt;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::IgnoreSize;
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

    let now = time::OffsetDateTime::now_utc();
    assert!(ts.year() <= now.year(), "head timestamp should be in the past");
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
        .historical_data(&contract, BarSize::Day)
        .duration(Duration::days(5))
        .fetch()
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
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
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
        .historical_data(&contract, BarSize::Min)
        .duration(Duration::seconds(1800))
        .fetch()
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
        .historical_data(&contract, BarSize::Hour)
        .what_to_show(WhatToShow::BidAsk)
        .duration(Duration::days(1))
        .fetch()
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
        .historical_data(&contract, BarSize::Hour)
        .what_to_show(WhatToShow::MidPoint)
        .duration(Duration::days(1))
        .fetch()
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
        .historical_schedules(&contract, Duration::months(1))
        .fetch()
        .await
        .expect("historical_schedules failed");

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
        .historical_ticks(&contract, 100)
        .ending(end)
        .trade()
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
        .historical_ticks(&contract, 100)
        .ending(end)
        .bid_ask(IgnoreSize::No)
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
        .historical_ticks(&contract, 100)
        .ending(end)
        .mid_point()
        .await
        .expect("historical_ticks_mid_point failed");

    let _tick = tokio::time::timeout(std::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(historical)]
async fn cancel_historical_ticks_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    // Cancel with an arbitrary request_id - should not error even if no request is pending
    rate_limit();
    let result = client.cancel_historical_ticks(99999).await;
    assert!(result.is_ok(), "cancel_historical_ticks failed: {:?}", result.err());
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
        .historical_data(&contract, BarSize::Min15)
        .duration(Duration::days(1))
        .stream()
        .await
        .expect("historical_data_streaming failed");

    // Should receive initial historical bars
    let item = subscription.next().await;
    assert!(item.is_some(), "expected initial historical data");
}
