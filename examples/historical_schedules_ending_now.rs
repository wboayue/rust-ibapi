use clap::{arg, Command};

use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
use ibapi::Client;

fn main() {
    env_logger::init();

    let matches = Command::new("historical_data")
        .about("Get last 30 days of daily data for given stock")
        .arg(arg!(<STOCK_SYMBOL>).required(true))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("STOCK_SYMBOL").expect("stock symbol is required");

    let client = Client::connect(&connection_string, 100).expect("connection failed");

    let contract = Contract::stock(stock_symbol);

    let historical_data = client
        .historical_schedules_ending_now(&contract, 30.days())
        .expect("historical schedule request failed");

    println!("{historical_data:?}");

    // for bar in bars {
    //     println!("{bar:?}");
    // }
}