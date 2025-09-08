# Asynchronous Examples

This directory contains asynchronous examples demonstrating how to use the rust-ibapi library with the async feature using Tokio.

## Running Examples

All async examples require the async feature flag and can be run using:

```bash
cargo run --features async --example async_<example_name>
```

For example:
```bash
cargo run --features async --example async_connect
```

## Connection

- **connect** - Basic async connection to TWS/IB Gateway
- **managed_accounts** - List managed accounts asynchronously

## Account & Portfolio

- **account_summary** - Account summary information using async/await
- **positions** - Get current positions asynchronously
- **pnl** - Real-time P&L updates with async streams

## Market Data - Real-time

- **market_data** - Real-time market data with async streams
- **market_depth** - Level II market depth asynchronously
- **tick_by_tick** - Tick-by-tick data (trades, bid/ask, midpoint)
- **tick_by_tick_last** - Tick-by-tick last trades only
- **realtime_bars** - 5-second real-time bars with async

## Market Data - Historical

- **historical_data** - Historical bar data retrieval
- **historical_ticks** - Historical tick data (all types)
- **historical_ticks_trade** - Historical trade ticks
- **historical_ticks_midpoint** - Historical midpoint ticks
- **historical_schedule** - Trading schedule information
- **head_timestamp** - Earliest available data timestamp
- **histogram_data** - Price distribution histogram

## Wall Street Horizon (WSH) Events

- **wsh_metadata** - WSH metadata asynchronously
- **wsh_event_data_by_contract** - WSH events for a specific contract
- **wsh_event_data_by_filter** - WSH events by filter with streaming

## Testing & Debugging

- **test_multiple_calls** - Test multiple sequential async calls

## Key Differences from Sync Examples

1. **Async/Await Syntax**: All methods use `.await` for asynchronous operations
2. **Tokio Runtime**: Examples use `#[tokio::main]` attribute
3. **Streams vs Iterators**: Subscriptions return `Stream` instead of `Iterator`
4. **Concurrent Operations**: Can easily run multiple operations concurrently

## Example Structure

Most async examples follow this pattern:

```rust
use ibapi::Client;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect asynchronously
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    
    // Make async API calls
    let result = client.some_method().await?;
    
    // Handle streams
    let mut stream = client.some_subscription().await?;
    while let Some(item) = stream.next().await {
        println!("{:?}", item?);
    }
    
    Ok(())
}
```

## Prerequisites

Same as sync examples:
1. Install IB Gateway or TWS
2. Enable API connections in the settings
3. Note the port number (default: 4002 for IB Gateway, 7497 for TWS)

## Dependencies

The async examples require these additional dependencies (already included when using `--features async`):
- `tokio` - Async runtime
- `futures` - Stream utilities
- `async-trait` - Async trait support

## Common Patterns

### Concurrent Requests
```rust
use futures::future::join_all;

// Make multiple requests concurrently
let futures = vec![
    client.method1(),
    client.method2(),
    client.method3(),
];
let results = join_all(futures).await;
```

### Stream Processing
```rust
use futures::StreamExt;

let mut stream = client.market_data(&contract)
    .generic_ticks(&["233"])  // RTVolume
    .subscribe()
    .await?;
while let Some(tick) = stream.next().await {
    match tick? {
        Tick::Price(price) => println!("Price: {}", price),
        Tick::Size(size) => println!("Size: {}", size),
        _ => {}
    }
}
```

## Known Issues

- **Hanging on Second Call**: There's a known issue where some async examples may hang on the second sequential call. This is being investigated and relates to channel cleanup in the async implementation.

## Environment Variables

- `RUST_LOG=debug` - Enable debug logging
- `IBAPI_RECORDING_DIR=/tmp/messages` - Record messages for debugging