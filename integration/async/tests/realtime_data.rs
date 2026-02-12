use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::market_data::{MarketDataType, TradingHours};
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
async fn market_data_snapshot() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .await
        .expect("market_data snapshot failed");

    let mut count = 0;
    while let Ok(Some(_tick)) = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await {
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

#[tokio::test]
async fn market_data_streaming() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client.market_data(&contract).subscribe().await.expect("market_data streaming failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    assert!(item.is_ok(), "market data timed out");
    assert!(item.unwrap().is_some(), "expected at least one market data tick");
}

#[tokio::test]
async fn market_data_generic_ticks() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .market_data(&contract)
        .generic_ticks(&["233"])
        .subscribe()
        .await
        .expect("market_data generic_ticks failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    assert!(item.is_ok(), "generic ticks timed out");
    assert!(item.unwrap().is_some(), "expected at least one tick");
}

#[tokio::test]
async fn switch_market_data_type() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    client
        .switch_market_data_type(MarketDataType::Delayed)
        .await
        .expect("switch to delayed failed");

    rate_limit();
    client
        .switch_market_data_type(MarketDataType::Realtime)
        .await
        .expect("switch to realtime failed");
}

#[tokio::test]
async fn realtime_bars_trades() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended)
        .await
        .expect("realtime_bars failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    if let Ok(Some(Ok(bar))) = item {
        assert!(bar.close > 0.0, "bar close should be positive");
    }
}

#[tokio::test]
async fn tick_by_tick_all_last() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .tick_by_tick_all_last(&contract, 0, false)
        .await
        .expect("tick_by_tick_all_last failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    if let Ok(Some(Ok(trade))) = item {
        assert!(trade.price > 0.0, "trade price should be positive");
    }
}

#[tokio::test]
async fn tick_by_tick_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .tick_by_tick_bid_ask(&contract, 0, false)
        .await
        .expect("tick_by_tick_bid_ask failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    if let Ok(Some(Ok(tick))) = item {
        assert!(tick.bid_price > 0.0, "bid price should be positive");
        assert!(tick.ask_price > 0.0, "ask price should be positive");
    }
}

#[tokio::test]
async fn tick_by_tick_midpoint() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .tick_by_tick_midpoint(&contract, 0, false)
        .await
        .expect("tick_by_tick_midpoint failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
    if let Ok(Some(Ok(mp))) = item {
        assert!(mp.mid_point > 0.0, "midpoint should be positive");
    }
}

#[tokio::test]
async fn market_depth_exchanges() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let exchanges = client.market_depth_exchanges().await.expect("market_depth_exchanges failed");

    assert!(!exchanges.is_empty(), "expected non-empty exchange list");
}

#[tokio::test]
async fn market_depth_receives_data() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client.market_depth(&contract, 5, false).await.expect("market_depth failed");

    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(15), subscription.next()).await;
}
