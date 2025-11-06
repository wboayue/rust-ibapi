# Fluent Conditional Order API Refactoring

## Overview

Refactor the order builder and conditional order builder to support a fluent API for adding conditions to orders.

## Target API

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .condition(price(123445, "SMART").greater_than(10.0))
    .and_condition(margin().less_than(20))
    .or_condition(time().greater_than("20251010 09:30:00 US/Eastern"))
    .or_condition(volume(123445, "SMART").greater_than(10_000_000))
    .submit()
    .expect("order submission failed!");
```

## Current State Analysis

### Current Architecture
- **OrderBuilder** (`src/orders/builder/order_builder.rs:14-827`) - Main builder with `.buy()`, `.market()`, etc.
- **Condition structs** (`src/orders/conditions.rs`) - 6 types: Price, Time, Margin, Execution, Volume, PercentChange
- Each condition has its own builder (e.g., `PriceConditionBuilder`)
- Conditions stored as `Vec<OrderCondition>` on OrderBuilder (line 32)
- **Current limitation:** No fluent API to add conditions during order building

### Current Usage Pattern
```rust
let condition = PriceCondition::builder(265598, "SMART", 150.0)
    .trigger_above()
    .build();

let mut order = order_builder::limit_order(Action::Buy, 100.0, 151.0);
order.conditions = vec![OrderCondition::Price(condition)];
order.conditions_ignore_rth = false;

client.submit_order(order_id, &contract, &order)?;
```

## Implementation Plan

### Phase 1: Refactor Existing Condition Builders

**Location:** Modify `src/orders/conditions.rs`

Refactor the existing condition builders to:
1. Remove the threshold parameter from the `builder()` constructor
2. Add `greater_than(value)` and `less_than(value)` methods that set both the threshold and direction
3. Mark `trigger_above()` and `trigger_below()` as deprecated (to be removed in a future version)

**Changes for each builder:**

#### PriceConditionBuilder
```rust
impl PriceCondition {
    // Change: Remove `price` parameter
    pub fn builder(contract_id: i32, exchange: impl Into<String>) -> PriceConditionBuilder {
        PriceConditionBuilder::new(contract_id, exchange)
    }
}

impl PriceConditionBuilder {
    // Change: Remove `price` parameter, add as optional field
    pub fn new(contract_id: i32, exchange: impl Into<String>) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            price: 0.0,              // Will be set by greater_than/less_than
            trigger_method: 0,
            is_more: true,
            is_conjunction: true,
        }
    }

    /// Set trigger when price is greater than the specified value.
    pub fn greater_than(mut self, price: f64) -> Self {
        self.price = price;
        self.is_more = true;
        self
    }

    /// Set trigger when price is less than the specified value.
    pub fn less_than(mut self, price: f64) -> Self {
        self.price = price;
        self.is_more = false;
        self
    }

    // Deprecated: Keep for backwards compatibility but mark as deprecated
    #[deprecated(since = "0.x.0", note = "Use `greater_than(price)` instead")]
    pub fn trigger_above(mut self) -> Self {
        self.is_more = true;
        self
    }

    #[deprecated(since = "0.x.0", note = "Use `less_than(price)` instead")]
    pub fn trigger_below(mut self) -> Self {
        self.is_more = false;
        self
    }
}
```

#### TimeConditionBuilder
```rust
impl TimeCondition {
    // Change: Remove `time` parameter
    pub fn builder() -> TimeConditionBuilder {
        TimeConditionBuilder::new()
    }
}

impl TimeConditionBuilder {
    pub fn new() -> Self {
        Self {
            time: String::new(),  // Will be set by greater_than/less_than
            is_more: true,
            is_conjunction: true,
        }
    }

    pub fn greater_than(mut self, time: impl Into<String>) -> Self {
        self.time = time.into();
        self.is_more = true;
        self
    }

    pub fn less_than(mut self, time: impl Into<String>) -> Self {
        self.time = time.into();
        self.is_more = false;
        self
    }

    // Deprecated methods...
}
```

#### ExecutionCondition - Remove Builder Pattern
```rust
// Remove ExecutionConditionBuilder struct entirely
// Remove ExecutionCondition::builder() method
// Remove ExecutionConditionBuilder::new() method
// Execution conditions will only be created via helper function:
//   - execution(symbol, security_type, exchange)
```

#### MarginConditionBuilder
```rust
impl MarginCondition {
    // Change: Remove `percent` parameter
    pub fn builder() -> MarginConditionBuilder {
        MarginConditionBuilder::new()
    }
}

