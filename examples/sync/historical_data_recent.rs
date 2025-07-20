//! Historical Data Recent example
//!
//! Gets recent historical data (last 7 days) for a given stock
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example historical_data_recent
//! ```

use clap::{arg, Command};

use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
use ibapi::Client;

fn main() {
    env_logger::init();

    let matches = Command::new("historical_data_recent")
        .about("Gets last 7 days of daily data for given stock")
        .arg(arg!(<STOCK_SYMBOL>).required(true))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("STOCK_SYMBOL").expect("stock symbol is required");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let contract = Contract::stock(stock_symbol);

    let historical_data = client
        .historical_data(&contract, None, 7.days(), BarSize::Day, WhatToShow::Trades, true)
        .expect("historical data request failed");

    println!("start_date: {}, end_date: {}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
