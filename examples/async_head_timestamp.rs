//! Async head timestamp example
//!
//! This example demonstrates how to get the earliest available historical data timestamp
//! for a contract using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_head_timestamp
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the contract and data type as desired

use std::sync::Arc;

use ibapi::{
    contracts::{Contract, SecurityType},
    market_data::historical::WhatToShow,
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Test different contracts and data types
    let contracts = vec![("AAPL", "Apple Inc."), ("TSLA", "Tesla Inc."), ("EUR", "Euro FX")];

    let data_types = vec![
        (WhatToShow::Trades, "Trades"),
        (WhatToShow::Bid, "Bid"),
        (WhatToShow::Ask, "Ask"),
        (WhatToShow::MidPoint, "MidPoint"),
    ];

    // For each contract, check earliest data for different types
    for (symbol, name) in contracts {
        println!("\n{} ({}):", symbol, name);

        let contract = if symbol == "EUR" {
            let mut forex = Contract::default();
            forex.symbol = "EUR".to_string();
            forex.security_type = SecurityType::ForexPair;
            forex.currency = "USD".to_string();
            forex.exchange = "IDEALPRO".to_string();
            forex
        } else {
            Contract::stock(symbol)
        };

        for (what_to_show, label) in &data_types {
            match client.head_timestamp(&contract, *what_to_show, true).await {
                Ok(timestamp) => {
                    println!("  {} - Earliest data: {}", label, timestamp);
                }
                Err(e) => {
                    println!("  {} - Error: {}", label, e);
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // Example with all available hours (not just RTH)
    println!("\nChecking non-RTH data for AAPL:");
    let contract = Contract::stock("AAPL");
    match client.head_timestamp(&contract, WhatToShow::Trades, false).await {
        Ok(timestamp) => {
            println!("  All hours - Earliest data: {}", timestamp);
        }
        Err(e) => {
            println!("  All hours - Error: {}", e);
        }
    }

    println!("\nExample completed!");
    Ok(())
}
