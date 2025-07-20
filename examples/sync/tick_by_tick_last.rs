//! Tick By Tick Last example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example tick_by_tick_last
//! ```

use std::time::Duration;

use ibapi::contracts::Contract;
use ibapi::Client;

// This example demonstrates how to stream tick by tick data for the last price of a contract.

fn main() {
    env_logger::init();

    let connection_string = "127.0.0.1:4002";
    println!("connecting to server @ {connection_string}");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let contract = Contract::stock("NVDA");
    let ticks = client.tick_by_tick_last(&contract, 0, false).expect("failed to get ticks");

    println!(
        "streaming last price for security_type: {:?}, symbol: {}",
        contract.security_type, contract.symbol
    );

    for (i, trade) in ticks.timeout_iter(Duration::from_secs(10)).enumerate() {
        println!("{}: {i:?} {trade:?}", contract.symbol);
    }

    // check for errors during streaming
    if let Some(error) = ticks.error() {
        println!("error: {error:?}");
    }
}
