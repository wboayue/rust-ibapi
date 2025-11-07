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

### Type-Safe Order Field Enums (#339)

Replace string-based and integer-based order fields with type-safe enums to improve compile-time safety and prevent runtime errors from invalid values:

- **TimeInForce** - Now an enum instead of String (Day, GTC, IOC, GTD, OnOpen, FillOrKill, DayTilCanceled, Auction)
- **OcaType** - Now an enum instead of i32 (None, CancelWithBlock, ReduceWithBlock, ReduceWithoutBlock)
- **OrderOrigin** - Now an enum instead of i32 (Customer, Firm)
- **ShortSaleSlot** - Now an enum instead of i32 (None, Broker, ThirdParty)
- **VolatilityType** - Now an enum instead of Option<i32> (Daily, Annual)
- **ReferencePriceType** - Now an enum instead of Option<i32> (Average, BidOrAsk)
- **AuctionStrategy** - Now an enum instead of Option<i32> (Match, Improvement, Transparent)

Benefits:
- Compile-time safety: Invalid values caught at compile time
- Better IDE support: Auto-completion and type hints
- Self-documenting code: Enum variants clearly show available options
- Reduced errors: No more typos or invalid values

Example:
```rust
use ibapi::orders::{TimeInForce, OcaType};

// Before: stringly-typed
let mut order = market_order(Action::Buy, 100.0);
order.tif = "GTC".to_string();  // Easy to typo
order.oca_type = 2;             // What does 2 mean?

// After: type-safe
let mut order = market_order(Action::Buy, 100.0);
order.tif = TimeInForce::GoodTilCanceled;  // Clear and type-safe
order.oca_type = OcaType::ReduceWithBlock;  // Self-documenting
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

### Type-Safe Order Fields (#339)

Several `Order` struct fields have been changed from strings/integers to enums for type safety. This affects code that directly constructs or modifies orders.

**Migration Guide:**

1. **TimeInForce** (was `String`, now `TimeInForce` enum):
```rust
// Before
order.tif = "GTC".to_string();

// After
order.tif = TimeInForce::GoodTilCanceled;
```

2. **OcaType** (was `i32`, now `OcaType` enum):
```rust
// Before
order.oca_type = 2;

// After
order.oca_type = OcaType::ReduceWithBlock;
```

3. **Order builder helper functions** now accept enum types:
```rust
// Before
let order = auction_limit(Action::Buy, 100.0, 50.0, 2);
let order = volatility(Action::Buy, 100.0, 0.04, 1);
let orders = one_cancels_all("OCA1", vec![order1, order2], 2);

// After
let order = auction_limit(Action::Buy, 100.0, 50.0, AuctionStrategy::Improvement);
let order = volatility(Action::Buy, 100.0, 0.04, VolatilityType::Daily);
let orders = one_cancels_all("OCA1", vec![order1, order2], OcaType::ReduceWithBlock);
```

4. **Default values** have changed:
   - `Order::tif` defaults to `TimeInForce::Day` (was empty string)
   - `Order::oca_type` defaults to `OcaType::None` (was 0)
   - `Order::origin` defaults to `OrderOrigin::Customer` (was 0)
   - `Order::short_sale_slot` defaults to `ShortSaleSlot::None` (was 0)
   - `Order::auction_strategy` defaults to `None` (was `Some(0)`)

All enums implement `From<i32>`, `From<&str>`, `ToString`, and `ToField` for seamless wire protocol compatibility.

## 🔗 Related Pull Requests

- #339 - Replace stringly-typed order fields with type-safe enums
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
