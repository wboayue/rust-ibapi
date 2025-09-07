#![allow(clippy::uninlined_format_args)]
//! Async historical trade ticks example
//!
//! This example demonstrates how to retrieve historical trade tick data
//! using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_historical_ticks_trade
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the contract and time range as desired

use std::sync::Arc;

use ibapi::prelude::*;
use time::macros::datetime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("TSLA").build();
    println!("\nRetrieving historical trade ticks for {}", contract.symbol);

    // Request historical trade ticks from a specific time range
    let start_time = datetime!(2024-01-05 15:55 UTC);
    let end_time = datetime!(2024-01-05 16:00 UTC);
    let start = Some(start_time);
    let end = Some(end_time);
    let number_of_ticks = 1000; // Max 1000 ticks per request
    let trading_hours = TradingHours::Regular; // Only regular trading hours

    let mut tick_subscription = client
        .historical_ticks_trade(&contract, start, end, number_of_ticks, trading_hours)
        .await?;

    println!("Time range: {} to {}", start_time, end_time);
    println!("\nTime                     | Price    | Size   | Exchange | Conditions");
    println!("-------------------------|----------|--------|----------|------------");

    let mut tick_count = 0;
    let mut total_volume = 0.0;
    let mut total_value = 0.0;
    let mut trades_by_exchange: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    // Process all historical ticks
    while let Some(tick) = tick_subscription.next().await {
        tick_count += 1;
        total_volume += tick.size as f64;
        total_value += tick.price * tick.size as f64;

        // Count trades by exchange
        *trades_by_exchange.entry(tick.exchange.clone()).or_insert(0) += 1;

        // Format timestamp
        let time_str = format!("{}", tick.timestamp);

        // Format trade conditions
        let conditions = if tick.special_conditions.is_empty() {
            "Regular".to_string()
        } else {
            tick.special_conditions.clone()
        };

        println!(
            "{} | ${:7.2} | {:6.0} | {:8} | {}",
            time_str, tick.price, tick.size, tick.exchange, conditions
        );

        // Show running statistics every 50 ticks
        if tick_count % 50 == 0 {
            println!(
                "\n--- After {} trades: Volume = {:.0}, VWAP = ${:.2} ---\n",
                tick_count,
                total_volume,
                total_value / total_volume
            );
        }
    }

    // Final statistics
    println!("\n========== Summary ==========");
    println!("Total trade ticks: {tick_count:?}");
    println!("Total volume: {:.0} shares", total_volume);
    println!("Total value: ${:.2}", total_value);
    if total_volume > 0.0 {
        println!("VWAP: ${:.2}", total_value / total_volume);
    }

    println!("\nTrades by exchange:");
    let mut exchanges: Vec<_> = trades_by_exchange.iter().collect();
    exchanges.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending
    for (exchange, count) in exchanges {
        let percentage = (*count as f64 / tick_count as f64) * 100.0;
        println!("  {:10} {:5} ({:5.1}%)", exchange, count, percentage);
    }

    // Example 2: Get most recent trades (no start time)
    println!("\n\nExample 2: Most recent {} trades", 20);
    let mut recent_trades = client.historical_ticks_trade(&contract, None, None, 20, TradingHours::Regular).await?;

    println!("\nTime                     | Price    | Size");
    println!("-------------------------|----------|------");

    let mut recent_count = 0;
    while let Some(tick) = recent_trades.next().await {
        recent_count += 1;
        let time_str = format!("{}", tick.timestamp);
        println!("{} | ${:7.2} | {:5.0}", time_str, tick.price, tick.size);
    }

    println!("\nRetrieved {} recent trades", recent_count);
    println!("\nExample completed!");
    Ok(())
}
