# Examples Guide

## Running Examples

Examples are organized by sync/async mode:

```bash
# Sync examples
cargo run --features sync --example connect
cargo run --features sync --example market_data
cargo run --features sync --example positions

# Async examples  
cargo run --features async --example async_connect
cargo run --features async --example async_market_data
cargo run --features async --example async_positions
```

## Example Categories

### Connection & Setup
- `connect` / `async_connect` - Basic connection to TWS/Gateway
- `connection_monitoring` / `async_connection_monitoring` - Monitor connection status

### Market Data
- `market_data` / `async_market_data` - Real-time quotes
- `historical_data` / `async_historical_data` - Historical bars
- `tick_by_tick_*` - Tick-by-tick data streams
- `realtime_bars` / `async_realtime_bars` - 5-second bars

### Account & Portfolio
- `positions` / `async_positions` - Current positions
- `account_summary` / `async_account_summary` - Account values
- `pnl` / `async_pnl` - Profit and loss tracking

### Orders & Execution
- `place_order` / `async_place_order` - Submit orders
- `order_management` / `async_order_management` - Modify/cancel orders
- `executions` / `async_executions` - Execution reports

### Options
- `option_chain` / `async_option_chain` - Option contracts
- `calculate_option_price` / `async_calculate_option_price` - Option pricing

### News & Events
- `news_bulletins` / `async_news_bulletins` - News headlines
- `wsh_metadata` / `async_wsh_metadata` - Wall Street Horizon events

## Writing Examples

### Basic Structure (Sync)
```rust
use ibapi::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect
    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected to TWS version {}", client.server_version());
    
    // Perform operations
    let time = client.server_time()?;
    println!("Server time: {}", time);
    
    Ok(())
}
```

### Basic Structure (Async)
```rust
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to TWS version {}", client.server_version());
    
    // Perform operations
    let time = client.server_time().await?;
    println!("Server time: {}", time);
    
    Ok(())
}
```

## Common Patterns in Examples

### Handling Subscriptions (Sync)
```rust
let subscription = client.market_data(&contract)?;

for update in subscription.timeout_iter(Duration::from_secs(30)) {
    match update? {
        MarketData::Price(price) => {
            println!("Price: {}", price);
        },
        MarketData::Size(size) => {
            println!("Size: {}", size);
        },
        _ => {}
    }
}
```

### Handling Subscriptions (Async)
```rust
use futures::StreamExt;
use tokio::time::timeout;

let mut subscription = client.market_data(&contract).await?;

while let Ok(Some(update)) = timeout(
    Duration::from_secs(30),
    subscription.next()
).await {
    match update? {
        MarketData::Price(price) => {
            println!("Price: {}", price);
        },
        MarketData::Size(size) => {
            println!("Size: {}", size);
        },
        _ => {}
    }
}
```

### Error Handling
```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Main logic here
    Ok(())
}
```

### Contract Creation
```rust
use ibapi::contracts::Contract;

// Stock
let stock = Contract::stock("AAPL");

// Future
let future = Contract::futures("ES")
    .last_trade_date_or_contract_month("202312")
    .build();

// Option
let option = Contract::option("AAPL", "20240119", 150.0, "C");

// Forex
let forex = Contract::forex("EUR", "USD");
```

## Environment Setup for Examples

### Enable Logging
```bash
# Basic info
RUST_LOG=info cargo run --features sync --example market_data

# Debug messages
RUST_LOG=debug cargo run --features sync --example market_data

# Trace everything
RUST_LOG=trace cargo run --features sync --example market_data

# Module-specific
RUST_LOG=ibapi::transport=debug cargo run --features sync --example market_data
```

### Record Messages
```bash
# Save all TWS communication
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --features sync --example market_data

# Files created:
# /tmp/tws-messages/requests_20240315_143022.txt
# /tmp/tws-messages/responses_20240315_143022.txt
```

## Adding New Examples

1. Create file in appropriate directory:
   - `examples/sync/` for synchronous
   - `examples/async/` for asynchronous

2. Add to `Cargo.toml`:
```toml
[[example]]
name = "your_example"
path = "examples/sync/your_example.rs"
required-features = ["sync"]

[[example]]
name = "async_your_example"
path = "examples/async/your_example.rs"
required-features = ["async"]
```

3. Follow naming convention:
   - Sync: `example_name.rs`
   - Async: `async_example_name.rs` (if there's a sync version)

4. Include helpful comments and error handling

5. Test both modes if applicable