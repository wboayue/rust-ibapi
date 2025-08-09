# Client Integration Test Tracking

This document tracks the integration testing status of public Client methods using MockGateway.

## Test Status Legend
- ✅ Tested - Integration test exists using MockGateway
- ❌ Not tested - No integration test exists
- ⚠️ Partially tested - Test exists but may be incomplete or ignored
- N/A - Not applicable for testing (e.g., simple getters)

## Connection & Core Methods

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `connect()` | ✅ | ✅ | Basic connection test exists |
| `client_id()` | ✅ | N/A | Simple getter, tested in sync |
| `server_version()` | N/A | N/A | Simple getter |
| `connection_time()` | N/A | N/A | Simple getter |
| `next_request_id()` | N/A | N/A | Simple ID generation |
| `next_order_id()` | N/A | N/A | Simple ID generation |
| `next_valid_order_id()` | ✅ | ✅ | Tested |
| `server_time()` | ✅ | ✅ | Tested |

## Account Management

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `managed_accounts()` | ✅ | ✅ | Tested |
| `positions()` | ✅ | ✅ | Tested |
| `positions_multi()` | ✅ | ✅ | Tested |
| `pnl()` | ✅ | ✅ | Tested |
| `pnl_single()` | ✅ | ✅ | Tested |
| `account_summary()` | ✅ | ✅ | Tested |
| `account_updates()` | ✅ | ✅ | Tested - Fixed PortfolioValue message format |
| `account_updates_multi()` | ✅ | ✅ | Tested |
| `family_codes()` | ✅ | ✅ | Tested |

## Contract Management

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `contract_details()` | ❌ | ❌ | Needs test |
| `matching_symbols()` | ❌ | ❌ | Needs test |
| `market_rule()` | ❌ | ❌ | Needs test |
| `calculate_option_price()` | ❌ | ❌ | Needs test |
| `calculate_implied_volatility()` | ❌ | ❌ | Needs test |
| `option_chain()` | ❌ | ❌ | Needs test |

## Order Management

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `place_order()` | ❌ | ❌ | Needs test |
| `submit_order()` | ❌ | ❌ | Needs test |
| `cancel_order()` | ❌ | ❌ | Needs test |
| `global_cancel()` | ❌ | ❌ | Needs test |
| `order_update_stream()` | ❌ | ❌ | Needs test |
| `open_orders()` | ❌ | ❌ | Needs test |
| `all_open_orders()` | ❌ | ❌ | Needs test |
| `auto_open_orders()` | ❌ | ❌ | Needs test |
| `completed_orders()` | ❌ | ❌ | Needs test |
| `executions()` | ❌ | ❌ | Needs test |
| `exercise_options()` | ❌ | ❌ | Needs test (async only) |

## Market Data - Real-time

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `market_data()` | ❌ | ❌ | Needs test |
| `realtime_bars()` | ❌ | ❌ | Needs test (async only) |
| `tick_by_tick_all_last()` | ❌ | ❌ | Needs test (async only) |
| `tick_by_tick_last()` | ❌ | ❌ | Needs test (async only) |
| `tick_by_tick_bid_ask()` | ❌ | ❌ | Needs test (async only) |
| `tick_by_tick_midpoint()` | ❌ | ❌ | Needs test (async only) |
| `market_depth()` | ❌ | ❌ | Needs test (async only) |
| `market_depth_exchanges()` | ❌ | ❌ | Needs test |
| `switch_market_data_type()` | ❌ | ❌ | Needs test |

## Market Data - Historical

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `head_timestamp()` | ❌ | ❌ | Needs test |
| `historical_data()` | ❌ | ❌ | Needs test |
| `historical_schedules()` | ❌ | N/A | Needs test (sync only) |
| `historical_schedules_ending_now()` | ❌ | N/A | Needs test (sync only) |
| `historical_schedule()` | N/A | ❌ | Needs test (async only) |
| `historical_ticks_bid_ask()` | ❌ | ❌ | Needs test |
| `historical_ticks_mid_point()` | ❌ | ❌ | Needs test |
| `historical_ticks_trade()` | ❌ | ❌ | Needs test |
| `histogram_data()` | ❌ | ❌ | Needs test |

## News

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `news_providers()` | ❌ | ❌ | Needs test |
| `news_bulletins()` | ❌ | ❌ | Needs test |
| `historical_news()` | ❌ | ❌ | Needs test |
| `news_article()` | ❌ | ❌ | Needs test |
| `contract_news()` | ❌ | ❌ | Needs test |
| `broad_tape_news()` | ❌ | ❌ | Needs test |

## Scanner

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `scanner_parameters()` | ❌ | ❌ | Needs test |
| `scanner_subscription()` | ❌ | ❌ | Needs test |

## Wall Street Horizon (WSH)

| Method | Sync Status | Async Status | Notes |
|--------|-------------|--------------|-------|
| `wsh_metadata()` | ❌ | ❌ | Needs test |
| `wsh_event_data_by_contract()` | ❌ | ❌ | Needs test |
| `wsh_event_data_by_filter()` | ❌ | ❌ | Needs test |

## Summary Statistics

- **Total testable methods**: ~55
- **Currently tested**: 12 (both sync and async)
- **Partially tested**: 0
- **Not tested**: ~43
- **Coverage**: ~22%

## Priority for Testing

### High Priority (Core functionality)
1. Order management methods (place_order, submit_order, cancel_order)
2. Market data methods (market_data, realtime_bars)
3. Contract details methods

### Medium Priority
1. Historical data methods
2. Additional account methods (positions_multi, pnl_single)
3. Market depth methods

### Low Priority
1. News methods
2. Scanner methods
3. WSH methods
4. Option calculation methods

## Notes

1. The `test_subscription_cancel_only_sends_once` test exists in sync but not async
2. Some methods are sync-only or async-only (marked in the tables)
3. The MockGateway pattern in `client/common.rs` provides excellent infrastructure for testing
4. Tests should follow the existing pattern: setup function → connect → call method → verify