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
