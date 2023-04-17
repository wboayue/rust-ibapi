use clap::{arg, Command};
use time::macros::{datetime, format_description};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use time_tz;

use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
use ibapi::Client;

fn main() {
    env_logger::init();

    let matches = Command::new("historical_ticks_mid_point")
        .about("Get historical ticks for the specified stock")
        .arg(
            arg!(<INTERVAL_START>)
                .required(true)
                .help("time at start of interval for tick. e.g. 20240415T12:00:00"),
        )
        .arg(arg!(<STOCK_SYMBOL>).required(true).help("stock symbol to retrieve data for"))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .arg(arg!(--number_of_ticks <VALUE>).default_value("100"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let interval_raw = matches.get_one::<String>("INTERVAL_START").expect("interval start is required");
    let stock_symbol = matches.get_one::<String>("STOCK_SYMBOL").expect("stock symbol is required");
    let number_of_ticks = 10;

    let local_offset = UtcOffset::local_offset_at(OffsetDateTime::now_utc()).unwrap();

    let interval_format = format_description!("[year][month][day]T[hour]:[minute]:[second]");
    let interval_start = PrimitiveDateTime::parse(interval_raw, interval_format)
        .unwrap()
        .assume_offset(local_offset);

    println!("start: #{interval_start}");

    let client = Client::connect(&connection_string, 100).expect("connection failed");

    let contract = Contract::stock(stock_symbol);

    let ticks = client
        .historical_ticks_mid_point(&contract, Some(interval_start), None, number_of_ticks, true)
        .expect("historical data request failed");

    for tick in ticks {
        println!("{tick:?}");
    }
}
