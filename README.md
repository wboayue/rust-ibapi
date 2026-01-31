[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.png?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

This library provides a comprehensive Rust implementation of the Interactive Brokers [TWS API](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/), offering a robust and user-friendly interface for TWS and IB Gateway. Designed with performance and simplicity in mind, `ibapi` is a good fit for automated trading systems, market analysis, real-time data collection and portfolio management tools.

With this fully featured API, you can retrieve account information, access real-time and historical market data, manage orders, perform market scans, and access news and Wall Street Horizons (WSH) event data. Future updates will focus on bug fixes, maintaining parity with the official API, and enhancing usability.

## Sync/Async Architecture

rust-ibapi ships both asynchronous (Tokio) and blocking (threaded) clients. The async client is enabled by default; opt into the blocking client with the `sync` feature and use both together when you need to mix execution models.

- **async** *(default)*: Non-blocking client using Tokio tasks and broadcast channels. Available as `ibapi::Client`.
- **sync**: Blocking client using crossbeam channels. Available as `ibapi::client::blocking::Client` (or `ibapi::Client` when `async` is disabled).

```toml
# Async only (default features)
ibapi = "2.7"

# Blocking only
ibapi = { version = "2.7", default-features = false, features = ["sync"] }

# Async + blocking together
ibapi = { version = "2.7", default-features = false, features = ["sync", "async"] }
```

```bash
# Async client (default configuration)
cargo test
cargo run --example async_connect

# Blocking client only
cargo test --no-default-features --features sync

# Validate both clients together
cargo test --all-features
```

When both features are enabled, import the blocking types explicitly:

```rust
use ibapi::Client;                    // async client
use ibapi::client::blocking::Client;  // blocking client
```

> **📚 Migrating from v1.x?** See the [Migration Guide](MIGRATION.md) for step-by-step upgrade instructions.

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
use ibapi::client::blocking::Client;
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

// Stock with customization - accepts string literals directly
let contract = Contract::stock("7203")
    .on_exchange("TSEJ")
    .in_currency("JPY")
    .build();
```

The builder API provides type-safe construction for all contract types with compile-time validation:

```rust
// Options - required fields enforced at compile time
let option = Contract::call("AAPL")
    .strike(150.0)
    .expires_on(2024, 12, 20)
    .build();

// Futures with convenience methods
let futures = Contract::futures("ES")
    .front_month()  // Next expiring contract
    .build();

// Forex pairs
let forex = Contract::forex("EUR", "USD").build();

// Bonds - simplified API for CUSIP and ISIN
let treasury = Contract::bond_cusip("912810RN0");
let euro_bond = Contract::bond_isin("DE0001102309");
```

See the [Contract Builder Guide](docs/contract-builder.md) for comprehensive documentation on all contract types.

For lower-level control, you can also create contracts directly using the type wrappers:

```rust
use ibapi::prelude::*;

// Create a contract directly using the struct and type wrappers
let contract = Contract {
    symbol: Symbol::from("TSLA"),
    security_type: SecurityType::Stock,
    currency: Currency::from("USD"),
    exchange: Exchange::from("SMART"),
    ..Default::default()
};
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

    let contract = Contract::stock("AAPL").build();

    let historical_data = client
        .historical_data(
            &contract,
            Some(datetime!(2023-04-11 20:00 UTC)),
            1.days(),
            HistoricalBarSize::Hour,
            Some(HistoricalWhatToShow::Trades),
            TradingHours::Regular,
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

    let contract = Contract::stock("AAPL").build();

    let historical_data = client
        .historical_data(
            &contract,
            Some(datetime!(2023-04-11 20:00 UTC)),
            1.days(),
            HistoricalBarSize::Hour,
            Some(HistoricalWhatToShow::Trades),
            TradingHours::Regular,
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
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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
    let contract_aapl = Contract::stock("AAPL").build();
    let contract_nvda = Contract::stock("NVDA").build();

    let subscription_aapl = client
        .realtime_bars(&contract_aapl, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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

    let contract = Contract::stock("AAPL").build();

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

    let contract = Contract::stock("AAPL").build();

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

#### Monitoring Order Updates

For real-time monitoring of order status, executions, and commissions, set up an order update stream before submitting orders:

##### Sync Example

```rust
use ibapi::prelude::*;
use std::thread;
use std::sync::Arc;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Arc::new(Client::connect(connection_url, 100).expect("connection to TWS failed!"));

    // Start background thread to monitor order updates
    let monitor_client = client.clone();
    let _monitor_handle = thread::spawn(move || {
        let stream = monitor_client.order_update_stream().expect("failed to create stream");

        for update in stream {
            match update {
                OrderUpdate::OrderStatus(status) => {
                    println!("Order {} Status: {}", status.order_id, status.status);
                    println!("  Filled: {}, Remaining: {}", status.filled, status.remaining);
                }
                OrderUpdate::OpenOrder(data) => {
                    println!("Open Order {}: {} {}",
                             data.order_id, data.order.action, data.contract.symbol);
                }
                OrderUpdate::ExecutionData(exec) => {
                    println!("Execution: {} shares @ {}",
                             exec.execution.shares, exec.execution.price);
                }
                OrderUpdate::CommissionReport(report) => {
                    println!("Commission: ${}", report.commission);
                }
                OrderUpdate::Message(msg) => {
                    println!("Message: {}", msg.message);
                }
            }
        }
    });

    // Give monitor time to start
    thread::sleep(std::time::Duration::from_millis(100));

    // Now submit orders - updates will be received by the monitoring thread
    let contract = Contract::stock("AAPL").build();
    let order_id = client.order(&contract)
        .buy(100)
        .market()
        .submit()
        .expect("order submission failed!");

    println!("Order {} submitted", order_id);

    // Keep main thread alive to receive updates
    thread::sleep(std::time::Duration::from_secs(10));
}
```

##### Async Example

```rust
use ibapi::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).await.expect("connection to TWS failed!");

    // Create order update stream before submitting orders
    let mut order_stream = client.order_update_stream().await.expect("failed to create stream");

    // Spawn task to monitor updates
    let monitor_handle = tokio::spawn(async move {
        while let Some(update) = order_stream.next().await {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!("Order {} Status: {}", status.order_id, status.status);
                    println!("  Filled: {}, Remaining: {}", status.filled, status.remaining);
                }
                Ok(OrderUpdate::OpenOrder(data)) => {
                    println!("Open Order {}: {} {}",
                             data.order_id, data.order.action, data.contract.symbol);
                }
                Ok(OrderUpdate::ExecutionData(exec)) => {
                    println!("Execution: {} shares @ {}",
                             exec.execution.shares, exec.execution.price);
                }
                Ok(OrderUpdate::CommissionReport(report)) => {
                    println!("Commission: ${}", report.commission);
                }
                Ok(OrderUpdate::Message(msg)) => {
                    println!("Message: {}", msg.message);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    break;
                }
            }
        }
    });

    // Give monitor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now submit orders - updates will be received by the monitoring task
    let contract = Contract::stock("AAPL").build();
    let order_id = client.order(&contract)
        .buy(100)
        .market()
        .submit()
        .await
        .expect("order submission failed!");

    println!("Order {} submitted", order_id);

    // Wait for updates
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Clean up
    monitor_handle.abort();
}
```

The order update stream provides real-time notifications for:
- **OrderStatus**: Status changes (Submitted, Filled, Cancelled, etc.)
- **OpenOrder**: Order details when opened or modified
- **ExecutionData**: Fill notifications with price and quantity
- **CommissionReport**: Commission charges for executions
- **Message**: System messages and notifications

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
            let contract = Contract::stock(symbol).build();
            let subscription = client
                .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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

            let contract = Contract::stock(symbol).build();
            let subscription = client
                .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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

    let contract = Contract::stock("AAPL").build();

    loop {
        // Request real-time bars data with 5-second intervals
        let subscription = client
            .realtime_bars(&contract, RealtimeBarSize::Sec5, RealtimeWhatToShow::Trades, TradingHours::Extended)
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
