//! Readme Multi Threading 2 example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example readme_multi_threading_2
//! ```

use ibapi::client::blocking::Client;
use std::thread;

use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let symbols = vec![("AAPL", 100), ("NVDA", 101)];
    let mut handles = vec![];

    for (symbol, client_id) in symbols {
        let handle = thread::spawn(move || {
            let connection_url = "127.0.0.1:4002";
            let client = Client::connect(connection_url, client_id).expect("connection to TWS failed!");

            let contract = Contract::stock(symbol).build();
            let subscription = client
                .realtime_bars(&contract)
                .trading_hours(TradingHours::Extended)
                .subscribe()
                .expect("realtime bars request failed!");

            for bar in subscription.iter_data() {
                match bar {
                    Ok(bar) => println!("bar: {bar:?}"),
                    Err(e) => {
                        eprintln!("error: {e:?}");
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
