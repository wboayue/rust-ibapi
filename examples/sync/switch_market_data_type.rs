//! Switch Market Data Type example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example switch_market_data_type
//! ```

use ibapi::market_data::MarketDataType;
use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let market_data_type = MarketDataType::Live;

    client.switch_market_data_type(market_data_type).expect("request failed");
    println!("market data switched: {:?}", market_data_type);
}
