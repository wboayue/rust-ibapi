//! Historical Ticks Trade example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example historical_ticks_trade
//! ```

use clap::{arg, Command};
use ibapi::client::blocking::Client;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};

use ibapi::contracts::Contract;
use ibapi::market_data::TradingHours;

fn main() {
    env_logger::init();

    let matches = Command::new("historical_ticks_last")
        .about("Get historical ticks for the specified stock")
        .arg(
            arg!(<INTERVAL_START>)
                .required(true)
                .help("time at start of interval for tick. e.g. 20230415T12:00:00Z"),
        )
        .arg(arg!(<STOCK_SYMBOL>).required(true).help("stock symbol to retrieve data for"))
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .arg(
            arg!(--number_of_ticks <VALUE>)
                .default_value("100")
                .value_parser(clap::value_parser!(i32)),
        )
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let interval_raw = matches.get_one::<String>("INTERVAL_START").expect("interval start is required");
    let stock_symbol = matches.get_one::<String>("STOCK_SYMBOL").expect("stock symbol is required");
    let number_of_ticks = matches.get_one::<i32>("number_of_ticks").expect("number of ticks required");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let interval_start = parse_interval(interval_raw);
    let contract = Contract::stock(stock_symbol.as_str()).build();

    let ticks = client
        .historical_ticks_trade(&contract, Some(interval_start), None, *number_of_ticks, TradingHours::Regular)
        .expect("historical data request failed");

    for tick in ticks {
        println!("{tick:?}");
    }
}

const INTERVAL_FORMAT: &[FormatItem] = format_description!("[year][month][day]T[hour]:[minute]:[second]Z");

fn parse_interval(text: &str) -> OffsetDateTime {
    PrimitiveDateTime::parse(text, INTERVAL_FORMAT)
        .expect("expected date in format 20230415T12:00:00Z")
        .assume_utc()
}
