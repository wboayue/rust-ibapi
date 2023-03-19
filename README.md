[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/twsapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/0.1.0/ibapi)

## Introduction

An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust. The official TWS API is an event driven API. This implementation provides a synchronous API that simplifies the development of trading strategies.

This is a work in progress and targets support for TWS API 10.20. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).

The list of open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). If you run into an issue or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) first and then report the issue if it is not already tracked.

Contributions are welcome. Open a pull request.

## Example 

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
