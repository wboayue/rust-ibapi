use std::collections::VecDeque;
use std::error::Error;

use ibapi::contracts::Contract;
use ibapi::market_data::{BarSize, RealTimeBar, WhatToShow};
use ibapi::orders::{order_builder, Action};
use ibapi::Client;

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::connect("localhost:4002")?;

    let symbol = "TSLA";
    let contract = Contract::stock(symbol);

    let bars = client.realtime_bars(&contract, &BarSize::Sec5, &WhatToShow::Trades, false)?;

    let mut breakout = BreakoutChannel::new(30);

    for bar in bars {
        breakout.consume(&bar);

        // Make sure we have enough data and no stop order is active.
        if !breakout.ready() || has_position(&client, symbol) {
            continue;
        }

        // Trade long only
        if bar.close > breakout.high() {
            let order_id = client.next_order_id();
            let order = order_builder::market_order(Action::Buy, 100.0);

            let results = client.place_order(order_id, &contract, &order)?;
            for status in results {
                println!("order status: {status:?}")
            }
        }
    }

    Ok(())
}

fn has_position(client: &Client, symbol: &str) -> bool {
    if let Ok(mut positions) = client.positions() {
        positions.find(|p| p.contract.symbol == symbol).is_some()
    } else {
        false
    }
}

struct BreakoutChannel {
    highs: VecDeque<f64>,
    size: usize,
}

impl BreakoutChannel {
    fn new(size: usize) -> BreakoutChannel {
        BreakoutChannel {
            highs: VecDeque::with_capacity(size + 1),
            size,
        }
    }

    fn ready(&self) -> bool {
        self.highs.len() >= self.size
    }

    fn consume(&mut self, bar: &RealTimeBar) {
        self.highs.push_back(bar.high);

        if self.highs.len() > self.size {
            self.highs.pop_front();
        }
    }

    fn high(&self) -> f64 {
        *self.highs.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&1.0)
    }
}
