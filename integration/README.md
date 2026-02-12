# Integration Tests

Integration tests for the `ibapi` crate. These tests run against a live IB Gateway or TWS instance.

## Prerequisites

A running IB Gateway or TWS connected to a **paper trading** account.

| Platform          | Port |
|-------------------|------|
| IB Gateway Paper  | 4002 |
| IB Gateway Live   | 4001 |
| TWS Paper         | 7497 |
| TWS Live          | 7496 |

Paper trading (port 4002) is recommended.

## Structure

```
integration/
├── common/         # ibapi-test: shared test helpers
│   └── src/lib.rs  # ClientId pool, rate limiter
├── async/          # async client tests and binaries
│   ├── src/lib.rs  # shared helpers
│   ├── tests/      # test modules (cargo test)
│   └── bin/        # standalone binaries (cargo run)
└── sync/           # sync client tests and binaries
    ├── src/lib.rs
    ├── tests/
    └── bin/
```

## Test Helpers (`ibapi-test`)

### Client ID Pool

Tests run in parallel and each need a unique client ID to avoid conflicts. Use `ClientId::get()` to acquire one — it's returned to the pool automatically when dropped.

```rust
use ibapi_test::ClientId;

#[test]
fn my_test() {
    let client_id = ClientId::get();
    let client = Client::connect("127.0.0.1:4002", client_id.id())
        .expect("connection failed");
    // client_id returned to pool when test ends
}
```

IDs are allocated from range 200–399 to avoid conflicts with manual testing.

### Rate Limiter

IBKR enforces a 50 requests/second limit. Call `rate_limit()` before each API request to stay under the limit. Uses a token bucket — the first 50 requests pass instantly, then requests are spaced to maintain the average.

```rust
use ibapi_test::{rate_limit, ClientId};

#[test]
fn my_test() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id())
        .expect("connection failed");

    rate_limit();
    let time = client.server_time().expect("failed to get server time");
}
```

### Test Serialization

Most tests run in parallel. Use `#[serial(group)]` for tests that modify shared gateway state.

```rust
use serial_test::serial;

// Runs in parallel (default)
#[test]
fn reads_contract_details() { ... }

// Runs serially within the "orders" group
#[test]
#[serial(orders)]
fn places_order() { ... }
```

See [docs/integration-tests.md](../docs/integration-tests.md) for full guidelines.

## Running Tests

```bash
# All integration tests
just integration

# Sync only
just integration-sync

# Async only
just integration-async

# Or directly with cargo
cargo test -p ibapi-integration-sync
cargo test -p ibapi-integration-async
```

## Running Binaries

```bash
cargo run -p ibapi-integration-async --bin <name>
cargo run -p ibapi-integration-sync --bin <name>
```

## Environment Variables

```bash
# Enable debug logging
RUST_LOG=debug just integration-sync
```
