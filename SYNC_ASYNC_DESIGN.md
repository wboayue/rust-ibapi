# Sync/Async Dual Support Design for rust-ibapi

## Overview

This document outlines the design for adding async support to rust-ibapi while maintaining backward compatibility with the existing synchronous API.

## 1. Feature Flag Strategy

```toml
[features]
default = ["sync"]
sync = []
async = ["tokio", "futures", "async-trait"]
full = ["sync", "async"]
```

- `sync`: Default feature, maintains current behavior
- `async`: Opt-in async support
- `full`: Both sync and async APIs available

## 2. Module Organization

```
src/
├── client/
│   ├── mod.rs          # Common traits and types
│   ├── sync.rs         # Sync implementation
│   └── async.rs        # Async implementation
├── transport/
│   ├── mod.rs          # Common transport traits
│   ├── sync/
│   │   ├── mod.rs
│   │   └── message_bus.rs
│   └── async/
│       ├── mod.rs
│       └── message_bus.rs
└── subscriptions/
    ├── mod.rs          # Common subscription traits
    ├── sync.rs         # Sync Subscription<T>
    └── async.rs        # Async Stream implementations
```

## 3. Client Design (Recommended Approach)

Single struct with conditional compilation provides the best ergonomics:

```rust
pub struct Client {
    #[cfg(feature = "sync")]
    message_bus: Arc<dyn SyncMessageBus>,
    #[cfg(feature = "async")]
    message_bus: Arc<dyn AsyncMessageBus>,
    // Common fields
    server_version: Option<i32>,
    connection_time: Option<OffsetDateTime>,
    request_id_generator: IdGenerator,
    order_id_generator: IdGenerator,
}

impl Client {
    #[cfg(feature = "sync")]
    pub fn connect(url: &str, client_id: i32) -> Result<Self> { 
        // Sync implementation
    }
    
    #[cfg(feature = "async")]
    pub async fn connect(url: &str, client_id: i32) -> Result<Self> { 
        // Async implementation
    }
    
    // Methods follow same pattern
    #[cfg(feature = "sync")]
    pub fn server_time(&self) -> Result<OffsetDateTime> { ... }
    
    #[cfg(feature = "async")]
    pub async fn server_time(&self) -> Result<OffsetDateTime> { ... }
}
```

## 4. Subscription/Stream Design

### Sync Version (Current)
```rust
#[cfg(feature = "sync")]
pub struct Subscription<T> {
    inner: InternalSubscription,
    _phantom: PhantomData<T>,
}

#[cfg(feature = "sync")]
impl<T> Subscription<T> {
    pub fn next(&self) -> Option<Result<T>> { ... }
    pub fn try_next(&self) -> Option<Result<T>> { ... }
    pub fn next_timeout(&self, timeout: Duration) -> Option<Result<T>> { ... }
}

#[cfg(feature = "sync")]
impl<T> Iterator for Subscription<T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> { ... }
}
```

### Async Version
```rust
#[cfg(feature = "async")]
pub struct AsyncSubscription<T> {
    receiver: tokio::sync::mpsc::UnboundedReceiver<Result<T>>,
}

#[cfg(feature = "async")]
impl<T> Stream for AsyncSubscription<T> {
    type Item = Result<T>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
```

## 5. MessageBus Architecture

### Sync MessageBus
```rust
#[cfg(feature = "sync")]
pub trait MessageBus: Send + Sync {
    fn send_request(&self, request: Request) -> Result<()>;
    fn subscribe(&self, request_id: i32) -> InternalSubscription;
    fn subscribe_shared(&self, channel_type: SharedChannel) -> InternalSubscription;
    fn subscribe_order(&self, order_id: i32) -> InternalSubscription;
}
```

### Async MessageBus
```rust
#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncMessageBus: Send + Sync {
    async fn send_request(&self, request: Request) -> Result<()>;
    async fn subscribe(&self, request_id: i32) -> AsyncSubscription<Response>;
    async fn subscribe_shared(&self, channel_type: SharedChannel) -> AsyncSubscription<Response>;
    async fn subscribe_order(&self, order_id: i32) -> AsyncSubscription<Response>;
}
```

## 6. Transport Layer Changes

### Sync Transport
- Continue using `std::net::TcpStream`
- Keep crossbeam channels for inter-thread communication
- Maintain dedicated thread for MessageBus dispatcher

### Async Transport
- Use `tokio::net::TcpStream`
- Replace crossbeam with `tokio::sync::mpsc` channels
- Convert MessageBus dispatcher to tokio task
- Use `tokio::io::{AsyncRead, AsyncWrite}` traits

## 7. Key Methods Requiring Async Variants

### Connection
- `connect()` → `async fn connect()`

### Single Request/Response Methods
- `server_time()` → `async fn server_time()`
- `family_codes()` → `async fn family_codes()`
- `managed_accounts()` → `async fn managed_accounts()`
- `contract_details()` → `async fn contract_details()`
- `matching_symbols()` → `async fn matching_symbols()`
- `market_rules()` → `async fn market_rules()`
- `place_order()` → `async fn place_order()`
- `cancel_order()` → `async fn cancel_order()`
- `open_orders()` → `async fn open_orders()`
- `executions()` → `async fn executions()`
- `news_providers()` → `async fn news_providers()`
- `news_article()` → `async fn news_article()`
- `historical_news()` → `async fn historical_news()`
- `scanner_parameters()` → `async fn scanner_parameters()`

