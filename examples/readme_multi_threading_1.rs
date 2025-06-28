use std::sync::Arc;
use std::thread;
use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Arc::new(Client::connect(connection_url, 100).expect("connection to TWS failed!"));

    let symbols = vec!["AAPL", "NVDA"];
    let mut handles = vec![];

    for symbol in symbols {
        let client = Arc::clone(&client);
        let handle = thread::spawn(move || {
            let contract = Contract::stock(symbol);
            let subscription = client
                .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
                .expect("realtime bars request failed!");

            for bar in subscription {
                // Process each bar here (e.g., print or use in calculations)
                println!("bar: {bar:?}");
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
