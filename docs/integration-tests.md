# Integration Tests

Integration tests live in `integration/` as separate workspace crates that depend on `ibapi` as an external dependency. They run against a live IB Gateway or TWS instance.

See also: [integration/README.md](../integration/README.md) for setup and running instructions.

## Crate Layout

| Crate | Path | Purpose |
|-------|------|---------|
| `ibapi-test` | `integration/common/` | Shared helpers (ClientId pool, market-hours checks) |
| `ibapi-integration-sync` | `integration/sync/` | Sync client tests; `connect` / `client_builder` helpers in `src/lib.rs` |
| `ibapi-integration-async` | `integration/async/` | Async client tests; `connect` / `client_builder` helpers in `src/lib.rs` |

Tests go in `tests/`. `ibapi-test` has no `ibapi` dependency, so helpers that need `ibapi` live in each test crate's `src/lib.rs`.

## Writing a Sync Test

```rust
use ibapi_integration_sync::connect;
use ibapi_test::ClientId;

#[test]
fn my_sync_test() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let time = client.server_time().expect("failed to get server time");
    assert!(time.year() >= 2025);
}
```

## Writing an Async Test

```rust
use ibapi_integration_async::connect;
use ibapi_test::ClientId;

#[tokio::test]
async fn my_async_test() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

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
let client = connect(&client_id);

// Bad — hardcoded ID, and bypasses the shared rate limiter
let client = Client::connect("127.0.0.1:4002", 100)?;
```

### 2. Connect with `connect` or `client_builder`

IBKR limits API messages to 50 per second across all clients of a gateway. The crate's `connect(&client_id)` installs a `RateLimiter` shared by every client in the test binary, so each request, order and cancel is paced at the transport. Don't pace requests by hand.

Need builder options (`startup_callback`, `channel_capacity`, `connect_with_notice_stream`, ...)? Start from `client_builder(&client_id)`, which has the address, client ID and limiter set. Never call `Client::connect` directly: that client would bypass the shared budget.

```rust
let client = connect(&client_id);
let time = client.server_time()?;
let details = client.contract_details(&contract)?;

let (client, notices) = client_builder(&client_id).connect_with_notice_stream()?;
```

Handshakes are not throttled; a handful of parallel connects stays well under the limit.

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
let client = connect(&client_id);
// ... use client ...

// Bad — the temporary guard drops at the end of the statement, returning
// the ID to the pool while the client still uses it
let client = connect(&ClientId::get());
```

## Running

```bash
just integration          # all integration tests
just integration-sync     # sync only
just integration-async    # async only
```
