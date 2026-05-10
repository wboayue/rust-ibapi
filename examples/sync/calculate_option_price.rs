//! Calculate Option Price example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example calculate_option_price
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::call("AAPL").strike(240.0).expires_on(2025, 6, 20).build();

    let calculation = client.calculate_option_price(&contract, 100.0, 235.0).expect("request failed");
    println!("calculation: {calculation:?}");
}
