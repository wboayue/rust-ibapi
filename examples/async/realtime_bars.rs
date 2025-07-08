//! Async real-time bars example
//!
//! This example demonstrates how to subscribe to 5-second real-time bars using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_realtime_bars
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)
//! - Modify WhatToShow to get different data (Trades, Bid, Ask, MidPoint)

use std::sync::Arc;

use futures::StreamExt;
use ibapi::{
    contracts::Contract,
    market_data::realtime::{BarSize, WhatToShow},
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Subscribing to real-time bars for {}", contract.symbol);

    // Request real-time bars
    // Note: Only 5-second bars are currently supported
    let realtime_bars = client
        .realtime_bars(
            &contract,
            BarSize::Sec5,      // 5-second bars
            WhatToShow::Trades, // Trade data
            true,               // Use regular trading hours
        )
        .await?;
    println!("Real-time bars subscription created");
    println!("Receiving 5-second bars...\n");

    // Process real-time bars stream
    let mut stream = realtime_bars;
    let mut bar_count = 0;

    while let Some(bar) = stream.next().await {
        let bar = bar?;
        bar_count += 1;

        println!("Bar #{} at {}", bar_count, bar.date);
        println!("  Open:   ${:.2}", bar.open);
        println!("  High:   ${:.2}", bar.high);
        println!("  Low:    ${:.2}", bar.low);
        println!("  Close:  ${:.2}", bar.close);
        println!("  Volume: {:.0}", bar.volume);
        println!("  VWAP:   ${:.2}", bar.wap);
        println!("  Trades: {}", bar.count);
        println!();

        // Stop after 10 bars for demo
        if bar_count >= 10 {
            println!("Received {} bars. Stopping example.", bar_count);
            break;
        }
    }

    Ok(())
}
