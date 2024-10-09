[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.svg?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

A Rust implementation of the Interactive Brokers [Trader Workstation (TWS) API](https://interactivebrokers.github.io/tws-api/introduction.html).
This implementation is a simplified version of the official TWS API, intended to make trading strategy development easier.

This project is a work in progress and has been tested with TWS version 10.19. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).

Open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). 
If you run into a problem or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) before reporting a new issue.

## Examples

The following examples provide an overview of the API.

### Connecting to TWS

The following is an example of connecting to TWS.

```rust
// Connect to the TWS API
let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed!");
```

Note that the connection is made using `127.0.0.1` instead of `localhost`. On some systems, `localhost` resolves to a 64-bit IP address, which may be blocked by TWS. TWS only allows specifying 32-bit IP addresses in the list of allowed IP addresses.

### Creating Contracts

```rust
let symbol = "TSLA";
// Create a contract for TSLA stock (default currency: USD, exchange: SMART)
let contract = Contract::stock(symbol);
```

### Requesting Market Data

```rust
// Request real-time bars data for TSLA with 5-second intervals
let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");

for bar in bars {
    // Process each bar here (e.g., print or use in calculations)
}
```

### Placing Orders

```rust
let order_id = client.next_order_id();
let order = order_builder::market_order(Action::Buy, 100.0);

let notices = client.place_order(order_id, &contract, &order).expect("place order request failed!");
for notice in notices {
    if let OrderNotification::ExecutionData(data) = notice {
        println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
    } else {
        println!("{:?}", notice);
    }
}
```

## Available APIs

The [Client documentation](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) provides comprehensive details on all currently available APIs, including trading, account management, and market data features, along with examples to help you get started.

## Contributions

We welcome contributions of all kinds! Feel free to propose new ideas, share bug fixes, or enhance the documentation. If you're interested in contributing to the project, start by reviewing the [contributor documentation](https://github.com/wboayue/rust-ibapi/tree/main/CONTRIBUTING.md).

For questions or discussions about contributions, feel free to open an issue or reach out via our [GitHub discussions page](https://github.com/wboayue/rust-ibapi/discussions).
