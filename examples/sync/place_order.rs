//! Place Order example
//!
//! Submits a single market order driven by CLI args, using the fluent builder.
//!
//! For status / execution monitoring, see `examples/sync/submit_order.rs`, which
//! sets up `client.order_update_stream()` in a background thread.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example place_order -- --stock AAPL --buy 100
//! ```

use clap::{arg, ArgMatches, Command};
use ibapi::client::blocking::Client;
use ibapi::contracts::Currency;
use ibapi::prelude::*;
use log::{debug, info};

fn main() {
    env_logger::init();

    let matches = Command::new("place_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Submits order to broker")
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .arg(arg!(--stock <SYMBOL>).required(true))
        .arg(arg!(--buy <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--sell <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("stock").expect("stock symbol is required");

    let (action, quantity) = parse_action(&matches).expect("specify --buy <QTY> or --sell <QTY>");
    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let client = Client::connect(connection_string, 100).expect("connection failed");
    info!("Connected {client:?}");

    let mut contract = Contract::stock(stock_symbol.as_str()).build();
    contract.currency = Currency::from("USD");
    debug!("contract template {contract:?}");

    let order_id = match action {
        Action::Buy => client.order(&contract).buy(quantity).market().submit(),
        Action::Sell => client.order(&contract).sell(quantity).market().submit(),
        _ => unreachable!("CLI surface only emits Buy or Sell"),
    }
    .expect("could not place order");
    println!("Submitted order: {order_id}");
}

fn parse_action(matches: &ArgMatches) -> Option<(Action, i32)> {
    if let Some(quantity) = matches.get_one::<i32>("buy") {
        Some((Action::Buy, *quantity))
    } else {
        matches.get_one::<i32>("sell").map(|q| (Action::Sell, *q))
    }
}
