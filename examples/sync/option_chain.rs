//! Option Chain example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example option_chain
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::SecurityType;

// This example demonstrates requesting option chain data from the TWS.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // One OptionChain per exchange listing AAPL options. The contract id is the
    // underlying's and is required; `.exchange(..)` exists for futures options only.
    let subscription = client
        .option_chain("AAPL", SecurityType::Stock, 265598)
        .subscribe()
        .expect("request option chain failed!");

    for option_chain in &subscription {
        println!("{option_chain:?}")
    }
}
