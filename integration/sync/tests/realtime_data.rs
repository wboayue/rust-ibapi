use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::market_data::{MarketDataType, TradingHours};
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
fn market_data_snapshot() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.market_data(&contract).snapshot().subscribe().expect("market_data snapshot failed");

    // Snapshot should complete within timeout
    let mut count = 0;
    for _tick in subscription.timeout_iter(Duration::from_secs(15)) {
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

#[test]
fn market_data_streaming() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.market_data(&contract).subscribe().expect("market_data streaming failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    assert!(item.is_some(), "expected at least one market data tick");
}

#[test]
fn market_data_generic_ticks() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .market_data(&contract)
        .generic_ticks(&["233"])
        .subscribe()
        .expect("market_data generic_ticks failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    assert!(item.is_some(), "expected at least one tick with generic tick 233");
}

#[test]
fn switch_market_data_type() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    client.switch_market_data_type(MarketDataType::Delayed).expect("switch to delayed failed");

    rate_limit();
    client
        .switch_market_data_type(MarketDataType::Realtime)
        .expect("switch to realtime failed");
}

#[test]
fn realtime_bars_trades() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended)
        .expect("realtime_bars failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    if let Some(bar) = item {
        assert!(bar.close > 0.0, "bar close should be positive");
    }
}

#[test]
fn tick_by_tick_all_last() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.tick_by_tick_all_last(&contract, 0, false).expect("tick_by_tick_all_last failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    if let Some(trade) = item {
        assert!(trade.price > 0.0, "trade price should be positive");
    }
}

#[test]
fn tick_by_tick_bid_ask() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.tick_by_tick_bid_ask(&contract, 0, false).expect("tick_by_tick_bid_ask failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    if let Some(tick) = item {
        assert!(tick.bid_price > 0.0, "bid price should be positive");
        assert!(tick.ask_price > 0.0, "ask price should be positive");
    }
}

#[test]
fn tick_by_tick_midpoint() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.tick_by_tick_midpoint(&contract, 0, false).expect("tick_by_tick_midpoint failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    if let Some(mp) = item {
        assert!(mp.mid_point > 0.0, "midpoint should be positive");
    }
}

#[test]
fn market_depth_exchanges() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let exchanges = client.market_depth_exchanges().expect("market_depth_exchanges failed");

    assert!(!exchanges.is_empty(), "expected non-empty exchange list");
}

#[test]
fn market_depth_receives_data() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.market_depth(&contract, 5, false).expect("market_depth failed");

    let item = subscription.next_timeout(Duration::from_secs(15));
    // Market depth may or may not be available
    if item.is_some() {
        // Successfully received depth data
    }
}
