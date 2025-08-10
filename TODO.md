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