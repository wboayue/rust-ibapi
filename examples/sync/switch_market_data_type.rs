//! Switch Market Data Type example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example switch_market_data_type
//! ```

use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let market_data_type = MarketDataType::Realtime;

    client.switch_market_data_type(market_data_type).expect("request failed");
    println!("market data switched: {market_data_type:?}");
}
