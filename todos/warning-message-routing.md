# Warning Message Routing Enhancement

## Status
**Priority:** Medium  
**Type:** Enhancement

## Description
Currently, warning messages (error codes 2100-2200) from IB Gateway/TWS are not routed to subscriptions. Instead, they are only logged as warnings in the transport layer. This affects methods that might receive informational/warning messages from IB.

## Current Behavior
- Warning messages are diverted to `error_event` function
- Only logged, not sent to subscriptions
- Users cannot programmatically react to warnings

## Proposed Solutions

### Option 1: Route warnings to subscriptions
Modify the routing logic in transport layers to send warning messages to subscriptions, not just log them.

### Option 2: Create a separate warning stream
Provide a way for clients to subscribe to warning messages separately from error messages.

### Option 3: Make it configurable
Allow users to opt-in to receiving warnings in their subscriptions via a configuration flag.

## Affected Code
- `src/transport/sync.rs:264-265` - Where warnings are diverted to `error_event` instead of subscriptions
- `src/transport/sync.rs:579-609` - The `error_event` function that only logs warnings
- `src/transport/sync.rs:30` - `WARNING_CODES` constant defining the range (2100..=2169)
- `src/transport/routing.rs:76` - `WARNING_CODE_RANGE` constant
- `src/transport/async.rs` - Async message routing logic (similar handling needed)

## Impact
All methods that might receive informational/warning messages from IB would benefit from being able to handle these programmatically rather than just having them logged.