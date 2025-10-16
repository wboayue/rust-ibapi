//! Calculate Option Price example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example calculate_option_price
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, SecurityType};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract {
        symbol: "AAPL".into(),
        security_type: SecurityType::Option,
        exchange: "SMART".into(),
        currency: "USD".into(),
        last_trade_date_or_contract_month: "20250620".into(), // Expiry date (YYYYMMDD)
        strike: 240.0,
        right: "C".into(), // Option type: "C" for Call, "P" for Put
        ..Default::default()
    };

    let calculation = client.calculate_option_price(&contract, 100.0, 235.0).expect("request failed");
    println!("calculation: {calculation:?}");
}
