# Async Market Data Test Coverage Analysis

## Current State

### Sync Market Data Tests (Found)

The sync implementation has comprehensive test coverage in `/src/market_data/realtime/sync.rs`:

1. **Realtime Market Data Tests**:
   - `test_realtime_bars` - Tests realtime bar data subscription
   - `test_tick_by_tick_all_last` - Tests tick-by-tick AllLast trades
   - `test_tick_by_tick_last` - Tests tick-by-tick Last trades  
   - `test_tick_by_tick_bid_ask` - Tests tick-by-tick bid/ask data
   - `test_tick_by_tick_midpoint` - Tests tick-by-tick midpoint data
   - `test_market_depth` - Tests market depth subscription
   - `test_market_depth_exchanges` - Tests market depth exchange list
   - `test_basic_market_data` - Tests various tick types (price, size, string, generic)
   - `test_market_data_with_combo_legs` - Tests market data with combo legs
   - `test_market_data_with_delta_neutral` - Tests market data with delta neutral contracts
   - `test_market_data_regulatory_snapshot` - Tests regulatory snapshot requests
   - `test_market_data_error_handling` - Tests error and notice handling
   - `test_validate_tick_by_tick_request` - Tests request validation

2. **Historical Market Data Tests** in `/src/market_data/historical/sync.rs`:
   - `test_head_timestamp` - Tests earliest data timestamp retrieval
   - `test_histogram_data` - Tests histogram data retrieval
   - `test_historical_data` - Tests historical bar data
   - `test_historical_schedule` - Tests trading schedule retrieval
   - `test_historical_ticks_bid_ask` - Tests historical bid/ask ticks
   - `test_historical_ticks_mid_point` - Tests historical midpoint ticks
   - `test_historical_ticks_trade` - Tests historical trade ticks
   - `test_historical_data_version_check` - Tests server version validation
   - `test_historical_data_adjusted_last_validation` - Tests AdjustedLast parameter validation
   - `test_historical_data_error_response` - Tests error response handling
   - `test_historical_data_unexpected_response` - Tests unexpected response handling
   - `test_tick_subscription_methods` - Tests tick subscription iterator methods
   - `test_tick_subscription_buffer_and_iteration` - Tests tick buffering and iteration
   - `test_tick_subscription_owned_iterator` - Tests owned iterator implementation
   - `test_tick_subscription_bid_ask` - Tests bid/ask tick subscription
   - `test_tick_subscription_midpoint` - Tests midpoint tick subscription
   - `test_historical_data_time_zone_handling` - Tests timezone handling
   - `test_time_zone_fallback` - Tests timezone fallback to UTC

### Async Market Data Tests (Missing)

The async implementation has **NO TESTS** in either:
- `/src/market_data/realtime/async.rs` 
- `/src/market_data/historical/async.rs`

## Missing Test Coverage

### Realtime Async Tests Needed:

1. **Basic Functionality**:
   - Async realtime bars subscription
   - Async tick-by-tick trades (AllLast, Last)
   - Async tick-by-tick bid/ask
   - Async tick-by-tick midpoint
   - Async market depth
   - Async market depth exchanges
   - Async market data (various tick types)

2. **Advanced Features**:
   - Async combo leg contracts
   - Async delta neutral contracts
   - Async regulatory snapshots
   - Async error/notice handling

3. **Async-Specific**:
   - Stream-based iteration (using futures::StreamExt)
   - Concurrent subscriptions
   - Cancellation handling
   - Timeout behavior

### Historical Async Tests Needed:

1. **Basic Functionality**:
   - Async head timestamp
   - Async histogram data
   - Async historical data
   - Async historical schedule
   - Async historical ticks (bid/ask, midpoint, trade)

2. **Advanced Features**:
   - Async error handling
   - Async retry logic
   - Async timezone handling

3. **Async-Specific**:
   - Async tick subscription streaming
   - Buffer management in async context
   - Concurrent historical requests

## Comparison with Other Modules

Other async modules like `accounts/async.rs` and `wsh/async.rs` have `#[tokio::test]` annotated tests, showing that async testing is supported in the codebase. The market data async modules are notably missing this test coverage.

## Examples vs Tests

While there are many async market data examples in `/examples/`:
- `async_market_data.rs`
- `async_market_depth.rs`
- `async_realtime_bars.rs`
- `async_tick_by_tick.rs`
- `async_historical_data.rs`
- etc.

These examples are not a substitute for proper unit tests with mocked responses.

## Recommendations

1. **Priority**: Add comprehensive async tests matching the sync test coverage
2. **Approach**: Use `#[tokio::test]` with `MessageBusStub` mocking similar to accounts module
3. **Coverage**: Ensure all public async functions have corresponding tests
4. **Async-specific**: Add tests for streaming, cancellation, and concurrent operations