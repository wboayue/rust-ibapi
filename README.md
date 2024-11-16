[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.svg?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

This library provides a comprehensive Rust implementation of the Interactive Brokers [TWS API](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/), offering a robust and user-friendly interface for TWS and IB Gateway. Designed with performance and simplicity in mind, `ibapi` is a good fit for automated trading systems, market analysis, real-time data collection and portfolio management tools.

With this fully featured API, you can retrieve account information, access real-time and historical market data, manage orders, perform market scans, and access news and Wall Street Horizons (WSH) event data. Future updates will focus on bug fixes, maintaining parity with the official API, and enhancing usability.

If you encounter any issues or require a missing feature, please review the [issues list](https://github.com/wboayue/rust-ibapi/issues) before submitting a new one.

## Available APIs

The [Client documentation](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) provides comprehensive details on all currently available APIs, including trading, account management, and market data features, along with examples to help you get started.

## Install

Check [crates.io/crates/ibapi](https://crates.io/crates/ibapi) for the latest available version and installation instructions.

## Examples

These examples demonstrate key features of the `ibapi` API.

### Connecting to TWS

The following example shows how to connect to TWS.

```rust
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";

    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    println!("Successfully connected to TWS at {connection_url}");
}
```
> **Note**: Use `127.0.0.1` instead of `localhost` for the connection. On some systems, `localhost` resolves to an IPv6 address, which TWS may block. TWS only allows specifying IPv4 addresses in the allowed IP addresses list.

### Creating Contracts

Here’s how to create a stock contract for TSLA using the [stock](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.stock) helper function.

```rust
// Create a contract for TSLA stock (default currency: USD, exchange: SMART)
let contract = Contract::stock("TSLA");
```

The [stock](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.stock), [futures](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.futures), and [crypto](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html#method.crypto) methods provide shortcuts for defining contracts with reasonable defaults that can be modified after creation.

For contracts requiring custom configurations:

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

For a complete list of contract attributes, explore the [Contract documentation](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html).

### Requesting Historical Market Data

The following is an example of requesting historical data from TWS.

```rust
use time::macros::datetime;

use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    let historical_data = client
        .historical_data(
            &contract,
            datetime!(2023-04-11 20:00 UTC),
            1.days(),
            BarSize::Hour,
            WhatToShow::Trades,
            true,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
```

### Requesting Realtime Market Data

The following is an example of requesting realtime data from TWS.

```rust
use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL");
    let subscription = client
        .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for bar in subscription {
        // Process each bar here (e.g., print or use in calculations)
        println!("bar: {bar:?}");
    }
}
```

In this example, the request for realtime bars returns a [Subscription](https://docs.rs/ibapi/latest/ibapi/struct.Subscription.html) that is implicitly converted into a blocking iterator over the bars. The subscription is automatically cancelled when it goes out of scope. The `Subscription` can also be used to iterate over bars in a non-blocking fashion.

```rust
// Example of non-blocking iteration
loop {
    match subscription.try_next() {
        Some(bar) => println!("bar: {bar:?}"),
        None => {
            // No new data yet; perform other tasks or sleep
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
```

Explore the [Subscription documentation](https://docs.rs/ibapi/latest/ibapi/struct.Subscription.html) for more details.

Since subscriptions can be converted to iterators, it is easy to iterate over multiple contracts.

```rust
use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract_aapl = Contract::stock("AAPL");
    let contract_nvda = Contract::stock("NVDA");

    let subscription_aapl = client
        .realtime_bars(&contract_aapl, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda, BarSize::Sec5, WhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for (bar_aapl, bar_nvda) in subscription_aapl.iter().zip(subscription_nvda.iter()) {
        // Process each bar here (e.g., print or use in calculations)
        println!("AAPL {}, NVDA {}", bar_aapl.close, bar_nvda.close);
    }
}
```
> **Note:** When using `zip`, the iteration will stop if either subscription ends. For independent processing, consider handling each subscription separately.

### Placing Orders

```rust
pub fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    // Creates a market order to purchase 100 shares
    let order_id = client.next_order_id();
    let order = order_builder::market_order(Action::Buy, 100.0);

    let subscription = client.place_order(order_id, &contract, &order).expect("place order request failed!");

    for event in &subscription {
        if let PlaceOrder::ExecutionData(data) = event {
            println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
        } else {
            println!("{:?}", event);
        }
    }
}
```

## Multi-Threading

The [Client](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) can be shared between threads to support concurrent operations. The following example demonstrates valid multi-threaded usage of [Client](https://docs.rs/ibapi/latest/ibapi/struct.Client.html).

```rust
use std::sync::Arc;
use std::thread;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Arc::new(Client::connect(connection_url, 100).expect("connection to TWS failed!"));

    let symbols = vec!["AAPL", "NVDA"];
    let mut handles = vec![];

    for symbol in symbols {
        let client = Arc::clone(&client);
        let handle = thread::spawn(move || {
            let contract = Contract::stock(symbol);
            let subscription = client
                .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
                .expect("realtime bars request failed!");

            for bar in subscription {
                // Process each bar here (e.g., print or use in calculations)
                println!("bar: {bar:?}");
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
```

Some TWS API calls do not have a unique request ID and are mapped back to the initiating request by message type instead. Since the message type is not unique, concurrent requests of the same message type (if not synchronized by the application) may receive responses for other requests of the same message type. [Subscriptions](https://docs.rs/ibapi/latest/ibapi/client/struct.Subscription.html) using shared channels are tagged with the [SharesChannel](https://docs.rs/ibapi/latest/ibapi/client/trait.SharesChannel.html) trait to highlight areas that the application may need to synchronize.

To avoid this issue, you can use a model of one client per thread. This ensures that each client instance handles only its own messages, reducing potential conflicts:

```rust
use std::thread;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::Client;

fn main() {
    let symbols = vec![("AAPL", 100), ("NVDA", 101)];
    let mut handles = vec![];

    for (symbol, client_id) in symbols {
        let handle = thread::spawn(move || {
            let connection_url = "127.0.0.1:4002";
            let client = Client::connect(connection_url, client_id).expect("connection to TWS failed!");

            let contract = Contract::stock(symbol);
            let subscription = client
                .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
                .expect("realtime bars request failed!");

            for bar in subscription {
                // Process each bar here (e.g., print or use in calculations)
                println!("bar: {bar:?}");
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
```

In this model, each client instance handles only the requests it initiates, improving the reliability of concurrent operations.

# Fault Tolerance

The API will automatically attempt to reconnect to the TWS server if a disconnection is detected. The API will attempt to reconnect up to 30 times using a Fibonacci backoff strategy. In some cases, it will retry the request in progress. When receiving responses via a [Subscription](https://docs.rs/ibapi/latest/ibapi/client/struct.Subscription.html), the application may need to handle retries manually, as shown below.

```rust
use ibapi::contracts::Contract;
use ibapi::market_data::realtime::{BarSize, WhatToShow};
use ibapi::{Client, Error};

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    loop {
        // Request real-time bars data with 5-second intervals
        let subscription = client
            .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
            .expect("realtime bars request failed!");

        for bar in &subscription {
            // Process each bar here (e.g., print or use in calculations)
            println!("bar: {bar:?}");
        }

        if let Some(Error::ConnectionReset) = subscription.error() {
            eprintln!("Connection reset. Retrying stream...");
            continue;
        }

        break;
    }
}
```

## Contributions

We welcome contributions of all kinds. Feel free to propose new ideas, share bug fixes, or enhance the documentation. If you'd like to contribute, please start by reviewing our [contributor documentation](https://github.com/wboayue/rust-ibapi/blob/main/CONTRIBUTING.md).

For questions or discussions about contributions, feel free to open an issue or reach out via our [GitHub discussions page](https://github.com/wboayue/rust-ibapi/discussions).
