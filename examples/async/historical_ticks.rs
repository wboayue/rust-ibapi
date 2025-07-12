//! Async historical tick data example
//!
//! This example demonstrates how to request historical tick-level data using the async API.
//! This includes trades, bid/ask quotes, and midpoint data.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_historical_ticks
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)
//! - Modify start/end times or number of ticks

use std::sync::Arc;

use futures::StreamExt;
use ibapi::{contracts::Contract, Client};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Requesting historical tick data for {}\n", contract.symbol);

    // Example 1: Get last 100 trades
    println!("=== Historical Trades (last 100) ===");
    let mut tick_subscription = client
        .historical_ticks_trade(
            &contract, None, // Start time (None = use number_of_ticks)
            None, // End time (None = now)
            100,  // Number of ticks
            true, // Use RTH
        )
        .await?;

    let mut trade_count = 0;
    while let Some(tick) = tick_subscription.next().await {
        trade_count += 1;
        if trade_count <= 5 {
            // Show first 5
            println!(
                "Trade {}: {} - ${:.2} x {} on {} [{}]",
                trade_count,
                tick.timestamp.format(&Rfc3339).unwrap(),
                tick.price,
                tick.size,
                tick.exchange,
                tick.special_conditions
            );
        }
    }
    println!("Total trades received: {trade_count}");

    // Example 2: Get trades from specific time period
    println!("\n=== Historical Trades (time range) ===");
    let end_time = OffsetDateTime::now_utc();
    let start_time = end_time - Duration::minutes(30); // Last 30 minutes

    let mut tick_subscription = client
        .historical_ticks_trade(
            &contract,
            Some(start_time),
            Some(end_time),
            0,    // 0 = get all ticks in range
            true, // Use RTH
        )
        .await?;

    let mut period_trades = 0;
    let mut total_volume = 0;
    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;

    while let Some(tick) = tick_subscription.next().await {
        period_trades += 1;
        total_volume += tick.size;
        min_price = min_price.min(tick.price);
        max_price = max_price.max(tick.price);
    }

    println!(
        "Period: {} to {}",
        start_time.format(&Rfc3339).unwrap(),
        end_time.format(&Rfc3339).unwrap()
    );
    println!("Trades in period: {period_trades}");
    println!("Total volume: {total_volume}");
    if period_trades > 0 {
        println!("Price range: ${min_price:.2} - ${max_price:.2}");
    }

    // Example 3: Get historical bid/ask quotes
    println!("\n=== Historical Bid/Ask Quotes ===");
    let mut tick_subscription = client
        .historical_ticks_bid_ask(
            &contract, None,  // Start time
            None,  // End time
            50,    // Number of ticks
            true,  // Use RTH
            false, // Don't ignore size
        )
        .await?;

    let mut quote_count = 0;
    let mut total_spread = 0.0;

    while let Some(tick) = tick_subscription.next().await {
        quote_count += 1;
        let spread = tick.price_ask - tick.price_bid;
        total_spread += spread;

        if quote_count <= 5 {
            println!(
                "Quote {}: {} - Bid: ${:.2} x {} | Ask: ${:.2} x {} | Spread: ${:.2}",
                quote_count,
                tick.timestamp.format(&Rfc3339).unwrap(),
                tick.price_bid,
                tick.size_bid,
                tick.price_ask,
                tick.size_ask,
                spread
            );
        }
    }

    println!("Total quotes received: {quote_count}");
    if quote_count > 0 {
        println!("Average spread: ${:.3}", total_spread / quote_count as f64);
    }

    // Example 4: Get historical midpoint data
    println!("\n=== Historical Midpoint Data ===");
    let mut tick_subscription = client
        .historical_ticks_mid_point(
            &contract, None, // Start time
            None, // End time
            30,   // Number of ticks
            true, // Use RTH
        )
        .await?;

    let mut midpoint_count = 0;
    while let Some(tick) = tick_subscription.next().await {
        midpoint_count += 1;
        if midpoint_count <= 5 {
            println!(
                "Midpoint {}: {} - ${:.2}",
                midpoint_count,
                tick.timestamp.format(&Rfc3339).unwrap(),
                tick.price
            );
        }
    }
    println!("Total midpoints received: {midpoint_count}");

    println!("\nHistorical ticks example completed!");
    Ok(())
}
