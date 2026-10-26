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
                oca_group: String, oca_type: OcaType, cond: Option<Condition>) { }

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
pub(in crate::<module>) fn encode_my_request(request_id: i32, param: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::MyRequest {
        req_id: Some(request_id),
        param: Some(param.to_string()),
    };
    Ok(encode_protobuf_message(OutgoingMessages::MyRequest as i32, &request.encode_to_vec()))
}

// src/<module>/common/decoders.rs
//
// The transport is protobuf-only. `message.proto()` decodes the payload as the
// type the `_proto` decoder takes, inferred from that parameter. It returns
// `Error::UnexpectedWireFormat` if a text-framed message reaches this decoder
// (a stale test fixture, or a future-version regression), and maps prost's
// error to `Error::ProtobufDecode`. Either fails the subscription; nothing here
// is silently skipped.
pub(in crate::<module>) fn decode_my_response(message: &ResponseMessage) -> Result<MyData, Error> {
    decode_my_response_proto(message.proto()?)
}

// Takes the proto struct, not bytes, so tests build `crate::proto::MyResponse`
// directly and `expect_proto` can pass it to a one-shot.
pub(crate) fn decode_my_response_proto(p: crate::proto::MyResponse) -> Result<MyData, Error> {
    Ok(MyData {
        field: p.field.unwrap_or_default(),
    })
}
```

For a one-shot, add the payload to `impl_proto_payload!` in `src/proto/payload.rs`
(`MyResponse => MyResponse`). That `ProtoPayload` impl carries the expected message type, so
`expect_proto` takes only the decoder. Full detail:
[one-shot narrowing](rules/wire/one-shot-narrowing.md).

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
use crate::common::request_helpers::{self, expect_proto};
use crate::client::sync::Client;

impl Client {
    pub fn my_function(&self, param: &str) -> Result<MyData, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::MyRequest,
            || encoders::encode_my_request(self.mint_request_id().raw(), param),
            expect_proto(decoders::decode_my_response_proto),
        )
    }
}
```

### Step 6: Implement Async Version

Add an `impl Client` block in the domain module's `async.rs`:

```rust
// src/<module>/async.rs
use super::common::{encoders, decoders};
use crate::common::request_helpers::{self, expect_proto};
use crate::Client;

impl Client {
    pub async fn my_function(&self, param: &str) -> Result<MyData, Error> {
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::MyRequest,
            || encoders::encode_my_request(self.mint_request_id().raw(), param),
            expect_proto(decoders::decode_my_response_proto),
        ).await
    }
}
```

No module re-exports needed — the `impl Client` methods are automatically available on the Client type.

### Step 7: Add Comprehensive Tests

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

### Step 8: Verify Both Modes

Test your implementation in both sync and async modes:

```bash
# Test async implementation  
cargo test <module>::async
cargo clippy

# Test sync implementation
cargo test <module>::sync --no-default-features --features sync
cargo clippy --no-default-features --features sync
```

### Step 9: Add Examples

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

// Validate non-empty inputs
let symbol = error_helpers::require_not_empty(symbol, "symbol")?;

// Validate with custom logic
let valid_value = error_helpers::require_with(some_option, || {
    "value must meet custom criteria".to_string()
})?;
```

### Request Helpers (`src/common/request_helpers.rs`)

Provides common request patterns for both sync and async modes:

```rust
use crate::common::request_helpers::{self, expect_proto};

// For one-shot requests (sync, inside impl Client) — every one-shot retries
// a connection reset; a closed stream is Error::UnexpectedEndOfStream, or chain
// .or_else(empty_on_end_of_stream) when an empty collection is the right answer.
pub fn my_api_call(&self) -> Result<MyData, Error> {
    request_helpers::blocking::one_shot_shared(
        self,
        OutgoingMessages::MyRequest,
        || encode_my_request(self.mint_request_id().raw()),
        expect_proto(decode_my_response_proto),
    )
}

// For one-shot requests (async, inside impl Client)
pub async fn my_api_call(&self) -> Result<MyData, Error> {
    request_helpers::one_shot_shared(
        self,
        OutgoingMessages::MyRequest,
        || encode_my_request(self.mint_request_id().raw()),
        expect_proto(decode_my_response_proto),
    ).await
}

// For requests with IDs and subscriptions (inside impl Client)
pub fn my_subscription(&self) -> Result<Subscription<MyData>, Error> {
    let builder = self.request();
    let request = encode_my_request(builder.request_id())?;
    builder.send(request)
}
```

### Retry Logic (`src/common/retry.rs`)

Handles connection reset scenarios:

Every one-shot helper above already retries; reach for these only in a hand-written request
path (e.g. `historical_data`). The first argument is the client, which waits for the reconnect
before the next attempt:

```rust
// sync
let result = crate::common::retry::blocking::retry_on_connection_reset(client, || my_operation())?;

// async
let result = crate::common::retry::retry_on_connection_reset(client, || async { my_operation().await }).await?;
```