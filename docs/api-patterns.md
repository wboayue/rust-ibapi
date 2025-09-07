# API Patterns

## Builder Patterns

The library provides unified builder patterns to simplify common operations in both sync and async modes.

### Contract Builder

The V2 contract builder API uses type-state patterns to ensure compile-time safety:

```rust
use ibapi::contracts::{Contract, Exchange, Currency};

// Stock builder - simple with defaults
let stock = Contract::stock("AAPL").build();

// Stock with customization
let intl_stock = Contract::stock("7203")
    .on_exchange(Exchange::Tsej)
    .in_currency(Currency::JPY)
    .build();

// Option builder - enforces required fields at compile time
let option = Contract::call("AAPL")
    .strike(150.0)  // Required - validates positive value
    .expires_on(2024, 12, 20)  // Required
    .build();  // Only available when all required fields are set

// This won't compile - missing required fields:
// let invalid = Contract::call("AAPL").build();  // Error: build() not available

// Futures with smart defaults
let futures = Contract::futures("ES")
    .expires_in(ContractMonth::new(2024, 3))
    .build();
```

The contract builder pattern provides:
- **Type-state tracking**: Required fields enforced at compile time
- **Smart defaults**: Sensible defaults for common use cases
- **Strong typing**: Enums for exchanges, currencies, and option rights
- **Zero invalid states**: Can't build incomplete contracts

For comprehensive documentation, see the [Contract Builder Guide](contract-builder.md).

### Request Builder

For client methods with request IDs:

```rust
// Sync mode
pub fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::PNL, "PnL not supported")?;
    
    let request = encode_request_pnl(builder.request_id(), account)?;
    builder.send(request)
}

// Async mode - identical except for .await
pub async fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::PNL, "PnL not supported")
        .await?;
    
    let request = encode_request_pnl(builder.request_id(), account)?;
    builder.send(request).await
}
```

### Shared Request Builder

For requests using shared channels (no request ID):

```rust
// Sync mode
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    let request = encode_request_positions()?;
    
    client
        .shared_request(OutgoingMessages::RequestPositions)
        .send(request)
}

// Async mode
pub async fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    let request = encode_request_positions()?;
    
    client
        .shared_request(OutgoingMessages::RequestPositions)
        .send(request)
        .await
}
```

### Order Request Builder

For order operations:

```rust
pub fn place_order(client: &Client, contract: &Contract, order: &Order) -> Result<(), Error> {
    let builder = client.order_request();
    let request = encode_order(builder.order_id(), contract, order)?;
    builder.send(request)?;  // .await for async
    Ok(())
}
```

### Subscription Builder

Create subscriptions with additional context:

```rust
pub fn market_depth(client: &Client, contract: &Contract, num_rows: i32) 
    -> Result<Subscription<MarketDepth>, Error> 
{
    let request_id = client.next_request_id();
    let request = encode_market_depth(request_id, contract, num_rows)?;
    
    client
        .subscription::<MarketDepth>()
        .with_smart_depth(true)
        .send_with_request_id(request_id, request)
        // .await for async version
}
```

## Protocol Version Checking

Use the protocol module for version-specific features:

```rust
use crate::protocol::{check_version, Features, is_supported};

// Check if feature is supported
pub fn tick_by_tick_trades(&self, contract: &Contract) 
    -> Result<Subscription<Trade>, Error> 
{
    check_version(self.server_version, Features::TICK_BY_TICK)?;
    // ... implementation
}

// Conditional field encoding
pub fn encode_order(order: &Order, server_version: i32) -> RequestMessage {
    let mut message = RequestMessage::new();
    
    // Always included
    message.push_field(&order.order_id);
    
    // Conditionally included based on server version
    if is_supported(server_version, Features::DECISION_MAKER) {
        message.push_field(&order.decision_maker);
    }
    
    message
}
```

## Subscription Patterns

### Sync Mode (Iterator)
```rust
let positions = client.positions()?;

// Iterate until completion
for position in positions {
    match position? {
        PositionUpdate::Position(p) => println!("Position: {:?}", p),
        PositionUpdate::End => break,
    }
}
```

### Async Mode (Stream)
```rust
use futures::StreamExt;

let mut positions = client.positions().await?;

// Stream until completion
while let Some(position) = positions.next().await {
    match position? {
        PositionUpdate::Position(p) => println!("Position: {:?}", p),
        PositionUpdate::End => break,
    }
}
```

## Error Handling Patterns

### Connection Errors
```rust
match client.market_data(contract) {
    Ok(subscription) => process_data(subscription),
    Err(Error::NotConnected) => {
        // Wait for reconnection
        while !client.is_connected() {
            thread::sleep(Duration::from_secs(1));
        }
        // Retry
    },
    Err(e) => return Err(e),
}
```

### Subscription Errors
```rust
for result in subscription {
    match result {
        Ok(data) => process(data),
        Err(Error::ConnectionReset) => {
            // Resubscribe after reconnection
            break;
        },
        Err(e) => log::error!("Error: {}", e),
    }
}
```

## Common Patterns

### Concurrent Subscriptions
```rust
use std::sync::Arc;
use std::thread;

let client = Arc::new(client);

let handles: Vec<_> = contracts
    .into_iter()
    .map(|contract| {
        let client = Arc::clone(&client);
        thread::spawn(move || {
            let data = client.market_data(&contract)?;
            process_market_data(data)
        })
    })
    .collect();
```

### Rate Limiting
```rust
use std::time::{Duration, Instant};

struct RateLimiter {
    last_request: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    fn wait_if_needed(&mut self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.min_interval {
            thread::sleep(self.min_interval - elapsed);
        }
        self.last_request = Instant::now();
    }
}
```

### Reconnection Handling
```rust
loop {
    if !client.is_connected() {
        log::info!("Waiting for reconnection...");
        thread::sleep(Duration::from_secs(5));
        continue;
    }
    
    match perform_operation(&client) {
        Ok(result) => return Ok(result),
        Err(Error::NotConnected) => continue,
        Err(e) => return Err(e),
    }
}
```