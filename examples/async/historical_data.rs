#![allow(clippy::uninlined_format_args)]
#![allow(clippy::format_in_format_args)]
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
    market_data::TradingHours,
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
    let head_timestamp = client.head_timestamp(&contract, WhatToShow::Trades, TradingHours::Regular).await?;
    println!("Earliest available historical data: {head_timestamp:?}");

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
            TradingHours::Regular,    // Use regular trading hours
        )
        .await?;

    println!("Period: {} to {}", historical_data.start, historical_data.end);
    println!("Total bars: {}", historical_data.bars.len());

    // Show first 5 and last 5 bars
    for (i, bar) in historical_data.bars.iter().take(5).enumerate() {
        println!(
            "Bar {}: {} - O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}",
            i + 1,
            format!("{:02}:{:02}", bar.date.hour(), bar.date.minute()),
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
                format!("{:02}:{:02}", bar.date.hour(), bar.date.minute()),
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
            TradingHours::Regular,    // Use regular trading hours
        )
        .await?;

    println!("Daily bars received: {}", daily_data.bars.len());
    for bar in daily_data.bars.iter().take(5) {
        println!(
            "{}: O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}K",
            format!("{:04}-{:02}-{:02}", bar.date.year(), bar.date.month() as u8, bar.date.day()),
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume / 1000.0
        );
    }

    // Example 4: Get different data types
    println!("\n=== Different Data Types ===");

    // Bid data (last 1 day)
    let bid_data = client
        .historical_data(
            &contract,
            Some(end_date),
            1.days(),
            BarSize::Min,
            Some(WhatToShow::Bid),
            TradingHours::Regular,
        )
        .await?;
    println!("Bid bars (1-min): {} bars", bid_data.bars.len());
    if let Some(bar) = bid_data.bars.first() {
        println!(
            "  First bar: {:02}:{:02}:{:02} - Bid: ${:.2}",
            bar.date.hour(),
            bar.date.minute(),
            bar.date.second(),
            bar.close
        );
    }

    // Ask data (last 1 day)
    let ask_data = client
        .historical_data(
            &contract,
            Some(end_date),
            1.days(),
            BarSize::Min,
            Some(WhatToShow::Ask),
            TradingHours::Regular,
        )
        .await?;
    println!("Ask bars (1-min): {} bars", ask_data.bars.len());
    if let Some(bar) = ask_data.bars.first() {
        println!(
            "  First bar: {:02}:{:02}:{:02} - Ask: ${:.2}",
            bar.date.hour(),
            bar.date.minute(),
            bar.date.second(),
            bar.close
        );
    }

    // Example 5: Get histogram data
    println!("\n=== Histogram Data ===");
    let histogram = client.histogram_data(&contract, TradingHours::Regular, BarSize::Day).await?;

    println!("Histogram entries: {}", histogram.len());
    for entry in histogram.iter().take(5) {
        println!("  Price: ${:.2}, Size: {}", entry.price, entry.size);
    }

    println!("\nHistorical data example completed!");
    Ok(())
}
