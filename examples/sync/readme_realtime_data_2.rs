//! Readme Realtime Data 2 example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example readme_realtime_data_2
//! ```

use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract_aapl = Contract::stock("AAPL").build();
    let contract_nvda = Contract::stock("NVDA").build();

    let subscription_aapl = client
        .realtime_bars(&contract_aapl, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
        .expect("realtime bars request failed!");

    for (bar_aapl, bar_nvda) in subscription_aapl.iter().zip(subscription_nvda.iter()) {
        // Process each bar here (e.g., print or use in calculations)
        println!("AAPL {}, NVDA {}", bar_aapl.close, bar_nvda.close);
    }
}
