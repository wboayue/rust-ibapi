# Async Implementation Refactoring Plan

## Overview

This document outlines the refactoring strategy for completing the async implementation in rust-ibapi while maintaining code clarity and clean architecture. The approach prioritizes explicit, debuggable code over maximum DRY principles, accepting some duplication in favor of maintainability.

## Current State Analysis

### What's Working
- ✅ Basic async client connection (`Client::connect`)
- ✅ Async transport layer (`AsyncTcpMessageBus`)
- ✅ Async connection handling with proper handshake
- ✅ Basic async subscription structure
- ✅ Message routing infrastructure
- ✅ Unified builder pattern for sync/async

### What's Missing
- ❌ Async versions of API methods (market data, orders, accounts, etc.)
- ❌ Proper decoder integration in async subscriptions
- ❌ Complete async examples beyond basic connection
- ❌ Stream-based iterators for async subscriptions

## Key Refactoring Opportunities

### 1. Explicit Sync/Async Implementations

Rather than using macros, we'll maintain separate sync and async implementations with shared encoding/decoding logic. This approach favors clarity and debuggability over DRY:

```rust
// src/market_data/sync.rs
impl Client {
    pub fn market_data(
        &self,
        contract: &Contract,
        generic_ticks: &str,
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<MarketData>, Error> {
        let builder = self.request()
            .check_version(MIN_SERVER_VER_SNAPSHOT_MKT_DATA, "market data")?;
        
        let message = encoders::encode_market_data(
            builder.request_id(),
            contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot,
        )?;
        
        builder.send(message)
    }
}

// src/market_data/async.rs
impl Client {
    pub async fn market_data(
        &self,
        contract: &Contract,
        generic_ticks: &str,
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<MarketData>, Error> {
        let builder = self.request()
            .check_version(MIN_SERVER_VER_SNAPSHOT_MKT_DATA, "market data")
            .await?;
        
        let message = encoders::encode_market_data(
            builder.request_id(),
            contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot,
        )?;
        
        builder.send(message).await
    }
}
```

Benefits:
- Easy to debug and understand
- Clear async/await boundaries
- No macro magic to reason about
- IDE support works perfectly
- Can optimize each implementation independently

### 2. Decoder Integration for Async Subscriptions

Implement proper decoder support for async subscriptions:

```rust
// src/subscriptions/async.rs
pub struct Subscription<T> {
    receiver: mpsc::Receiver<Result<ResponseMessage, Error>>,
    decoder: Box<dyn Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send>,
    client: Arc<Client>,
}

impl<T> Subscription<T> {
    pub fn new<D>(
        internal: AsyncInternalSubscription,
        client: Arc<Client>,
        decoder: D,
    ) -> Self
    where
        D: Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send + 'static,
    {
        Self {
            receiver: internal.receiver,
            decoder: Box::new(decoder),
            client,
        }
    }
}

// Stream implementation
impl<T> Stream for Subscription<T> {
    type Item = Result<T, Error>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(Ok(mut msg))) => {
                let result = (self.decoder)(&self.client, &mut msg);
                Poll::Ready(Some(result))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
```

### 3. Common Message Processing

Extract common message routing logic:

```rust
// src/transport/routing.rs
#[derive(Debug)]
pub enum RoutingDecision {
    ByRequestId(i32),
    ByOrderId(i32),
    ByMessageType(IncomingMessages),
    SharedMessage(IncomingMessages),
}

pub fn determine_routing(message: &ResponseMessage) -> RoutingDecision {
    if let Some(request_id) = request_id_index(message.message_type()) {
        if let Ok(id) = message.peek_int(request_id) {
            return RoutingDecision::ByRequestId(id);
        }
    }
    
    if let Some(order_id) = order_id_index(message.message_type()) {
        if let Ok(id) = message.peek_int(order_id) {
            return RoutingDecision::ByOrderId(id);
        }
    }
    
    match message.message_type() {
        IncomingMessages::ManagedAccounts |
        IncomingMessages::NextValidId |
        IncomingMessages::CurrentTime => RoutingDecision::SharedMessage(message.message_type()),
        _ => RoutingDecision::ByMessageType(message.message_type()),
    }
}
```

### 4. Connection Logic Abstraction

Create shared connection establishment logic:

```rust
// src/connection/common.rs
pub struct HandshakeData {
    pub min_version: i32,
    pub max_version: i32,
    pub server_version: i32,
    pub server_time: String,
}

pub trait ConnectionProtocol {
    type Error;
    
    fn format_handshake(&self) -> Vec<u8>;
    fn parse_handshake_response(&self, data: &[u8]) -> Result<HandshakeData, Self::Error>;
    fn format_start_api(&self, client_id: i32) -> RequestMessage;
    fn parse_account_info(&self, message: &ResponseMessage) -> Result<String, Self::Error>;
}

// Shared implementation
pub struct ConnectionHandler;

impl ConnectionProtocol for ConnectionHandler {
    type Error = Error;
    
    fn format_handshake(&self) -> Vec<u8> {
        format!("API\0{MIN_CLIENT_VER}..{MAX_CLIENT_VER}\0").into_bytes()
    }
    
    // ... other methods
}
```

