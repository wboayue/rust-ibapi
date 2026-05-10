//! Breakout example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example breakout
//! ```

use std::collections::VecDeque;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::realtime::Bar;
use ibapi::market_data::TradingHours;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let symbol = "TSLA";
    let contract = Contract::stock(symbol).build(); // defaults to USD and SMART exchange.

    let bars = client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe().unwrap();

    let mut channel = BreakoutChannel::new(30);

    for bar in bars.iter_data() {
        let bar = match bar {
            Ok(bar) => bar,
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        };
        channel.add_bar(&bar);

        // Ensure enough bars and no open positions.
        if !channel.ready() {
            continue;
        }

        let order_id = if bar.close > channel.high() {
            client.order(&contract).buy(100).market().submit()
        } else if bar.close < channel.low() {
            client.order(&contract).sell(100).market().submit()
        } else {
            continue;
        };

        match order_id {
            Ok(id) => println!("Submitted breakout order {id}"),
            Err(e) => eprintln!("error: {e}"),
        }
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
