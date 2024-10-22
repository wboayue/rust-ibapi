use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::{Client, Error};

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");
    stream_bars(&client, &contract);
}

// Request real-time bars data with 5-second intervals
fn stream_bars(client: &Client, contract: &Contract) {
    let subscription = client
        .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for bar in &subscription {
        // Process each bar here (e.g., print or use in calculations)
        println!("bar: {bar:?}");
    }

    if let Some(Error::ConnectionReset) = subscription.error() {
        println!("Connection reset. Retrying stream...");
        stream_bars(client, contract);
    }
}
