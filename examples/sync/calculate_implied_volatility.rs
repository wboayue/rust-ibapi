//! Calculate Implied Volatility example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example calculate_implied_volatility
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::option("AAPL", "20250620", 240.0, "C");

    let calculation = client.calculate_implied_volatility(&contract, 25.0, 235.0).expect("request failed");
    println!("calculation: {calculation:?}");
}
