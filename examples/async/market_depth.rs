#![allow(clippy::uninlined_format_args)]
//! Async market depth (Level II) example
//!
//! This example demonstrates how to subscribe to market depth data using the async API.
//! Market depth shows the order book with multiple price levels.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_market_depth
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the stock symbol if desired (default: AAPL)
//! - Modify number_of_rows to see more/fewer price levels (default: 5)

use std::sync::Arc;

use ibapi::{contracts::Contract, market_data::realtime::MarketDepths, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Create a stock contract
    let contract = Contract::stock("AAPL");
    println!("Subscribing to market depth for {}", contract.symbol);

    // First, get available market depth exchanges
    println!("\nAvailable market depth exchanges:");
    let exchanges = client.market_depth_exchanges().await?;
    for exchange in exchanges.iter().take(5) {
        println!(
            "  {} - {} ({})",
            exchange.exchange_name, exchange.security_type, exchange.service_data_type
        );
    }

    // Request market depth
    let market_depth = client
        .market_depth(
            &contract, 5,     // Number of rows (price levels)
            false, // Not smart depth
        )
        .await?;
    println!("\nMarket depth subscription created");
    println!("Showing order book updates...\n");

    // Track the order book
    let mut bid_book = [None; 5];
    let mut ask_book = [None; 5];

    // Process market depth stream
    let mut market_depth = market_depth;
    let mut update_count = 0;

    while let Some(depth_update) = market_depth.next().await {
        update_count += 1;
        if update_count > 30 {
            break;
        } // Take first 30 updates for demo

        match depth_update? {
            MarketDepths::MarketDepth(depth) => {
                let side = if depth.side == 1 { "Bid" } else { "Ask" };
                let operation = match depth.operation {
                    0 => "Insert",
                    1 => "Update",
                    2 => "Delete",
                    _ => "Unknown",
                };

                println!("Update #{}: {} {} at position {}", update_count, operation, side, depth.position);

                // Update our order book
                if depth.side == 1 {
                    // Bid
                    match depth.operation {
                        0 | 1 => {
                            // Insert or Update
                            if depth.position < bid_book.len() as i32 {
                                bid_book[depth.position as usize] = Some((depth.price, depth.size));
                            }
                        }
                        2 => {
                            // Delete
                            if depth.position < bid_book.len() as i32 {
                                bid_book[depth.position as usize] = None;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Ask
                    match depth.operation {
                        0 | 1 => {
                            // Insert or Update
                            if depth.position < ask_book.len() as i32 {
                                ask_book[depth.position as usize] = Some((depth.price, depth.size));
                            }
                        }
                        2 => {
                            // Delete
                            if depth.position < ask_book.len() as i32 {
                                ask_book[depth.position as usize] = None;
                            }
                        }
                        _ => {}
                    }
                }

                // Display current order book
                println!("\nOrder Book:");
                println!("  {:>10} {:>10} | {:>10} {:>10}", "Bid Size", "Bid", "Ask", "Ask Size");
                println!("  {:-<10} {:-<10} | {:-<10} {:-<10}", "", "", "", "");
                for i in 0..5 {
                    let bid = bid_book.get(i).and_then(|&x| x);
                    let ask = ask_book.get(i).and_then(|&x| x);

                    match (bid, ask) {
                        (Some((bid_price, bid_size)), Some((ask_price, ask_size))) => {
                            println!("  {bid_size:>10.0} {bid_price:>10.2} | {ask_price:>10.2} {ask_size:>10.0}");
                        }
                        (Some((bid_price, bid_size)), None) => {
                            println!("  {:>10.0} {:>10.2} | {:>10} {:>10}", bid_size, bid_price, "", "");
                        }
                        (None, Some((ask_price, ask_size))) => {
                            println!("  {:>10} {:>10} | {:>10.2} {:>10.0}", "", "", ask_price, ask_size);
                        }
                        (None, None) => {
                            println!("  {:>10} {:>10} | {:>10} {:>10}", "", "", "", "");
                        }
                    }
                }
                println!();
            }
            MarketDepths::MarketDepthL2(depth) => {
                println!(
                    "L2 Update: {} {} at {} - ${:.2} x {:.0}",
                    depth.market_maker,
                    if depth.side == 1 { "Bid" } else { "Ask" },
                    depth.position,
                    depth.price,
                    depth.size
                );
            }
            MarketDepths::Notice(notice) => {
                println!("Notice ({}): {}", notice.code, notice.message);
            }
        }
    }

    println!("\nReceived {update_count} updates. Example completed!");
    Ok(())
}
