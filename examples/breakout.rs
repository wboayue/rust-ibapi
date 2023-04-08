use std::collections::VecDeque;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{Bar, BarSize, WhatToShow};
use ibapi::orders::{order_builder, Action, OrderNotification};
use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let symbol = "TSLA";
    let contract = Contract::stock(symbol); // defaults to USD and SMART exchange.

    let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).unwrap();

    let mut channel = BreakoutChannel::new(30);

    for bar in bars {
        channel.add_bar(&bar);

        // Ensure enough bars and no open positions.
        if !channel.ready() || has_position(&client, symbol) {
            continue;
        }

        let action = if bar.close > channel.high() {
            Action::Buy
        } else if bar.close < channel.low() {
            Action::Sell
        } else {
            continue;
        };

        let order_id = client.next_order_id();
        let order = order_builder::market_order(action, 100.0);

        let notices = client.place_order(order_id, &contract, &order).unwrap();
        for notice in notices {
            if let OrderNotification::ExecutionData(data) = notice {
                println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
            } else {
                println!("{:?}", notice);
            }
        }
    }
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

    fn add_bar(&mut self, bar: &Bar) {
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
