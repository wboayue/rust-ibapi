# rust-ibapi Examples

Comprehensive examples demonstrating the rust-ibapi library for Interactive Brokers TWS/Gateway connections.

## Quick Start

```bash
# Synchronous examples (70+ available)
cargo run --features sync --example server_time

# Asynchronous examples (40+ available)  
cargo run --features async --example connect
```

## Prerequisites

1. **Install IB Gateway or TWS** - Download from Interactive Brokers
2. **Enable API Access** - In Gateway/TWS: Configure → Settings → API → Settings
   - Enable "Enable ActiveX and Socket Clients"
   - Add 127.0.0.1 to trusted IPs
   - Default ports: 4002 (Gateway paper), 7497 (TWS paper)
3. **Market Data Subscriptions** - Required for market data examples

## Example Organization

Examples are organized by execution mode:
- `sync/` - Synchronous examples using threads and blocking calls
- `async/` - Asynchronous examples using Tokio and async/await
- Root directory - Utility and debugging tools

Examples cover these functional areas:
- **Core**: Connection, market data, historical data, orders, account management, contracts
- **Advanced**: News, scanners, WSH events, options, trading strategies  
- **Utilities**: Message recording, debugging tools, builder demonstrations

## Key Differences: Sync vs Async

**Synchronous** (`sync/`)
- Uses blocking calls and threads
- Returns values directly or via iterators
- Simpler for sequential operations

**Asynchronous** (`async/`)  
- Uses `async`/`await` with Tokio runtime
- Returns Futures and Streams
- Better for concurrent operations

### Async Pattern Example
```rust
use futures::StreamExt;

let mut stream = client.market_data(&contract)
    .subscribe()
    .await?;
while let Some(tick) = stream.next().await {
    // Process tick
}
```

## Common Issues

1. **Connection Refused** - Ensure Gateway/TWS is running with API enabled
2. **Client ID in Use** - Each connection needs unique ID  
3. **No Market Data** - Verify market data subscriptions

## Environment Variables

```bash
RUST_LOG=debug cargo run --example <name>           # Enable debug logs
IBAPI_RECORDING_DIR=/tmp cargo run --example <name>  # Record messages
```

## Contributing

When adding examples:
1. Place in appropriate folder (`sync/` or `async/`)
2. Add clear documentation header
3. Keep examples focused on demonstrating specific features
4. Follow existing naming conventions