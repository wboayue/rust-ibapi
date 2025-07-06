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

### Sync Subscription (`subscriptions/sync.rs`)
```rust
pub struct Subscription<T> {
    inner: InternalSubscription,
    _phantom: PhantomData<T>,
}

impl<T> Subscription<T> {
    pub fn next(&self) -> Option<Result<T>> { ... }
    pub fn try_next(&self) -> Option<Result<T>> { ... }
    pub fn next_timeout(&self, timeout: Duration) -> Option<Result<T>> { ... }
}

impl<T> Iterator for Subscription<T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> { ... }
}
```

### Async Subscription (`subscriptions/async.rs`)
```rust
pub struct Subscription<T> {
    receiver: tokio::sync::mpsc::UnboundedReceiver<Result<T>>,
    _phantom: PhantomData<T>,
}

impl<T> Stream for Subscription<T> {
    type Item = Result<T>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
```

### Crate Level Export
```rust
// subscriptions/mod.rs
#[cfg(feature = "sync")]
pub use self::sync::Subscription;

#[cfg(feature = "async")]
pub use self::async::Subscription;
```

## 5. MessageBus Architecture

```rust
// Single trait definition with conditional compilation
pub trait MessageBus: Send + Sync {
    #[cfg(feature = "sync")]
    fn send_request(&self, request: Request) -> Result<()>;
    
    #[cfg(feature = "async")]
    async fn send_request(&self, request: Request) -> Result<()>;
    
    #[cfg(feature = "sync")]
    fn subscribe(&self, request_id: i32) -> InternalSubscription;
    
    #[cfg(feature = "async")]
    async fn subscribe(&self, request_id: i32) -> InternalSubscription;
    
    #[cfg(feature = "sync")]
    fn subscribe_shared(&self, channel_type: SharedChannel) -> InternalSubscription;
    
    #[cfg(feature = "async")]
    async fn subscribe_shared(&self, channel_type: SharedChannel) -> InternalSubscription;
    
    #[cfg(feature = "sync")]
    fn subscribe_order(&self, order_id: i32) -> InternalSubscription;
    
    #[cfg(feature = "async")]
    async fn subscribe_order(&self, order_id: i32) -> InternalSubscription;
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

### Subscription Methods (return unified Subscription<T> type)
- `positions()` → `async fn positions()` returns `Subscription<Position>`
- `positions_multi()` → `async fn positions_multi()` returns `Subscription<Position>`
- `pnl()` → `async fn pnl()` returns `Subscription<PnL>`
- `pnl_single()` → `async fn pnl_single()` returns `Subscription<PnLSingle>`
- `account_summary()` → `async fn account_summary()` returns `Subscription<AccountSummary>`
- `account_update()` → `async fn account_update()` returns `Subscription<AccountUpdate>`
- `market_data()` → `async fn market_data()` returns `Subscription<MarketData>`
- `realtime_bars()` → `async fn realtime_bars()` returns `Subscription<Bar>`
- `market_depth()` → `async fn market_depth()` returns `Subscription<MarketDepth>`
- `tick_by_tick_*()` → `async fn tick_by_tick_*()` returns appropriate `Subscription<T>`
- `news_bulletins()` → `async fn news_bulletins()` returns `Subscription<NewsBulletin>`
- `scanner_subscription()` → `async fn scanner_subscription()` returns `Subscription<ScannerData>`
- `submit_order()` → `async fn submit_order()` returns `Subscription<OrderUpdate>`
- `order_update_stream()` → `async fn order_update_stream()` returns `Subscription<OrderUpdate>`

## 8. API Surface Consistency

Both sync and async implementations expose the same types at the crate level:
- `ibapi::Client` - The main client type (sync or async based on feature)
- `ibapi::Subscription<T>` - The subscription type (Iterator or Stream based on feature)
- All domain types remain the same: `Contract`, `Order`, `Position`, etc.

This means users only need to:
1. Change their `Cargo.toml` to select the desired feature
2. Add `async`/`await` keywords where needed
3. Use `StreamExt` trait for async subscriptions

The import paths and type names remain identical, providing a smooth migration experience.

## 9. Usage Examples

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

### Feature Guards
```rust
// lib.rs
#[cfg(all(feature = "sync", feature = "async"))]
compile_error!("Features 'sync' and 'async' are mutually exclusive. Please enable only one.");

