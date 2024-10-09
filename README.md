[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.svg?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

A Rust implementation of the Interactive Brokers [Trader Workstation (TWS) API](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/).
This implementation is a simplified version of the official TWS API, intended to make trading strategy development easier.

This project is a work in progress and has been tested with TWS version 10.19. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).

Open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues).
If you encounter a problem or need a missing feature, please check the [issues list](https://github.com/wboayue/rust-ibapi/issues) before reporting it.

## Examples

The following examples demonstrate how to use the key features of the API.

### Connecting to TWS

The following is an example of connecting to TWS.

```rust
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";

    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    println!("Successfully connected to TWS at {connection_url}");
}
```

Note that the connection is made using `127.0.0.1` instead of `localhost`. On some systems, `localhost` resolves to a 64-bit IP address, which may be blocked by TWS. TWS only allows specifying 32-bit IP addresses in the list of allowed IP addresses.

### Creating Contracts

The following example demonstrates how to create a stock contract for TSLA using the `stock` helper function.

```rust
// Create a contract for TSLA stock (default currency: USD, exchange: SMART)
let contract = Contract::stock("TSLA");
```

The [stock](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.stock), [futures](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.futures), and [crypto](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.crypto) builders provide shortcuts for defining contracts with reasonable defaults that can be modified after creation.

Alternatively, contracts that require customized configurations can be fully specified as follows:

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

Explore the [Contract documentation](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html) for a detailed list of contract attributes.

### Requesting Historical Market Data

The following is an example of requesting historical data from TWS.

```rust
use ibapi::contracts::Contract;
use ibapi::Client;
use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("NVDA");

    let subscription = client
        .historical_data(
            &contract,
            datetime!(2023-04-11 20:00 UTC),
            1.days(),
            BarSize::Hour,
            WhatToShow::Trades,
            true,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", subscription.start, subscription.end);

    for bar in &subscription.bars {
        println!("{bar:?}");
    }
}
```

### Requesting Realtime Market Data

The following is an example of requesting realtime data from TWS.

```rust
// make this runnable code

// Request real-time bars data for TSLA with 5-second intervals
let subscription = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");

for bar in subscription {
    // Process each bar here (e.g., print or use in calculations)

    // when the session end subscription can be cancelled.
    subscription.cancel();
}
```

In this example we request realtime bars from TWS. If the request is successful, we receive a subscription. The subscription in this example is converted to an iterator, which blocks and waits until the next bar becomes available.

The syntactic sugar for the for loop can be expanded into.

```rust
while let Some(bar) = subscription.next() {
    // Process each bar here (e.g., print or use in calculations)
}
```

Using this form you could easily stream multiple contracts

```rust
// make this runnable code

let subscription_nvda = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");
let subscription_aapl = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("realtime bars request failed!");

while let (Some(bar_nvda), Some(bar_aapl)) = (subscription_nvda.next(), subscription_aapl.next()) {
    // Process each bar here (e.g., print or use in calculations)
}
```

Subscriptions also support non-blocking processing with the try_next and next_timeout methods. Explore the [BoundedSubscription] and [UnboundedSubscription] documentation for more examples.

### Placing Orders

```rust
// make this runnable code

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
