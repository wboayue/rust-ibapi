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
//! cargo run --example async_historical_data
//! cargo run --example async_historical_data -- --asset forex
//! cargo run --example async_historical_data -- --asset futures
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Use --asset to select: stock (AAPL), forex (EUR.USD), or futures (ES)
//! - Modify duration and bar size to get different data periods

use std::sync::Arc;

use clap::{Parser, ValueEnum};
use ibapi::contracts::SecurityType;
use ibapi::market_data::historical::HistoricalBarUpdate;
use ibapi::prelude::*;
use time::OffsetDateTime;

#[derive(Parser)]
#[command(name = "historical_data")]
#[command(about = "Fetch historical bar data from IB")]
struct Args {
    /// Asset type to use
    #[arg(long, value_enum, default_value = "stock")]
    asset: AssetType,

    /// Skip to streaming test only
    #[arg(long, short = 's')]
    streaming_only: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum AssetType {
    Stock,
    Forex,
    Futures,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create contract based on asset type
    let contract = match args.asset {
        AssetType::Stock => Contract::stock("AAPL").build(),
        AssetType::Forex => Contract::forex("EUR", "USD").build(),
        AssetType::Futures => {
            // For futures, we need to resolve the front-month contract via contract_details
            println!("Resolving front-month contract for ES...");
            let query = Contract {
                symbol: "ES".into(),
                security_type: SecurityType::Future,
                exchange: "CME".into(),
                currency: "USD".into(),
                ..Default::default()
            };

            let details = client.contract_details(&query).await?;
            if details.is_empty() {
                return Err("No futures contracts found for ES".into());
            }

            // Sort by contract month and take front-month
            let mut sorted: Vec<_> = details
                .into_iter()
                .filter(|d| !d.contract.last_trade_date_or_contract_month.is_empty())
                .collect();
            sorted.sort_by(|a, b| {
                a.contract
                    .last_trade_date_or_contract_month
                    .cmp(&b.contract.last_trade_date_or_contract_month)
            });

            let front = sorted.into_iter().next().expect("No valid contracts");
            println!(
                "  Found front-month: local_symbol='{}', contract_month='{}'",
                front.contract.local_symbol, front.contract.last_trade_date_or_contract_month
            );
            front.contract
        }
    };
    println!("Requesting historical data for {} ({:?})", contract.symbol, args.asset);
    println!(
        "  local_symbol: '{}', exchange: {}, contract_month: '{}'\n",
        contract.local_symbol, contract.exchange, contract.last_trade_date_or_contract_month
    );

    if !args.streaming_only {
        // Example 1: Get the earliest available data timestamp
        println!("=== Head Timestamp ===");
        let head_timestamp = client
            .head_timestamp(&contract, HistoricalWhatToShow::Trades, TradingHours::Regular)
            .await?;
        println!("Earliest available historical data: {head_timestamp:?}");

        // Example 2: Get recent intraday data (5-minute bars for last day)
        println!("\n=== Recent Intraday Data (5-min bars) ===");
        let end_date = OffsetDateTime::now_utc();
        let historical_data = client
            .historical_data(
                &contract,
                Some(end_date),
                1.days(),                           // Duration: 1 day
                HistoricalBarSize::Min5,            // 5-minute bars
                Some(HistoricalWhatToShow::Trades), // Trade data
                TradingHours::Regular,              // Use regular trading hours
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
                1.months(),                         // Duration: 1 month
                HistoricalBarSize::Day,             // Daily bars
                Some(HistoricalWhatToShow::Trades), // Trade data
                TradingHours::Regular,              // Use regular trading hours
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
                HistoricalBarSize::Min,
                Some(HistoricalWhatToShow::Bid),
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
                HistoricalBarSize::Min,
                Some(HistoricalWhatToShow::Ask),
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
        let histogram = client.histogram_data(&contract, TradingHours::Regular, HistoricalBarSize::Day).await?;

        println!("Histogram entries: {}", histogram.len());
        for entry in histogram.iter().take(5) {
            println!("  Price: ${:.2}, Size: {}", entry.price, entry.size);
        }
    }

    // Example 6: Streaming historical data with keepUpToDate=true
    println!("\n=== Streaming Historical Data (keepUpToDate=true) ===");
    println!("Press Ctrl+C to stop streaming...\n");

    // Use appropriate data type per asset
    let what_to_show = match args.asset {
        AssetType::Forex => HistoricalWhatToShow::MidPoint,
        _ => HistoricalWhatToShow::Trades,
    };

    let mut subscription = client
        .historical_data_streaming(
            &contract,
            1.days(),               // Duration: 1 day of history
            HistoricalBarSize::Min, // 1-minute bars
            Some(what_to_show),
            TradingHours::Extended,
            true, // keep_up_to_date: stream live updates
        )
        .await?;

    while let Some(update) = subscription.next().await {
        match update {
            HistoricalBarUpdate::Historical(data) => {
                println!("Received {} initial historical bars", data.bars.len());
                if let Some(bar) = data.bars.last() {
                    println!(
                        "  Latest: {} - O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}",
                        format!("{:02}:{:02}", bar.date.hour(), bar.date.minute()),
                        bar.open,
                        bar.high,
                        bar.low,
                        bar.close
                    );
                }
            }
            HistoricalBarUpdate::HistoricalEnd => {
                println!("Initial historical data complete. Now streaming updates...");
            }
            HistoricalBarUpdate::Update(bar) => {
                println!(
                    "UPDATE: {} - O: ${:.2}, H: ${:.2}, L: ${:.2}, C: ${:.2}, V: {:.0}",
                    format!("{:02}:{:02}:{:02}", bar.date.hour(), bar.date.minute(), bar.date.second()),
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume
                );
            }
        }
    }

    println!("Stream ended");

    println!("\nHistorical data example completed!");
    Ok(())
}
