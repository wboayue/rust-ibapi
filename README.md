[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/twsapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/0.1.0/ibapi)

## Introduction

An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust.
This implementation is not a direct port of the offical TWS API.
It provides a synchronous API simplifies the development of trading strategies.

This is a work in progress and was tested against TWS 10.20. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).

Open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). 
If you run into a problem or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) before reporting a new issue.

Contributions are welcome.

## Example

The following example gives a flavor of the API style. It is not a trading strategy recommendation.

```rust
use std::error::Error;

use ibapi::client::Client;
use ibapi::contracts::{Contract};
use ibapi::market_data::{RealTimeBar, realtime, BarSize, WhatToShow};
use ibapi::orders::{self, order_builder};

struct BreakoutPeriod {
    high: f64,
    low: f64,
}

impl BreakoutPeriod {
    fn ready(&self) -> bool {
        false
    }
    fn consume(&self, bar: &RealTimeBar) {
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::connect("localhost:4002")?;
    let contract = Contract::stock("TSLA");

    let bars = realtime::realtime_bars(&client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;

    let breakout = BreakoutPeriod{
        high: 0.0,
        low: 0.0,
    };

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
```

## Available APIs

### Accounts

* [positions](https://docs.rs/ibapi/0.1.0/ibapi/struct.Client.html#method.positions)

### Contracts

* contract_details
* market_rule
* matching_symbols

### Market Data

* realtime_bars
* tick_by_tick_all_last
* tick_by_tick_bid_ask
* tick_by_tick_last
* tick_by_tick_midpoint

### Orders

* all_open_orders
* auto_open_orders
* cancel_order
* completed_orders
* executions
* global_cancel
* next_valid_order_id
* open_orders
* place_order
