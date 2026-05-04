//! Market Depth example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_depth
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

// This example demonstrates how to request market depth data.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL").build();

    let subscription = client.market_depth(&contract, 5, true).expect("error requesting market depth");
    for row in subscription.iter_data() {
        match row {
            Ok(row) => println!("row: {row:?}"),
            Err(error) => {
                println!("error: {error:?}");
                break;
            }
        }
    }
}
