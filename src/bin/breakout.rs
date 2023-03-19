use std::error::Error;

use ibapi::contracts::Contract;
use ibapi::market_data::{realtime, BarSize, RealTimeBar, WhatToShow};
use ibapi::orders::{self, order_builder};
use ibapi::Client;

struct BreakoutPeriod {
    high: f64,
    low: f64,
}

impl BreakoutPeriod {
    fn ready(&self) -> bool {
        false
    }
    fn consume(&self, bar: &RealTimeBar) {}
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::connect("localhost:4002")?;
    let contract = Contract::stock("TSLA");

    let bars = realtime::realtime_bars(&client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;

    let breakout = BreakoutPeriod { high: 0.0, low: 0.0 };

    for bar in bars {
        breakout.consume(&bar);

        if !breakout.ready() {
            continue;
        }

        if bar.close > breakout.high {
            let order_id = client.next_order_id();
            let order = order_builder::market_order(orders::Action::Buy, 100.0);
            let results = orders::place_order(&client, order_id, &contract, &order)?;
        }

        if bar.close < breakout.low {
            let order_id = client.next_order_id();
            let order = order_builder::trailing_stop(orders::Action::Sell, 100.0, 0.3, bar.close);
            let results = orders::place_order(&client, order_id, &contract, &order)?;
        }
    }

    Ok(())
}
