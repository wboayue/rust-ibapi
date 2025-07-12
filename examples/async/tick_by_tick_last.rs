//! Async tick-by-tick Last trades example
//!
//! This example demonstrates how to receive tick-by-tick Last trade data (trades only)
//! using the async API. This differs from AllLast which includes both trades and quotes.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_tick_by_tick_last
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: SPY)

use std::sync::Arc;

use futures::StreamExt;
use ibapi::{contracts::Contract, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a liquid stock contract
    let contract = Contract::stock("SPY");
    println!("\nSubscribing to tick-by-tick Last trades for {}", contract.symbol);

    // Request tick-by-tick Last trades
    // Parameters:
    // - contract: The contract to get data for
    // - number_of_ticks: 0 for streaming data, or 1-1000 for historical ticks
    // - ignore_size: false to include size information
    let mut trades = client.tick_by_tick_last(&contract, 0, false).await?;

    println!("Subscription created. Receiving Last trades only...\n");
    println!("Time                     | Price    | Size   | Exchange | Conditions");
    println!("-------------------------|----------|--------|----------|------------");

    // Process the trade stream
    let mut trade_count = 0;
    let mut total_volume = 0.0;
    let mut total_value = 0.0;

    // Process first 20 trades for demo
    let mut trades = trades.take(20);
    while let Some(trade_result) = trades.next().await {
        match trade_result {
            Ok(trade) => {
                trade_count += 1;
                total_volume += trade.size;
                total_value += trade.price * trade.size;

                // Format timestamp
                let time_str = format!("{}", trade.time);

                // Format trade conditions
                let conditions = if trade.special_conditions.is_empty() {
                    "Regular".to_string()
                } else {
                    trade.special_conditions.clone()
                };

                println!(
                    "{} | ${:7.2} | {:6.0} | {:8} | {}",
                    time_str, trade.price, trade.size, trade.exchange, conditions
                );

                // Show running statistics every 5 trades
                if trade_count % 5 == 0 {
                    let vwap = if total_volume > 0.0 { total_value / total_volume } else { 0.0 };
                    println!(
                        "\n--- Stats: {} trades, Volume: {:.0}, VWAP: ${:.2} ---\n",
                        trade_count, total_volume, vwap
                    );
                }
            }
            Err(e) => {
                eprintln!("Error receiving trade: {e:?}");
                break;
            }
        }
    }

    // Final statistics
    println!("\n========== Summary ==========");
    println!("Total trades received: {trade_count:?}");
    println!("Total volume: {:.0} shares", total_volume);
    println!("Total value: ${:.2}", total_value);
    if total_volume > 0.0 {
        println!("VWAP: ${:.2}", total_value / total_volume);
    }

    println!("\nExample completed!");
    Ok(())
}
