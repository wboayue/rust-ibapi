# Extending the API

This guide covers advanced topics for extending the rust-ibapi functionality.

## Anti-Patterns to Avoid

These examples demonstrate violations of principles in [code-style.md](code-style.md#design-principles).

### Duplicated Logic
```rust
// Bad: duplicated validation in sync and async
pub fn my_func(client: &Client, param: &str) -> Result<Data, Error> {
    if param.is_empty() { return Err(Error::InvalidParam); }
    // ...
}
pub async fn my_func(client: &Client, param: &str) -> Result<Data, Error> {
    if param.is_empty() { return Err(Error::InvalidParam); }  // duplicate!
    // ...
}
```

```rust
// Good: shared validation in common/
pub(crate) fn validate_param(param: &str) -> Result<(), Error> {
    if param.is_empty() { return Err(Error::InvalidParam); }
    Ok(())
}

// Usage in sync.rs and async.rs
validate_param(param)?;
```

### Monolithic Functions
```rust
// Bad: function does encoding, validation, and error handling
pub fn place_order(client: &Client, order: &Order) -> Result<(), Error> {
    // 100+ lines of mixed concerns
}
```

```rust
// Good: split by responsibility
pub fn place_order(client: &Client, order: &Order) -> Result<(), Error> {
    validate_order(order)?;
    let request = encode_order(order)?;
    send_and_handle_response(client, request)
}
```

### Large Parameter Lists
```rust
// Bad: 4+ params signal need for builder
fn create_order(action: Action, qty: f64, price: f64, tif: TimeInForce,
                oca: Option<String>, cond: Option<Condition>) { }

// Good: use builder pattern
order_builder::limit_order(action, qty, price)
    .time_in_force(tif)
    .oca_group(oca)
    .condition(cond)
    .build()
```

## Module Organization

Each API module follows a consistent structure to support both sync and async modes:

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

## Module Structure Pattern

Follow this pattern when creating new modules:

```rust
// src/<module>/mod.rs
//! Module description

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags
#[derive(Debug)]
pub struct MyData {
    pub field: String,
}

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
pub use sync::{my_function};

#[cfg(feature = "async")]
pub use r#async::{my_function};
```

## Adding New API Functionality

### Step 1: Define Public Types and API Interface

Define data types in the module's `mod.rs` file - these should be available regardless of feature flags. The API exposed to the user is defined on the Client struct. Define the interface for the new API on the Client struct with proper docstrings and examples.

### Step 2: Ensure Message Identifiers Are Defined

Make sure the appropriate incoming message and outgoing message identifiers are defined in `src/messages.rs`.

### Step 3: Update Message Type to Request ID Map

When processing messages received from TWS, the request ID needs to be determined. A map of message type to request ID position is maintained in `src/messages.rs` and may need to be updated.

### Step 4: Implement Shared Business Logic

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
    let mut fields = message.into_iter();
    fields.next(); // Skip message type
    
    let field = fields.next_string()?;
    Ok(MyData { field })
}
```

### Step 5: Implement Sync Version

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

### Step 6: Implement Async Version

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

### Step 7: Update Module Exports

```rust
// src/<module>/mod.rs
#[cfg(feature = "sync")]
pub use sync::{my_function};

#[cfg(feature = "async")]
pub use r#async::{my_function};
```

### Step 8: Add Comprehensive Tests

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

### Step 9: Verify Both Modes

Test your implementation in both sync and async modes:

```bash
# Test sync implementation
cargo test <module>::sync --features sync
cargo clippy --features sync

# Test async implementation  
cargo test <module>::async --features async
cargo clippy --features async
```

### Step 10: Add Examples

Add examples showing the API usage to the examples folder:
- Sync examples: `examples/sync/my_feature.rs`
- Async examples: `examples/async/my_feature.rs`

Ensure examples are well-documented and demonstrate proper usage patterns.

## Common Utilities

The `src/common/` directory contains shared utilities used by both sync and async implementations:

### Error Helpers (`src/common/error_helpers.rs`)

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

### Request Helpers (`src/common/request_helpers.rs`)

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

### Retry Logic (`src/common/retry.rs`)

Handles connection reset scenarios:

```rust
use crate::common::retry;

// Automatically retry on connection reset
let result = retry::retry_on_connection_reset(|| {
    // Your operation that might fail due to connection reset
    my_operation()
})?;
```