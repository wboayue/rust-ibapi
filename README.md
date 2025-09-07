[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.png?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

This library provides a comprehensive Rust implementation of the Interactive Brokers [TWS API](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/), offering a robust and user-friendly interface for TWS and IB Gateway. Designed with performance and simplicity in mind, `ibapi` is a good fit for automated trading systems, market analysis, real-time data collection and portfolio management tools.

With this fully featured API, you can retrieve account information, access real-time and historical market data, manage orders, perform market scans, and access news and Wall Street Horizons (WSH) event data. Future updates will focus on bug fixes, maintaining parity with the official API, and enhancing usability.

## Sync/Async Architecture

The rust-ibapi library requires you to explicitly choose between synchronous (thread-based) and asynchronous (tokio-based) operation modes:

- **sync**: Traditional synchronous API using threads and crossbeam channels
- **async**: Asynchronous API using tokio tasks and broadcast channels

You must specify exactly one feature when using this crate:

```toml
# From crates.io (Note: v2.0 not yet published, use GitHub for now):
ibapi = { version = "2.0", features = ["sync"] }   # For synchronous API
# OR
ibapi = { version = "2.0", features = ["async"] }  # For asynchronous API

# From GitHub (recommended until v2.0 is published):
ibapi = { git = "https://github.com/wboayue/rust-ibapi", features = ["sync"] }
# OR
ibapi = { git = "https://github.com/wboayue/rust-ibapi", features = ["async"] }
```

```bash
# Build and test examples:
cargo build --features sync
cargo test --features sync

# Or for async:
cargo build --features async
cargo test --features async
cargo run --features async --example async_connect
```

> **🚧 Work in Progress**: Version 2.0 is currently under active development and includes significant architectural improvements, async/await support, and enhanced features. The current release (1.x) remains stable and production-ready.

> **📚 Migrating from v1.x?** See the [Migration Guide](MIGRATION.md) for detailed instructions on upgrading to v2.0.

If you encounter any issues or require a missing feature, please review the [issues list](https://github.com/wboayue/rust-ibapi/issues) before submitting a new one.

## Available APIs

The [Client documentation](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) provides comprehensive details on all currently available APIs, including trading, account management, and market data features, along with examples to help you get started.

## Install

Check [crates.io/crates/ibapi](https://crates.io/crates/ibapi) for the latest available version and installation instructions.

## Examples

These examples demonstrate key features of the `ibapi` API.

### Connecting to TWS

#### Sync Example

```rust
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";

    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    println!("Successfully connected to TWS at {connection_url}");
}
```

#### Async Example

```rust
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    let connection_url = "127.0.0.1:4002";

    let client = Client::connect(connection_url, 100).await.expect("connection to TWS failed!");
    println!("Successfully connected to TWS at {connection_url}");
}
```
> **Note**: Use `127.0.0.1` instead of `localhost` for the connection. On some systems, `localhost` resolves to an IPv6 address, which TWS may block. TWS only allows specifying IPv4 addresses in the allowed IP addresses list.

### Creating Contracts

The library provides a powerful type-safe contract builder API. Here's how to create a stock contract for TSLA:

```rust
use ibapi::prelude::*;

// Simple stock contract with defaults (USD, SMART routing)
let contract = Contract::stock("TSLA").build();

// Stock with customization
let contract = Contract::stock("7203")
    .on_exchange(Exchange::TSEJ)
    .in_currency(Currency::JPY)
    .build();
```

The builder API provides type-safe construction for all contract types:

```rust
// Options - enforces required fields at compile time
let call = Contract::call("AAPL")
    .strike(150.0)  // Validates positive strike price
    .expires_on(2024, 12, 20)
    .build();

// Options with convenience methods
let weekly = Contract::put("SPY")
    .strike(450.0)
    .expires_weekly()  // Next Friday expiration
    .build();

// Futures with smart defaults
let es = Contract::futures("ES")
    .front_month()  // Next expiring contract
    .build();

// Quarterly futures
let nq = Contract::futures("NQ")
    .next_quarter()  // Next Mar/Jun/Sep/Dec expiration
    .build();

// Forex pairs
let eur_usd = Contract::forex(Currency::EUR, Currency::USD)
    .amount(100_000)
    .build();

// Bonds by CUSIP or ISIN
let bond = Contract::bond(BondIdentifier::Cusip(Cusip::new("912810RN0")));

// Index with smart defaults
let spx = Contract::index("SPX");  // Auto-configures CBOE exchange, USD

// Spreads with convenience methods
let iron_condor = Contract::spread()
    .iron_condor(put_long_id, put_short_id, call_short_id, call_long_id)
    .build()?;
```

For comprehensive documentation on creating all contract types including stocks, options, futures, forex, crypto, and complex spreads, see the [Contract Builder Guide](docs/contract-builder.md).

For lower-level control, you can also create contracts directly:

```rust
use ibapi::prelude::*;

// Create a fully specified contract
Contract {
    symbol: "TSLA".to_string(),
    security_type: SecurityType::Stock,
    currency: "USD".to_string(),
    exchange: "SMART".to_string(),
    ..Default::default()
}
```

For a complete list of contract attributes, explore the [Contract documentation](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html).

### Requesting Historical Market Data

#### Sync Example

```rust
use time::macros::datetime;
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    let historical_data = client
        .historical_data(
            &contract,
            Some(datetime!(2023-04-11 20:00 UTC)),
            1.days(),
            HistoricalBarSize::Hour,
            HistoricalWhatToShow::Trades,
            true,
        )
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
```

#### Async Example

```rust
use time::macros::datetime;
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).await.expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    let historical_data = client
        .historical_data(
            &contract,
            Some(datetime!(2023-04-11 20:00 UTC)),
            1.days(),
            HistoricalBarSize::Hour,
            HistoricalWhatToShow::Trades,
            true,
        )
        .await
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
```

### Requesting Realtime Market Data

#### Sync Example

```rust
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL");
    let subscription = client
        .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for bar in subscription {
        // Process each bar here (e.g., print or use in calculations)
        println!("bar: {bar:?}");
    }
}
```

#### Async Example

```rust
use ibapi::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).await.expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL");
    let mut subscription = client
        .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
        .await
        .expect("realtime bars request failed!");

    while let Some(bar) = subscription.next().await {
        // Process each bar here (e.g., print or use in calculations)
        println!("bar: {bar:?}");
    }
}
```

In both examples, the request for realtime bars returns a [Subscription](https://docs.rs/ibapi/latest/ibapi/struct.Subscription.html) that can be used as an iterator (sync) or stream (async). The subscription is automatically cancelled when it goes out of scope.

#### Non-blocking Iteration (Sync)

```rust
use std::time::Duration;

// Example of non-blocking iteration in sync mode
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
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract_aapl = Contract::stock("AAPL");
    let contract_nvda = Contract::stock("NVDA");

    let subscription_aapl = client
        .realtime_bars(&contract_aapl, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
        .expect("realtime bars request failed!");

    for (bar_aapl, bar_nvda) in subscription_aapl.iter().zip(subscription_nvda.iter()) {
        // Process each bar here (e.g., print or use in calculations)
        println!("AAPL {}, NVDA {}", bar_aapl.close, bar_nvda.close);
    }
}
```
> **Note:** When using `zip`, the iteration will stop if either subscription ends. For independent processing, consider handling each subscription separately.

### Placing Orders

For a comprehensive guide on all supported order types and their usage, see the [Order Types Guide](docs/order-types.md).

#### Sync Example

```rust
use ibapi::prelude::*;

pub fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    // Create and submit a market order to purchase 100 shares using the fluent API
    let order_id = client.order(&contract)
        .buy(100)
        .market()
        .submit()
        .expect("order submission failed!");

    println!("Order submitted with ID: {}", order_id);

    // Example of a more complex order: limit order with time in force
    let order_id = client.order(&contract)
        .sell(50)
        .limit(150.00)
        .good_till_cancel()
        .outside_rth()
        .submit()
        .expect("order submission failed!");

    println!("Limit order submitted with ID: {}", order_id);
}
```

#### Async Example

```rust
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).await.expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    // Create and submit a market order to purchase 100 shares using the fluent API
    let order_id = client.order(&contract)
        .buy(100)
        .market()
        .submit()
        .await
        .expect("order submission failed!");

    println!("Order submitted with ID: {}", order_id);

    // Example of a bracket order: entry with take profit and stop loss
    let bracket_ids = client.order(&contract)
        .buy(100)
        .bracket()
        .entry_limit(150.00)
        .take_profit(160.00)
        .stop_loss(145.00)
        .submit_all()
        .await
        .expect("bracket order submission failed!");

    println!("Bracket order IDs - Parent: {}, TP: {}, SL: {}", 
             bracket_ids.parent, bracket_ids.take_profit, bracket_ids.stop_loss);
}
```

## Multi-Threading

The [Client](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) can be shared between threads to support concurrent operations. The following example demonstrates valid multi-threaded usage of [Client](https://docs.rs/ibapi/latest/ibapi/struct.Client.html).

```rust
use std::sync::Arc;
use std::thread;
use ibapi::prelude::*;

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
                .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
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
use ibapi::prelude::*;

fn main() {
    let symbols = vec![("AAPL", 100), ("NVDA", 101)];
    let mut handles = vec![];

    for (symbol, client_id) in symbols {
        let handle = thread::spawn(move || {
            let connection_url = "127.0.0.1:4002";
            let client = Client::connect(connection_url, client_id).expect("connection to TWS failed!");

            let contract = Contract::stock(symbol);
            let subscription = client
                .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
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
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    loop {
        // Request real-time bars data with 5-second intervals
        let subscription = client
            .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, false)
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
