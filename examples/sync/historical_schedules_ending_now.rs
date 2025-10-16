//! Historical Schedules Ending Now example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example historical_schedules_ending_now
//! ```

use clap::{arg, Command};

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::ToDuration;

fn main() {
    env_logger::init();

    let matches = Command::new("historical_schedules_ending_now")
        .about("Gets last 7 days of schedules for given stock")
        .arg(arg!(<STOCK_SYMBOL>).required(true))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("STOCK_SYMBOL").expect("stock symbol is required");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let contract = Contract::stock(stock_symbol.as_str()).build();

    let schedule = client
        .historical_schedules_ending_now(&contract, 7.days())
        .expect("historical schedule request failed");

    println!("start: {}, end: {}, time_zone: {}", schedule.start, schedule.end, schedule.time_zone);

    for session in &schedule.sessions {
        println!("{session:?}");
    }
}
