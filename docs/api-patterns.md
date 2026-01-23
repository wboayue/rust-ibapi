# API Patterns

## Builder Patterns

The library provides unified builder patterns to simplify common operations in both sync and async modes.

### Contract Builder

The V2 contract builder API uses type-state patterns to ensure compile-time safety:

```rust
use ibapi::contracts::{Contract, ContractMonth};

// Stock builder - simple with defaults
let stock = Contract::stock("AAPL").build();

// Stock with customization
let intl_stock = Contract::stock("7203")
    .on_exchange("TSEJ")
    .in_currency("JPY")
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
- **Strong typing**: Type-safe wrappers for exchanges, currencies, and option rights
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

### Conditional Order Builder Pattern

The library provides a fluent API for building conditional orders with type-safe condition builders and ergonomic helper functions.

#### Helper Functions

Helper functions provide a concise way to create condition builders:

```rust
use ibapi::orders::builder::{price, time, margin, volume, execution, percent_change};
use ibapi::orders::order_builder;

// Helper functions return partially-built condition builders
let price_cond = price(265598, "SMART").greater_than(150.0);
let time_cond = time().greater_than("20251230 14:30:00 US/Eastern");
let margin_cond = margin().less_than(30);
let volume_cond = volume(76792991, "SMART").greater_than(50_000_000);
let pct_change_cond = percent_change(756733, "SMART").greater_than(2.0);

// Execution condition returns OrderCondition directly (no threshold)
let exec_cond = execution("TSLA", "STK", "SMART");

// Create order with single condition
let order = order_builder::market_order(Action::Buy, 100.0)
    .condition(price_cond)
    .build();
```

#### Fluent Condition Chaining

The `OrderBuilder` provides methods for chaining conditions with AND/OR logic:

```rust
use ibapi::orders::builder::{price, time, volume};
use ibapi::orders::order_builder;

