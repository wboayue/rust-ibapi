# rust-ibapi Client Public Methods

This document lists all public methods available on the sync and async `Client` implementations. This list can be used to verify that examples exist for each public method.

## Connection Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `connect` | ✓ | ✓ | Establishes connection to TWS or Gateway |
| `client_id` | ✓ | ✗ | Returns the ID assigned to the Client |
| `next_request_id` | ✓ | ✗ | Returns the next request ID |
| `next_order_id` | ✓ | ✓ | Returns and increments the order ID |
| `next_valid_order_id` | ✓ | ✓ | Gets the next valid order ID from TWS server |
| `server_version` | ✓ | ✓ | Returns the version of the TWS API server |
| `connection_time` | ✓ | ✓ | Returns the time of the server when connected |

## Account Management Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `server_time` | ✓ | ✓ | TWS's current time |
| `positions` | ✓ | ✓ | Subscribes to position updates for all accessible accounts |
| `positions_multi` | ✓ | ✓ | Subscribes to position updates for account and/or model |
| `pnl` | ✓ | ✓ | Creates subscription for real time daily PnL and unrealized PnL updates |
| `pnl_single` | ✓ | ✓ | Requests real time updates for daily PnL of individual positions |
| `account_summary` | ✓ | ✓ | Requests a specific account's summary |
| `account_updates` | ✓ | ✓ | Subscribes to a specific account's information and portfolio |
| `account_updates_multi` | ✓ | ✓ | Requests account updates for account and/or model |
| `managed_accounts` | ✓ | ✓ | Requests the accounts to which the logged user has access to |
| `family_codes` | ✓ | ✓ | Get current family codes for all accessible accounts |

## Contract Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `contract_details` | ✓ | ✗ | Requests contract information |
| `market_rule` | ✓ | ✗ | Requests details about a given market rule |
| `matching_symbols` | ✓ | ✗ | Requests matching stock symbols |
| `calculate_option_price` | ✓ | ✗ | Calculates an option's price based on volatility |
| `calculate_implied_volatility` | ✓ | ✗ | Calculates implied volatility based on option price |
| `option_chain` | ✓ | ✗ | Requests security definition option parameters |

## Order Management Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `all_open_orders` | ✓ | ✓ | Requests all current open orders in associated accounts |
| `auto_open_orders` | ✓ | ✓ | Requests status updates about future orders placed from TWS |
| `cancel_order` | ✓ | ✓ | Cancels an active Order |
| `completed_orders` | ✓ | ✓ | Requests completed Orders |
| `executions` | ✓ | ✓ | Requests current day's executions matching the filter |
| `global_cancel` | ✓ | ✓ | Cancels all open Orders |
| `open_orders` | ✓ | ✓ | Requests all open orders placed by this specific API client |
| `place_order` | ✓ | ✓ | Places or modifies an Order with subscription |
| `submit_order` | ✓ | ✓ | Submits an Order without returning a subscription |
| `order_update_stream` | ✓ | ✓ | Creates a subscription stream for real-time order updates |
| `exercise_options` | ✓ | ✓ | Exercises an options contract |

## Historical Market Data Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `head_timestamp` | ✓ | ✓ | Returns timestamp of earliest available historical data |
| `historical_data` | ✓ | ✓ | Requests interval of historical data |
| `historical_schedules` | ✓ | ✗ | Requests Schedule for an interval |
| `historical_schedules_ending_now` | ✓ | ✗ | Requests Schedule for interval ending at current time |
| `historical_schedule` | ✗ | ✓ | Requests historical schedule |
| `historical_ticks_bid_ask` | ✓ | ✓ | Requests historical time & sales data (Bid/Ask) |
| `historical_ticks_mid_point` | ✓ | ✓ | Requests historical time & sales data (Midpoint) |
| `historical_ticks_trade` | ✓ | ✓ | Requests historical time & sales data (Trades) |
| `histogram_data` | ✓ | ✓ | Requests data histogram of specified contract |

## Realtime Market Data Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `realtime_bars` | ✓ | ✓ | Requests realtime bars |
| `tick_by_tick_all_last` | ✓ | ✓ | Requests tick by tick AllLast ticks |
| `tick_by_tick_bid_ask` | ✓ | ✓ | Requests tick by tick BidAsk ticks |
| `tick_by_tick_last` | ✓ | ✓ | Requests tick by tick Last ticks |
| `tick_by_tick_midpoint` | ✓ | ✓ | Requests tick by tick MidPoint ticks |
| `switch_market_data_type` | ✓ | ✗ | Switches market data type |
| `market_depth` | ✓ | ✓ | Requests the contract's market depth (order book) |
| `market_depth_exchanges` | ✓ | ✓ | Requests venues for which market data is returned |
| `market_data` | ✓ | ✓ | Requests real time market data |

## News Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `news_providers` | ✓ | ✗ | Requests news providers which the user has subscribed to |
| `news_bulletins` | ✓ | ✗ | Subscribes to IB's News Bulletins |
| `historical_news` | ✓ | ✗ | Requests historical news headlines |
| `news_article` | ✓ | ✗ | Requests news article body given articleId |
| `contract_news` | ✓ | ✗ | Requests realtime contract specific news |
| `broad_tape_news` | ✓ | ✗ | Requests realtime BroadTape News |

## Scanner Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `scanner_parameters` | ✓ | ✗ | Requests an XML list of scanner parameters |
| `scanner_subscription` | ✓ | ✗ | Starts a subscription to market scan results |

## Wall Street Horizon (WSH) Methods

| Method | Sync | Async | Description |
|--------|------|-------|-------------|
| `wsh_metadata` | ✓ | ✓ | Requests metadata from the WSH calendar |
| `wsh_event_data_by_contract` | ✓ | ✓ | Requests event data for a specified contract |
| `wsh_event_data_by_filter` | ✓ | ✓ | Requests event data using a JSON filter |

## Summary

### Methods Available in Both Sync and Async
- Connection: `connect`, `next_order_id`, `next_valid_order_id`, `server_version`, `connection_time`
- Account Management: All methods
- Order Management: All methods
- Historical Market Data: `head_timestamp`, `historical_data`, `historical_ticks_*`, `histogram_data`
- Realtime Market Data: `realtime_bars`, `tick_by_tick_*`, `market_depth`, `market_depth_exchanges`, `market_data`
- WSH: All methods

### Methods Only in Sync
- Connection: `client_id`, `next_request_id`
- Contract: All methods
- Historical Market Data: `historical_schedules`, `historical_schedules_ending_now`
- Realtime Market Data: `switch_market_data_type`
- News: All methods
- Scanner: All methods

### Methods Only in Async
- Historical Market Data: `historical_schedule`

## Notes
- The async client uses `async`/`await` and returns `Future`s
- The sync client uses blocking calls
- Both clients share most of the same functionality with appropriate adaptations for their execution model
- Some methods are missing from async because they haven't been implemented yet