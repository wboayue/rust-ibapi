# Integration Tests

Integration tests live in `integration/` as separate workspace crates that depend on `ibapi` as an external dependency. They run against a live IB Gateway or TWS instance.

See also: [integration/README.md](../integration/README.md) for setup and running instructions.

## Crate Layout

| Crate | Path | Purpose |
|-------|------|---------|
| `ibapi-test` | `integration/common/` | Shared helpers (ClientId pool, rate limiter) |
| `ibapi-integration-sync` | `integration/sync/` | Sync client tests and binaries |
| `ibapi-integration-async` | `integration/async/` | Async client tests and binaries |

Tests go in `tests/`, standalone binaries in `bin/`.

## Writing a Sync Test

```rust
use std::sync::{Arc, Mutex};

use ibapi::client::blocking::Client;
use ibapi_test::{rate_limit, ClientId};

#[test]
fn my_sync_test() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id())
        .expect("connection failed");

    rate_limit();
    let time = client.server_time().expect("failed to get server time");
    assert!(time.year() >= 2025);
}
```

## Writing an Async Test

```rust
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId};

#[tokio::test]
async fn my_async_test() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id())
        .await
        .expect("connection failed");

    rate_limit();
    let time = client.server_time().await.expect("failed to get server time");
    assert!(time.year() >= 2025);
}
```

## Required Patterns

### 1. Use `ClientId::get()` for every connection

Never hardcode client IDs. `ClientId::get()` allocates a unique ID (range 200–399) and returns it to the pool when dropped. This enables parallel test execution without conflicts.

```rust
// Good
let client_id = ClientId::get();
let client = Client::connect("127.0.0.1:4002", client_id.id())?;

// Bad — will conflict with parallel tests
let client = Client::connect("127.0.0.1:4002", 100)?;
```

### 2. Call `rate_limit()` before every API request

IBKR enforces a 50 requests/second limit. Call `rate_limit()` before each call that sends a message to the gateway (connect, server_time, market_data, place_order, etc.).

```rust
rate_limit();
let client = Client::connect("127.0.0.1:4002", client_id.id())?;

rate_limit();
let time = client.server_time()?;

rate_limit();
let details = client.contract_details(&contract)?;
```

### 3. Use `#[serial]` for tests that modify shared gateway state

Most tests run in parallel by default. Use `#[serial(group)]` from `serial_test` when tests share mutable gateway state (e.g., orders, account subscriptions). Tests within the same group run serially; different groups and unmarked tests still run in parallel.

```rust
use serial_test::serial;

// Parallel (default) — read-only operations
#[test]
fn reads_market_data() { ... }

// Serial within "orders" group — tests that place/cancel orders
#[test]
#[serial(orders)]
fn places_order() { ... }

#[test]
#[serial(orders)]
fn cancels_order() { ... }

// Serial within "account" group — independent of "orders"
#[test]
#[serial(account)]
fn account_updates() { ... }
```

Common groups:
- `orders` — order placement, modification, cancellation
- `account` — account subscriptions and updates

Only serialize when necessary. Read-only operations (market data, contract details, server time) should remain parallel.

### 4. Keep `ClientId` alive for the duration of the connection

The `ClientId` guard returns its ID to the pool on drop. Ensure it outlives the client.

```rust
// Good — client_id lives for the entire test
let client_id = ClientId::get();
let client = Client::connect("127.0.0.1:4002", client_id.id())?;
// ... use client ...

// Bad — ID returned to pool immediately, another test could reuse it
let id = ClientId::get().id();
let client = Client::connect("127.0.0.1:4002", id)?;
```

## Running

```bash
just integration          # all integration tests
just integration-sync     # sync only
just integration-async    # async only
```
