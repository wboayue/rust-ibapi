# Migration Guide: 1.x to 2.x

This guide walks through the changes required to upgrade from rust-ibapi v1.2.2 to v2.0.0.

## Highlights

- Async client is production ready and enabled by default.
- Blocking client remains available; when built alongside async it lives under `client::blocking`.
- Contracts, market data, and orders now use fluent, type-safe builders.
- Trading hour configuration uses the `TradingHours` enum instead of bare booleans.
- Tracing, request helpers, and integration coverage have been expanded for both execution models.

## Choose Your Execution Model

Version 2.0 ships both asynchronous (Tokio) and blocking (threaded) clients. The async client is on by default, so a bare dependency activates it automatically.

| Scenario | Cargo.toml snippet | Primary client type |
|----------|-------------------|---------------------|
| Async only (default) | `ibapi = "2.0"` | `ibapi::Client` (async) |
| Blocking only | `ibapi = { version = "2.0", default-features = false, features = ["sync"] }` | `ibapi::client::blocking::Client` |
| Both clients | `ibapi = { version = "2.0", default-features = false, features = ["sync", "async"] }` | Async: `ibapi::Client`; Blocking: `ibapi::client::blocking::Client` |

When both features are enabled the top-level `ibapi::Client` continues to refer to the async implementation. Import the blocking client explicitly:

```rust
use ibapi::Client;                    // async client
use ibapi::client::blocking::Client;  // blocking client
```

If you disable default features without opting back into `sync` or `async`, the build fails with a compile error that lists the supported combinations.

## Breaking Changes

### 1. Blocking API moved under `client::blocking`

In 1.x the `Client` type was blocking by default. In 2.0 the async client owns the root name and the blocking client, subscriptions, and trace helpers live under a `blocking` module.

```rust
// v1.x
use ibapi::Client;

// v2.0 (blocking)
use ibapi::client::blocking::Client;
let client = Client::connect("127.0.0.1:4002", 100)?;
```

Related helpers gained matching namespaces when both features are compiled (for example `trace::blocking::record_request` and `client::blocking::Subscription`). Code that only enables the `sync` feature continues to work without the extra module path.

### 2. Contract builders now require `.build()`

The contract API switched to fluent, type-safe builders. Every builder call must end with `.build()`:

```rust
// v1.x
let contract = Contract::stock("AAPL");

// v2.0
let contract = Contract::stock("AAPL").build();
let futures = Contract::futures("ES").front_month().build();
```

This change ensures required fields are set at compile time and prevents partially constructed contracts.

### 3. Market data subscriptions use a fluent builder

`Client::market_data` now returns a builder that configures the subscription before you call `.subscribe()`.

```rust
// v1.x
let sub = client.market_data(&contract, &["233"], false, false)?;

// v2.0 (blocking)
let sub = client.market_data(&contract)
    .generic_ticks(&["233"])
    .subscribe()?;

// v2.0 (async)
let mut sub = client.market_data(&contract)
    .snapshot()
    .subscribe()
    .await?;
```

Snapshot mode, regulatory snapshots, and streaming toggles are now explicit builder methods, improving readability and discoverability.

### 4. `TradingHours` enum replaces `use_rth` booleans

All APIs that accepted a `bool` for regular versus extended trading hours now take the `TradingHours` enum:

```rust
// v1.x
client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, true)?;

// v2.0
use ibapi::market_data::TradingHours;

client.realtime_bars(
    &contract,
    BarSize::Sec5,
    WhatToShow::Trades,
    TradingHours::Regular,
)?;
```

The enum makes intent explicit, aligns with documentation, and leaves room for additional hour modes.

### 5. Order placement offers a fluent builder (optional but recommended)

You can continue to construct `Order` manually, but the new builder dramatically reduces boilerplate and validates combinations at compile time:

```rust
use ibapi::orders::order_builder::OrderBuilder;
use ibapi::orders::Action;

let order = OrderBuilder::market(Action::Buy, 100).build();
```

Builders exist for market, limit, stop, bracket, and combination orders, mirroring Interactive Brokers terminology.

### 6. Tracing helpers renamed when async is present

If you call the trace API while building both clients, switch to the blocking namespace:

```rust
// v1.x
trace::record_request("REQ|123|AAPL|".into());

// v2.0 (blocking with async feature also enabled)
trace::blocking::record_request("REQ|123|AAPL|".into());
```

Async tracing is available directly as `trace::record_request` and uses asynchronous storage under the hood.

## Quick Migration Checklist

1. Update your dependency according to the execution model you need.
2. Adjust imports to use `client::blocking::Client` (and `trace::blocking`, `market_data::blocking`, etc.) when compiling both features.
3. Add `.build()` to every contract builder chain.
4. Switch market data calls to the new fluent builder with `.subscribe()`.
5. Replace `use_rth` booleans with `TradingHours`.
6. (Optional) Adopt the order builder for new code and tests.
7. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features`, and your test suite for each feature flag you support.

## Trying the async client

```toml
[dependencies]
ibapi = "2.0"
tokio = { version = "1", features = ["full"] }
```

```rust
use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    let time = client.server_time().await?;
    println!("Server time: {time:?}");
    Ok(())
}
```

To exercise both clients in CI, run:

```bash
cargo test                 # async (default)
cargo test --no-default-features --features sync
cargo test --all-features  # async + blocking
```

## New capabilities in 2.0

- Production-ready async client with reconnection helpers and `Client::is_connected()`.
- Fluent builders for contracts, market data subscriptions, and order placement.
- Safer trading hour handling via the `TradingHours` enum.
- Expanded integration tests, recorded fixtures, and improved error messages.
- Interaction recording that works uniformly in both async and blocking modes.

## Need help?

- Examples: `examples/async` and `examples/sync`
- Documentation: <https://docs.rs/ibapi/2.0.0>
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
