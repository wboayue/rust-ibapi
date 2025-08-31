# Migration Guide: 1.x to 2.x

This guide helps you migrate from rust-ibapi v1.x (last version: v1.2.2) to v2.x.

## Major New Feature: Async Support

Version 2.x introduces first-class async support! You can now choose between synchronous (thread-based) and asynchronous (tokio-based) implementations.

## Breaking Changes

### 1. Explicit Feature Selection Required

In v2.x, you must explicitly choose between `sync` and `async` features. There is no longer a default feature.

#### Before (v1.x)
```toml
# Cargo.toml
[dependencies]
ibapi = "1.2"  # Only sync was available
```

#### After (v2.x)
```toml
# Cargo.toml
[dependencies]
# For synchronous (blocking) API - same behavior as v1.x:
ibapi = { version = "2.0", features = ["sync"] }

# OR for the new asynchronous API:
ibapi = { version = "2.0", features = ["async"] }
```

#### Why This Change?

1. **Clarity**: Makes it explicit which execution model you're using
2. **Smaller binaries**: Only includes the dependencies you actually need  
3. **Clean separation**: Sync and async are truly independent implementations
4. **Future flexibility**: Allows for divergent optimizations per mode

#### Compilation Errors

If you upgrade without specifying a feature, you'll see:
```
error: Either 'sync' or 'async' feature must be enabled.
       Use: features = ["sync"] or features = ["async"]
```

### 2. TradingHours Enum Replaces Boolean Parameters

All market data methods that previously used `use_rth: bool` now use the `TradingHours` enum for better type safety and clarity.

#### Before (v1.x)
```rust
use ibapi::Client;

let client = Client::connect("127.0.0.1:4002", 100)?;
let contract = Contract::stock("AAPL");

// Old API with boolean parameter
let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, true)?;  // true for RTH
let data = client.historical_data(&contract, None, 1.days(), BarSize::Hour, WhatToShow::Trades, false)?;  // false for extended hours
```

#### After (v2.x)
```rust
use ibapi::Client;
use ibapi::market_data::TradingHours;

let client = Client::connect("127.0.0.1:4002", 100)?;
let contract = Contract::stock("AAPL");

// New API with TradingHours enum
let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Regular)?;
let data = client.historical_data(&contract, None, 1.days(), BarSize::Hour, WhatToShow::Trades, TradingHours::Extended)?;
```

#### Affected Methods

The following methods now use `TradingHours` instead of `use_rth: bool`:

- `Client::realtime_bars()`
- `Client::head_timestamp()`
- `Client::historical_data()`
- `Client::historical_ticks_bid_ask()`
- `Client::historical_ticks_mid_point()`
- `Client::historical_ticks_trade()`
- `Client::histogram_data()`

#### Why This Change?

1. **Type safety**: Can't accidentally pass the wrong boolean value
2. **Self-documenting**: `TradingHours::Regular` is clearer than `true`
3. **Future extensibility**: Easy to add more trading hour options if needed
4. **IDE support**: Better autocomplete and documentation

## Quick Migration Steps

### For Existing v1.x Users

All v1.x users were using the synchronous API. Your code remains unchanged:

```rust
use ibapi::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("127.0.0.1:4002", 100)?;
    let time = client.server_time()?;
    // ... rest of your code works exactly the same
}
```

Just update your `Cargo.toml`:
```toml
[dependencies]
ibapi = { version = "2.0", features = ["sync"] }
```

### Trying the New Async API

If you want to try the new async support:
```toml
[dependencies]
ibapi = { version = "2.0", features = ["async"] }
tokio = { version = "1", features = ["full"] }
```

```rust
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    let time = client.server_time().await?;
    // ... async version of your code
}
```

## Feature Comparison

| Feature | v1.x | v2.x |
|---------|------|------|
| Default | `sync` | None (must choose) |
| Sync + Async | `async` overrides `sync` | Not allowed together |
| Feature guards | `#[cfg(all(feature = "sync", not(feature = "async")))]` | `#[cfg(feature = "sync")]` |

## Common Issues and Solutions

### Issue: Both features enabled
```toml
# This will cause a compilation error in v2.x
ibapi = { version = "2.0", features = ["sync", "async"] }
```

**Solution**: Choose one:
```toml
ibapi = { version = "2.0", features = ["sync"] }  # OR "async"
```

### Issue: Conditional compilation in your code
If you have code like:
```rust
#[cfg(feature = "async")]
use tokio;
```

This will continue to work. However, you no longer need complex patterns like:
```rust
#[cfg(all(feature = "sync", not(feature = "async")))]
```

### Issue: Workspace dependencies
If you're using workspace dependencies:
```toml
# workspace Cargo.toml
[workspace.dependencies]
ibapi = { version = "2.0", features = ["sync"] }

# member Cargo.toml
[dependencies]
ibapi.workspace = true
```

## New Features in v2.x

While migrating, you might want to take advantage of new features:

1. **Improved async support**: Pre-created broadcast channels eliminate race conditions
2. **Trace functionality**: Record interactions when debug logging is enabled
3. **Better error messages**: More descriptive errors throughout

## Getting Help

- Check examples in `/examples` (sync) and `/examples/async` directories
- File issues at: https://github.com/wboayue/rust-ibapi/issues
- See full documentation at: https://docs.rs/ibapi/2.0.0

## Summary

For most users, migration is as simple as:

1. Update version to `2.0`
2. Add `features = ["sync"]` to your dependency
3. Run `cargo build` to verify

That's it! Your existing code should work without modifications.