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
│   └── src/lib.rs  # ClientId pool, market-hours checks
├── async/          # async client tests
│   ├── src/lib.rs  # connect / client_builder, shared helpers
│   └── tests/      # test modules (cargo test)
└── sync/           # sync client tests
    ├── src/lib.rs
    └── tests/
```

## Test Helpers (`ibapi-test`)

### Client ID Pool

Tests run in parallel and each need a unique client ID to avoid conflicts. Use `ClientId::get()` to acquire one — it's returned to the pool automatically when dropped.

```rust
use ibapi_test::ClientId;

#[test]
fn my_test() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
    // client_id returned to pool when test ends
}
```

IDs are allocated from range 200–399 to avoid conflicts with manual testing.

### Rate Limiting

IBKR limits API messages to 50 per second. Connect with the test crate's `connect(&client_id)` (or `client_builder(&client_id)` for builder options) rather than `Client::connect`: it installs an `ibapi::RateLimiter` shared by every client in the test binary, which paces each request, order and cancel at the transport.

```rust
use ibapi_integration_sync::connect;
use ibapi_test::ClientId;

#[test]
fn my_test() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
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

## Environment Variables

```bash
# Enable debug logging
RUST_LOG=debug just integration-sync
```
