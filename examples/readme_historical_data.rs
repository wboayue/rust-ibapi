use time::macros::datetime;

use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("NVDA");

    let subscription = client
        .historical_data(
            &contract,
            datetime!(2023-04-11 20:00 UTC),
            1.days(),
            BarSize::Hour,
            WhatToShow::Trades,
            true,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", subscription.start, subscription.end);

    for bar in &subscription.bars {
        println!("{bar:?}");
    }
}
