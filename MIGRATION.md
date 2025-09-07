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

### 2. New Contract Builder API (v2)

The contract creation API has been completely redesigned for better type safety and ergonomics.

#### Before (v1.x)
```rust
use ibapi::contracts::Contract;

// Old API - less type safe
let contract = Contract {
    symbol: "AAPL".to_string(),
    security_type: "STK".to_string(),
    exchange: "SMART".to_string(),
    currency: "USD".to_string(),
    ..Default::default()
};
```

#### After (v2.x)
```rust
use ibapi::contracts::Contract;

// New API - type-safe builder pattern
let contract = Contract::stock("AAPL").build();

// With customization
let contract = Contract::stock("7203")
    .on_exchange("TSEJ")
    .in_currency("JPY")
    .build();
```

#### Key Improvements

1. **Type-safe builders**: Separate builders for each contract type
2. **Required fields enforced**: Can't build invalid contracts
3. **Smart defaults**: Less boilerplate for common cases
4. **Better discovery**: IDE autocomplete guides you

For detailed migration instructions, see the [Contract Builder Guide](docs/contract-builder.md).

### 3. TradingHours Enum Replaces Boolean Parameters

All market data methods that previously used `use_rth: bool` now use the `TradingHours` enum for better type safety and clarity.

#### Before (v1.x)
```rust
use ibapi::Client;

let client = Client::connect("127.0.0.1:4002", 100)?;
let contract = Contract::stock("AAPL").build();

// Old API with boolean parameter
let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, true)?;  // true for RTH
let data = client.historical_data(&contract, None, 1.days(), BarSize::Hour, WhatToShow::Trades, false)?;  // false for extended hours
```

#### After (v2.x)
```rust
use ibapi::Client;
use ibapi::market_data::TradingHours;

let client = Client::connect("127.0.0.1:4002", 100)?;
let contract = Contract::stock("AAPL").build();

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

All v1.x users were using the synchronous API. You'll need to make minor updates:

1. **Update Cargo.toml** - add explicit feature selection:
```toml
[dependencies]
ibapi = { version = "2.0", features = ["sync"] }
```

2. **Update contract creation** - use the new builder API:
```rust
// Old (v1.x)
let contract = Contract::stock("AAPL");

// New (v2.x)
let contract = Contract::stock("AAPL").build();
```

3. **Update trading hours parameters** - use enum instead of bool:
```rust
// Old (v1.x)
client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, true)?;

// New (v2.x)
use ibapi::market_data::TradingHours;
client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Regular)?;
```

Your updated code:
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

1. **Async support**: Choose between sync and async implementations
2. **Type-safe contract builder**: New builder API with compile-time validation
3. **Improved type safety**: TradingHours enum replaces boolean parameters
4. **Trace functionality**: Record interactions when debug logging is enabled
5. **Better error messages**: More descriptive errors throughout

## Getting Help

- Check examples in `/examples` (sync) and `/examples/async` directories
- File issues at: https://github.com/wboayue/rust-ibapi/issues
- See full documentation at: https://docs.rs/ibapi/2.0.0

## Summary

Migration from v1.x to v2.x requires these changes:

1. **Update Cargo.toml**: Add `features = ["sync"]` to your dependency
2. **Update contract creation**: Add `.build()` to contract factory methods
3. **Update trading hours**: Replace `bool` with `TradingHours` enum
4. **Run `cargo build`** to catch any remaining issues

The changes are minimal and mostly mechanical - your application logic remains the same!