impl MarginConditionBuilder {
    pub fn new() -> Self {
        Self {
            percent: 0,  // Will be set by greater_than/less_than
            is_more: true,
            is_conjunction: true,
        }
    }

    pub fn greater_than(mut self, percent: i32) -> Self {
        self.percent = percent;
        self.is_more = true;
        self
    }

    pub fn less_than(mut self, percent: i32) -> Self {
        self.percent = percent;
        self.is_more = false;
        self
    }

    // Deprecated methods...
}
```

#### VolumeConditionBuilder
```rust
impl VolumeCondition {
    // Change: Remove `volume` parameter
    pub fn builder(contract_id: i32, exchange: impl Into<String>) -> VolumeConditionBuilder {
        VolumeConditionBuilder::new(contract_id, exchange)
    }
}

impl VolumeConditionBuilder {
    pub fn new(contract_id: i32, exchange: impl Into<String>) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            volume: 0,  // Will be set by greater_than/less_than
            is_more: true,
            is_conjunction: true,
        }
    }

    pub fn greater_than(mut self, volume: i32) -> Self {
        self.volume = volume;
        self.is_more = true;
        self
    }

    pub fn less_than(mut self, volume: i32) -> Self {
        self.volume = volume;
        self.is_more = false;
        self
    }

    // Deprecated methods...
}
```

#### PercentChangeConditionBuilder
```rust
impl PercentChangeCondition {
    // Change: Remove `percent` parameter
    pub fn builder(contract_id: i32, exchange: impl Into<String>) -> PercentChangeConditionBuilder {
        PercentChangeConditionBuilder::new(contract_id, exchange)
    }
}

impl PercentChangeConditionBuilder {
    pub fn new(contract_id: i32, exchange: impl Into<String>) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            percent: 0.0,  // Will be set by greater_than/less_than
            is_more: true,
            is_conjunction: true,
        }
    }

    pub fn greater_than(mut self, percent: f64) -> Self {
        self.percent = percent;
        self.is_more = true;
        self
    }

    pub fn less_than(mut self, percent: f64) -> Self {
        self.percent = percent;
        self.is_more = false;
        self
    }

    // Deprecated methods...
}
```

**New Usage Pattern:**
```rust
// Old way (deprecated):
let condition = PriceCondition::builder(265598, "SMART", 150.0)
    .trigger_above()
    .build();

// New way:
let condition = PriceCondition::builder(265598, "SMART")
    .greater_than(150.0)
    .build();
```

### Phase 2: Create Helper Functions for Condition Builders

**Location:** New module `src/orders/builder/condition_helpers.rs`

Create ergonomic helper functions that return partially-built condition builders:

```rust
// Price condition helper - returns builder
pub fn price(contract_id: impl Into<i32>, exchange: impl Into<String>) -> PriceConditionBuilder

// Time condition helper - returns builder
pub fn time() -> TimeConditionBuilder

// Margin condition helper - returns builder
pub fn margin() -> MarginConditionBuilder

// Volume condition helper - returns builder
pub fn volume(contract_id: i32, exchange: impl Into<String>) -> VolumeConditionBuilder

// Execution condition helper - returns OrderCondition directly (no threshold)
pub fn execution(symbol: impl Into<String>, security_type: impl Into<String>, exchange: impl Into<String>) -> OrderCondition

// Percent change condition helper - returns builder
pub fn percent_change(contract_id: i32, exchange: impl Into<String>) -> PercentChangeConditionBuilder
```

**Implementation:**
```rust
use crate::orders::conditions::*;
use crate::orders::OrderCondition;

pub fn price(contract_id: impl Into<i32>, exchange: impl Into<String>) -> PriceConditionBuilder {
    PriceCondition::builder(contract_id.into(), exchange)
}

pub fn time() -> TimeConditionBuilder {
    TimeCondition::builder()
}

pub fn margin() -> MarginConditionBuilder {
    MarginCondition::builder()
}

pub fn volume(contract_id: i32, exchange: impl Into<String>) -> VolumeConditionBuilder {
    VolumeCondition::builder(contract_id, exchange)
}

pub fn execution(symbol: impl Into<String>, security_type: impl Into<String>, exchange: impl Into<String>) -> OrderCondition {
    OrderCondition::Execution(ExecutionCondition {
        symbol: symbol.into(),
        security_type: security_type.into(),
        exchange: exchange.into(),
        is_conjunction: true,
    })
}

