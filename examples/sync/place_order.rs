//! Place Order example
//!
//! Submits a market order using the fluent builder and monitors order updates
//! through `order_update_stream`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example place_order -- --stock AAPL --buy 100
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use clap::{arg, ArgMatches, Command};
use ibapi::client::blocking::Client;
use ibapi::contracts::Currency;
use ibapi::prelude::*;
use log::{debug, info};

enum Side {
    Buy,
    Sell,
}

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

    let (side, quantity) = parse_side(&matches).expect("specify --buy <QTY> or --sell <QTY>");
    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let client = Arc::new(Client::connect(connection_string, 100).expect("connection failed"));
    info!("Connected {client:?}");

    // Run a background monitor for all order updates before submitting.
    let monitor_client = client.clone();
    let _monitor = thread::spawn(move || drain_order_updates(&monitor_client));
    thread::sleep(Duration::from_millis(100));

    let mut contract = Contract::stock(stock_symbol.as_str()).build();
    contract.currency = Currency::from("USD");
    debug!("contract template {contract:?}");

    // Fluent submit: side dispatches to .buy() or .sell(); .submit() allocates the id internally.
    let order_id = match side {
        Side::Buy => client.order(&contract).buy(quantity).market().submit(),
        Side::Sell => client.order(&contract).sell(quantity).market().submit(),
    }
    .expect("could not place order");
    println!("Submitted order: {order_id}");

    // Wait for status / executions / commission to flow through the monitor.
    thread::sleep(Duration::from_secs(10));
}

fn drain_order_updates(client: &Client) {
    let stream = match client.order_update_stream() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to open order update stream: {e}");
            return;
        }
    };
    for update in stream.iter_data() {
        match update {
            Ok(update) => match update {
                OrderUpdate::OrderStatus(s) => println!("order status: {s:?}"),
                OrderUpdate::OpenOrder(o) => println!("open order: {o:?}"),
                OrderUpdate::ExecutionData(e) => println!("execution: {e:?}"),
                OrderUpdate::CommissionReport(r) => println!("commission report: {r:?}"),
            },
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }
}

fn parse_side(matches: &ArgMatches) -> Option<(Side, i32)> {
    if let Some(quantity) = matches.get_one::<i32>("buy") {
        Some((Side::Buy, *quantity))
    } else {
        matches.get_one::<i32>("sell").map(|q| (Side::Sell, *q))
    }
}
