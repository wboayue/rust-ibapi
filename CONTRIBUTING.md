# Contributing to rust-ibapi

## Table of Contents
- [Overview](#overview)
- [Getting Started](#getting-started)
- [Coding Standards](#coding-standards)
- [Domain Type Pattern](#domain-type-pattern)
- [Sync/Async Architecture](#syncasync-architecture)
- [Core Components](#core-components)
- [Request and Response Handling](#request-and-response-handling)
- [Extending the API](#extending-the-api)
- [Troubleshooting](#troubleshooting)
- [Creating and Publishing Releases](#creating-and-publishing-releases)

## Overview

The API is designed to provide a robust, efficient, and flexible interface for communicating with TWS (Trader Workstation) or IB Gateway. This API allows developers to build trading applications in Rust, leveraging its performance and safety features. The architecture supports both **synchronous** (thread-based) and **asynchronous** (tokio-based) operation modes through feature flags.

### Architecture Overview

The rust-ibapi crate supports two mutually exclusive modes:
- **Sync mode** (default): Uses threads and crossbeam channels
- **Async mode**: Uses tokio tasks and mpsc channels

**Core Components:**
- **Client**: Main interface for user interactions (adapts to sync/async mode)
- **MessageBus**: Handles connection and message routing (runs on thread/task)
- **Common Utilities**: Shared patterns and helpers for both modes

The MessageBus establishes the connection to TWS, sends messages from the client to TWS, and listens for and routes messages from TWS to the client via channels.

## Getting Started

1. [Install Rust](https://www.rust-lang.org/tools/install).

2. Install additional development tools:

* [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for code coverage analysis.
* [cargo-audit](https://rustsec.org/) for checking vulnerabilities in dependencies.

```bash
cargo install cargo-tarpaulin
cargo install cargo-audit
```

3. Create a [fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/fork-a-repo) of the repository.

4. Clone your fork and make sure tests are working:

```bash
git clone https://github.com/<your-github-username>/rust-ibapi
cd rust-ibapi

# Test sync mode (default)
cargo test

# Test async mode
cargo test --features async
```

5. Set up your development environment:
   - We recommend using an IDE with Rust support, such as VS Code with the rust-analyzer extension.
   - Configure your IDE to use rustfmt and clippy for code formatting and linting.

6. Make your changes.

* Ensure tests are still passing and coverage hasn't dropped:

```bash
# Test both sync and async modes
cargo test
cargo test --features async

# Check for linting issues
cargo clippy
cargo clippy --features async

# Format code
cargo fmt

# Generate coverage report
cargo tarpaulin -o html
```

* The coverage report will be saved as tarpaulin-report.html. Open it in your browser to view the coverage details.

7. Submit a Pull Request

* Follow GitHub's guide on [creating a pull request from a fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request-from-a-fork).

## Coding Standards

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Please ensure your code adheres to these guidelines. Use `cargo fmt` to format your code and `cargo clippy` to catch common mistakes and improve your Rust code.

## Domain Type Pattern

The codebase uses a newtype pattern for domain-specific types to provide type safety and clarity. This pattern should be followed when adding new domain types.

### Example Implementation

```rust
use std::fmt;
use std::ops::Deref;

/// Account identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

impl Deref for AccountId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AccountId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AccountId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
```

### Key Components

1. **Newtype Wrapper**: Use a tuple struct with a single field (e.g., `AccountId(pub String)`)
2. **Common Traits**: Implement the following traits as appropriate:
   - `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash` - Usually derived
   - `Deref` - Allows transparent access to the inner type's methods
   - `Display` - For user-friendly string representation
   - `From<T>` - For ergonomic conversions from the inner type

3. **Benefits**:
   - **Type Safety**: Prevents mixing up different types of IDs or values
   - **Self-Documenting**: Function signatures clearly indicate expected types
   - **Zero-Cost Abstraction**: No runtime overhead compared to using raw types
   - **IDE Support**: Better autocomplete and type hints

### When to Use This Pattern

Apply this pattern when you have:
- Domain-specific identifiers (AccountId, OrderId, TickerId)
- Domain-specific codes or keys (ModelCode, Symbol)
- Values that could be confused if using primitive types
- Values that benefit from additional type safety

### Testing Domain Types

Include comprehensive tests for your domain types:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deref() {
        let id = AccountId("U123456".to_string());
        assert_eq!(&*id, "U123456");
        assert_eq!(id.len(), 7);  // Uses str method via Deref
    }

    #[test]
    fn test_display() {
        let id = AccountId("U123456".to_string());
        assert_eq!(format!("{}", id), "U123456");
    }

    #[test]
    fn test_from_string() {
        let id = AccountId::from("U123456".to_string());
        assert_eq!(id.0, "U123456");
    }

    #[test]
    fn test_equality() {
        let id1 = AccountId::from("U123456");
        let id2 = AccountId::from("U123456");
        assert_eq!(id1, id2);
    }
}
```

## Sync/Async Architecture

The codebase supports both synchronous and asynchronous operation modes through feature flags. This dual-mode architecture requires specific patterns and organization to maintain consistency and avoid code duplication.

### Feature Flags

The library uses mutually exclusive feature flags:
- **`sync`** (default): Traditional synchronous API using threads
- **`async`**: Asynchronous API using tokio

When both features are enabled, async takes precedence. This allows users to simply add `--features async` for library compilation and testing, but examples require `--no-default-features --features async`.

```bash
# Build with sync mode (default)
cargo build

# Build/test library with async mode (async takes precedence when both features enabled)
cargo build --features async
cargo test --features async

# Build/run async examples (requires --no-default-features)
cargo run --no-default-features --features async --example async_connect
```

### Module Organization

Each API module follows a consistent structure to support both modes:

```
src/<module>/
├── mod.rs         # Public types and module exports
├── common/        # Shared implementation details
│   ├── mod.rs     # Export encoders/decoders
│   ├── encoders.rs # Message encoding functions
│   ├── decoders.rs # Message decoding functions
│   ├── test_tables.rs # Shared test cases (optional)
│   └── test_data.rs # Common test fixtures (optional)
├── sync.rs        # Synchronous implementation
└── async.rs       # Asynchronous implementation
```

### Module Structure Pattern

Follow this pattern when creating new modules:

```rust
// src/<module>/mod.rs
//! Module description

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(all(feature = "sync", not(feature = "async")))]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags
#[derive(Debug)]
pub struct MyData {
    pub field: String,
}

// Re-export API functions based on active feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{my_function};

#[cfg(feature = "async")]
pub use r#async::{my_function};
```

### Feature Guard Pattern

**CRITICAL**: When adding new sync-specific code, ALWAYS use:

```rust
#[cfg(all(feature = "sync", not(feature = "async")))]
```

**NOT** just:
```rust
#[cfg(feature = "sync")]  // DON'T use this alone!
```

This ensures that async mode properly overrides sync mode when both features are enabled.

For async-specific code, use:
```rust
#[cfg(feature = "async")]
```

### Common Utilities

The `src/common/` directory contains shared utilities used by both sync and async implementations:

#### Error Helpers (`src/common/error_helpers.rs`)

Provides validation and error handling utilities:

```rust
use crate::common::error_helpers;

// Validate required parameters
let value = error_helpers::require(some_option, "parameter is required")?;
let request_id = error_helpers::require_request_id_for(request_id, "my operation")?;

// Validate ranges
let port = error_helpers::require_range(port, 1, 65535, "port")?;

// Validate with custom logic
let valid_value = error_helpers::require_with(some_option, || {
    "value must meet custom criteria".to_string()
})?;
```

#### Request Helpers (`src/common/request_helpers.rs`)

Provides common request patterns for both sync and async modes:

```rust
use crate::common::request_helpers;

// For one-shot requests with retry logic (sync)
pub fn my_api_call(client: &Client) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encode_my_request(client.next_request_id()),
        |message| decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    )
}

// For one-shot requests with retry logic (async)
pub async fn my_api_call(client: &Client) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encode_my_request(client.next_request_id()),
        |message| decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    ).await
}

// For requests with IDs and subscriptions
pub fn my_subscription(client: &Client) -> Result<Subscription<MyData>, Error> {
    request_helpers::request_with_id(client, Features::MY_FEATURE, |request_id| {
        encode_my_request(request_id)
    })
}
```

#### Retry Logic (`src/common/retry.rs`)

Handles connection reset scenarios:

```rust
use crate::common::retry;

// Automatically retry on connection reset
let result = retry::retry_on_connection_reset(|| {
    // Your operation that might fail due to connection reset
    my_operation()
})?;
```

### Testing Patterns

#### Table-Driven Tests

Use table-driven tests for comprehensive coverage with shared data:

```rust
// src/<module>/common/test_tables.rs
pub struct ApiTestCase {
    pub name: &'static str,
    pub input: MyInput,
    pub expected: MyExpected,
}

pub const MY_API_TESTS: &[ApiTestCase] = &[
    ApiTestCase {
        name: "valid input",
        input: MyInput { field: "test" },
        expected: MyExpected::Success,
    },
    // ... more test cases
];

// In sync.rs and async.rs tests
#[test] // or #[tokio::test] for async
fn test_my_api_table() {
    use crate::<module>::common::test_tables::MY_API_TESTS;
    
    for test_case in MY_API_TESTS {
        // Test implementation using shared test case
        let result = my_api(&test_case.input);
        assert_eq!(result, test_case.expected, "Test '{}' failed", test_case.name);
    }
}
```

#### Shared Test Data

Create reusable test fixtures:

```rust
// src/<module>/common/test_data.rs
pub const TEST_REQUEST_ID: i32 = 9000;
pub const TEST_VALUE: &str = "test_value";

pub fn build_test_response(message_type: &str, data: &str) -> String {
    format!("{}|{}|{}|", message_type, TEST_REQUEST_ID, data)
}

pub fn create_test_client() -> Client {
    // Standard test client setup
}
```

### Implementation Guidelines

#### Shared Business Logic

Put shared logic in `common/` modules:

```rust
// src/<module>/common/encoders.rs
pub(in crate::<module>) fn encode_my_request(request_id: i32, data: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::MyRequest);
    message.push_field(&request_id);
    message.push_field(data);
    Ok(message)
}

// src/<module>/common/decoders.rs  
pub(in crate::<module>) fn decode_my_response(message: ResponseMessage) -> Result<MyData, Error> {
    let mut fields = message.into_iter();
    fields.next(); // Skip message type
    
    let field = fields.next_string()?;
    Ok(MyData { field })
}
```

#### Sync Implementation

```rust
// src/<module>/sync.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;

pub fn my_function(client: &Client, data: &str) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encoders::encode_my_request(client.next_request_id(), data),
        |message| decoders::decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    )
}
```

#### Async Implementation

```rust
// src/<module>/async.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;

pub async fn my_function(client: &Client, data: &str) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encoders::encode_my_request(client.next_request_id(), data),
        |message| decoders::decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    ).await
}
```

### Testing Both Modes

Ensure your changes work in both sync and async modes:

```bash
# Run tests for sync mode
cargo test <module>

# Run tests for async mode  
cargo test --features async <module>

# Check clippy for both modes
cargo clippy
cargo clippy --features async
```

## Core Components

### MessageBus

The `MessageBus` is a crucial component of the API, running on its own dedicated thread. Its responsibilities include:

* Establishing and maintaining the connection to TWS
* Sending messages from the client to TWS
* Listening for messages from TWS
* Routing incoming messages to the appropriate client channels

Explore [MessageBus implementation](https://github.com/wboayue/rust-ibapi/blob/main/src/client/transport.rs) for more details.

### Client

The Client component runs on the main thread and provides the interface for user interactions with the API. It is responsible for:

* Encoding user requests into the format expected by TWS
* Sending requests to the MessageBus
* Receiving responses from the MessageBus via channels
* Decoding responses and presenting them to the user

Explore [Client API](https://github.com/wboayue/rust-ibapi/blob/main/src/client.rs) for more details.

## Request and Response Handling

The API uses a combination of request IDs and channels to manage the flow of messages:

1. For requests with a request or order ID:

* The Client generates a unique ID for the request.
* The MessageBus creates a dedicated channel for responses based on the request ID.
* Responses related to this request are sent through these channels.

2. For requests without a request or order ID (due to TWS API design):

* The MessageBus creates a shared channel for responses of that request type.
* Responses related to these requests are routed through these shared channels.
* **Note**: Since these responses are not tied to specific request IDs, distinguishing between responses from concurrent requests of the same type requires careful handling.

The recommended application design is a separate Client instance per thread to avoid message routing issues.

## Extending the API

Follow these steps to add new API functionality while maintaining consistency with the sync/async architecture:

### 1. Define Public Types and API Interface

* Define data types in the module's `mod.rs` file - these should be available regardless of feature flags
* The API exposed to the user is defined on the [Client struct](https://github.com/wboayue/rust-ibapi/blob/main/src/client.rs#L33)
* Define the interface for the new API on the Client struct with proper docstrings and examples
* Group the API in the appropriate module: accounts, contracts, market_data, orders, news, or wsh

### 2. Ensure Message Identifiers Are Defined

* Make sure the appropriate [incoming message](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L15) and [outgoing message](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L222) identifiers are defined
* Message identifiers for [incoming messages](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/IncomingMessage.cs) and [outgoing messages](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/OutgoingMessages.cs) can be found in the Interactive Brokers codebase

### 3. Update Message Type to Request ID Map

* When processing messages received from TWS, the request ID needs to be determined
* A [map of message type to request ID position](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L199) is maintained and may need to be updated

### 4. Implement Shared Business Logic

Create the common implementation that both sync and async will use:

```rust
// src/<module>/common/encoders.rs
pub(in crate::<module>) fn encode_my_request(request_id: i32, param: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::MyRequest);
    message.push_field(&request_id);
    message.push_field(param);
    Ok(message)
}

// src/<module>/common/decoders.rs
pub(in crate::<module>) fn decode_my_response(message: ResponseMessage) -> Result<MyData, Error> {
    // Decode TWS response into your data type
}
```

### 5. Implement Sync Version

```rust
// src/<module>/sync.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;

pub fn my_function(client: &Client, param: &str) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encoders::encode_my_request(client.next_request_id(), param),
        |message| decoders::decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    )
}
```

### 6. Implement Async Version

```rust
// src/<module>/async.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;

