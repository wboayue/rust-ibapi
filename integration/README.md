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
├── async/          # async client tests and binaries
│   ├── src/lib.rs  # shared helpers
│   ├── tests/      # test modules (cargo test)
│   └── bin/        # standalone binaries (cargo run)
└── sync/           # sync client tests and binaries
    ├── src/lib.rs
    ├── tests/
    └── bin/
```

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
