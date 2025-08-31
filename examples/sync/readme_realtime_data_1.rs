//! Readme Realtime Data 1 example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example readme_realtime_data_1
//! ```

use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL");
    let subscription = client
        .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
        .expect("realtime bars request failed!");

    for bar in subscription {
        // Process each bar here (e.g., print or use in calculations)
        println!("bar: {bar:?}");
    }
}
