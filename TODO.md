# TODO

## Test Exercise Options with Valid Contract

The `exercise_options()` method has been updated to properly route OrderStatus and OpenOrder messages by using `send_order()` instead of `send_request()`. This should be verified with a real valid options contract to ensure it works correctly in production.

### Testing Requirements

1. Obtain a valid, exercisable option contract (e.g., SPY or AAPL option)
2. Test with both American-style options (can exercise before expiry) and European-style options (exercise only at expiry)
3. Verify that OrderStatus messages are received properly
4. Verify that the exercise request is processed by IB Gateway/TWS
5. Test edge cases:
   - Attempting to exercise out-of-the-money options with override flag
   - Exercising options close to expiry
   - Partial exercises (if supported)

### Example Test Code

See `examples/sync/options_exercise.rs` for a working example that queries the option chain and attempts to exercise a valid contract.

## Warning Message Routing

Currently, warning messages (error codes 2100-2200) from IB Gateway/TWS are not routed to subscriptions. Instead, they are only logged as warnings in the transport layer (see `transport/sync.rs:error_event`).

This affects methods that might receive informational/warning messages from IB.

### Proposed Solution

Consider one of the following approaches:

1. **Route warnings to subscriptions**: Modify the routing logic in `transport/sync.rs:dispatch_message` to send warning messages to subscriptions, not just log them.

2. **Create a separate warning stream**: Provide a way for clients to subscribe to warning messages separately from error messages.

3. **Make it configurable**: Allow users to opt-in to receiving warnings in their subscriptions.

### Affected Areas

- `transport/sync.rs`: Message routing logic
- `transport/async.rs`: Async message routing logic  
- All methods that might receive informational/warning messages from IB

### Related Code

- `transport/sync.rs:264-265`: Where warnings are diverted to `error_event` instead of subscriptions
- `transport/sync.rs:579-609`: The `error_event` function that only logs warnings
- `transport/sync.rs:31`: `WARNING_CODES` constant defining the range (2100..=2169)

## SnapshotEnd Message Routing Issue

During integration testing of real-time market data methods, it was discovered that `SnapshotEnd` messages (IncomingMessages type 17) are not being properly routed to subscriptions in the test environment. The debug logs show "no recipient found for: ResponseMessage { i: 0, fields: ["17", "1", "9000"] }" when the SnapshotEnd message arrives.

### Issue Details

- The SnapshotEnd message is sent by MockGateway but not received by the subscription
- Other tick messages (Price, Size, Generic, String) are routed correctly
- This affects snapshot market data requests where we expect a SnapshotEnd to signal completion

### Files Affected

- `src/client/sync.rs`: test_market_data() test has commented assertion for snapshot_end
- `src/client/async.rs`: Same issue likely exists in async version
- `src/transport/sync.rs`: Message routing logic may need investigation
- `src/market_data/realtime/mod.rs`: TickTypes::SnapshotEnd variant handling

### Potential Causes

1. The subscription might be closing/dropping before the SnapshotEnd message arrives
2. Message routing logic might not handle SnapshotEnd messages correctly
3. The subscription channel might be full or closed when SnapshotEnd arrives

### Test Case

See `src/client/sync.rs:test_market_data()` where the SnapshotEnd assertion is currently commented out.