pub fn percent_change(contract_id: i32, exchange: impl Into<String>) -> PercentChangeConditionBuilder {
    PercentChangeCondition::builder(contract_id, exchange)
}
```

### Phase 3: Add Condition Methods to OrderBuilder

**Location:** Modify `src/orders/builder/order_builder.rs`

Add three new methods to `OrderBuilder<'a, C>`:

```rust
impl<'a, C> OrderBuilder<'a, C> {
    /// Add a condition to the order. First condition is always treated as AND.
    /// The condition's is_conjunction flag determines how it combines with the NEXT condition.
    pub fn condition(mut self, condition: OrderCondition) -> Self {
        // First condition - always set to conjunction (AND)
        let mut cond = condition;
        set_conjunction(&mut cond, true);
        self.conditions.push(cond);
        self
    }

    /// Add a condition that must be met along with previous conditions (AND logic)
    pub fn and_condition(mut self, condition: OrderCondition) -> Self {
        let mut cond = condition;
        // Set previous condition to use AND logic with this one
        if let Some(prev) = self.conditions.last_mut() {
            set_conjunction(prev, true);
        }
        self.conditions.push(cond);
        self
    }

    /// Add a condition where either this OR previous conditions trigger the order (OR logic)
    pub fn or_condition(mut self, condition: OrderCondition) -> Self {
        let mut cond = condition;
        // Set previous condition to use OR logic with this one
        if let Some(prev) = self.conditions.last_mut() {
            set_conjunction(prev, false);
        }
        self.conditions.push(cond);
        self
    }
}

// Helper function to set conjunction flag on OrderCondition enum
fn set_conjunction(condition: &mut OrderCondition, is_conjunction: bool) {
    match condition {
        OrderCondition::Price(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Time(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Margin(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Execution(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Volume(c) => c.is_conjunction = is_conjunction,
        OrderCondition::PercentChange(c) => c.is_conjunction = is_conjunction,
    }
}
```

#### Key Design Decision

- `is_conjunction` on each condition controls how it combines with the **next** condition
- The `and_condition()` and `or_condition()` methods modify the **previous** condition's flag
- This matches TWS API semantics where each condition has a flag for the next relationship

### Phase 4: Re-export Helper Functions

**Location:** Modify `src/orders/builder/mod.rs`

Make helper functions easily accessible:

```rust
// In src/orders/builder/mod.rs
pub mod condition_helpers;
pub use condition_helpers::*;

// Or create a prelude module for convenient imports
pub mod prelude {
    pub use super::condition_helpers::*;
}
```

This allows usage like:
```rust
use ibapi::orders::builder::*;
// or
use ibapi::orders::builder::prelude::*;
```

### Phase 5: Update Documentation and Examples

#### Code Documentation (src/ comments)

1. **Update condition builder doc comments in `src/orders/conditions.rs`:**
   - Update all `PriceCondition::builder()` examples to remove price parameter
   - Update all builder examples to show `greater_than(value)` / `less_than(value)` pattern
   - Update `TimeCondition` examples to show new API
   - Update `MarginCondition` examples to show new API
   - Update all other condition examples

2. **Add doc examples to OrderBuilder methods in `src/orders/builder/order_builder.rs`:**
   - Add example to `.condition()` method
   - Add example to `.and_condition()` method
   - Add example to `.or_condition()` method

3. **Add doc comments to helper functions in `src/orders/builder/condition_helpers.rs`:**
   - Document each helper function with usage examples

#### Example Code Updates

4. **Update existing example: `examples/conditional_orders.rs`:**
   - Migrate to use new API
   - Show the new fluent pattern
   - Add comments explaining AND/OR logic

5. **Update integration tests: `tests/conditional_orders_integration.rs`:**
   - Update test cases to use new API
   - Ensure all condition types are tested with new pattern
   - Add tests for `.condition()`, `.and_condition()`, `.or_condition()` methods

6. **Create new comprehensive example: `examples/conditional_orders_advanced.rs`** (optional):
   - Single condition
   - Multiple AND conditions
   - Multiple OR conditions
   - Mixed AND/OR conditions
   - All condition types (Price, Time, Margin, Volume, PercentChange, Execution)

#### Documentation Updates (docs/ directory)

7. **Update `docs/order-types.md`:**
   - Expand the "Conditional Orders" section (currently only has MIT/LIT)
   - Add comprehensive section for "Conditional Orders with Conditions"
   - Document the new fluent condition API pattern
   - Include examples for all condition types:
     - Price conditions (trigger on price movements)
     - Time conditions (trigger at specific times)
     - Margin conditions (trigger on margin levels)
     - Volume conditions (trigger on volume thresholds)
     - Percent change conditions (trigger on price % changes)
     - Execution conditions (trigger on order fills)
   - Show single condition, multiple AND, multiple OR, and mixed AND/OR examples
   - Add section on "How Conditions Work" explaining:
     - `is_conjunction` flag behavior
     - AND vs OR logic chaining
     - How conditions combine with orders
     - `conditions_ignore_rth` and `conditions_cancel_order` flags
   - Add "Best Practices" subsection for conditional orders

