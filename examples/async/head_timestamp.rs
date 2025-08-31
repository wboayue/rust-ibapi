#![allow(clippy::uninlined_format_args)]
//! Async head timestamp example
//!
//! This example demonstrates how to get the earliest available historical data timestamp
//! for a contract using the async API.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features async --example async_head_timestamp AAPL
//! ```
//!
//! # Arguments
//!
//! - `SYMBOL` - The stock symbol to query (e.g., AAPL, MSFT)
//! - `--connection_string` - IB Gateway/TWS connection string (default: 127.0.0.1:4002)

use clap::{arg, Command};

use ibapi::contracts::Contract;
use ibapi::market_data::historical::WhatToShow;
use ibapi::market_data::TradingHours;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = Command::new("async_head_timestamp")
        .arg(arg!(<SYMBOL>).required(true))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("SYMBOL").expect("stock symbol is required");

    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let client = Client::connect(connection_string, 100).await?;

    let contract = Contract::stock(stock_symbol);
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let head_timestamp = client.head_timestamp(&contract, what_to_show, trading_hours).await?;

    println!("head_timestamp: {head_timestamp}");
    Ok(())
}
