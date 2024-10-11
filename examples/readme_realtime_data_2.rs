use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract_aapl = Contract::stock("AAPL");
    let contract_nvda = Contract::stock("NVDA");

    let mut subscription_aapl = client
        .realtime_bars(&contract_aapl, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");
    let mut subscription_nvda = client
        .realtime_bars(&contract_nvda, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    while let (Some(bar_nvda), Some(bar_aapl)) = (subscription_nvda.next(), subscription_aapl.next()) {
        // Process each bar here (e.g., print or use in calculations)
        println!("NVDA {}, AAPL {}", bar_nvda.close, bar_aapl.close);

        // when your algorithm is done, cancel subscription
        subscription_aapl.cancel().expect("cancel failed");
        subscription_nvda.cancel().expect("cancel failed");
    }
}
