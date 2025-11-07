# Release Notes - v2.1.0

## Overview

This release adds comprehensive support for conditional orders and auction/protection order types, significantly expanding the trading capabilities of rust-ibapi. The release also includes extensive documentation improvements and API enhancements.

## ✨ New Features

### Conditional Orders Support (#325, #330, #337)

Complete implementation of TWS conditional orders with all 6 condition types:

- **Price Condition** - Trigger orders when a security reaches a specific price
- **Time Condition** - Trigger orders at a specific date/time
- **Margin Condition** - Trigger based on account margin cushion percentage
- **Execution Condition** - Trigger when another order executes
- **Volume Condition** - Trigger when trading volume reaches a threshold
- **Percent Change Condition** - Trigger based on price change percentage

**Fluent API** - Ergonomic builder pattern with helper functions:
```rust
use ibapi::orders::builder::{price, time, margin};

// Simple condition
let order = order_builder()
    .action(Action::Buy)
    .total_quantity(100.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .build();

// Complex multi-condition with AND/OR logic
let order = order_builder()
    .action(Action::Buy)
    .total_quantity(100.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
    .or_condition(margin().less_than(30))
    .build();
```

- 364 new unit tests covering all condition types and edge cases
- Full integration tests with TWS API message encoding/decoding
- Comprehensive example: `examples/conditional_orders.rs`

### Auction and Protection Order Types (#336)

New order types for market open/close and futures protection:

- **Market on Close (MOC)** and **Limit on Close (LOC)** orders
- **Market on Open** and **Limit on Open** using `OpeningAuction` TIF
- **Market with Protection** and **Stop with Protection** for futures
- New `OpeningAuction` variant added to `TimeInForce` enum

Example:
```rust
// Market on Close order
let moc = order_builder()
    .action(Action::Buy)
    .total_quantity(100.0)
    .order_type(OrderType::MarketOnClose)
    .build();

// Limit on Open order
let loo = order_builder()
    .action(Action::Buy)
    .total_quantity(100.0)
    .limit_price(150.0)
    .time_in_force(TimeInForce::OpeningAuction)
    .build();
```

## 📚 Documentation Improvements

### New Documentation Files (#328, #336)
- Added dual API documentation references (GitHub + IBKR Campus)
- Extensive updates to `docs/order-types.md` with 339 new lines covering:
  - All conditional order patterns
  - Auction and protection order types
  - Custom order construction examples
- New `docs/api-patterns.md` with 209 lines documenting:
  - Conditional order builder pattern
  - Helper function usage
  - Common API patterns

### API Documentation (#327, #332)
- Added comprehensive documentation for 12 sync API functions that were previously undocumented
- Updated parameter documentation from `use_rth` to `trading_hours` for clarity
- Added order update stream monitoring examples for both sync and async APIs
- Updated installation examples to reference version 2.1

### README Enhancements
- Added 161 lines of new content covering conditional orders and new order types
- Updated examples with current API patterns
- Improved quick start documentation

## 🐛 Bug Fixes

### Visibility and Routing Fixes (#333, #335)
- Changed order and WSH functions to `pub(crate)` visibility for proper encapsulation
- Fixed order rejection routing to ensure proper error handling
- Cleaned up feature guards for better sync/async feature separation

## 🔧 Internal Improvements

- **Code additions**: 4,671 insertions, 172 deletions across 34 files
- **New modules**:
  - `src/orders/conditions.rs` (998 lines) - Core condition types and builders
  - `src/orders/builder/condition_helpers.rs` (195 lines) - Fluent API helpers
  - `tests/conditional_orders_integration.rs` (259 lines) - Integration tests
- Enhanced message encoding/decoding for conditional orders (588 new lines)
- Improved transport layer error handling (49 async, 14 sync)

## 📦 Breaking Changes

None - this release is backward compatible with v2.0.0

## 🔗 Related Pull Requests

- #337 - Add fluent conditional order API
- #336 - Add auction and protection order types
- #335 - Fix order rejection routing and cleanup feature guards
- #333 - Fix order and WSH function visibility
- #332 - Update parameter documentation
- #330 - Implement conditional orders support (part 2)
- #328 - Add dual API documentation references
- #327 - Add missing documentation for sync API functions
- #325 - Implement conditional orders support (part 1)

## 🙏 Acknowledgments

Thanks to all contributors who helped with this release!
