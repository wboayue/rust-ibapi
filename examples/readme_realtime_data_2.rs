use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract_aapl = Contract::stock("AAPL");
    let contract_nvda = Contract::stock("NVDA");

    let subscription_aapl = client
        .realtime_bars(&contract_aapl, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for (bar_aapl, bar_nvda) in subscription_aapl.iter().zip(subscription_nvda.iter()) {
        // Process each bar here (e.g., print or use in calculations)
        println!("AAPL {}, NVDA {}", bar_nvda.close, bar_aapl.close);

        // You can simply break the or explicitly cancel the subscription.
        // Subscriptions are automatically canceled when they go out of scope.
        subscription_aapl.cancel();
        subscription_nvda.cancel();
    }
}
