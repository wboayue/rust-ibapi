# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the library with sync support
cargo build --features sync

# Build with async support
cargo build --features async

# Build with optimizations
cargo build --release --features sync  # or --features async

# Build all targets including examples
cargo build --all-targets --features sync  # or --features async

# Run tests (sync)
cargo test --features sync

# Run async tests
cargo test --features async

# Run a specific test
cargo test <test_name> --features sync  # or --features async

# Run tests in a specific module
cargo test --package ibapi <module>:: --features sync  # or --features async

# Run tests with verbose output
cargo test --features sync -- --nocapture

# Check code with clippy
cargo clippy --features sync
cargo clippy --features async  # Check both

# Format code
cargo fmt

# Generate code coverage report
cargo tarpaulin -o html
# or
just cover
```

## Feature Flags

The library requires you to explicitly choose one of two mutually exclusive features:
- **sync**: Traditional synchronous API using threads
- **async**: Asynchronous API using tokio

There is no default feature. You must specify exactly one:

```bash
# For sync mode
cargo build --features sync

# For async mode
cargo build --features async
```

If you don't specify a feature, you'll get a helpful compile error explaining how to use the crate.

### Important: Feature Guard Pattern

Since the features are mutually exclusive, you can use simple feature guards:
```rust
#[cfg(feature = "sync")]
```

```rust
#[cfg(feature = "async")]
```

The crate enforces that exactly one feature is enabled at compile time.

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
   - `wsh`: Wall Street Horizon event data
   - `transport`: Connection and message handling
   - `messages`: Message definitions and routing
   - `protocol`: Protocol version constants and feature checking

### Module Organization

Each API module follows a consistent structure to support both sync and async modes:

```
src/<module>/
├── mod.rs         # Public types and module exports
├── common/        # Shared implementation details
│   ├── mod.rs     # Export encoders/decoders
│   ├── encoders.rs # Message encoding functions
│   └── decoders.rs # Message decoding functions
├── sync.rs        # Synchronous implementation
└── async.rs       # Asynchronous implementation
```

Example module structure:
```rust
// src/accounts/mod.rs
//! Account management module with types and API

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(all(feature = "sync", not(feature = "async")))]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags
#[derive(Debug)]
pub enum AccountSummaries {
    Summary(AccountSummary),
    End,
}

#[derive(Debug)]
pub struct AccountSummary {
    pub account: String,
    pub tag: String,
    pub value: String,
    pub currency: String,
}

#[derive(Debug)]
pub struct PnL {
    pub daily_pnl: f64,
    pub unrealized_pnl: Option<f64>,
    pub realized_pnl: Option<f64>,
}

// ... other types ...

// Re-export API functions based on active feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{positions, pnl, account_summary, managed_accounts, server_time};

#[cfg(feature = "async")]
pub use r#async::{positions, pnl, account_summary, managed_accounts, server_time};

// src/accounts/common/mod.rs
pub(super) mod decoders;
pub(super) mod encoders;

// src/accounts/common/encoders.rs
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::Error;

pub(in crate::accounts) fn encode_request_positions() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestPositions);
    message.push_field(&1); // version
    Ok(message)
}

// src/accounts/sync.rs
use super::common::{decoders, encoders};
use super::{AccountSummaries, PnL, PositionUpdate};
use crate::{Client, Error};

impl DataStream<AccountSummaries> for AccountSummaries {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaries::Summary(
                decoders::decode_account_summary(client.server_version, message)?
            )),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaries::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }
}

// src/accounts/async.rs
use super::common::{decoders, encoders};
use super::{AccountSummaries, PnL, PositionUpdate};
use crate::{Client, Error};

impl AsyncDataStream<AccountSummaries> for AccountSummaries {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaries::Summary(
                decoders::decode_account_summary(client.server_version(), message)?
            )),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaries::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }
}
```

This structure ensures:
- Public types are defined in mod.rs and always available
- Shared business logic in common/encoders and common/decoders
- Clear separation between public API (types) and implementation (common/)
- No code duplication for encoding/decoding
- Easy debugging and maintenance

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

To use the crate in your project:
```toml
# For sync mode
ibapi = { version = "2.0", features = ["sync"] }

# For async mode  
ibapi = { version = "2.0", features = ["async"] }
```

## Testing Best Practices

### Integration Test Pattern for Client Module

The client module uses a MockGateway pattern for integration testing that simulates the actual IB Gateway/TWS protocol:

#### Key Components

1. **MockGateway** (`client/common.rs`): A test server that simulates IB Gateway/TWS
   - Binds to a random TCP port and accepts connections
   - Handles the full handshake protocol including magic token exchange
   - Records all incoming requests for verification
   - Sends pre-configured responses based on defined interactions

2. **Shared Test Setup Functions** (`client/common.rs`):
   ```rust
   pub fn setup_connect() -> (MockGateway, String)
   pub fn setup_server_time() -> (MockGateway, String, OffsetDateTime)
   ```

3. **Integration Tests**: Both sync and async modules have identical test cases
   - Tests use actual TCP connections, not mocks
   - Verify the complete protocol flow from connection to response
   - Check both request format and response parsing

#### Example Test Pattern

```rust
// Sync test
#[test]
fn test_server_time() {
    let (gateway, address, expected_server_time) = setup_server_time();
    let client = Client::connect(&address, CLIENT_ID).expect("Failed to connect");
    let server_time = client.server_time().unwrap();
    assert_eq!(server_time, expected_server_time);
    assert_eq!(gateway.requests()[0], "49\01\0");  // Verify exact request format
}

