# Type Safety Review of src/client Public API

## ✅ Strengths

1. **Good use of newtype pattern**: The codebase uses newtypes like `AccountId`, `ContractId`, `ModelCode`, `AccountGroup` in function signatures instead of raw strings/integers.

2. **Proper enum usage**: Using enums for `SecurityType`, `MarketDataType`, `WhatToShow`, `BarSize`, `ErrorCategory` instead of string constants.

3. **Strong error handling**: Well-defined `ErrorCategory` enum and comprehensive error categorization functions.

4. **Type-safe ID generation**: Separate `IdGenerator` and `ClientIdManager` structs ensure proper ID handling.

## ⚠️ Areas for Improvement

### 1. String parameters in public API (src/client/sync.rs)
- `cancel_order()` takes `manual_order_cancel_time: &str` - should be a proper timestamp type
- `news_article()` takes raw `provider_code: &str` and `article_id: &str` - could use newtypes
- `account_summary()` takes `tags: &[&str]` - could be an enum or strongly-typed tags

### 2. Raw integers for IDs
- Functions like `market_rule(market_rule_id: i32)` use raw i32 instead of newtype
- `cancel_order(order_id: i32)` - could use `OrderId` newtype
- Request and order IDs throughout are raw i32 values

### 3. Boolean flags instead of enums
- `use_rth: bool` parameter appears frequently - could be `enum TradingHours { Regular, Extended }`
- `ignore_size: bool` in tick functions - could be more descriptive enum
- `all_messages: bool` in `news_bulletins()` - could be `enum MessageFilter`
- `api_only: bool` in `completed_orders()` - could be `enum OrderSource`

### 4. Untyped filter structures
- `ExecutionFilter` and scanner filters use raw strings/options instead of strongly-typed fields

### 5. Magic constants
- `INITIAL_REQUEST_ID: i32 = 9000` is a magic number without clear type safety

## 🔧 Recommendations

### Create newtypes for IDs
```rust
pub struct RequestId(i32);
pub struct OrderId(i32);
pub struct MarketRuleId(i32);
```

### Replace boolean parameters with enums
```rust
pub enum TradingHours { 
    Regular,    // RTH - Regular Trading Hours
    Extended    // Include pre-market and after-hours
}

pub enum TickSizeHandling { 
    IncludeSize, 
    IgnoreSize 
}

pub enum MessageFilter {
    All,
    ImportantOnly
}

pub enum OrderSource {
    ApiOnly,
    AllSources
}
```

### Type-safe timestamps
```rust
use time::OffsetDateTime;

pub struct ManualCancelTime(OffsetDateTime);

impl ManualCancelTime {
    pub fn parse(s: &str) -> Result<Self, Error> {
        // Parse and validate timestamp
    }
}
```

### Strongly-typed provider/article IDs
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArticleId(String);

impl ProviderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Enum for account summary tags
```rust
pub enum AccountSummaryTag {
    NetLiquidation,
    TotalCashValue,
    SettledCash,
    AccruedCash,
    BuyingPower,
    EquityWithLoanValue,
    PreviousEquityWithLoanValue,
    GrossPositionValue,
    RegTEquity,
    RegTMargin,
    SMA,
    InitMarginReq,
    MaintMarginReq,
    AvailableFunds,
    ExcessLiquidity,
    Cushion,
    FullInitMarginReq,
    FullMaintMarginReq,
    FullAvailableFunds,
    FullExcessLiquidity,
    // ... etc
}

impl AccountSummaryTag {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NetLiquidation => "NetLiquidation",
            Self::TotalCashValue => "TotalCashValue",
            // ... etc
        }
    }
}
```

## Implementation Priority

1. **High Priority** (Breaking changes, high impact):
   - Create `OrderId` and `RequestId` newtypes
   - Replace `use_rth: bool` with `TradingHours` enum
   - Create `AccountSummaryTag` enum

2. **Medium Priority** (Improves type safety):
   - Replace other boolean parameters with enums
   - Add newtypes for provider/article IDs
   - Type-safe timestamp handling

3. **Low Priority** (Nice to have):
   - Additional validation in newtype constructors
   - Helper methods for conversions
   - Builder patterns for complex types

## Migration Strategy

1. Add new types alongside existing API (deprecate old methods)
2. Provide conversion traits/methods for backward compatibility
3. Update examples and documentation
4. Remove deprecated methods in next major version

## Benefits

- **Compile-time safety**: Impossible to pass wrong ID types
- **Self-documenting**: Enum variants clarify intent better than booleans
- **Maintainability**: Changes to ID types or valid values are centralized
- **IDE support**: Better autocomplete and type hints
- **Reduced bugs**: Fewer runtime errors from invalid string values