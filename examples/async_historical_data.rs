//! Async historical data example
//!
//! This example demonstrates how to request historical bar data using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_historical_data
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)
//! - Modify duration and bar size to get different data periods

use std::sync::Arc;

use ibapi::{
    contracts::Contract,
    market_data::historical::{BarSize, ToDuration, WhatToShow},
    Client,
};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Requesting historical data for {}\n", contract.symbol);

    // Example 1: Get the earliest available data timestamp
    println!("=== Head Timestamp ===");
    let head_timestamp = client.head_timestamp(&contract, WhatToShow::Trades, true).await?;
    println!(
        "Earliest available historical data: {}",
        head_timestamp.format("%Y-%m-%d %H:%M:%S").unwrap()
    );

    // Example 2: Get recent intraday data (5-minute bars for last day)
    println!("\n=== Recent Intraday Data (5-min bars) ===");
    let end_date = OffsetDateTime::now_utc();
    let historical_data = client
        .historical_data(
            &contract,
            Some(end_date),
            1.days(),                 // Duration: 1 day
            BarSize::Min5,            // 5-minute bars
            Some(WhatToShow::Trades), // Trade data
            true,                     // Use regular trading hours
        )
        .await?;

    println!(
        "Period: {} to {}",
        historical_data.start.format("%Y-%m-%d %H:%M:%S").unwrap(),
        historical_data.end.format("%Y-%m-%d %H:%M:%S").unwrap()
    );
    println!("Total bars: {}", historical_data.bars.len());

    // Show first 5 and last 5 bars
    for (i, bar) in historical_data.bars.iter().take(5).enumerate() {
        println!(
            "Bar {}: {} - O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}",
            i + 1,
            bar.date.format("%H:%M").unwrap(),
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume
        );
    }
    if historical_data.bars.len() > 10 {
        println!("...");
        let start_idx = historical_data.bars.len() - 5;
        for (i, bar) in historical_data.bars.iter().skip(start_idx).enumerate() {
            println!(
                "Bar {}: {} - O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}",
                start_idx + i + 1,
                bar.date.format("%H:%M").unwrap(),
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume
            );
        }
    }

    // Example 3: Get daily data for past month
    println!("\n=== Daily Data (past month) ===");
    let daily_data = client
        .historical_data(
            &contract,
            Some(end_date),
            1.months(),               // Duration: 1 month
            BarSize::Day,             // Daily bars
            Some(WhatToShow::Trades), // Trade data
            true,                     // Use regular trading hours
        )
        .await?;

    println!("Daily bars received: {}", daily_data.bars.len());
    for bar in daily_data.bars.iter().take(5) {
        println!(
            "{}: O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}K",
            bar.date.format("%Y-%m-%d").unwrap(),
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume / 1000.0
        );
    }

    // Example 4: Get different data types
    println!("\n=== Different Data Types ===");

    // Bid data
    let bid_data = client
        .historical_data(&contract, Some(end_date), 2.hours(), BarSize::Min, Some(WhatToShow::Bid), true)
        .await?;
    println!("Bid bars (1-min): {} bars", bid_data.bars.len());
    if let Some(bar) = bid_data.bars.first() {
        println!("  First bar: {} - Bid: ${:.2}", bar.date.format("%H:%M:%S").unwrap(), bar.close);
    }

    // Ask data
    let ask_data = client
        .historical_data(&contract, Some(end_date), 2.hours(), BarSize::Min, Some(WhatToShow::Ask), true)
        .await?;
    println!("Ask bars (1-min): {} bars", ask_data.bars.len());
    if let Some(bar) = ask_data.bars.first() {
        println!("  First bar: {} - Ask: ${:.2}", bar.date.format("%H:%M:%S").unwrap(), bar.close);
    }

    // Example 5: Get histogram data
    println!("\n=== Histogram Data ===");
    let histogram = client.histogram_data(&contract, true, BarSize::Day).await?;

    println!("Histogram entries: {}", histogram.len());
    for entry in histogram.iter().take(5) {
        println!("  Price: ${:.2}, Size: {}", entry.price, entry.size);
    }

    println!("\nHistorical data example completed!");
    Ok(())
}
