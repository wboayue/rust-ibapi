# Feature Flags

`ibapi` ships with the asynchronous client enabled by default. The optional `sync` feature adds the blocking client and can be enabled on its own (after disabling defaults) or alongside the async client.

## Configurations at a Glance

| Mode | Command | Primary client path | Blocking client path |
|------|---------|---------------------|----------------------|
| Default async | `cargo build` | `client::Client` | – |
| Sync only | `cargo build --no-default-features --features sync` | `client::Client` | – |
| Async + sync | `cargo build --all-features` (or `--no-default-features --features "sync async"`) | `client::Client` (async) | `client::blocking::Client` |

## Available Features

- **`async`** (default): Tokio-based, non-blocking client and supporting types.
- **`sync`**: Threaded client using crossbeam channels, plus blocking subscription helpers.

## Feature Guard Patterns

Use explicit guards so intent stays clear in each configuration:

```rust
#[cfg(feature = "sync")]
use std::thread;

#[cfg(feature = "async")]
use tokio::task;

// Code that should only compile when sync is enabled without async
#[cfg(all(feature = "sync", not(feature = "async")))]
fn sync_only_behavior() {}
```

The crate enforces that at least one of the two features is enabled; both may be active simultaneously.

## Usage in Cargo.toml

```toml
[dependencies]
# Default async client
ibapi = "2.0"

# Sync-only (disable defaults)
ibapi = { version = "2.0", default-features = false, features = ["sync"] }

# Async + blocking
ibapi = { version = "2.0", features = ["sync"] }
```

## Testing with Features

Always exercise the configurations your change touches:

```bash
# Default async build
cargo test

# Sync-only build
cargo test --no-default-features --features sync

# Combined build
cargo test --all-features
```

Apply the same pattern to `cargo clippy` and other verification commands.

## Key Differences

### Sync Mode
- Uses `std::thread` for concurrency
- Crossbeam channels for communication
- Blocking I/O operations
- Returns `Result<T, Error>`
- Subscriptions implement `Iterator`

### Async Mode
- Uses the `tokio` runtime
- `tokio::sync` primitives
- Non-blocking I/O with `.await`
- Returns `Result<T, Error>` (with `.await`)
- Subscriptions implement `Stream`
