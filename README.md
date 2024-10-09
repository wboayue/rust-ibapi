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
If you encounter a problem or need a missing feature, please check the [issues list](https://github.com/wboayue/rust-ibapi/issues) before reporting it.

## Examples

The following examples demonstrate how to use the key features of the API.

### Connecting to TWS

The following is an example of connecting to TWS.

```rust
// Connect to the TWS API
let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed!");
```

Note that the connection is made using `127.0.0.1` instead of `localhost`. On some systems, `localhost` resolves to a 64-bit IP address, which may be blocked by TWS. TWS only allows specifying 32-bit IP addresses in the list of allowed IP addresses.

### Creating Contracts

The following example demonstrates how to create a stock contract for TSLA using the `stock` helper function.

```rust
// Create a contract for TSLA stock (default currency: USD, exchange: SMART)
let contract = Contract::stock("TSLA");
```

The stock, futures, and crypto methods provide shortcuts for defining contracts with reasonable defaults that can be modified after creation.

Alternatively, contracts can be fully specified as follows:

```rust
// Create a fully specified contract for TSLA stock
Contract {
    symbol: "TSLA",
    security_type: SecurityType::Stock,
    currency: "USD".to_string(),
    exchange: "SMART".to_string(),
    ..Default::default()
}
```

### Requesting Market Data

The following is an example of requesting realtime data from TWS.

```rust
// Request real-time bars data for TSLA with 5-second intervals
let subscription = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");

for bar in subscription {
    // Process each bar here (e.g., print or use in calculations)
}
```

In this example we request realtime bars from TWS. If the request is successful, we receive a subscription. The subscription in this example is converted to an iterator, which blocks and waits until the next bar becomes available. The subscription also supports a non-blocking request for the next item or a request for the next item with a timeout.

To request the next bar in a non-blocking manner.

```rust
loop {
    // Check if the next bar is available without waiting
    if let Some(bar) = subscription.try_next() {
        // Process the available bar (e.g., use it in calculations)
    }
    // Perform other work before checking for the next bar
}
```

The next bar could also be requested with a timeout.

```rust
loop {
    // Check if the next bar is available waiting for specified time
    if let Some(bar) = subscription.next_timeout() {
        // Process the available bar (e.g., use it in calculations)
    }
    // do some work
}
```

Explore the Subscription documentation for more examples.

### Placing Orders

```rust
// Creates a market order to purchase 100 shares
let order_id = client.next_order_id();
let order = order_builder::market_order(Action::Buy, 100.0);

let subscription = client.place_order(order_id, &contract, &order).expect("place order request failed!");

for notice in subscription {
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

We welcome contributions of all kinds! Feel free to propose new ideas, share bug fixes, or enhance the documentation. If you'd like to contribute, please start by reviewing our [contributor documentation](https://github.com/wboayue/rust-ibapi/tree/main/CONTRIBUTING.md).

For questions or discussions about contributions, feel free to open an issue or reach out via our [GitHub discussions page](https://github.com/wboayue/rust-ibapi/discussions).
