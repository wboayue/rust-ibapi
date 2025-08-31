# Exercise Options Testing

## Status
**Priority:** Low  
**Type:** Production Testing Required

## Description
The `exercise_options()` method has been updated to properly route OrderStatus and OpenOrder messages by using `send_order()` instead of `send_request()`. This needs to be verified with a real valid options contract to ensure it works correctly in production.

## Testing Requirements

1. Obtain a valid, exercisable option contract (e.g., SPY or AAPL option)
2. Test with both American-style options (can exercise before expiry) and European-style options (exercise only at expiry)
3. Verify that OrderStatus messages are received properly
4. Verify that the exercise request is processed by IB Gateway/TWS
5. Test edge cases:
   - Attempting to exercise out-of-the-money options with override flag
   - Exercising options close to expiry
   - Partial exercises (if supported)

## Related Files
- `examples/sync/options_exercise.rs` - Working example that queries the option chain and attempts to exercise a valid contract
- `src/orders/sync.rs` - Sync implementation of exercise_options
- `src/orders/async.rs` - Async implementation of exercise_options

## Notes
- Example code exists and appears to work with mock data
- Needs real market testing to confirm production behavior