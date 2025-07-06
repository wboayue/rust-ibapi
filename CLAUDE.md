# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the library (sync by default)
cargo build

# Build with async support
cargo build --no-default-features --features async

# Build with optimizations
cargo build --release

# Build all targets including examples
cargo build --all-targets

# Run tests
cargo test

# Run async tests
cargo test --no-default-features --features async

# Run a specific test
cargo test <test_name>

# Run tests in a specific module
cargo test --package ibapi <module>::

# Run tests with verbose output
cargo test -- --nocapture

# Check code with clippy
cargo clippy

# Format code
cargo fmt

# Generate code coverage report
cargo tarpaulin -o html
# or
just cover
```

## Feature Flags

The library supports two mutually exclusive features:
- **sync** (default): Traditional synchronous API using threads
- **async**: Asynchronous API using tokio

When both features are enabled, async takes precedence. This allows users to simply add `--features async` without needing `--no-default-features`.

To use async:
```bash
cargo build --features async
```

### Important: Feature Guard Pattern

When adding new sync-specific code, ALWAYS use:
```rust
#[cfg(all(feature = "sync", not(feature = "async")))]
```

NOT just:
```rust
#[cfg(feature = "sync")]  // DON'T use this alone!
```

This ensures that async mode properly overrides sync mode when both features are enabled.

For async-specific code, use:
```rust
#[cfg(feature = "async")]
```

### Examples

For async examples, add to Cargo.toml:
```toml
[[example]]
name = "your_async_example"
required-features = ["async"]
```

## Environment Variables for Debugging

```bash
# Set log level (trace, debug, info, warn, error)
RUST_LOG=debug cargo run --example <example_name>

# Log messages between library and TWS to a directory
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example <example_name>
```

## Connection Settings for Examples

When running examples, use IB Gateway default ports:
- **Live trading**: 4001
- **Paper trading**: 4002

TWS ports (7496/7497) can also be used but Gateway is preferred for automated trading and examples.

## Architecture Overview

The rust-ibapi crate is a Rust implementation of the Interactive Brokers TWS API. The architecture supports both synchronous (thread-based) and asynchronous (tokio-based) operation modes through feature flags.

### Core Components

1. **Client** - Main interface for user interactions
   - **Sync mode**: Uses threads and crossbeam channels
   - **Async mode**: Uses tokio tasks and mpsc channels
   - Encodes user requests
   - Sends requests to the MessageBus
   - Receives responses via channels
   - Decodes responses for the user

2. **MessageBus** - Handles connection and message routing
   - **Sync mode**: Runs on a dedicated thread
   - **Async mode**: Runs as a tokio task
   - Establishes and maintains connection to TWS/Gateway
   - Sends messages from client to TWS
   - Listens for messages from TWS
   - Routes incoming messages to appropriate client channels

3. **Request/Response Flow**:
   - For requests with IDs: MessageBus creates dedicated channels for responses
   - For requests without IDs: MessageBus uses shared channels for responses
   - Order-based requests: Special handling for order tracking

4. **Key Modules**:
   - `accounts`: Account-related functionality
   - `contracts`: Contract definitions and operations
   - `market_data`: Real-time and historical market data
   - `orders`: Order management functionality
   - `news`: News-related functionality
   - `transport`: Connection and message handling
   - `messages`: Message definitions and routing
   - `protocol`: Protocol version constants and feature checking

## Multi-Threading Model

The `Client` can be shared between threads for concurrent operations:

1. **Thread-Safe Design**: Use `Arc<Client>` when sharing a client across threads
2. **One Client Per Thread**: Recommended for operations with shared channels
3. **Subscription Model**: Subscriptions can be converted to iterators for convenient data handling

## Important Design Considerations

1. **Message Handling**: Some TWS API calls don't have unique request IDs and are mapped by message type. These shared channels can cause confusion if multiple concurrent requests of the same type are made.

2. **Fault Tolerance**: The API automatically attempts to reconnect on disconnection, using a Fibonacci backoff strategy.

3. **Testing**: For important changes, ensure tests are passing and coverage is maintained. Use `cargo tarpaulin` to generate coverage reports.

4. **Examples**: Numerous examples in the `/examples` directory demonstrate API usage.

## Builder Patterns

The library provides unified builder patterns to simplify common operations in both sync and async modes. The builders are located in `client/builders/` and provide a consistent API regardless of the feature flag used.

### Request Builder

The request builder reduces boilerplate for client methods with request IDs:

```rust
// Sync mode
pub fn pnl(client: &Client, account: &str) -> Result<Subscription<PnL>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::PNL, "PnL not supported")?;
    
    let request = encode_request_pnl(builder.request_id(), account)?;
    builder.send(request)
}