#[cfg(not(any(feature = "sync", feature = "async")))]
compile_error!("Either 'sync' or 'async' feature must be enabled.");
```

## 10. Migration Strategy

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

## 11. Implementation Considerations

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

## 12. Alternative Approaches Considered

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

## 13. Example Structure

To provide clear examples for both sync and async APIs, examples will be organized with a simple naming convention:

```
examples/
├── sync_market_data.rs
├── sync_realtime_bars.rs
├── sync_positions.rs
├── sync_orders.rs
├── sync_account_summary.rs
├── async_market_data.rs
├── async_realtime_bars.rs
├── async_positions.rs
├── async_orders.rs
└── async_account_summary.rs
```

### Running Examples

Examples can be run with their respective features:

```bash
# Sync examples
cargo run --example sync_market_data --features sync
cargo run --example sync_positions --features sync

# Async examples  
cargo run --example async_market_data --features async
cargo run --example async_positions --features async
```

### Example Template

Each example will include:
- Clear documentation indicating sync vs async version
- Feature gate with helpful error message if run without correct feature
- Consistent structure for easy comparison between sync and async versions

**Sync Example (`sync_market_data.rs`):**
```rust
//! Market data subscription example (synchronous)
//! 
//! Run with: cargo run --example sync_market_data --features sync

#[cfg(not(feature = "sync"))]
fn main() {
    eprintln!("This example requires the 'sync' feature. Run with: cargo run --example sync_market_data --features sync");
}

#[cfg(feature = "sync")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ibapi::Client;
    use ibapi::contracts::Contract;
    
    env_logger::init();
    
    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected to TWS version {}", client.server_version());
    
    let contract = Contract::stock("AAPL");
    let subscription = client.market_data(&contract, &["233"], false, false)?;
    
    for tick in subscription.iter().take(10) {
        println!("Tick: {:?}", tick);
    }
    
    Ok(())
}
```

**Async Example (`async_market_data.rs`):**
```rust
//! Market data subscription example (asynchronous)
//! 
//! Run with: cargo run --example async_market_data --features async

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("This example requires the 'async' feature. Run with: cargo run --example async_market_data --features async");
}

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ibapi::AsyncClient;
    use ibapi::contracts::Contract;
    use futures::StreamExt;
    
    env_logger::init();
    
    let client = AsyncClient::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to TWS version {}", client.server_version());
    
    let contract = Contract::stock("AAPL");
    let mut subscription = client.market_data(&contract, &["233"], false, false).await?;
    
    while let Some(tick) = subscription.next().await {
        match tick {
            Ok(tick) => println!("Tick: {:?}", tick),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

This approach provides:
- No Cargo.toml configuration needed for each example
- Clear naming convention (`sync_*` vs `async_*` prefix)
- Simple discovery with `cargo run --example`
- Feature protection with clear error messages
- Easy comparison between sync and async implementations

## 14. Consolidated Error Handling

A centralized error handling module (`client/error_handler.rs`) provides consistent error management:

### Key Functions

- **`is_connection_error()`** - Detects connection-related errors requiring reconnection
- **`is_timeout_error()`** - Identifies timeouts that can be safely ignored  
- **`should_retry_request()`** - Determines if an error is transient and should be retried
- **`is_fatal_error()`** - Identifies unrecoverable errors
- **`error_message()`** - Provides user-friendly error descriptions
- **`categorize_error()`** - Groups errors for logging/metrics (Connection, Parsing, Timeout, etc.)

### Benefits

- Consistent error handling patterns across sync/async code
- Centralized retry logic with configurable limits
- Clear error categorization for appropriate handling
- Simplified error checking in transport and client layers

### Example Usage

```rust
// In transport layer
match self.read_message() {
    Err(ref err) if is_timeout_error(err) => Ok(()), // Ignore timeouts
    Err(ref err) if is_connection_error(err) => self.attempt_reconnect(),
    Err(err) => Err(err), // Fatal error
    Ok(msg) => self.process_message(msg),
}

// In client methods with retry
let mut retry_count = 0;
loop {
    match do_request() {
        Ok(result) => return Ok(result),
        Err(e) if should_retry_request(&e, retry_count) => {
            retry_count += 1;
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

## 15. Request/Response Builder Pattern

A fluent builder pattern (`client/request_builder.rs`) simplifies client method implementations by reducing boilerplate:

### Builder Types

- **`RequestBuilder`** - For requests with auto-generated request IDs
- **`SharedRequestBuilder`** - For requests without IDs (shared channels)
- **`OrderRequestBuilder`** - For order-specific requests
- **`MessageBuilder`** - For simple messages without responses

### Benefits

- Reduces repetitive code for version checking and ID management
- Provides chainable, fluent API
- Integrates error handling into the builder flow
- Maintains type safety and correct usage patterns

### Example: Simplifying Client Methods

```rust
// Before: Manual version check, ID generation, and subscription creation
pub fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    client.check_server_version(server_versions::PNL, "PnL not supported")?;
    
    let request_id = client.next_request_id();
    let request = encode_request_pnl(request_id, account)?;
    let subscription = client.send_request(request_id, request)?;
    
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// After: Fluent builder pattern
pub fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::PNL, "PnL not supported")?;
    
    let request = encode_request_pnl(builder.request_id(), account)?;
    builder.send(request)
}

