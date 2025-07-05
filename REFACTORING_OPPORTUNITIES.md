# Refactoring Opportunities for Sync/Async Support

## Overview

After successfully extracting the Subscription implementation to its own module, several refactoring opportunities have been identified that would simplify the sync/async dual support implementation.

## Key Refactoring Opportunities

### 1. **Extract MessageBus Trait to Common Module** ✅ COMPLETED

Currently, the `MessageBus` trait is in `transport/sync/sync_message_bus.rs`. We should extract it to a common location:

```rust
// src/transport/mod.rs
#[cfg(feature = "sync")]
pub trait MessageBus: Send + Sync {
    fn send_message(&self, packet: &RequestMessage) -> Result<(), Error>;
    fn create_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;
    // ... other methods
}

#[cfg(feature = "async")]
#[async_trait]
pub trait MessageBus: Send + Sync {
    async fn send_message(&self, packet: &RequestMessage) -> Result<(), Error>;
    async fn create_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;
    // ... other async methods
}
```

### 2. **Simplify Transport Layer Structure** ✅ COMPLETED

The current structure has some redundancy. We could simplify:

```
Current:
transport/
├── mod.rs
├── sync/
│   ├── mod.rs
│   ├── sync_message_bus.rs
│   └── sync_message_bus/
│       └── tests.rs

Better:
transport/
├── mod.rs (common traits/types)
├── sync.rs (sync implementation)
├── async.rs (async implementation)
└── tests/
    ├── sync.rs
    └── async.rs
```

### 3. **Extract Common Response Processing Logic** ✅ COMPLETED

Much of the response processing logic in Subscription could be shared:

```rust
// src/subscriptions/common.rs
pub(crate) fn should_retry_error(error: &Error) -> bool {
    matches!(error, Error::UnexpectedResponse(_))
}

pub(crate) fn is_stream_end(error: &Error) -> bool {
    matches!(error, Error::EndOfStream)
}

pub(crate) fn should_store_error(error: &Error) -> bool {
    !is_stream_end(error)
}
```

### 4. **Separate Connection Logic** ✅ COMPLETED

The `Connection` struct and logic could be better organized:

```rust
// src/connection/mod.rs
pub struct ConnectionMetadata {
    pub server_version: i32,
    pub connection_time: Option<OffsetDateTime>,
    pub time_zone: Option<&'static Tz>,
    pub client_id: i32,
    pub next_order_id: i32,
}

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;
```

### 5. **Create Request/Response Builder Pattern** ✅ COMPLETED

Many client methods follow a similar pattern. We could extract this:

```rust
// src/client/request_builder.rs
pub struct RequestBuilder<'a> {
    client: &'a Client,
    message_type: OutgoingMessages,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(client: &'a Client, message_type: OutgoingMessages) -> Self {
        Self { client, message_type }
    }

    #[cfg(feature = "sync")]
    pub fn send(self, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.client.send_request(self.message_type, message)
    }

    #[cfg(feature = "async")]
    pub async fn send(self, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.client.send_request(self.message_type, message).await
    }
}
```

### 6. **Consolidate Error Handling** ✅ COMPLETED

Create a common error handling module:

```rust
// src/client/error_handler.rs
pub(crate) fn handle_connection_error(error: &Error) -> bool {
    matches!(error, Error::Io(_) if is_connection_reset(error))
}

pub(crate) fn should_retry_request(error: &Error, retry_count: u32) -> bool {
    handle_connection_error(error) && retry_count < MAX_RETRIES
}
```

### 7. **Extract ID Generation to Separate Module**

The ID generation logic could be its own module:

```rust
// src/client/id_generator.rs
use std::sync::atomic::{AtomicI32, Ordering};

#[derive(Debug)]
pub struct IdGenerator {
    next_id: AtomicI32,
}

impl IdGenerator {
    pub fn new(start: i32) -> Self {
        Self {
            next_id: AtomicI32::new(start),
        }
    }

    pub fn next(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}
```

### 8. **Simplify Client Method Organization**

Group related methods into modules:

```rust
// src/client/sync.rs
mod accounts;
mod market_data;
mod orders;
mod contracts;

impl Client {
    // Core methods stay here
    pub fn connect(...) -> Result<Self> { ... }
    pub fn server_version(&self) -> i32 { ... }
    
    // Delegate to modules
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        accounts::positions(self)
    }
}
```

### 9. **Create Protocol Version Constants Module**

Extract all the server version checks:

```rust
// src/protocol/versions.rs
pub const MIN_SERVER_VERSION: i32 = 100;
pub const SUPPORTS_SCANNER_GENERIC_OPTS: i32 = 109;
pub const SUPPORTS_ALGO_ID: i32 = 113;
// ... etc

pub fn check_version(server: i32, required: i32, feature: &str) -> Result<(), Error> {
    if server < required {
        Err(Error::unsupported_version(server, required, feature))
    } else {
        Ok(())
    }
}
```

### 10. **Unified Subscription Creation**

Create a builder pattern for subscriptions:

```rust
// src/client/subscription_builder.rs
pub struct SubscriptionBuilder<'a, T> {
    client: &'a Client,
    request_type: OutgoingMessages,
    context: ResponseContext,
    _phantom: PhantomData<T>,
}

impl<'a, T: DataStream<T>> SubscriptionBuilder<'a, T> {
    #[cfg(feature = "sync")]
    pub fn build(self, subscription: InternalSubscription) -> Subscription<'a, T> {
        Subscription::new(self.client, subscription, self.context)
    }
    
    #[cfg(feature = "async")]
    pub async fn build(self, subscription: InternalSubscription) -> AsyncSubscription<T> {
        AsyncSubscription::new(subscription, self.context)
    }
}
```

## Benefits of These Refactorings

1. **Clearer separation** between sync/async code paths
2. **Less duplication** when implementing async versions
3. **Easier testing** with extracted logic
4. **Better organization** for maintenance
5. **Simpler feature flag management**
6. **Reusable patterns** for both sync and async

## Priority Order

1. **Extract MessageBus trait** (foundational) ✅ COMPLETED
2. **Simplify transport structure** (reduces complexity) ✅ COMPLETED
3. **Extract common response processing** (immediate reuse) ✅ COMPLETED
4. **Separate Connection logic** ✅ COMPLETED
5. **Create request/response builders** (simplifies client methods)
6. **Extract ID generation**
7. **Create protocol version module**
8. **Other refactorings** as time permits

## Implementation Status

- [x] Extract Subscription to separate module
- [x] Extract MessageBus trait to common module (COMPLETED)
- [x] Simplify transport layer structure (COMPLETED)
- [x] Extract common response processing logic (COMPLETED)
- [x] Separate Connection logic (COMPLETED)
- [x] Create request/response builder pattern (COMPLETED)
- [x] Consolidate error handling (COMPLETED)
- [ ] Extract ID generation
- [ ] Simplify client method organization
- [ ] Create protocol version constants module
- [ ] Unified subscription creation