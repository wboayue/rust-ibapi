# Async Message Bus Redesign - Proper Async Patterns

## Root Cause Analysis

The async implementation incorrectly separated subscribe and send operations:

**Sync (Correct)**:
```rust
fn send_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error>
```

**Async (Incorrect)**:
```rust
async fn subscribe(&self, request_id: i32) -> AsyncInternalSubscription;
async fn send_request(&self, request: RequestMessage) -> Result<(), Error>;
```

This separation creates an inherent race condition where responses can arrive before subscriptions exist.

## Proposed Solution

### 1. Fix the AsyncMessageBus Trait

```rust
#[async_trait]
pub trait AsyncMessageBus: Send + Sync {
    /// Atomic subscribe + send for requests with IDs
    async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;
    
    /// Atomic subscribe + send for orders
    async fn send_order_request(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;
    
    /// Atomic subscribe + send for shared channels
    async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;
    
    /// Send without expecting response
    async fn send_message(&self, message: RequestMessage) -> Result<(), Error>;
    
    /// Cancel operations
    async fn cancel_subscription(&self, request_id: i32, message: RequestMessage) -> Result<(), Error>;
    async fn cancel_order_subscription(&self, order_id: i32, message: RequestMessage) -> Result<(), Error>;
    
    /// Order update stream
    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error>;
}
```

### 2. Update AsyncTcpMessageBus Implementation

```rust
#[async_trait]
impl AsyncMessageBus for AsyncTcpMessageBus {
    async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Create channel first
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Insert into map BEFORE sending
        {
            let mut channels = self.request_channels.write().await;
            channels.insert(request_id, sender);
        }
        
        // Now send the request - any response will find the channel
        self.connection.write_message(&message).await?;
        
        // Return subscription with cleanup
        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Request(request_id)
        ))
    }
    
    async fn send_order_request(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Same pattern for orders
        let (sender, receiver) = mpsc::unbounded_channel();
        
        {
            let mut channels = self.order_channels.write().await;
            channels.insert(order_id, sender);
        }
        
        self.connection.write_message(&message).await?;
        
        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Order(order_id)
        ))
    }
    
    async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // For shared channels, we might have existing subscribers
        let (sender, receiver) = mpsc::unbounded_channel();
        
        {
            let mut channels = self.shared_channels.write().await;
            // TODO: Consider if we need multicast for shared channels
            channels.insert(message_type, sender);
        }
        
        self.connection.write_message(&message).await?;
        
        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Shared(message_type)
        ))
    }
    
    async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
        // For fire-and-forget messages
        self.connection.write_message(&message).await
    }
}
```

### 3. Update Client Builders

The builders should use the atomic operations:

```rust
// request_builder.rs
impl RequestBuilder {
    pub async fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error> {
        // Use atomic send_request
        let subscription = self.client
            .message_bus
            .send_request(self.request_id, message)
            .await?;
            
        Ok(Subscription::new_from_internal::<T>(subscription))
    }
}

// shared_request_builder.rs
impl SharedRequestBuilder {
    pub async fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error> {
        // Use atomic send_shared_request
        let subscription = self.client
            .message_bus
            .send_shared_request(self.message_type, message)
            .await?;
            
        Ok(Subscription::new_from_internal::<T>(subscription))
    }
}

// order_request_builder.rs  
impl OrderRequestBuilder {
    pub async fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error> {
        // Use atomic send_order_request
        let subscription = self.client
            .message_bus
            .send_order_request(self.order_id, message)
            .await?;
            
        Ok(Subscription::new_from_internal::<T>(subscription))
    }
}
```

### 4. Remove Old Methods

After updating all usage sites, remove the separate subscribe methods from AsyncMessageBus trait to prevent future misuse.

### 5. Benefits

1. **No Race Conditions**: Channel exists before request is sent
2. **Simpler API**: Matches sync pattern
3. **No Buffering Needed**: Proper ordering eliminates need for hacks
4. **Type Safe**: Can't accidentally send without subscribing
5. **Better Performance**: No need to check/buffer unrouted messages

### 6. Migration Steps

1. Add new atomic methods to AsyncMessageBus trait
2. Implement them in AsyncTcpMessageBus
3. Update all builders to use new methods
4. Update all direct usage of message bus
5. Remove old subscribe/send methods
6. Add tests to verify no race conditions

### 7. Testing Strategy

```rust
#[tokio::test]
async fn test_no_race_condition() {
    // Inject artificial delay in message processing
    // Verify messages are still delivered
    
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    
    // Run many iterations to catch races
    for _ in 0..100 {
        let time = client.server_time().await?;
        assert!(time.unix_timestamp() > 0);
    }
}

#[tokio::test] 
async fn test_concurrent_requests() {
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    
    // Launch many concurrent requests
    let handles: Vec<_> = (0..50).map(|_| {
        let client = client.clone();
        tokio::spawn(async move {
            client.server_time().await
        })
    }).collect();
    
    // All should succeed
    for handle in handles {
        assert!(handle.await?.is_ok());
    }
}
```