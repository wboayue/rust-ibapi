# Fluent Interface Implementation Review

## Summary
The proposed fluent interface in `order-placement-fluent-interface-implementation.md` covers many common order types but is missing several specialized order types that are already implemented in `src/orders/common/order_builder.rs`.

## Currently Supported Order Types in Fluent Interface

### Basic Order Types
✅ Market
✅ Limit  
✅ Stop
✅ Stop Limit
✅ Trailing Stop
✅ Trailing Stop Limit
✅ Market on Close
✅ Limit on Close
✅ Market on Open
✅ Limit on Open

### Pegged Orders
✅ Pegged to Market
✅ Pegged to Stock  
✅ Pegged to Midpoint

### Special Orders
✅ Volatility
✅ Bracket Orders (with special builder)
✅ Auction Limit
✅ Auction Relative

## Missing Order Types from order_builder.rs

The following order types have builder functions in `order_builder.rs` but are not represented in the fluent interface:

### 1. Market Orders Variants
- **Market if Touched (MIT)** - `market_if_touched()`
- **Market to Limit (MTL)** - `market_to_limit()`
- **Market with Protection** - `market_with_protection()`
- **Midpoint Match** - `midpoint_match()`
- **Midprice** - `midprice()`

### 2. Stop Order Variants  
- **Stop with Protection** - `stop_with_protection()`
- **Limit if Touched (LIT)** - `limit_if_touched()`

### 3. Specialized Order Types
- **At Auction** - `at_auction()` - Uses MTL order type with AUC time in force
- **Discretionary** - `discretionary()` - Limit order with hidden discretionary amount
- **Sweep to Fill** - `sweep_to_fill()` - For speed of execution over price
- **Block** - `block()` - For large volume option orders
- **Box Top** - `box_top()` - BOX exchange specific

### 4. Relative/Passive Orders
- **Relative Pegged to Primary** - `relative_pegged_to_primary()`
- **Passive Relative** - `passive_relative()`

### 5. Combo Orders
- **Combo Limit** - `combo_limit_order()`
- **Combo Market** - `combo_market_order()`
- **Relative Limit Combo** - `relative_limit_combo()`
- **Relative Market Combo** - `relative_market_combo()`
- **Combo with Leg Prices** - `limit_order_for_combo_with_leg_prices()`

### 6. Advanced Pegged Orders
- **Pegged to Benchmark** - `pegged_to_benchmark()`
- **Auction Pegged to Stock** - `auction_pegged_to_stock()`
- **Peg Best** - `peg_best_order()`
- **Peg Best Up to Mid** - `peg_best_up_to_mid_order()`
- **Peg Mid** - `peg_mid_order()` (different from pegged_to_midpoint)

### 7. Special Features
- **One Cancels All (OCA)** - `one_cancels_all()`
- **FX Hedge** - `market_f_hedge()`
- **Adjustable Orders** - `attach_adjustable_to_stop()`, `attach_adjustable_to_stop_limit()`, `attach_adjustable_to_trail()`
- **IBKRATS Limit** - `limit_ibkrats()`
- **Limit with Cash Quantity** - `limit_order_with_cash_qty()`
- **Manual Order Time** - `limit_order_with_manual_order_time()`

## Issues Found

### 1. Hidden Attribute
- The `hidden` field is included (line 351, 386, 525-527, 706) 
- Documentation should clarify it only works for NASDAQ-routed orders
- Consider adding a doc comment to the `hidden()` method

### 2. OrderType Enum Incomplete
The `OrderType` enum (lines 249-268) is missing several order type strings used in order_builder.rs:
- "MIT" - Market if Touched
- "MTL" - Market to Limit  
- "MKT PRT" - Market with Protection
- "STP PRT" - Stop with Protection
- "LIT" - Limit if Touched
- "MIDPRICE" - Midprice order
- "PASSV REL" - Passive Relative
- "REL + LMT" - Relative Limit Combo
- "REL + MKT" - Relative Market Combo
- "PEG BENCH" - Pegged to Benchmark
- "PEG BEST" - Peg Best

### 3. Missing Validation
Several order types have specific validation requirements not covered:
- Volatility orders need volatility_type validation
- Combo orders need special handling for leg prices
- Pegged orders have complex parameter requirements

## Recommendations

### 1. Extend OrderType Enum
Add all missing order types to ensure complete coverage.

### 2. Add Missing Builder Methods
Consider adding fluent methods for commonly used order types:

```rust
impl<'a, C> OrderBuilder<'a, C> {
    /// Market if Touched order
    pub fn market_if_touched(mut self, trigger_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::MarketIfTouched);
        self.aux_price = Some(trigger_price.into());
        self
    }
    
    /// Discretionary order with hidden amount off limit price
    pub fn discretionary(mut self, limit_price: impl Into<f64>, discretionary_amt: f64) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.discretionary_amt = Some(discretionary_amt);
        self
    }
    
    /// Sweep to fill for speed over price
    pub fn sweep_to_fill(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.sweep_to_fill = true;
        self
    }
    
    // ... more methods
}
```

### 3. Create Specialized Builders
For complex order types like combos and OCA orders, consider specialized builders:

```rust
pub struct ComboOrderBuilder<'a, C> {
    // ... combo-specific fields
}

pub struct OCAOrderBuilder<'a, C> {
    // ... OCA group management
}
```

### 4. Document Limitations
Add clear documentation about:
- Hidden orders only working on NASDAQ
- Which order types are available for which products
- Exchange-specific order types (BOX, ISE, etc.)

### 5. Add Missing Fields to OrderBuilder
The OrderBuilder struct is missing several fields needed for advanced orders:
- `sweep_to_fill: bool`
- `block_order: bool`  
- `not_held: bool`
- `min_trade_qty: Option<i32>`
- `min_compete_size: Option<i32>`
- `compete_against_best_offset: Option<f64>`
- `mid_offset_at_whole: Option<f64>`
- `mid_offset_at_half: Option<f64>`
- Additional fields for adjustable orders

### 6. Consider Feature Flags
Some specialized order types might benefit from feature flags to keep the API surface manageable:

```rust
#[cfg(feature = "advanced-orders")]
impl<'a, C> OrderBuilder<'a, C> {
    // Advanced order methods
}
```

## Priority Implementation Order

1. **High Priority** - Common order types used frequently:
   - Market if Touched
   - Limit if Touched  
   - Discretionary
   - OCA orders
   - Sweep to fill

2. **Medium Priority** - Specialized but useful:
   - Combo orders
   - Relative orders
   - Pegged variants
   - Block orders

3. **Low Priority** - Exchange-specific or rarely used:
   - BOX-specific orders
   - Adjustable orders
   - IBKRATS orders

## Conclusion

The proposed fluent interface is a good foundation but needs expansion to match the full capability set already available in the codebase. Priority should be given to commonly used order types while maintaining the clean, intuitive API design already established.