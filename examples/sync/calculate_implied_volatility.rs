//! Calculate Implied Volatility example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example calculate_implied_volatility
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, ExpirationDate, OptionRight};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::option("AAPL", ExpirationDate::new(2025, 6, 20), 240.0, OptionRight::Call);

    let calculation = client.calculate_implied_volatility(&contract, 25.0, 235.0).expect("request failed");
    println!("calculation: {calculation:?}");
}