// For shared requests (no request ID)
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    let request = encode_request_positions()?;
    
    client
        .shared_request(OutgoingMessages::RequestPositions)
        .check_version(server_versions::ACCOUNT_SUMMARY, "Positions not supported")?
        .send(request)
}
```

This pattern works identically for both sync and async implementations, providing consistency across the API.

## 16. Protocol Version Constants Module

A centralized protocol module (`protocol.rs`) provides consistent version checking across the codebase:

### Key Components

- **`ProtocolFeature`** - Represents a feature requiring minimum server version
- **`Features`** - Namespace containing all protocol features as constants
- **`check_version()`** - Returns error if server version is too old for feature
- **`is_supported()`** - Boolean check for feature support
- **`include_if_supported()`** - Conditionally includes fields based on version

### Benefits

- Centralized version constants (no more scattered server_versions references)
- Consistent error messages for unsupported features
- Type-safe feature checking
- Clear documentation of what each version enables

### Example Usage

```rust
use crate::protocol::{check_version, Features, is_supported};

// In client methods - fail if unsupported
pub fn tick_by_tick_trades(&self, contract: &Contract) -> Result<Subscription<Trade>, Error> {
    check_version(self.server_version, Features::TICK_BY_TICK)?;
    // ... implementation
}

// In encoders - conditionally include fields
pub fn encode_order(order: &Order, server_version: i32) -> RequestMessage {
    let mut message = RequestMessage::new();
    
    // Always included fields
    message.push_field(&order.order_id);
    message.push_field(&order.action);
    
    // Conditionally included based on server version
    if is_supported(server_version, Features::DECISION_MAKER) {
        message.push_field(&order.decision_maker);
    }
    
    if is_supported(server_version, Features::MIFID_EXECUTION) {
        message.push_field(&order.mifid_execution);
    }
    
    message
}
```

This centralizes all version-related logic and makes it easier to understand feature requirements across the codebase.

## 17. Unified Subscription Creation

A subscription builder (`client/subscription_builder.rs`) provides a consistent pattern for creating subscriptions:

### Key Components

- **`SubscriptionBuilder`** - Fluent builder for creating subscriptions
- **`SubscriptionBuilderExt`** - Extension trait adding `.subscription()` method to Client
- Supports three types of requests:
  - Request ID based: `send_with_request_id(request_id, message)`
  - Shared channel: `send_shared(message_type, message)`
  - Order based: `send_order(order_id, message)`

### Benefits

- Consistent subscription creation across all API methods
- Reduces boilerplate code in client methods
- Context configuration (smart depth, request type) is centralized
- Same pattern works for both sync and async implementations

### Example Usage

```rust
use crate::client::subscription_builder::SubscriptionBuilderExt;

// Simple shared channel subscription
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    let request = encode_request_positions()?;
    
    client
        .subscription::<PositionUpdate>()
        .send_shared(OutgoingMessages::RequestPositions, request)
}

// Request ID based subscription
pub fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    let request_id = client.next_request_id();
    let request = encode_request_pnl(request_id, account)?;
    
    client
        .subscription::<PnL>()
        .send_with_request_id(request_id, request)
}

// Subscription with special context
pub fn market_depth(client: &Client, contract: &Contract, num_rows: i32) -> Result<Subscription<MarketDepth>, Error> {
    let request_id = client.next_request_id();
    let request = encode_market_depth(request_id, contract, num_rows)?;
    
    client
        .subscription::<MarketDepth>()
        .with_smart_depth(true)
        .send_with_request_id(request_id, request)
}
```

This pattern eliminates the need to manually create subscriptions with `Subscription::new()` and ensures consistent handling of response contexts.

## 18. Conclusion

The proposed design with single struct and conditional compilation provides:
- ✅ Full backward compatibility
- ✅ Opt-in async support
- ✅ Minimal API surface changes
- ✅ Clear migration path
- ✅ Maintainable codebase

This approach allows the library to evolve with Rust's async ecosystem while serving users who prefer or require synchronous APIs.