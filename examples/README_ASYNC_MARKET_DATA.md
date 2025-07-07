# Async Market Data Examples

This directory contains comprehensive examples demonstrating all async market data functionality in the ibapi crate.

## Available Examples

### Real-time Market Data

1. **async_market_data.rs** - Real-time market data subscription
   - Demonstrates `market_data()` method
   - Shows how to receive price, size, and string ticks
   - Usage: `cargo run --features async --example async_market_data`

2. **async_realtime_bars.rs** - 5-second real-time bars
   - Demonstrates `realtime_bars()` method
   - Shows OHLCV bar data streaming
   - Usage: `cargo run --features async --example async_realtime_bars`

3. **async_tick_by_tick.rs** - Tick-by-tick all data types
   - Demonstrates `tick_by_tick_all_last()`, `tick_by_tick_bid_ask()`, `tick_by_tick_midpoint()`
   - Shows trades, quotes, and midpoint ticks
   - Usage: `cargo run --features async --example async_tick_by_tick`

4. **async_tick_by_tick_last.rs** - Tick-by-tick trades only
   - Demonstrates `tick_by_tick_last()` method
   - Shows Last trades without quotes
   - Usage: `cargo run --features async --example async_tick_by_tick_last`

5. **async_market_depth.rs** - Market depth (Level II) data
   - Demonstrates `market_depth()` and `market_depth_exchanges()` methods
   - Shows order book updates and available exchanges
   - Usage: `cargo run --features async --example async_market_depth`

### Historical Market Data

6. **async_head_timestamp.rs** - Earliest data availability
   - Demonstrates `head_timestamp()` method
   - Shows how to check when historical data begins
   - Usage: `cargo run --features async --example async_head_timestamp`

7. **async_historical_data.rs** - Historical bar data
   - Demonstrates `historical_data()` method
   - Shows how to retrieve OHLCV bars for past periods
   - Usage: `cargo run --features async --example async_historical_data`

8. **async_historical_schedule.rs** - Trading schedule information
   - Demonstrates `historical_schedule()` method
   - Shows trading sessions and market hours
   - Usage: `cargo run --features async --example async_historical_schedule`

9. **async_historical_ticks.rs** - Historical bid/ask ticks
   - Demonstrates `historical_ticks_bid_ask()` method
   - Shows historical quote data retrieval
   - Usage: `cargo run --features async --example async_historical_ticks`

10. **async_historical_ticks_midpoint.rs** - Historical midpoint ticks
    - Demonstrates `historical_ticks_mid_point()` method
    - Shows historical midpoint price data
    - Usage: `cargo run --features async --example async_historical_ticks_midpoint`

11. **async_historical_ticks_trade.rs** - Historical trade ticks
    - Demonstrates `historical_ticks_trade()` method
    - Shows historical trade data with analysis
    - Usage: `cargo run --features async --example async_historical_ticks_trade`

12. **async_histogram_data.rs** - Price distribution histogram
    - Demonstrates `histogram_data()` method
    - Shows price frequency distribution
    - Usage: `cargo run --features async --example async_histogram_data`

## Prerequisites

- IB Gateway or TWS running with API connections enabled
- Default connection: 127.0.0.1:4002 (IB Gateway paper trading)
- For TWS: use port 7496 (live) or 7497 (paper)

## Common Patterns

All examples follow similar patterns:

1. **Connection**: Establish async connection to IB Gateway/TWS
2. **Contract Creation**: Define the instrument to query
3. **Request Data**: Call the appropriate market data method
4. **Process Stream**: Handle the async stream of data using futures
5. **Error Handling**: Properly handle errors and connection issues

## Notes

- Market data subscriptions require appropriate market data permissions
- Some data types may not be available for all instruments
- Rate limits apply - avoid making too many requests too quickly
- Examples use common liquid symbols (AAPL, SPY, TSLA, EUR/USD)
- Adjust symbols and parameters as needed for your use case