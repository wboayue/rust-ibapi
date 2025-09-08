# Example Parity Tracking

This document tracks the implementation status of examples across sync and async versions of the rust-ibapi library.

## Summary
- **Total Sync Examples**: 71
- **Total Async Examples**: 43
- **Examples with Both Sync & Async**: 20
- **Sync-Only Examples**: 51
- **Async-Only Examples**: 23

## Implementation Status

### Connection & Setup
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `connect` | ✓ | ✓ | Basic connection |
| `server_time` | ✓ | ✗ | Server time verification |
| `managed_accounts` | ✓ | ✓ | List accessible accounts |
| `connection_monitoring` | ✗ | ✓ | Monitor connection status |

### Account & Portfolio
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `account_summary` | ✓ | ✓ | Account values and balances |
| `account_updates` | ✓ | ✗ | Real-time account updates |
| `account_updates_multi` | ✓ | ✗ | Multi-account updates |
| `positions` | ✓ | ✓ | Current positions |
| `positions_multi` | ✓ | ✗ | Multi-account positions |
| `pnl` | ✓ | ✓ | Real-time P&L |
| `pnl_single` | ✓ | ✗ | Single position P&L |
| `family_codes` | ✓ | ✗ | Family code information |

### Real-time Market Data
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `market_data` | ✓ | ✓ | Live quotes and trades |
| `market_depth` | ✓ | ✓ | Level II order book |
| `market_depth_exchanges` | ✓ | ✗ | Available depth exchanges |
| `realtime_bars` | ✓ | ✓ | 5-second bars |
| `tick_by_tick_last` | ✓ | ✓ | Trade ticks |
| `tick_by_tick_bid_ask` | ✓ | ✓ | Quote ticks |
| `tick_by_tick_midpoint` | ✓ | ✓ | Midpoint ticks |
| `tick_by_tick_all_last` | ✓ | ✗ | All trades and quotes |
| `stream_bars` | ✓ | ✗ | Streaming bar data |
| `stream_retry` | ✓ | ✗ | Stream with retry logic |
| `switch_market_data_type` | ✓ | ✗ | Live/frozen/delayed switching |

### Historical Market Data
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `historical_data` | ✓ | ✓ | Historical bars |
| `historical_data_adjusted` | ✓ | ✗ | Split-adjusted data |
| `historical_data_recent` | ✓ | ✗ | Last 7 days |
| `historical_data_options` | ✓ | ✗ | Options historical data |
| `historical_ticks_trade` | ✓ | ✓ | Historical trades |
| `historical_ticks_bid_ask` | ✓ | ✗ | Historical quotes |
| `historical_ticks_mid_point` | ✓ | ✓ | Historical midpoints |
| `historical_schedules` | ✓ | ✗ | Trading schedules |
| `historical_schedule` | ✗ | ✓ | Trading schedule (async) |
| `historical_schedules_ending_now` | ✓ | ✗ | Schedules ending now |
| `head_timestamp` | ✓ | ✓ | Earliest data available |
| `histogram_data` | ✓ | ✓ | Price distribution |

### Orders & Execution
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `place_order` | ✓ | ✗ | Submit orders |
| `bracket_order` | ✓ | ✗ | Entry + stop loss + profit |
| `orders` | ✓ | ✗ | List open orders |
| `cancel_orders` | ✓ | ✗ | Cancel orders |
| `completed_orders` | ✓ | ✗ | Completed order history |
| `executions` | ✓ | ✗ | Execution reports |
| `submit_order` | ✓ | ✗ | Submit without subscription |
| `next_order_id` | ✓ | ✗ | Get next valid ID |
| `options_purchase` | ✓ | ✗ | Buy options |
| `options_exercise` | ✓ | ✗ | Exercise options |

### Contracts & Instruments
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `contract_details` | ✓ | ✗ | Contract specifications |
| `matching_symbols` | ✓ | ✗ | Symbol search |
| `option_chain` | ✓ | ✗ | Options chain |
| `market_rule` | ✓ | ✗ | Price increments |
| `calculate_option_price` | ✓ | ✗ | Theoretical pricing |
| `calculate_implied_volatility` | ✓ | ✗ | IV calculation |

### News
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `news_providers` | ✓ | ✗ | Available providers |
| `news_bulletins` | ✓ | ✗ | IB bulletins |
| `news_article` | ✓ | ✗ | Article content |
| `contract_news` | ✓ | ✗ | Contract-specific news |
| `historical_news` | ✓ | ✗ | News history |
| `broad_tape_news` | ✓ | ✗ | Broad tape feed |

### Scanner
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `scanner_parameters` | ✓ | ✗ | Available parameters |
| `scanner_subscription_active_stocks` | ✓ | ✗ | Most active stocks |
| `scanner_subscription_complex_orders` | ✓ | ✗ | Complex order flow |

### WSH Events
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `wsh_metadata` | ✓ | ✓ | Wall Street Horizon metadata |
| `wsh_event_data_by_contract` | ✓ | ✓ | Events by contract |
| `wsh_event_data_by_filter` | ✓ | ✓ | Filtered events |

### Trading Strategies
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `breakout` | ✓ | ✗ | Breakout strategy example |

### Testing & Debugging
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `test_multiple_calls` | ✗ | ✓ | Test multiple sequential calls |

### README Examples
| Example | Sync | Async | Notes |
|---------|------|-------|-------|
| `readme_connection` | ✓ | ✗ | Connection example from docs |
| `readme_historical_data` | ✓ | ✗ | Historical data example |
| `readme_place_order` | ✓ | ✗ | Order placement example |
| `readme_realtime_data_1` | ✓ | ✗ | Realtime data example 1 |
| `readme_realtime_data_2` | ✓ | ✗ | Realtime data example 2 |
| `readme_multi_threading_1` | ✓ | ✗ | Multi-threading example 1 |
| `readme_multi_threading_2` | ✓ | ✗ | Multi-threading example 2 |

## Priority for Async Implementation

Based on functionality importance, these sync-only examples should be prioritized for async implementation:

### High Priority
1. **Orders & Execution** - All order-related examples are missing async versions
2. **Contracts & Instruments** - Core functionality for contract discovery
3. **Account Updates** - Important for real-time portfolio management

### Medium Priority
4. **News** - All news examples lack async implementation
5. **Scanner** - Market scanning functionality
6. **Market Data Types** - `switch_market_data_type`, `market_depth_exchanges`

### Low Priority
7. **README Examples** - Documentation examples
8. **Specialized Historical** - `historical_data_adjusted`, `historical_data_recent`

## Notes

- The async implementation appears to be newer and less complete
- Order management is a significant gap in async support
- Some examples may have different names between sync/async (e.g., `historical_schedules` vs `historical_schedule`)
- Utility examples in the root directory are not included in this tracking