pub async fn my_function(client: &Client, param: &str) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::MyRequest,
        || encoders::encode_my_request(client.next_request_id(), param),
        |message| decoders::decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    ).await
}
```

### 7. Update Module Exports

```rust
// src/<module>/mod.rs
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{my_function};

#[cfg(feature = "async")]
pub use r#async::{my_function};
```

### 8. Add Comprehensive Tests

Create table-driven tests that work for both sync and async:

```rust
// src/<module>/common/test_tables.rs
pub struct MyApiTestCase {
    pub name: &'static str,
    pub input: &'static str,
    pub expected_result: ApiExpectedResult,
}

pub const MY_API_TESTS: &[MyApiTestCase] = &[
    MyApiTestCase {
        name: "valid request",
        input: "test_input",
        expected_result: ApiExpectedResult::Success { /* ... */ },
    },
    // ... more test cases
];
```

### 9. Verify Both Modes

Test your implementation in both sync and async modes:

```bash
# Test sync implementation
cargo test <module>::sync
cargo clippy

# Test async implementation  
cargo test --features async <module>::async
cargo clippy --features async
```

### 10. Add Examples

* Add examples showing the API usage to the [examples folder](https://github.com/wboayue/rust-ibapi/tree/main/examples)
* Create both sync and async examples if applicable:
  - Sync examples: `examples/sync/my_feature.rs`
  - Async examples: `examples/async/my_feature.rs` 
* Ensure examples are well-documented and demonstrate proper usage patterns

### 11. Update Documentation

* Run coverage analysis to ensure your addition maintains [current coverage](https://coveralls.io/github/wboayue/rust-ibapi?branch=main)
* Use `cargo tarpaulin` to generate coverage reports
* Update any relevant documentation or README sections

## Troubleshooting

The following environment variables are useful for troubleshooting:

* `RUST_LOG` - Changes the log level. Possible values are `trace`, `debug`, `info`, `warn`, `error`.
* `IBAPI_RECORDING_DIR` - If this is set, the library logs messages between the library and TWS to the specified directory.

For example, the following sets the log level to `debug` and instructs the library to log messages between it and TWS to `/tmp/tws-messages`:

```bash
RUST_LOG=debug IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --bin find_contract_details
```

## Creating and publishing releases.

1. Ensure build is clean and tests are passing.

```bash
cargo build --all-targets
cargo test
```

2. Update version number.

* Update version number in [Cargo.toml](https://github.com/wboayue/rust-ibapi/blob/main/Cargo.toml#L3) using [semantic versioning](https://semver.org/).
* Commit and push your changes.

3. Create tag with new version number.

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

4. Create a release.

* [Create release](https://github.com/wboayue/rust-ibapi/releases/new) pointing to new tag.
* Describe changes in release.

5. Publish to crates.io.

* Before publishing, run a dry run to catch any issues:

```bash
cargo publish --dry-run
```

* If everything looks good, publish the crate:

```bash
cargo publish
```