### Subscription Methods (return Stream instead of Iterator)
- `positions()` → `async fn positions()` returns `AsyncSubscription<Position>`
- `positions_multi()` → `async fn positions_multi()` returns `AsyncSubscription<Position>`
- `pnl()` → `async fn pnl()` returns `AsyncSubscription<PnL>`
- `pnl_single()` → `async fn pnl_single()` returns `AsyncSubscription<PnLSingle>`
- `account_summary()` → `async fn account_summary()` returns `AsyncSubscription<AccountSummary>`
- `account_update()` → `async fn account_update()` returns `AsyncSubscription<AccountUpdate>`
- `market_data()` → `async fn market_data()` returns `AsyncSubscription<MarketData>`
- `realtime_bars()` → `async fn realtime_bars()` returns `AsyncSubscription<Bar>`
- `market_depth()` → `async fn market_depth()` returns `AsyncSubscription<MarketDepth>`
- `tick_by_tick_*()` → `async fn tick_by_tick_*()` returns appropriate `AsyncSubscription<T>`
- `news_bulletins()` → `async fn news_bulletins()` returns `AsyncSubscription<NewsBulletin>`
- `scanner_subscription()` → `async fn scanner_subscription()` returns `AsyncSubscription<ScannerData>`
- `submit_order()` → `async fn submit_order()` returns `AsyncSubscription<OrderUpdate>`
- `order_update_stream()` → `async fn order_update_stream()` returns `AsyncSubscription<OrderUpdate>`

## 8. Usage Examples

### Sync Usage (Current Behavior)
```rust
use ibapi::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to TWS/Gateway
    let client = Client::connect("127.0.0.1:7497", 1)?;
    
    // Get server time
    let time = client.server_time()?;
    println!("Server time: {}", time);
    
    // Subscribe to positions
    let positions = client.positions()?;
    for position in positions {
        println!("Position: {:?}", position?);
    }
    
    Ok(())
}
```

### Async Usage
```rust
use ibapi::Client;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to TWS/Gateway
    let client = Client::connect("127.0.0.1:7497", 1).await?;
    
    // Get server time
    let time = client.server_time().await?;
    println!("Server time: {}", time);
    
    // Subscribe to positions
    let mut positions = client.positions().await?;
    while let Some(position) = positions.next().await {
        println!("Position: {:?}", position?);
    }
    
    Ok(())
}
```

### Using Both APIs (with `full` feature)
```rust
// You would need to use different types or modules
#[cfg(feature = "sync")]
use ibapi::sync::Client as SyncClient;

#[cfg(feature = "async")]
use ibapi::async::Client as AsyncClient;
```

## 9. Migration Strategy

### Phase 1: Implementation (v0.x)
- Implement async versions alongside sync
- Both behind feature flags
- Default to `sync` for backward compatibility
- Mark as experimental in documentation

### Phase 2: Stabilization (v0.x+1)
- Gather user feedback
- Refine async API based on usage
- Improve performance and error handling
- Add comprehensive async examples

### Phase 3: Adoption (v1.0)
- Consider making `async` the default feature
- Provide migration guide for sync → async
- Maintain sync API for compatibility

### Phase 4: Future (v2.0)
- Potentially deprecate sync API
- Full async-first design
- Remove dual implementation overhead

## 10. Implementation Considerations

### Shared Code
- Message encoding/decoding remains the same
- Contract, Order, and other domain types are shared
- Only transport and subscription layers differ

### Testing Strategy
- Maintain existing sync tests
- Add parallel async test suite
- Use test fixtures that work for both
- Consider property-based testing for behavior equivalence

### Documentation
- Clearly mark which features enable which APIs
- Provide examples for both sync and async
- Document performance characteristics
- Include migration guide

### Performance
- Async version may have slight overhead for simple requests
- Benefits show in concurrent operations
- Subscription streams more efficient than polling

## 11. Alternative Approaches Considered

### Option A: Trait-based Design
```rust
pub trait ClientBase {
    type Error;
    type Subscription<T>;
}

pub trait Client: ClientBase {
    fn connect(url: &str, client_id: i32) -> Result<Self>;
}

#[async_trait]
pub trait AsyncClient: ClientBase {
    async fn connect(url: &str, client_id: i32) -> Result<Self>;
}
```

**Rejected because:**
- More complex API surface
- Requires users to understand trait system
- Makes migration harder

### Option B: Separate Crates
- `ibapi` for sync
- `ibapi-async` for async

**Rejected because:**
- Duplicates maintenance effort
- Harder to keep in sync
- Splits the community

## 12. Conclusion

The proposed design with single struct and conditional compilation provides:
- ✅ Full backward compatibility
- ✅ Opt-in async support
- ✅ Minimal API surface changes
- ✅ Clear migration path
- ✅ Maintainable codebase

This approach allows the library to evolve with Rust's async ecosystem while serving users who prefer or require synchronous APIs.