# Synchronous Examples

This directory contains synchronous examples demonstrating how to use the rust-ibapi library with the default sync feature.

## Running Examples

All sync examples can be run using:

```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example server_time
```

## Connection Examples

- **connect** - Basic connection to TWS/IB Gateway
- **server_time** - Get current server time
- **managed_accounts** - List managed accounts
- **readme_connection** - Connection example from README

## Account & Portfolio

- **positions** - Get current positions
- **positions_multi** - Get positions for multiple accounts/models
- **pnl** - Real-time P&L updates
- **pnl_single** - P&L for a single position
- **account_summary** - Account summary information
- **account_updates** - Real-time account updates
- **account_updates_multi** - Account updates for multiple accounts
- **family_codes** - Family codes information

## Market Data - Real-time

- **market_data** - Real-time market data subscription
- **market_depth** - Level II market depth
- **market_depth_exchanges** - Market depth exchanges
- **tick_by_tick_last** - Tick-by-tick last trades
- **tick_by_tick_bid_ask** - Tick-by-tick bid/ask quotes
- **tick_by_tick_midpoint** - Tick-by-tick midpoint
- **tick_by_tick_all_last** - All last trades and quotes
- **realtime_bars** - 5-second real-time bars
- **stream_bars** - Streaming bar data
- **stream_retry** - Stream with retry logic
- **switch_market_data_type** - Switch between live/frozen/delayed data

## Market Data - Historical

- **historical_data** - Historical bar data
- **historical_data_adjusted** - Adjusted historical data
- **historical_data_recent** - Recent historical data (last 7 days)
- **historical_data_options** - Historical options data
- **historical_ticks_trade** - Historical trade ticks
- **historical_ticks_bid_ask** - Historical bid/ask ticks
- **historical_ticks_mid_point** - Historical midpoint ticks
- **historical_schedules** - Trading schedules
- **historical_schedules_ending_now** - Trading schedules up to now
- **head_timestamp** - Earliest available data timestamp
- **histogram_data** - Price distribution histogram

## Orders & Execution

- **place_order** - Place a simple order
- **bracket_order** - Place bracket orders (entry + stop loss + take profit)
- **orders** - List open orders
- **cancel_orders** - Cancel orders
- **completed_orders** - List completed orders
- **executions** - Execution details
- **submit_order** - Submit order example
- **next_order_id** - Get next valid order ID
- **options_purchase** - Options purchase order
- **options_exercise** - Exercise options

## Trading Strategies

- **breakout** - Breakout trading strategy example

## Contracts & Instruments

- **contract_details** - Get contract details
- **matching_symbols** - Search for matching symbols
- **option_chain** - Options chain data
- **market_rule** - Market rules (price increments)
- **calculate_option_price** - Calculate theoretical option price
- **calculate_implied_volatility** - Calculate implied volatility

## News

- **news_providers** - List news providers
- **news_bulletins** - News bulletins
- **news_article** - Retrieve news articles
- **contract_news** - News for specific contract
- **historical_news** - Historical news
- **broad_tape_news** - Broad tape news feed

## Scanner

- **scanner_parameters** - Available scanner parameters
- **scanner_subscription_active_stocks** - Scan for active stocks
- **scanner_subscription_complex_orders** - Scan for complex orders

## Wall Street Horizon (WSH) Events

- **wsh_metadata** - WSH metadata
- **wsh_event_data_by_contract** - WSH events for a contract
- **wsh_event_data_by_filter** - WSH events by filter

## README Examples

These examples correspond to code snippets from the main README:

- **readme_connection** - Basic connection
- **readme_historical_data** - Historical data retrieval
- **readme_multi_threading_1** - Multi-threading example 1
- **readme_multi_threading_2** - Multi-threading example 2
- **readme_place_order** - Order placement
- **readme_realtime_data_1** - Real-time data example 1
- **readme_realtime_data_2** - Real-time data example 2

## Prerequisites

1. Install IB Gateway or TWS
2. Enable API connections in the settings
3. Note the port number (default: 4002 for IB Gateway, 7497 for TWS)

## Common Issues

- **Connection Refused**: Make sure IB Gateway/TWS is running and API is enabled
- **Port Already in Use**: Another application might be using the same client ID
- **No Market Data Permissions**: Check your account has appropriate market data subscriptions

## Environment Variables

- `RUST_LOG=debug` - Enable debug logging
- `IBAPI_RECORDING_DIR=/tmp/messages` - Record messages for debugging