//! Histogram Data example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example histogram_data
//! ```

use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("GM");

    let histogram = client
        .histogram_data(&contract, TradingHours::Regular, HistoricalBarSize::Week)
        .expect("histogram request failed");

    for item in &histogram {
        println!("{item:?}");
    }
}
