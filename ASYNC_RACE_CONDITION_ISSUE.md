# Async Race Condition Root Cause

## The Problem

The async implementation has TWO race conditions:

### 1. Subscribe/Send Race (FIXED)
- Old code subscribed and sent as separate operations
- Fixed by making these atomic in the MessageBus trait

### 2. Connection Setup Race (STILL EXISTS)
During connection establishment, `AsyncConnection::receive_account_info()` reads messages directly from the socket:

```rust
// In receive_account_info()
loop {
    let mut message = self.read_message().await?;  // Direct socket read!
    let info = self.connection_handler.parse_account_info(&mut message)?;
    // ... up to 100 messages
}
```

This consumes ANY messages that arrive, including:
- CurrentTime responses
- Market data
- Order updates
- Any other async responses

These messages are parsed, logged as "Error during account info", and DISCARDED.

## Why Sync Works

The sync version likely has better timing:
1. Connection setup completes
2. Message dispatcher starts
3. Then user requests are made

The async version has concurrent operations that create race conditions.

## Proper Fix

The connection establishment should:
1. Start message processing task FIRST
2. Use the message bus for ALL communication
3. Subscribe to NextValidId and ManagedAccounts channels
4. Send start_api request
5. Wait for responses through subscriptions
6. Never read directly from socket except in the message processing task

## Temporary Workaround

Add a delay after connection to ensure message processing is running:
```rust
let client = Client::connect("127.0.0.1:4002", 100).await?;
tokio::time::sleep(Duration::from_millis(100)).await;  // Let message processing start
```

## Long-term Solution

Redesign connection establishment to use the message bus for all communication, ensuring no messages are lost during startup.