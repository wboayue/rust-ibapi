# Extending the API

This guide covers advanced topics for extending the rust-ibapi functionality.

## Anti-Patterns to Avoid

These examples demonstrate violations of principles in [code-style.md](code-style.md#design-principles).

### Duplicated Logic
```rust
// Bad: duplicated validation in sync and async
impl Client {
    pub fn my_func(&self, param: &str) -> Result<Data, Error> {
        if param.is_empty() { return Err(Error::InvalidParam); }
        // ...
    }
}
impl Client {
    pub async fn my_func(&self, param: &str) -> Result<Data, Error> {
        if param.is_empty() { return Err(Error::InvalidParam); }  // duplicate!
        // ...
    }
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
// Bad: method does encoding, validation, and error handling
impl Client {
    pub fn place_order(&self, order: &Order) -> Result<(), Error> {
        // 100+ lines of mixed concerns
    }
}
```

```rust
// Good: split by responsibility
impl Client {
    pub fn place_order(&self, order: &Order) -> Result<(), Error> {
        validate_order(order)?;
        let request = encode_order(order)?;
        send_and_handle_response(self, request)
    }
}
```

### Large Parameter Lists
```rust
// Bad: 4+ params signal need for builder.
fn create_order(action: Action, qty: f64, price: f64, tif: TimeInForce,
                oca_group: String, oca_type: i32, cond: Option<Condition>) { }

// Good: fluent builder on Client.
client.order(&contract)
    .buy(qty)
    .limit(price)
    .time_in_force(tif)
    .oca_group(oca_group, oca_type)
    .condition(cond)
    .submit()
```

## Module Organization

Each API module follows a consistent structure to support both sync and async modes:

```
src/<module>/
├── mod.rs         # Public types and module exports
├── common/        # Shared implementation details
│   ├── mod.rs     # Export encoders/decoders
│   ├── encoders.rs # Message encoding functions
│   ├── decoders.rs # Protobuf decoding functions
│   └── test_tables.rs # Shared test cases (optional)
├── sync.rs        # Synchronous implementation
├── async.rs       # Asynchronous implementation
├── sync_tests.rs  # Tests for sync.rs (flat sibling, never inline `mod tests`)
└── async_tests.rs # Tests for async.rs
```

Response fixtures do **not** live in the module. They belong to the crate-wide
`src/testdata/builders/<domain>.rs`, one builder per domain implementing `ResponseProtoEncoder`.

Why the client surface belongs here rather than in `client/`, and where helper modules may
nest: [domain module layout](rules/style/domain-module-layout.md).

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

```

## Adding New API Functionality

### Step 1: Define Public Types and API Interface

Define data types in the module's `mod.rs` file - these should be available regardless of feature flags. The API is exposed as `impl Client` methods in the domain module's `sync.rs` / `async.rs` files (not in `client/sync.rs` or `client/async.rs`).

### Step 2: Ensure Message Identifiers Are Defined

Make sure the appropriate incoming message and outgoing message identifiers are defined in `src/messages.rs`.

### Step 3: Register the Message for Request-ID Routing

When processing messages received from TWS, the dispatcher needs to know which request a
message belongs to. `text_request_id_field(kind) -> Option<usize>` in `src/messages.rs` is
the single source of truth; `routes_by_request_id(kind) -> bool` is a thin wrapper over it.
Add an entry for any new inbound message type that correlates to a request.

This is an allow-list, not a sentinel — it deliberately prevents misrouting messages where
`int @ tag 1` means something else (`MarketRule.market_rule_id`, `OrderBound.perm_id`).
`ResponseMessage::request_id()` short-circuits on a missing entry *before* reaching its
protobuf-envelope branch, so a message with no entry silently never routes.

`MessageBusStub` tests sit below the dispatcher and pass with the registration missing, but
they do run the subscription constructor, where `debug_assert_request_id_routable` catches it
(#730). Full detail: [proto-aware accessors](rules/wire/proto-aware-accessors.md).

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
//
// The transport is protobuf-only. `require_proto()` hands back the payload from
// `raw_bytes`, or returns `Error::UnexpectedWireFormat` if a text-framed message
// reaches this decoder (a stale test fixture, or a future-version regression).
// It fails the subscription; nothing here is silently skipped.
pub(in crate::<module>) fn decode_my_response(message: &ResponseMessage) -> Result<MyData, Error> {
    decode_my_response_proto(message.require_proto()?)
}

pub(crate) fn decode_my_response_proto(bytes: &[u8]) -> Result<MyData, Error> {
    let p = crate::proto::MyResponse::decode(bytes)?;
    Ok(MyData {
        field: p.field.unwrap_or_default(),
    })
}
```

For any `String` field that looks like it carries a fixed vocabulary, verify against captured
wire fixtures and the C# reference before typing it as an enum — field-name resemblance is
misleading. Once verified, add `impl FromStr<Err = Error>` and decode with the generic
`parse_required` / `parse_optional` helpers in `src/proto/decoders.rs` rather than falling
back to `T::default()`, which masks incomplete TWS responses. Full detail:
[wire enum typing](rules/wire/enum-typing.md).

For `StreamDecoder`, list every type your `decode` match handles in `RESPONSE_MESSAGE_IDS` — the subscription drivers skip anything not listed there before `decode` runs, so an arm for an unlisted type is dead code. End the match with `_ => Err(Error::unexpected_response(message))` as a backstop for the two disagreeing; never `NotImplemented` or `Simple`. Full detail: [proto-only decoding](rules/wire/proto-only-decoding.md).

### Step 5: Implement Sync Version

Add an `impl Client` block in the domain module's `sync.rs`:

```rust
// src/<module>/sync.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;
use crate::client::sync::Client;

impl Client {
    pub fn my_function(&self, param: &str) -> Result<MyData, Error> {
        request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::MyRequest,
            || encoders::encode_my_request(self.next_request_id(), param),
            |message| decoders::decode_my_response(message),
            || Err(Error::UnexpectedEndOfStream),
        )
    }
}
```

### Step 6: Implement Async Version

Add an `impl Client` block in the domain module's `async.rs`:

```rust
// src/<module>/async.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers;
use crate::Client;

impl Client {
    pub async fn my_function(&self, param: &str) -> Result<MyData, Error> {
        request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::MyRequest,
            || encoders::encode_my_request(self.next_request_id(), param),
            |message| decoders::decode_my_response(message),
            || Err(Error::UnexpectedEndOfStream),
        ).await
    }
}
```

No module re-exports needed — the `impl Client` methods are automatically available on the Client type.

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

// For one-shot requests with retry logic (sync, inside impl Client)
pub fn my_api_call(&self) -> Result<MyData, Error> {
    request_helpers::blocking::one_shot_with_retry(
        self,
        OutgoingMessages::MyRequest,
        || encode_my_request(self.next_request_id()),
        |message| decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    )
}

// For one-shot requests with retry logic (async, inside impl Client)
pub async fn my_api_call(&self) -> Result<MyData, Error> {
    request_helpers::one_shot_with_retry(
        self,
        OutgoingMessages::MyRequest,
        || encode_my_request(self.next_request_id()),
        |message| decode_my_response(message),
        || Err(Error::UnexpectedEndOfStream),
    ).await
}

// For requests with IDs and subscriptions (inside impl Client)
pub fn my_subscription(&self) -> Result<Subscription<MyData>, Error> {
    request_helpers::blocking::request_with_id(self, Features::MY_FEATURE, |request_id| {
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