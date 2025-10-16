//! Head Timestamp example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example head_timestamp
//! ```

use clap::{arg, Command};

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::WhatToShow;
use ibapi::market_data::TradingHours;

fn main() {
    env_logger::init();

    let matches = Command::new("head_timestamp")
        .arg(arg!(<SYMBOL>).required(true))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("SYMBOL").expect("stock symbol is required");

    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let contract = Contract::stock(stock_symbol.as_str()).build();
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let head_timestamp = client
        .head_timestamp(&contract, what_to_show, trading_hours)
        .expect("head timestamp failed");

    println!("head_timestamp: {head_timestamp}");
}