### 5. Module Organization Template

Each API module should follow this structure:

```rust
// src/market_data/mod.rs
mod common;  // Shared types, encoders, decoders

#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Re-export based on features
#[cfg(feature = "sync")]
pub use sync::*;

#[cfg(feature = "async")]
pub use r#async::*;

// Always export common types
pub use common::*;

// src/market_data/common.rs
pub mod encoders {
    pub fn encode_market_data(
        request_id: i32, 
        contract: &Contract,
        generic_ticks: &str,
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<RequestMessage, Error> {
        // Shared encoding logic
        let mut message = RequestMessage::new();
        message.push_field(&OutgoingMessages::RequestMarketData);
        message.push_field(&request_id);
        // ... encode contract and other fields
        Ok(message)
    }
}

pub mod decoders {
    pub fn decode_market_data(server_version: i32, message: &mut ResponseMessage) -> Result<MarketData, Error> {
        // Shared decoding logic
        let tick_type = message.next_int()?;
        let price = message.next_double()?;
        // ... decode remaining fields
        Ok(MarketData { tick_type, price, ... })
    }
}

// src/market_data/sync.rs
use super::common::{encoders, decoders};
use crate::client::{Client, DataStream};

// DataStream implementation
impl DataStream<MarketData> for MarketData {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::TickPrice,
        IncomingMessages::TickSize,
        IncomingMessages::TickGeneric,
        IncomingMessages::TickString,
    ];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_market_data(client.server_version, message)
    }
}

// Client methods
impl Client {
    pub fn market_data(
        &self,
        contract: &Contract,
        generic_ticks: &str,
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<MarketData>, Error> {
        let builder = self.request()
            .check_version(MIN_SERVER_VER_SNAPSHOT_MKT_DATA, "market data")?;
        
        let message = encoders::encode_market_data(
            builder.request_id(),
            contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot,
        )?;
        
        builder.send(message)
    }
}

// src/market_data/async.rs
use super::common::{encoders, decoders};
use crate::client::Client;

impl Client {
    pub async fn market_data(
        &self,
        contract: &Contract,
        generic_ticks: &str,
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<MarketData>, Error> {
        let builder = self.request()
            .check_version(MIN_SERVER_VER_SNAPSHOT_MKT_DATA, "market data")
            .await?;
        
        let message = encoders::encode_market_data(
            builder.request_id(),
            contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot,
        )?;
        
        // Create subscription with decoder
        let internal = self.send_request(builder.request_id(), message).await?;
        Ok(Subscription::new_with_decoder(
            internal,
            self.clone(),
            |client, msg| decoders::decode_market_data(client.server_version, msg),
        ))
    }
}
```

## Implementation Plan

### Phase 1: Foundation (Priority: High)
1. Implement decoder integration for async subscriptions
2. Extract common routing logic
3. Create connection protocol abstraction
4. Set up module structure template for all API modules

### Phase 2: Core Modules (Priority: High)
1. Implement async accounts module
2. Implement async market_data module
3. Implement async orders module
4. Add proper error handling and cancellation

### Phase 3: Additional Modules (Priority: Medium)
1. Implement async contracts module
2. Implement async news module
3. Implement async scanner module
4. Implement async historical_data module

### Phase 4: Polish (Priority: Low)
1. Add comprehensive async examples
2. Add async-specific tests
3. Update documentation
4. Performance optimization

## Testing Strategy

1. **Unit Tests**: Test each async method individually
2. **Integration Tests**: Test full workflows (connect → subscribe → receive data)
3. **Comparison Tests**: Ensure sync and async produce identical results
4. **Stress Tests**: Test with high message volumes
5. **Cancellation Tests**: Ensure proper cleanup on cancellation

## Migration Guide for Users

```rust
// Sync code
let client = Client::connect("127.0.0.1:4002", 100)?;
let positions = client.positions()?;
for position in positions {
    println!("{:?}", position?);
}

// Async equivalent
let client = Client::connect("127.0.0.1:4002", 100).await?;
let mut positions = client.positions().await?;
while let Some(position) = positions.next().await {
    println!("{:?}", position?);
}
```

## Success Metrics

1. **Code Clarity**: Prioritize readability and debuggability over DRY
2. **API Consistency**: Identical method signatures between sync/async (except for async/await)
3. **Performance**: Async should handle 10k+ messages/second
4. **Maintainability**: Clear separation of concerns with shared business logic
5. **Test Coverage**: Maintain 80%+ coverage for both sync and async
6. **Developer Experience**: Easy to understand, modify, and extend

## Risks and Mitigations

1. **Risk**: Breaking existing sync API
   - **Mitigation**: Comprehensive test suite, careful refactoring

2. **Risk**: Performance regression
   - **Mitigation**: Benchmarking, profiling

3. **Risk**: Increased complexity
   - **Mitigation**: Clear documentation, examples

4. **Risk**: Tokio version conflicts
   - **Mitigation**: Use stable tokio features only