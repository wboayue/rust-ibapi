//! Readme Historical Data example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example readme_historical_data
//! ```

use ibapi::prelude::*;
use time::macros::datetime;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL").build();

    let historical_data = client
        .historical_data(
            &contract,
            Some(datetime!(2023-04-11 20:00 UTC)),
            1.days(),
            HistoricalBarSize::Hour,
            HistoricalWhatToShow::Trades,
            TradingHours::Regular,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
