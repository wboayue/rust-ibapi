# rust-ibapi Examples

This directory contains comprehensive examples demonstrating how to use the rust-ibapi library for connecting to Interactive Brokers TWS/Gateway.

## Organization

Examples are organized into two main categories:

- **[sync/](sync/)** - Synchronous examples using threads and crossbeam channels
- **[async/](async/)** - Asynchronous examples using Tokio and async/await

## Quick Start

### Synchronous Example
```bash
# Run a sync example
cargo run --features sync --example server_time
```

### Asynchronous Example
```bash
# Run an async example
cargo run --features async --example async_connect
```

## Prerequisites

1. **Install IB Gateway or TWS**
   - Download from Interactive Brokers website
   - IB Gateway is recommended for automated trading

2. **Configure API Access**
   - In Gateway: Configure → Settings → API → Settings
   - Enable "Enable ActiveX and Socket Clients"
   - Add your IP to trusted IPs (e.g., 127.0.0.1)
   - Note the port number:
     - IB Gateway: 4002 (paper), 4001 (live)
     - TWS: 7497 (paper), 7496 (live)

3. **Market Data Subscriptions**
   - Many examples require market data permissions
   - Check your account subscriptions in Account Management

## Featured Examples

### Getting Started
- `connect` / `async_connect` - Basic connection
- `server_time` - Verify connection is working
- `managed_accounts` - List available accounts

### Market Data
- `market_data` / `async_market_data` - Real-time quotes
- `historical_data` / `async_historical_data` - Historical bars
- `tick_by_tick` / `async_tick_by_tick` - Tick data

### Trading
- `place_order` - Submit orders
- `positions` / `async_positions` - View positions
- `executions` - Execution reports

### Account Information
- `account_summary` / `async_account_summary` - Account values
- `pnl` / `async_pnl` - Real-time P&L

## Common Issues

1. **Connection Refused**
   - Ensure IB Gateway/TWS is running
   - Check the port number matches your configuration
   - Verify API is enabled

2. **Client ID Already in Use**
   - Each connection needs a unique client ID
   - Change the client ID in the example or restart Gateway/TWS

3. **No Market Data**
   - Verify you have appropriate market data subscriptions
   - Some symbols may require specific permissions

## Environment Variables

- `RUST_LOG=debug` - Enable detailed logging
- `IBAPI_RECORDING_DIR=/path/to/dir` - Record messages for debugging

## Additional Resources

- [Sync Examples README](sync/README.md) - Detailed sync examples documentation
- [Async Examples README](async/README.md) - Detailed async examples documentation
- [Market Data Examples](README_ASYNC_MARKET_DATA.md) - Comprehensive async market data guide

## Contributing

When adding new examples:
1. Place in appropriate folder (sync/ or async/)
2. Add documentation header with usage instructions
3. Update the relevant README
4. Keep examples focused on demonstrating specific features