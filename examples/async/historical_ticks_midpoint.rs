#![allow(clippy::uninlined_format_args)]
//! Async historical midpoint ticks example
//!
//! This example demonstrates how to retrieve historical midpoint tick data
//! using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_historical_ticks_midpoint
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the contract and time range as desired

use std::sync::Arc;

use ibapi::{
    contracts::{Contract, SecurityType},
    market_data::TradingHours,
    Client,
};
use time::macros::datetime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create contract for highly liquid forex pair
    let contract = Contract {
        symbol: "EUR".to_string(),
        security_type: SecurityType::ForexPair,
        currency: "USD".to_string(),
        exchange: "IDEALPRO".to_string(),
        ..Default::default()
    };

    println!("\nRetrieving historical midpoint ticks for EUR/USD");

    // Request historical midpoint ticks from a specific time range
    let start = datetime!(2024-01-05 15:00 UTC);
    let end = datetime!(2024-01-05 15:05 UTC);
    let number_of_ticks = 1000; // Max 1000 ticks per request
    let trading_hours = TradingHours::Extended; // Include all hours for forex

    let mut tick_subscription = client
        .historical_ticks_mid_point(&contract, Some(start), Some(end), number_of_ticks, trading_hours)
        .await?;

    println!("Time range: {} to {}", start, end);
    println!("\nTime                     | Midpoint Price");
    println!("-------------------------|---------------");

    let mut tick_count = 0;
    let mut prices = Vec::new();

    // Process all historical ticks
    while let Some(tick) = tick_subscription.next().await {
        tick_count += 1;
        prices.push(tick.price);

        // Format timestamp
        let time_str = format!("{}", tick.timestamp);

        println!("{} | {:.5}", time_str, tick.price);

        // Show statistics every 20 ticks
        if tick_count % 20 == 0 {
            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;

            println!("\n--- Stats after {} ticks ---", tick_count);
            println!("Min: {:.5}, Max: {:.5}, Avg: {:.5}", min_price, max_price, avg_price);
            println!("Spread: {:.5} pips\n", (max_price - min_price) * 10000.0);
        }
    }

    // Final statistics
    if !prices.is_empty() {
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;

        // Calculate simple volatility (standard deviation)
        let variance = prices.iter().map(|&price| (price - avg_price).powi(2)).sum::<f64>() / prices.len() as f64;
        let std_dev = variance.sqrt();

        println!("\n========== Summary ==========");
        println!("Total midpoint ticks: {tick_count:?}");
        println!("Price range: {:.5} - {:.5}", min_price, max_price);
        println!("Average price: {:.5}", avg_price);
        println!("Range in pips: {:.1}", (max_price - min_price) * 10000.0);
        println!("Standard deviation: {:.5}", std_dev);
        println!("Volatility (pips): {:.1}", std_dev * 10000.0);
    }

    // Example 2: Request recent midpoint ticks (last hour)
    println!("\n\nExample 2: Recent midpoint ticks (last hour)");
    let contract2 = Contract::stock("AAPL");

    // Use end time only to get most recent data
    let start2 = None;
    let end2 = None; // Current time
    let mut tick_subscription2 = client
        .historical_ticks_mid_point(&contract2, start2, end2, 100, TradingHours::Regular)
        .await?;

    println!("\nLast 10 midpoint ticks for AAPL:");
    println!("Time                     | Midpoint Price");
    println!("-------------------------|---------------");

    let mut recent_ticks = Vec::new();
    while let Some(tick) = tick_subscription2.next().await {
        recent_ticks.push(tick);
    }

    // Show last 10 ticks
    for tick in recent_ticks.iter().rev().take(10).rev() {
        let time_str = format!("{}", tick.timestamp);
        println!("{} | ${:.2}", time_str, tick.price);
    }

    println!("\nExample completed!");
    Ok(())
}
