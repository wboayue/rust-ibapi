//! Market Depth Exchanges example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_depth_exchanges
//! ```

use ibapi::client::blocking::Client;

// This example demonstrates how to request market depth exchanges.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let exchanges = client.market_depth_exchanges().expect("error requesting market depth exchanges");

    for exchange in &exchanges {
        println!("{exchange:?}");
    }
}