// Async mode
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

For requests that use shared channels (no request ID):

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

For order-specific operations:

```rust
// Sync mode
pub fn place_order(client: &Client, contract: &Contract, order: &Order) -> Result<(), Error> {
    let builder = client.order_request();
    let request = encode_order(builder.order_id(), contract, order)?;
    builder.send(request)?;
    Ok(())
}

// Async mode
pub async fn place_order(client: &Client, contract: &Contract, order: &Order) -> Result<(), Error> {
    let builder = client.order_request();
    let request = encode_order(builder.order_id(), contract, order)?;
    builder.send(request).await?;
    Ok(())
}
```

### Subscription Builder

The subscription builder provides a fluent API for creating subscriptions with additional context:

```rust
// Sync mode with smart depth context
pub fn market_depth(client: &Client, contract: &Contract, num_rows: i32) -> Result<Subscription<MarketDepth>, Error> {
    let request_id = client.next_request_id();
    let request = encode_market_depth(request_id, contract, num_rows)?;
    
    client
        .subscription::<MarketDepth>()
        .with_smart_depth(true)
        .send_with_request_id(request_id, request)
}

// Async mode with smart depth context
pub async fn market_depth(client: &Client, contract: &Contract, num_rows: i32) -> Result<Subscription<MarketDepth>, Error> {
    let request_id = client.next_request_id();
    let request = encode_market_depth(request_id, contract, num_rows)?;
    
    client
        .subscription::<MarketDepth>()
        .with_smart_depth(true)
        .send_with_request_id(request_id, request)
        .await
}
```

### Builder Types

All builders are internal types (`pub(crate)`) and are accessed through extension traits:

- **ClientRequestBuilders**: Provides `request()`, `shared_request()`, `order_request()`, and `message()` methods
- **SubscriptionBuilderExt**: Provides `subscription<T>()` method

The builders automatically adapt to sync or async mode based on the active feature flag, maintaining the same method names and patterns while changing the underlying implementation.

## Protocol Version Constants

The `protocol` module provides centralized version checking:

```rust
use crate::protocol::{check_version, Features, is_supported};

// Check version support
pub fn tick_by_tick_trades(&self, contract: &Contract) -> Result<Subscription<Trade>, Error> {
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

## Sync vs Async Usage

### Sync Example
```rust
use ibapi::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("127.0.0.1:4002", 100)?;
    
    // Blocking call
    let time = client.server_time()?;
    println!("Server time: {}", time);
    
    // Iterator-based subscription
    let positions = client.positions()?;
    for position in positions {
        println!("Position: {:?}", position?);
    }
    
    Ok(())
}
```

### Async Example
```rust
use ibapi::Client;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    
    // Async call
    let time = client.server_time().await?;
    println!("Server time: {}", time);
    
    // Stream-based subscription
    let mut positions = client.positions().await?;
    while let Some(position) = positions.next().await {
        println!("Position: {:?}", position?);
    }
    
    Ok(())
}
```

To compile async code:
```bash
cargo build --features async
```

## Running Examples

Examples follow a naming convention to indicate their mode:

```bash
# Sync examples (default)
cargo run --example market_data
cargo run --example positions

# Async examples (note: no need for --no-default-features)
cargo run --features async --example async_connect
cargo run --features async --example async_market_data
```