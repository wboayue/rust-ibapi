#![allow(clippy::uninlined_format_args)]
//! Test multiple async calls example
//!
//! This example tests making multiple sequential async calls to verify
//! that channel cleanup and request handling work correctly.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features async --example async_test_multiple_calls
//! ```

use ibapi::contracts::Contract;
use ibapi::market_data::historical::WhatToShow;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 105).await?;
    println!("Connected to IB Gateway");

    let contract = Contract::stock("AAPL");

    // Make 3 sequential calls to head_timestamp
    for i in 1..=3 {
        println!("Making call {i:?}");
        match client.head_timestamp(&contract, WhatToShow::Trades, true).await {
            Ok(timestamp) => {
                println!("Call {i} - Success: {timestamp}");
            }
            Err(e) => {
                println!("Call {i} - Error: {e}");
            }
        }

        // Small delay between calls
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("All calls completed!");
    Ok(())
}
