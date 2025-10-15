//! Historical Data Options example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example historical_data_options
//! ```

use ibapi::client::blocking::Client;
use ibapi::market_data::historical::ToDuration;
use ibapi::prelude::*;

// This example demonstrates how to request historical data for an options contract.
// Historical data is not available to expired options contracts.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = build_contract();

    let historical_data = client
        .historical_data(
            &contract,
            None,
            10.days(),
            HistoricalBarSize::Hour,
            HistoricalWhatToShow::AdjustedLast,
            TradingHours::Regular,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}

fn build_contract() -> Contract {
    Contract {
        security_type: SecurityType::Option,
        symbol: "AMZN".into(),
        exchange: "SMART".into(),
        last_trade_date_or_contract_month: "20250131".into(),
        strike: 230.0,
        right: "C".into(),
        ..Default::default()
    }
}
