[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.svg?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust.
This implementation is not a direct port of the official TWS API.
It provides a synchronous API that simplifies the development of trading strategies.

This is a work in progress and was tested using TWS 10.19. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).

Open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). 
If you run into a problem or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) before reporting a new issue.

Contributions are welcome.

## Example

The following example gives a flavor of the API style. It is not a trading strategy recommendation and not a complete implementation.

```rust
use std::collections::VecDeque;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, Bar, WhatToShow};
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
```

## Available APIs

### Accounts

* [positions](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.positions)

### Contracts

* [contract_details](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.contract_details)
* [market_rule](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.market_rule)
* [matching_symbols](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.matching_symbols)

### Historical Market Data

* [head_timestamp](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.head_timestamp)
* [historical_data](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.historical_data)
* [historical_data_ending_now](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.historical_data_ending_now)
* [historical_schedules](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.historical_schedules)
* [historical_schedules_ending_now](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.historical_schedules_ending_now)

### Realtime Market Data

* [realtime_bars](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.realtime_bars)
* [tick_by_tick_all_last](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.tick_by_tick_all_last)
* [tick_by_tick_bid_ask](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.tick_by_tick_bid_ask)
* [tick_by_tick_last](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.tick_by_tick_last)
* [tick_by_tick_midpoint](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.tick_by_tick_midpoint)

### Orders

* [all_open_orders](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.all_open_orders)
* [auto_open_orders](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.auto_open_orders)
* [cancel_order](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.cancel_order)
* [completed_orders](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.completed_orders)
* [executions](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.executions)
* [global_cancel](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.global_cancel)
* [next_valid_order_id](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.next_valid_order_id)
* [open_orders](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.open_orders)
* [place_order](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.place_order)