// Async test (identical logic with async/await)
#[tokio::test]
async fn test_server_time() {
    let (gateway, address, expected_server_time) = setup_server_time();
    let client = Client::connect(&address, CLIENT_ID).await.expect("Failed to connect");
    let server_time = client.server_time().await.unwrap();
    assert_eq!(server_time, expected_server_time);
    assert_eq!(gateway.requests()[0], "49\01\0");
}
```

#### Benefits

- **Real Network Testing**: Uses actual TCP connections to test the full network stack
- **Protocol Verification**: Tests the complete handshake and message exchange
- **Request Recording**: Captures all requests for detailed verification
- **Interaction-Based**: Pre-configure expected request/response pairs
- **Shared Logic**: Common setup ensures sync/async tests are identical

### Table-Driven Tests with Shared Data

The codebase uses table-driven tests to ensure comprehensive coverage while sharing test data between sync and async implementations. This approach reduces duplication and ensures both modes are tested identically.

#### Test Structure

Each module should follow this testing pattern:

```
src/<module>/
├── common/
│   ├── test_tables.rs  # Shared test cases and data
│   └── test_data.rs    # Common test fixtures (optional)
├── sync.rs             # Sync implementation with tests
└── async.rs            # Async implementation with tests
```

#### Shared Test Tables

Define test cases in `common/test_tables.rs`:

```rust
// src/<module>/common/test_tables.rs
pub struct ApiTestCase {
    pub name: &'static str,
    pub input: TestInput,
    pub expected: ExpectedResult,
}

pub const API_TEST_CASES: &[ApiTestCase] = &[
    ApiTestCase {
        name: "valid_request",
        input: TestInput { param: "test" },
        expected: ExpectedResult::Success,
    },
    ApiTestCase {
        name: "invalid_parameter",
        input: TestInput { param: "" },
        expected: ExpectedResult::Error("parameter cannot be empty"),
    },
    // ... more test cases
];

// Common test data
pub const TEST_REQUEST_ID: i32 = 9000;
pub const TEST_SERVER_VERSION: i32 = 176;
```

#### Sync Test Implementation

```rust
// In src/<module>/sync.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::<module>::common::test_tables::{API_TEST_CASES, TEST_REQUEST_ID};

    #[test]
    fn test_api_table_driven() {
        for test_case in API_TEST_CASES {
            let result = run_test_case(test_case);
            assert_matches!(result, test_case.expected, 
                "Test '{}' failed", test_case.name);
        }
    }

    fn run_test_case(test_case: &ApiTestCase) -> TestResult {
        // Test implementation using shared test case
        // This logic is specific to sync mode but uses shared test data
    }
}
```

#### Async Test Implementation

```rust
// In src/<module>/async.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::<module>::common::test_tables::{API_TEST_CASES, TEST_REQUEST_ID};

    #[tokio::test]
    async fn test_api_table_driven() {
        for test_case in API_TEST_CASES {
            let result = run_test_case(test_case).await;
            assert_matches!(result, test_case.expected, 
                "Test '{}' failed", test_case.name);
        }
    }

    async fn run_test_case(test_case: &ApiTestCase) -> TestResult {
        // Same test logic as sync but using async/await
        // Uses the same shared test data
    }
}
```

#### Benefits of This Approach

1. **Consistency**: Both sync and async modes are tested with identical test cases
2. **Maintainability**: Add test cases once in the shared table
3. **Coverage**: Comprehensive test scenarios without duplication
4. **Debugging**: Easy to identify which specific test case failed
5. **Documentation**: Test cases serve as documentation of expected behavior

#### Test Data Organization

Use `test_data.rs` for reusable fixtures:

```rust
// src/<module>/common/test_data.rs
pub fn create_test_client() -> Client {
    // Standard test client setup
}

pub fn build_test_message(msg_type: &str, data: &[&str]) -> String {
    format!("{}|{}|", msg_type, data.join("|"))
}

pub const SAMPLE_RESPONSES: &[&str] = &[
    "AccountSummary|9000|DU123456|NetLiquidation|25000.00|USD|",
    "AccountSummaryEnd|9000|",
];
```

### Running Tests for Both Modes

Always test both sync and async implementations:

```bash
# Test sync mode
cargo test <module> --features sync

# Test async mode
cargo test <module> --features async

# Test specific function in both modes
cargo test <module>::<function_name> --features sync
cargo test <module>::<function_name> --features async
```

## Running Examples

Examples follow a naming convention to indicate their mode:

```bash
# Sync examples
cargo run --features sync --example market_data
cargo run --features sync --example positions

# Async examples
cargo run --features async --example async_connect
cargo run --features async --example async_market_data
```

## Testing Patterns

### Testing RequestMessage Fields

When testing `RequestMessage` objects, use direct indexing to assert on individual fields:

```rust
#[test]
fn test_cancel_message() {
    let request = AccountSummaryResult::cancel_message(server_version, Some(request_id), None).unwrap();
    
    // Use direct indexing to test fields
    assert_eq!(request[0], OutgoingMessages::CancelAccountSummary.to_string());
    assert_eq!(request[1], request_id.to_string());
}
```

This approach:
- Tests exact field values and positions
- Avoids substring matching which can give false positives
- Makes test failures clearer by showing exactly which field is wrong
- Works because `RequestMessage` implements `Index<usize>` trait