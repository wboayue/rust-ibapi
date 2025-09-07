//! Market Depth example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_depth
//! ```

use ibapi::contracts::Contract;
use ibapi::Client;

// This example demonstrates how to request market depth data.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL").build();

    let subscription = client.market_depth(&contract, 5, true).expect("error requesting market depth");
    for row in &subscription {
        println!("row: {row:?}")
    }

    if let Some(error) = subscription.error() {
        println!("error: {error:?}");
    }
}
