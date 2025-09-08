# Order Builder - Complete Reference

## Status: ✅ Implemented (PR #311)

The Order builder with fluent interface has been successfully implemented, providing a type-safe, discoverable API for creating and submitting orders.

## Key Features

- **Fluent Interface**: Chainable methods guide users through order creation
- **Type Safety**: Invalid combinations prevented at compile time  
- **Backward Compatible**: Coexists with traditional Order API
- **Full Validation**: All validation deferred to build() to avoid silent failures
- **Sync/Async Support**: Works with both sync and async clients

## Usage Examples

```rust
// Simple market order
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .submit()?;

// Limit order with conditions
let order_id = client.order(&contract)
    .sell(200)
    .limit(150.50)
    .good_till_cancel()
    .outside_rth()
    .submit()?;

// Bracket order (parent + take profit + stop loss)
let bracket_ids = client.order(&contract)
    .buy(100)
    .bracket()
    .entry_limit(50.0)
    .take_profit(55.0)
    .stop_loss(45.0)
    .submit_all()?;

// Advanced order types
client.order(&contract)
    .buy(100)
    .trailing_stop(5.0, 95.0)  // 5% trailing stop at $95
    .submit()?;

client.order(&contract)
    .sell(500)
    .sweep_to_fill(49.95)  // Prioritize speed over price
    .submit()?;
```

## Module Structure

```
src/orders/
├── builder/
│   ├── order_builder.rs     # Main OrderBuilder struct
│   │   └── tests.rs         # Comprehensive test suite
│   ├── types.rs             # NewType wrappers and enums
│   └── validation.rs        # Order validation logic
```

## Core Types

### OrderId & BracketOrderIds
```rust
pub struct OrderId(pub i32);
pub struct BracketOrderIds {
    pub parent: OrderId,
    pub take_profit: OrderId,
    pub stop_loss: OrderId,
}
```

### Order Types Supported
- **Basic**: Market, Limit, Stop, StopLimit
- **Trailing**: TrailingStop, TrailingStopLimit
- **Time-based**: MarketOnClose, LimitOnClose, MarketOnOpen, LimitOnOpen
- **Conditional**: MarketIfTouched, LimitIfTouched
- **Advanced**: Discretionary, SweepToFill, Midprice, Relative, PassiveRelative
- **Special**: Volatility, BoxTop, AuctionLimit, Block

### Time in Force Options
- Day, GoodTillCancel, ImmediateOrCancel, GoodTillDate, FillOrKill, Auction

## Product & Exchange Limitations

### Exchange-Specific Features
- **Hidden Orders**: NASDAQ-routed orders only
- **Block Orders**: ISE exchange for options (min 50 contracts)
- **Box Top Orders**: BOX exchange only
- **Midprice Orders**: TWS 975+, US stocks with Smart routing only

### Product-Specific Restrictions
- **Discretionary Orders**: Stocks (STK) only
- **Sweep to Fill**: CFD, STK, WAR only
- **Volatility Orders**: US options (FOP, OPT) only
- **Cash Quantity**: Forex (CASH) orders

## Current Validation Errors

The implementation provides basic error messages:

```rust
pub enum ValidationError {
    InvalidQuantity(f64),
    InvalidPrice(f64),
    MissingRequiredField(&'static str),
    InvalidCombination(String),
    InvalidStopPrice { stop: f64, current: f64 },
    InvalidLimitPrice { limit: f64, current: f64 },
    InvalidBracketOrder(String),
}
```

## 📝 Proposed Enhancement: Improved Error Messages

### Enhanced ValidationError with Context

```rust
pub enum ValidationError {
    InvalidQuantity {
        value: f64,
        reason: QuantityErrorReason,  // Zero, Negative, NotANumber, etc.
    },
    InvalidPrice {
        field: PriceField,  // LimitPrice, StopPrice, EntryPrice, etc.
        value: f64,
        reason: PriceErrorReason,
    },
    MissingRequiredField {
        field: &'static str,
        context: &'static str,  // "order creation - must specify buy() or sell()"
    },
    InvalidOrderType {
        order_type: String,
        missing_fields: Vec<&'static str>,
        reason: String,
    },
    // ... additional contextual variants
}
```

### Benefits of Enhancement

**Current Error:**
```
Error: Invalid quantity: -100
Error: Missing required field: limit_price
```

**Enhanced Error:**
```
Error: Invalid quantity -100: quantity cannot be negative
Error: Missing required field 'limit_price' for order creation - LMT orders require a limit price
```

### Implementation Impact
- Better developer experience with actionable error messages
- Easier debugging with complete context
- Type-safe error handling with enums
- No breaking changes to existing API

## Future Enhancements

### Near Term
1. **Combo Order Builder**: Dedicated builder for complex combo orders
2. **OCA Order Group Builder**: Simplified One-Cancels-All API
3. **Additional Order Types**: Adjustable orders, Pegged to Benchmark, FX Hedge

### Long Term
1. **Template Orders**: Save and reuse common configurations
2. **Strategy Builders**: Higher-level abstractions
3. **Order Modification Builder**: Fluent API for modifying existing orders

## Migration Notes

Both APIs coexist - gradual migration is supported:

```rust
// Traditional API
let mut order = Order::default();
order.action = Action::Buy;
order.order_type = "LMT".to_string();
order.total_quantity = 100.0;
order.limit_price = Some(50.0);

// Fluent API - same result, cleaner code
let order = client.order(&contract)
    .buy(100)
    .limit(50.0)
    .build()?;
```

## Performance

- **Zero-cost abstractions**: NewType wrappers compile to zero overhead
- **Minimal allocations**: Mostly stack-based builder pattern
- **Single validation pass**: All validation in build() method
- **Same runtime performance**: As manual Order construction