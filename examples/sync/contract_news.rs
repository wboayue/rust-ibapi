//! Contract News example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example contract_news
//! ```

use ibapi::{contracts::Contract, Client};

// This example demonstrates how live news for a contract can be requested.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL");
    let provider_codes = ["DJ-N"];

    let subscription = client.contract_news(&contract, &provider_codes).expect("request contract news failed");
    for article in subscription {
        println!("{article:?}");
    }
}
