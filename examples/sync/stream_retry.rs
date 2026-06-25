//! Stream Retry example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example stream_retry
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::TradingHours;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL").build();

    'retry: loop {
        // Request real-time bars data with 5-second intervals
        let subscription = client
            .realtime_bars(&contract)
            .trading_hours(TradingHours::Extended)
            .subscribe()
            .expect("realtime bars request failed!");

        for bar in subscription.iter_data() {
            match bar {
                Ok(bar) => println!("bar: {bar:?}"),
                Err(e) if e.is_connection_lost() => {
                    eprintln!("Connection lost. Retrying stream...");
                    continue 'retry;
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    break 'retry;
                }
            }
        }

        break;
    }
}
