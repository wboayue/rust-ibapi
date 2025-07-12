#![allow(clippy::uninlined_format_args)]
//! Async real-time market data example
//!
//! This example demonstrates how to subscribe to real-time market data using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_market_data
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)
//! - Modify generic tick list to receive different data types

use std::sync::Arc;

use ibapi::{contracts::Contract, market_data::realtime::TickTypes, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Subscribing to market data for {}", contract.symbol);

    // Request market data
    // Generic tick list:
    // - 233: RTVolume (last trade price, size, time, volume)
    // - 236: Shortable shares
    // - 258: Fundamental ratios
    let market_data = client.market_data(&contract, &["233", "236"], false, false).await?;
    println!("Market data subscription created");

    // Process market data stream
    let mut market_data = market_data;
    let mut tick_count = 0;

    while let Some(tick) = market_data.next().await {
        tick_count += 1;
        if tick_count > 20 {
            break;
        } // Take first 20 ticks for demo
        match tick? {
            TickTypes::Price(tick) => {
                println!("[{}] Price - {}: ${:.2}", tick_count, tick.tick_type, tick.price);
                if tick.attributes.can_auto_execute {
                    println!("  -> Can auto-execute");
                }
            }
            TickTypes::Size(tick) => {
                println!("[{}] Size - {}: {:.0}", tick_count, tick.tick_type, tick.size);
            }
            TickTypes::String(tick) => {
                println!("[{}] String - {}: {}", tick_count, tick.tick_type, tick.value);
            }
            TickTypes::Generic(tick) => {
                println!("[{}] Generic - {}: {:.2}", tick_count, tick.tick_type, tick.value);
            }
            TickTypes::OptionComputation(comp) => {
                println!(
                    "[{}] Option - IV: {:.2}%, Delta: {:.3}",
                    tick_count,
                    comp.implied_volatility.unwrap_or(0.0) * 100.0,
                    comp.delta.unwrap_or(0.0)
                );
            }
            TickTypes::SnapshotEnd => {
                println!("[{tick_count}] Snapshot completed");
                break;
            }
            TickTypes::Notice(notice) => {
                println!("[{}] Notice ({}): {}", tick_count, notice.code, notice.message);
            }
            _ => {}
        }
    }

    println!("\nReceived {tick_count} ticks. Example completed!");
    Ok(())
}
