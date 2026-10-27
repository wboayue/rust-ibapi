[![Build](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/ibapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/latest/ibapi/)
[![Coverage Status](https://coveralls.io/repos/github/wboayue/rust-ibapi/badge.png?branch=main)](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)

## Introduction

An unofficial Rust client for the Interactive Brokers [TWS API](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/), with async (Tokio) and blocking clients for TWS and IB Gateway.

Use it to retrieve account information, request real-time and historical market data, place and monitor orders, run market scans, and read news and Wall Street Horizon (WSH) event data. Contract builders check required fields at compile time, subscriptions cancel themselves when dropped, and the client reconnects automatically after a dropped connection.

> **📚 Migrating?**
> - [v4.x → v5.0](docs/migration-5.0.md)
> - [v3.x → v4.x](docs/migration-4.0.md)
> - [v2.x → v3.0](docs/migration-3.0.md): `Subscription` shape and notification handling
> - [v1.x → v2.0](MIGRATION.md)

## Sync/Async Architecture

The async client is enabled by default; opt into the blocking client with the `sync` feature and use both together when you need to mix execution models.

- **async** *(default)*: Non-blocking client using Tokio tasks and broadcast channels. Available as `ibapi::Client`.
- **sync**: Blocking client using crossbeam channels. Available as `ibapi::client::blocking::Client` (or `ibapi::Client` when `async` is disabled).

Add to your `Cargo.toml`:

```toml
# Async only (default features)
ibapi = "5.3"

# Blocking only
ibapi = { version = "5.3", default-features = false, features = ["sync"] }

# Async + blocking together
ibapi = { version = "5.3", default-features = false, features = ["sync", "async"] }
```

When both features are enabled, `ibapi::Client` is the async client; alias the blocking one to use both in the same module:

```rust
use ibapi::Client;                                     // async client
use ibapi::client::blocking::Client as BlockingClient; // blocking client
```

## Available APIs

The [Client documentation](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) lists every available API, covering trading, account management, and market data, with examples.

## Examples

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

Contracts are built with a type-safe builder. Here's how to create a stock contract for TSLA:

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

Required fields are checked at compile time for all contract types:

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

See the [Contract Builder Guide](docs/contract-builder.md) for documentation on all contract types.

For a complete list of contract attributes, explore the [Contract documentation](https://docs.rs/ibapi/latest/ibapi/contracts/struct.Contract.html).

### Requesting Historical Market Data

#### Sync Example

```rust
use time::macros::datetime;
use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL").build();

    let historical_data = client
        .historical_data(&contract, HistoricalBarSize::Hour)
        .ending(datetime!(2023-04-11 20:00 UTC))
        .duration(1.days())
        .what_to_show(HistoricalWhatToShow::Trades)
        .trading_hours(TradingHours::Regular)
        .fetch()
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
        .historical_data(&contract, HistoricalBarSize::Hour)
        .ending(datetime!(2023-04-11 20:00 UTC))
        .duration(1.days())
        .what_to_show(HistoricalWhatToShow::Trades)
        .trading_hours(TradingHours::Regular)
        .fetch()
        .await
        .expect("historical data request failed");

    println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);

    for bar in &historical_data.bars {
        println!("{bar:?}");
    }
}
```

### Requesting Real-Time Market Data

These examples use `iter_data()` (sync) / `filter_data()` (async) to focus on bars; per-subscription notices are filtered (and logged at `warn!`). See [Handling notifications](#handling-notifications) for the pattern when you also want to observe `Notice` items.

#### Sync Example

```rust
use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .realtime_bars(&contract)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .expect("realtime bars request failed!");

    for bar in subscription.iter_data() {
        match bar {
            Ok(bar) => println!("bar: {bar:?}"),
            Err(e) => {
                eprintln!("error: {e:?}");
                break;
            }
        }
    }
}
```

#### Async Example

```rust
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection to TWS failed!");

    // Request real-time bars data for AAPL with 5-second intervals
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .realtime_bars(&contract)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .await
        .expect("realtime bars request failed!");

    let mut data = subscription.filter_data();
    while let Some(item) = data.next().await {
        match item {
            Ok(bar) => println!("bar: {bar:?}"),
            Err(e)  => { eprintln!("error: {e}"); break; }
        }
    }
}
```

In both examples, the request for realtime bars returns a [Subscription](https://docs.rs/ibapi/latest/ibapi/subscriptions/struct.Subscription.html) that can be used as an iterator (sync) or stream (async). The subscription is automatically cancelled when it goes out of scope.

#### Non-blocking Iteration (Sync)

Continuing from the sync example above, `try_iter_data()` polls without blocking:

```rust
use std::time::Duration;

// Example of non-blocking iteration in sync mode
loop {
    match subscription.try_iter_data().next() {
        Some(Ok(bar)) => println!("bar: {bar:?}"),
        Some(Err(e)) => { eprintln!("error: {e}"); break; }
        None => {
            // No new data yet; perform other tasks or sleep
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
```

#### Multiple Subscriptions

Since subscriptions can be converted to iterators, it is easy to iterate over multiple contracts.

```rust
use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection to TWS failed!");

    let contract_aapl = Contract::stock("AAPL").build();
    let contract_nvda = Contract::stock("NVDA").build();

    let subscription_aapl = client
        .realtime_bars(&contract_aapl)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .expect("realtime bars request failed!");
    let subscription_nvda = client
        .realtime_bars(&contract_nvda)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .expect("realtime bars request failed!");

    for (bar_aapl, bar_nvda) in subscription_aapl.iter_data().zip(subscription_nvda.iter_data()) {
        let (bar_aapl, bar_nvda) = match (bar_aapl, bar_nvda) {
            (Ok(a), Ok(n)) => (a, n),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("error: {e}");
                break;
            }
        };
        println!("AAPL {}, NVDA {}", bar_aapl.close, bar_nvda.close);
    }
}
```
> **Note:** When using `zip`, the iteration will stop if either subscription ends. For independent processing, consider handling each subscription separately.

#### One-Shot Snapshot

For a single snapshot rather than a streaming subscription, `snapshot_once(timeout)` requests snapshot mode, collects the ticks until the snapshot completes (or the timeout elapses), and returns them as a `Vec<TickTypes>` — no hand-written collect loop.

```rust
use ibapi::client::blocking::Client;
use ibapi::prelude::*;
use std::time::Duration;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection to TWS failed!");
    let contract = Contract::stock("AAPL").build();

    let ticks = client
        .market_data(&contract)
        .snapshot_once(Duration::from_secs(5))
        .expect("snapshot request failed!");

    for tick in ticks {
        println!("tick: {tick:?}");
    }
}
```

#### Collecting Results

Any `Subscription<T>` can also be collected without a loop: `collect_for(timeout)` returns whatever arrived in time, and `collect_until(timeout, stop)` returns early once the `stop` predicate accepts the items collected so far. For requests that end, such as contract details, `collect_to_end_within(timeout)` collects the whole result or fails with `Error::Timeout`.

Requests that return a value rather than a subscription, such as `historical_data(..).fetch()` or `head_timestamp`, are bounded client-wide instead: set `Client::builder().one_shot_timeout(duration)`, and such a request that TWS hasn't answered in time fails with `Error::Timeout`.

Explore the [Subscription documentation](https://docs.rs/ibapi/latest/ibapi/subscriptions/struct.Subscription.html) for more details.

### Placing Orders

For all supported order types and their usage, see the [Order Types Guide](docs/order-types.md).

#### Sync Example

```rust
use ibapi::client::blocking::Client;
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
        .good_till_canceled()
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

For real-time monitoring of order status, executions, and commissions, set up an order update stream before submitting orders. TWS does not replay order events, so updates sent before the stream exists are lost:

##### Sync Example

```rust
use ibapi::client::blocking::Client;
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

        // `iter_data()` strips notice items; the inner item is `Result<OrderUpdate, Error>`.
        // Use `stream.iter()` with `Ok(SubscriptionItem::Data(_))` if you also want to
        // observe non-fatal notices.
        for update in stream.iter_data() {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!("Order {} Status: {}", status.order_id, status.status);
                    println!("  Filled: {}, Remaining: {}", status.filled, status.remaining);
                    if status.status.is_terminal() {
                        break;
                    }
                }
                Ok(OrderUpdate::OpenOrder(data)) => {
                    println!("Open Order {}: {} {}",
                             data.order_id, data.order.action, data.contract.symbol);
                }
                Ok(OrderUpdate::ExecutionData(exec)) => {
                    println!("Execution: {} shares @ {}",
                             exec.execution.shares, exec.execution.price);
                }
                Ok(OrderUpdate::OrderBound(binding)) => println!("Order binding: {binding:?}"),
                Ok(OrderUpdate::CommissionReport(report)) => {
                    println!("Commission: ${}", report.commission);
                }
                Err(e) => {
                    eprintln!("order update stream error: {e:?}");
                    break;
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
    let order_stream = client.order_update_stream().await.expect("failed to create stream");

    // Spawn task to monitor updates. `filter_data()` strips non-fatal notices so
    // the loop body only sees data items; switch to `order_stream.next()` and a
    // `Ok(SubscriptionItem::Data(_))` arm if you want to observe notices too.
    let monitor_handle = tokio::spawn(async move {
        let mut order_stream = order_stream.filter_data();
        while let Some(update) = order_stream.next().await {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!("Order {} Status: {}", status.order_id, status.status);
                    println!("  Filled: {}, Remaining: {}", status.filled, status.remaining);
                    if status.status.is_terminal() {
                        break;
                    }
                }
                Ok(OrderUpdate::OpenOrder(data)) => {
                    println!("Open Order {}: {} {}",
                             data.order_id, data.order.action, data.contract.symbol);
                }
                Ok(OrderUpdate::ExecutionData(exec)) => {
                    println!("Execution: {} shares @ {}",
                             exec.execution.shares, exec.execution.price);
                }
                Ok(OrderUpdate::OrderBound(binding)) => println!("Order binding: {binding:?}"),
                Ok(OrderUpdate::CommissionReport(report)) => {
                    println!("Commission: ${}", report.commission);
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
- **OrderStatus**: Status changes typed as [`OrderStatusKind`](https://docs.rs/ibapi/latest/ibapi/orders/enum.OrderStatusKind.html) (`Submitted`, `Filled`, `Cancelled`, …) with `is_active()` / `is_terminal()` helpers
- **OpenOrder**: Order details when opened or modified
- **ExecutionData**: Fill notifications with price and quantity
- **OrderBound**: Binding of an API order ID to a TWS permanent ID
- **CommissionReport**: Commission charges for executions

Per-order TWS warnings (e.g. quote-throttling codes in the 2100 band scoped to an order) are not delivered through `OrderUpdate`.

They arrive as `SubscriptionItem::Notice` only on the subscription returned by the lower-level `client.place_order(order_id, &contract, &order)`; `submit()` returns just the order ID and does not expose them. See [Handling notifications](#handling-notifications).

## Handling notifications

TWS emits two flavors of notification alongside subscription data. Routing depends on whether a notice carries a `request_id`, not on its code alone:

- **Per-subscription notices** — warnings in the 2100..=2199 band and order-cancel code 202 that carry a `request_id` mapping back to a specific subscription. They arrive on that subscription as `SubscriptionItem::Notice(_)`; the stream stays open.
  - **Scanner "no items retrieved"** — an empty scan (common before regular hours) reports error code 165 with that text. A scanner subscription delivers it as `SubscriptionItem::Notice(_)` and stays open; later batches, including empty ones, still arrive. The notice's `category()` is still `Error`, so don't end a scanner loop on `!notice.is_informational()` alone. On other requests, and with any other text, 165 still ends the stream.
- **Globally routed notices** — connectivity codes 1100/1101/1102 and farm-status codes (2103–2108, 2119, 2157, 2158 — inside the 2100 band, but sent without a `request_id`) are not delivered to any subscription; subscribe via [`Client::notice_stream()`](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.notice_stream) instead.

### Per-subscription notices

`Subscription<T>::next()` returns `Option<Result<SubscriptionItem<T>, Error>>`, so a single match handles data, notices, and terminal errors:

#### Sync Example

```rust
use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    let contract = Contract::stock("AAPL").build();
    let subscription = client
        .realtime_bars(&contract)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .expect("realtime bars request failed");

    for item in &subscription {
        match item {
            Ok(SubscriptionItem::Data(bar))    => println!("bar: {bar:?}"),
            Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
            Err(e)                             => { eprintln!("error: {e}"); break; }
        }
    }
}
```

#### Async Example

```rust
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client
        .realtime_bars(&contract)
        .trading_hours(TradingHours::Extended)
        .subscribe()
        .await
        .expect("realtime bars request failed");

    while let Some(item) = subscription.next().await {
        match item {
            Ok(SubscriptionItem::Data(bar))    => println!("bar: {bar:?}"),
            Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
            Err(e)                             => { eprintln!("error: {e}"); break; }
        }
    }
}
```

### Filtering vs. observing

Pick the iterator/stream shape based on whether the call site cares about notices:

| Want                                  | Sync                            | Async                          |
|---------------------------------------|---------------------------------|--------------------------------|
| **Data only** (notices logged at warn) | `subscription.iter_data()` / `next_data()` | `subscription.filter_data()` (then `.next().await`) |
| **Data + notices** (full visibility)   | `subscription.iter()` / `next()` | `subscription.next().await` (matches on `SubscriptionItem`) |

Most call sites (downstream business logic, indicators, paper-trading loops) want `iter_data()` / `filter_data()` — notices are observability concerns and already log at `warn!`. UI status indicators, custom logging, and audit pipelines match on `SubscriptionItem::Data` vs `SubscriptionItem::Notice` directly.

### Handshake-time notices

Farm-status codes typically arrive *before* `Client::connect` returns, so a `notice_stream` registered afterwards will not see that initial burst. To capture handshake-time notices — useful for connection-state UIs and startup telemetry — use the builder's `connect_with_notice_stream` terminal instead of `connect`:

```rust
use ibapi::client::blocking::Client;
use ibapi::ConnectivityStatus;

fn main() {
    let (client, notices) = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(100)
        .connect_with_notice_stream()
        .expect("connection failed");

    for n in notices.iter() {
        match n.connectivity_status() {
            // Data-farm sub-state, refined from the 2100..=2199 warning band.
            Some(ConnectivityStatus::Ok) => println!("farm online: {n}"),
            Some(ConnectivityStatus::Broken) => println!("farm down: {n}"),
            Some(ConnectivityStatus::Inactive | ConnectivityStatus::Connecting) => println!("farm idle: {n}"),
            // Connectivity codes 1100/1101/1102 and everything else.
            None if n.is_system_message() => println!("connectivity: {n}"),
            None => println!("notice: {n}"),
        }
    }
    drop(client);
}
```

The stream survives auto-reconnects and receives the same notices as [`Client::notice_stream`](https://docs.rs/ibapi/latest/ibapi/struct.Client.html#method.notice_stream), which you can call after connecting instead if you don't need the handshake burst.

## Multi-Threading

The [Client](https://docs.rs/ibapi/latest/ibapi/struct.Client.html) can be shared between threads with `Arc`:

```rust
use std::sync::Arc;
use std::thread;
use ibapi::client::blocking::Client;
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
                .realtime_bars(&contract)
                .trading_hours(TradingHours::Extended)
                .subscribe()
                .expect("realtime bars request failed!");

            for bar in subscription.iter_data() {
                match bar {
                    Ok(bar) => println!("bar: {bar:?}"),
                    Err(e) => {
                        eprintln!("error: {e:?}");
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
```

Some TWS API calls have no request ID, so their responses are routed by message type. Every live subscription for a message type receives every response of that type: concurrent requests of the same kind, or of kinds that share response types such as `open_orders` and `all_open_orders`, see each other's responses, and nothing identifies which request caused which response.

Either serialize those requests yourself, or give each thread its own client. Each client needs a distinct client ID and receives only the responses to its own requests:

```rust
use std::thread;
use ibapi::client::blocking::Client;
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
                .realtime_bars(&contract)
                .trading_hours(TradingHours::Extended)
                .subscribe()
                .expect("realtime bars request failed!");

            for bar in subscription.iter_data() {
                match bar {
                    Ok(bar) => println!("bar: {bar:?}"),
                    Err(e) => {
                        eprintln!("error: {e:?}");
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    handles.into_iter().for_each(|handle| handle.join().unwrap());
}
```

## Fault Tolerance

When the connection drops, the client:

- reconnects up to 20 times with Fibonacci backoff (configurable with `ClientBuilder::max_reconnect_attempts` and `ClientBuilder::reconnect_forever`);
- retries one-shot requests that are safe to repeat, such as `server_time` and `managed_accounts`;
- re-sends the market data type set with `switch_market_data_type`, since TWS does not keep it across reconnects.

Subscriptions are not resumed. A stream open when the connection drops ends right away, before the reconnect, with `Some(Err(Error::ConnectionReset))` from `Subscription::next()`. Wait for the reconnect with `Client::wait_connected`, then resubscribe; `wait_connected` returns `Error::ConnectionFailed` once the reconnect gives up:

```rust
use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection to TWS failed!");
    let contract = Contract::stock("AAPL").build();

    'outer: loop {
        // Waits out a reconnect in progress; fails only once the client gave
        // up (ConnectionFailed) or was shut down (Shutdown).
        if let Err(e) = client.wait_connected() {
            eprintln!("connection gone: {e}");
            return;
        }

        let subscription = match client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe() {
            Ok(subscription) => subscription,
            // Dropped again before the request went out: wait again.
            Err(e) if e.is_connection_lost() => continue,
            Err(e) => {
                eprintln!("realtime bars request failed: {e}");
                return;
            }
        };

        for item in &subscription {
            match item {
                Ok(SubscriptionItem::Data(bar)) => println!("bar: {bar:?}"),
                Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
                Err(e) if e.is_connection_lost() => {
                    eprintln!("Connection lost. Resubscribing after the reconnect...");
                    continue 'outer;
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            }
        }
        return;
    }
}
```

## Contributions

We welcome contributions of all kinds: new ideas, bug fixes, or documentation improvements. If you'd like to contribute, please start by reviewing our [contributor documentation](https://github.com/wboayue/rust-ibapi/blob/main/CONTRIBUTING.md).

If you encounter a bug or require a missing feature, please review the [issues list](https://github.com/wboayue/rust-ibapi/issues) before submitting a new one.

For questions or discussions about contributions, feel free to open an issue or reach out via our [GitHub discussions page](https://github.com/wboayue/rust-ibapi/discussions).
