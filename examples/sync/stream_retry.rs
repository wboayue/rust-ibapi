//! Stream Retry example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example stream_retry
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
        // Waits out a reconnect in progress; fails only once the client gave
        // up (ConnectionFailed) or was shut down (Shutdown).
        if let Err(e) = client.wait_connected() {
            eprintln!("connection gone: {e}");
            return;
        }

        let subscription = match client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe() {
            Ok(subscription) => subscription,
            // Dropped again before the request went out: wait again.
            Err(e) if e.is_connection_lost() => continue,
            Err(e) => {
                eprintln!("realtime bars request failed: {e}");
                return;
            }
        };

        for bar in subscription.iter_data() {
            match bar {
                Ok(bar) => println!("bar: {bar:?}"),
                Err(e) if e.is_connection_lost() => {
                    eprintln!("Connection lost. Resubscribing after the reconnect...");
                    continue 'retry;
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            }
        }

        return;
    }
}