// Multiple conditions with AND logic (all must be true)
let order = order_builder::limit_order(Action::Buy, 100.0, 151.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(volume(265598, "SMART").greater_than(80_000_000))
    .and_condition(time().greater_than("20251230 10:00:00 US/Eastern"))
    .build();

// Multiple conditions with OR logic (any can trigger)
let order = order_builder::market_order(Action::Sell, 100.0)
    .condition(margin().less_than(25))
    .or_condition(price(265598, "SMART").less_than(140.0))
    .or_condition(time().greater_than("20251230 15:55:00 US/Eastern"))
    .build();

// Mixed AND/OR logic
let order = order_builder::limit_order(Action::Buy, 50.0, 452.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(volume(265598, "SMART").greater_than(50_000_000))  // Price AND Volume
    .or_condition(time().greater_than("20251230 14:00:00 US/Eastern")) // OR Time
    .build();
```

#### Type-State Pattern for Conditions

Each condition builder uses the type-state pattern to ensure valid configuration:

```rust
use ibapi::orders::conditions::{PriceCondition, TriggerMethod};

// Builder starts with required fields only
let builder = PriceCondition::builder(265598, "SMART");

// Threshold and direction set together (type-safe)
let condition = builder.greater_than(150.0);  // Sets price + direction
// or
let condition = builder.less_than(140.0);     // Sets price + direction

// Optional configuration
let condition = condition
    .trigger_method(TriggerMethod::Last)  // Use last price
    .conjunction(true);                   // AND with next condition

// Convert to OrderCondition
let condition = condition.build();
```

All condition builders follow this pattern:
- **Required parameters** in the constructor (contract ID, exchange, etc.)
- **Threshold and direction** set together via `greater_than()` or `less_than()`
- **Optional parameters** via chainable methods
- **Type conversion** via `.build()` or automatic `Into<OrderCondition>`

#### Old vs New API Comparison

**Before (v0.x):**
```rust
// Threshold in constructor, separate trigger direction method
let price_cond = PriceCondition::builder(265598, "SMART", 150.0)
    .trigger_above()
    .build();

let time_cond = TimeCondition::builder("20251230 14:30:00 US/Eastern")
    .trigger_after()
    .build();

// Manual condition assignment
let mut order = order_builder::market_order(Action::Buy, 100.0);
order.conditions = vec![
    OrderCondition::Price(price_cond),
    OrderCondition::Time(time_cond),
];
```

**After (v1.0+):**
```rust
// Threshold and direction combined, fluent chaining
let order = order_builder::market_order(Action::Buy, 100.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
    .build();

// Or using builders directly
let price_cond = PriceCondition::builder(265598, "SMART")
    .greater_than(150.0)
    .build();

let time_cond = TimeCondition::builder()
    .greater_than("20251230 14:30:00 US/Eastern")
    .build();
```

Key improvements:
- **More ergonomic**: Helper functions reduce boilerplate
- **Type-safe**: Threshold and direction set atomically
- **Fluent**: Method chaining for AND/OR logic
- **Explicit**: `greater_than()` vs `less_than()` is clearer than `trigger_above()` vs `trigger_below()`
- **Consistent**: All condition types follow the same pattern

#### Sync and Async Usage

Conditional orders work identically in both sync and async modes:

**Sync Mode:**
```rust
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::builder::{price, order_builder};
use ibapi::orders::Action;

let client = Client::connect("127.0.0.1:7497", 100)?;
let contract = Contract::stock("AAPL").build();

let order = order_builder::market_order(Action::Buy, 100.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .build();

let order_id = client.next_valid_order_id()?;
client.submit_order(order_id, &contract, &order)?;
```

**Async Mode:**
```rust
use ibapi::client::Client;
use ibapi::contracts::Contract;
use ibapi::orders::builder::{price, order_builder};
use ibapi::orders::Action;

let client = Client::connect("127.0.0.1:4002", 100).await?;
let contract = Contract::stock("AAPL").build();

let order = order_builder::market_order(Action::Buy, 100.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .build();

let order_id = client.next_valid_order_id().await?;
client.submit_order(order_id, &contract, &order).await?;
```

The only difference is the `.await` calls on client methods. The order building logic is identical.

#### Advanced Pattern: Reusable Condition Components

Create reusable condition components for common trading scenarios:

```rust
use ibapi::orders::builder::{price, time, volume, margin};
use ibapi::orders::conditions::OrderCondition;

// Reusable condition builders
fn liquidity_check(contract_id: i32, min_volume: i32) -> impl Into<OrderCondition> {
    volume(contract_id, "SMART").greater_than(min_volume)
}

fn trading_hours_only() -> impl Into<OrderCondition> {
    time()
        .greater_than("20251230 09:30:00 US/Eastern")
        .conjunction(true)
}

fn risk_guard() -> impl Into<OrderCondition> {
    margin().less_than(30)
}

// Compose conditions
let order = order_builder::market_order(Action::Buy, 100.0)
    .condition(liquidity_check(265598, 50_000_000))
    .and_condition(trading_hours_only())
    .and_condition(risk_guard())
    .build();
```

For comprehensive conditional order documentation, see [Order Types - Conditional Orders](order-types.md#conditional-orders-with-conditions).

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

## Trading Hours Support

The `TradingHours` enum controls whether data includes extended hours (pre-market and after-hours). However, not all market data APIs support this parameter at the TWS protocol level.

| API | TradingHours Support | Notes |
|-----|---------------------|-------|
| `realtime_bars()` | ✓ Yes | Server-side filtering via `use_rth` |
| `historical_data()` | ✓ Yes | Server-side filtering via `use_rth` |
| `market_data()` | ✗ No | TWS protocol doesn't support filtering |

### Realtime Bars (Extended Hours Supported)

```rust
use ibapi::market_data::TradingHours;

// Regular trading hours only
let bars = client.realtime_bars(
    &contract,
    BarSize::Sec5,
    WhatToShow::Trades,
    TradingHours::Regular,  // Excludes pre/post-market
).await?;

// Include extended hours
let bars = client.realtime_bars(
    &contract,
    BarSize::Sec5,
    WhatToShow::Trades,
    TradingHours::Extended,  // Includes pre/post-market
).await?;
```

### Market Data Subscriptions (No Extended Hours Filtering)

The TWS API's `reqMktData` does not support a `useRth` parameter. Streaming tick data automatically includes all available data, including pre-market and after-hours quotes when the exchange reports them.

```rust
// Market data subscriptions receive ALL available data
// including extended hours - no filtering option exists
let ticks = client.market_data(&contract)
    .subscribe()
    .await?;
```

To filter for regular trading hours only, you must filter client-side based on timestamp and the trading session times for your specific exchange.

## Error Handling Patterns

### Connection Errors
```rust
match client.market_data(contract).subscribe() {
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
            let data = client.market_data(&contract).subscribe()?;
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

## Trait Composition Patterns

### Domain Traits for Shared Behavior
```rust
// Use domain traits when:
// - 2+ types need the same operation (encode, decode, validate)
// - You want to write generic functions over those types
pub trait Encodable {
    fn encode(&self, message: &mut RequestMessage) -> Result<(), Error>;
}

pub trait Decodable: Sized {
    fn decode(fields: &mut FieldIter) -> Result<Self, Error>;
}

// Implement for types that need this behavior
impl Encodable for Order { /* ... */ }
impl Encodable for Contract { /* ... */ }
```

### Extension via Composition
```rust
// Use composition when:
// - A type needs capabilities from multiple sources
// - Behavior should be added without modifying the original type
pub struct Subscription<T> {
    receiver: Receiver<T>,
    cancel_fn: Box<dyn Fn() + Send>,
}

// Add behavior via trait impls
impl<T> Iterator for Subscription<T> { /* ... */ }
impl<T> Drop for Subscription<T> { /* ... */ }
```

### Newtype Wrappers for Domain Constraints
```rust
// Bad: raw i32 allows invalid IDs and type confusion
fn lookup(contract_id: i32, order_id: i32) -> Contract { /* ... */ }  // easy to swap args

// Good: newtype wrappers prevent mistakes
// Use newtype wrappers when:
// - A primitive has domain constraints (non-zero, positive, etc.)
// - Type confusion is possible (ContractId vs OrderId)
pub struct ContractId(i32);

impl ContractId {
    pub fn new(id: i32) -> Result<Self, Error> {
        if id <= 0 { return Err(Error::InvalidContractId); }
        Ok(Self(id))
    }
}

// Type system prevents invalid states
fn lookup(id: ContractId) -> Contract { /* ... */ }  // Can't pass raw i32
```