8. **Update `docs/api-patterns.md`:**
   - Add new section "Conditional Order Builder Pattern"
   - Document the helper function pattern (price(), time(), margin(), etc.)
   - Show the fluent condition chaining pattern
   - Include comparison of old vs new API
   - Explain the type-state pattern for conditions
   - Add examples showing both sync and async usage

9. **Update `CLAUDE.md`:**
   - Add brief note about the fluent condition API pattern
   - Reference the order-types.md documentation for details

## API Usage Examples

### Single Condition
```rust
client.order(&contract)
    .buy(100)
    .market()
    .condition(price(265598, "SMART").greater_than(150.0))
    .submit()?
```

### Multiple AND Conditions
```rust
client.order(&contract)
    .sell(50)
    .limit(155.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(margin().greater_than(30))
    .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
    .submit()?
```

### Multiple OR Conditions
```rust
client.order(&contract)
    .buy(100)
    .market()
    .condition(price(265598, "SMART").less_than(100.0))
    .or_condition(volume(265598, "SMART").greater_than(50_000_000))
    .submit()?
```

### Mixed AND/OR Logic
```rust
client.order(&contract)
    .buy(100)
    .market()
    .condition(price(123445, "SMART").greater_than(10.0))
    .and_condition(margin().less_than(20))
    .or_condition(time().greater_than("20251010 09:30:00 US/Eastern"))
    .or_condition(volume(123445, "SMART").greater_than(10_000_000))
    .submit()?
```

**Logic:** `(price > 10 AND margin < 20) OR time > X OR volume > Y`

## Implementation Considerations

### 1. Contract ID Resolution
- Helper functions take contract IDs as strings or integers for convenience
- Users must obtain contract IDs via `contract_details()` first
- Consider adding a future helper to resolve symbols to contract IDs automatically

### 2. Type Safety
- OrderCondition is returned from helpers, enforcing type correctness
- Condition helpers return `OrderCondition` enum, not raw structs
- Compiler ensures only valid conditions are added

### 3. Backwards Compatibility
- Existing condition builder API remains unchanged
- Users can still construct conditions manually if needed
- Direct access to `conditions` field remains available

### 4. Feature Flags
- No feature flag concerns - conditions work for both sync and async
- Helper functions and OrderBuilder methods are feature-agnostic

### 5. Error Handling
- Invalid contract IDs will fail at submission time (TWS validation)
- Consider adding validation helpers for time format strings
- Quantity/price validations already exist in `build()`

### 6. Testing Strategy
- Unit tests for helper functions
- Unit tests for condition ordering logic (AND/OR chaining)
- Integration test with mock or paper trading account
- Table-driven tests for various condition combinations

## Files to Modify/Create

### New Files
1. `src/orders/builder/condition_helpers.rs` - Helper functions and intermediate types

### Modified Files
1. `src/orders/builder/order_builder.rs` - Add `.condition()`, `.and_condition()`, `.or_condition()` methods
2. `src/orders/builder/mod.rs` - Re-export helpers
3. `examples/conditional_orders_fluent.rs` - New example (optional)
4. `docs/api-patterns.md` - Document the new pattern (optional)

## Estimated Complexity

- **Level:** Low-Medium
- **Lines of Code:** ~200-300 total
- **Time Estimate:** 2-3 hours for implementation and basic testing
- **Breaking Changes:** None - purely additive

## Status

- [ ] Phase 1: Refactor existing condition builders (add greater_than/less_than, remove TimeConditionBuilder)
- [ ] Phase 2: Create helper functions in condition_helpers.rs
- [ ] Phase 3: Add OrderBuilder methods (.condition(), .and_condition(), .or_condition())
- [ ] Phase 4: Re-export helpers from mod.rs
- [ ] Phase 5: Documentation and examples
  - [ ] Code documentation (src/ comments)
  - [ ] Example code updates
  - [ ] Documentation updates (docs/ directory)
- [ ] Testing and validation

## Notes

- This is a purely additive change with no breaking changes
- The fluent API is more ergonomic but the existing builder API remains available
- Contract ID handling may need improvement in the future (symbol → contract ID resolution)
