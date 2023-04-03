use std::collections::VecDeque;
use std::error::Error;

use ibapi::contracts::Contract;
use ibapi::market_data::{BarSize, RealTimeBar, WhatToShow};
use ibapi::orders::{order_builder, Action};
use ibapi::Client;

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::connect("localhost:4002", 100)?;

    let symbol = "TSLA";
    let contract = Contract::stock(symbol); // defaults to USD and SMART exchange.

    let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)?;

    let mut channel = BreakoutChannel::new(30);

    for bar in bars {
        channel.consume(&bar);

        // Ensure enough bars and no open positions.
        if !channel.ready() || has_position(&client, symbol) {
            continue;
        }

        if bar.close > channel.high() {
            let order_id = client.next_order_id();
            let order = order_builder::market_order(Action::Buy, 100.0);

            let results = client.place_order(order_id, &contract, &order)?;
            for status in results {
                println!("order status: {status:?}")
            }
        }

        if bar.close < channel.low() {
            let order_id = client.next_order_id();
            let order = order_builder::market_order(Action::SellShort, 100.0);

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
    ticks: VecDeque<(f64, f64)>,
    size: usize,
}

impl BreakoutChannel {
    fn new(size: usize) -> BreakoutChannel {
        BreakoutChannel {
            ticks: VecDeque::with_capacity(size + 1),
            size,
        }
    }

    fn ready(&self) -> bool {
        self.ticks.len() >= self.size
    }

    fn consume(&mut self, bar: &RealTimeBar) {
        self.ticks.push_back((bar.high, bar.low));

        if self.ticks.len() > self.size {
            self.ticks.pop_front();
        }
    }

    fn high(&self) -> f64 {
        self.ticks.iter().map(|x| x.0).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
    }

    fn low(&self) -> f64 {
        self.ticks.iter().map(|x| x.1).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
    }
}
