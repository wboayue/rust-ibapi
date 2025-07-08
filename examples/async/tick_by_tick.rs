//! Async tick-by-tick data example
//!
//! This example demonstrates how to subscribe to tick-by-tick data using the async API.
//! It shows trades, bid/ask quotes, and midpoint ticks.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_tick_by_tick
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)

use std::sync::Arc;

use futures::StreamExt;
use ibapi::{contracts::Contract, Client};
use time::macros::format_description;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Subscribing to tick-by-tick data for {}\n", contract.symbol);

    // Example 1: All Last trades (includes all trades)
    println!("=== All Last Trades ===");
    let all_last_ticks = client.tick_by_tick_all_last(&contract, 0, false).await?;

    let mut stream = all_last_ticks.take(5);
    while let Some(trade) = stream.next().await {
        let trade = trade?;
        println!(
            "Trade at {}: ${:.2} x {} on {} [{}]",
            trade
                .time
                .format(format_description!("[hour]:[minute]:[second].[subsecond digits:3]"))
                .unwrap(),
            trade.price,
            trade.size,
            trade.exchange,
            trade.special_conditions
        );
        if trade.trade_attribute.past_limit {
            println!("  -> Past limit");
        }
        if trade.trade_attribute.unreported {
            println!("  -> Unreported");
        }
    }

    // Example 2: Bid/Ask quotes
    println!("\n=== Bid/Ask Quotes ===");
    let bid_ask_ticks = client.tick_by_tick_bid_ask(&contract, 0, false).await?;

    let mut stream = bid_ask_ticks.take(5);
    while let Some(quote) = stream.next().await {
        let quote = quote?;
        println!(
            "Quote at {}: Bid ${:.2} x {} | Ask ${:.2} x {}",
            quote
                .time
                .format(format_description!("[hour]:[minute]:[second].[subsecond digits:3]"))
                .unwrap(),
            quote.bid_price,
            quote.bid_size,
            quote.ask_price,
            quote.ask_size
        );
        if quote.bid_ask_attribute.bid_past_low {
            println!("  -> Bid past low");
        }
        if quote.bid_ask_attribute.ask_past_high {
            println!("  -> Ask past high");
        }
    }

    // Example 3: Midpoint ticks
    println!("\n=== Midpoint Ticks ===");
    let midpoint_ticks = client.tick_by_tick_midpoint(&contract, 0, false).await?;

    let mut stream = midpoint_ticks.take(5);
    while let Some(midpoint) = stream.next().await {
        let midpoint = midpoint?;
        println!(
            "Midpoint at {}: ${:.2}",
            midpoint
                .time
                .format(format_description!("[hour]:[minute]:[second].[subsecond digits:3]"))
                .unwrap(),
            midpoint.mid_point
        );
    }

    println!("\nTick-by-tick example completed!");
    Ok(())
}
