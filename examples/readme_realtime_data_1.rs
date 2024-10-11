use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL");
    let subscription = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");

    for bar in subscription {
        // Process each bar here (e.g., print or use in calculations)

        // when the session end subscription can be cancelled.
       // subscription.cancel();
    }
}
