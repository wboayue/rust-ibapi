# Type Safety Improvements: Magic Number Elimination

This document tracks opportunities to replace magic numbers and string constants with type-safe enums throughout the API.

## Completed ✅

### TriggerMethod Enum
- **Location**: `src/orders/conditions.rs`
- **Status**: ✅ Completed
- **Used by**:
  - `PriceCondition.trigger_method`
  - `Order.trigger_method`
- **Enum**:
  ```rust
  pub enum TriggerMethod {
      Default = 0,           // Last for most securities, double bid/ask for OTC/options
      DoubleBidAsk = 1,      // Two consecutive bid or ask prices
      Last = 2,              // Last traded price
      DoubleLast = 3,        // Two consecutive last prices
      BidAsk = 4,            // Current bid or ask price
      LastOrBidAsk = 7,      // Last price or bid/ask if unavailable
      Midpoint = 8,          // Mid-point between bid and ask
  }
  ```

## High Priority 🔴

### 1. TimeInForce Enum
- **Location**: `src/orders/mod.rs:121`
- **Current**: `pub tif: String` with FIXME comment
- **Impact**: High - Used in every order
- **Current Values**: "DAY", "GTC", "IOC", "GTD", "OPG", "FOK", "DTC"
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum TimeInForce {
      Day,                  // DAY - Valid for the day only
      GoodTilCanceled,      // GTC - Good until canceled
      ImmediateOrCancel,    // IOC - Fill what's available, cancel rest
      GoodTilDate,          // GTD - Good until specific date (use with good_till_date)
      OnOpen,               // OPG - Market/Limit on open (MOO/LOO)
      FillOrKill,           // FOK - All or nothing, immediate
      DayTilCanceled,       // DTC - Day until canceled
  }
  ```
- **Implementation Notes**:
  - Needs `ToField` implementation that converts to string
  - Needs `From<String>` and `From<&str>` for deserialization
  - Update `Order::default()` to use `TimeInForce::Day`
  - Update encoders/decoders in `src/orders/common/`
  - Update all order builders to accept `TimeInForce`

### 2. AuctionStrategy Enum
- **Location**: `src/orders/mod.rs:245`
- **Current**: `pub auction_strategy: Option<i32>` with FIXME comment
- **Impact**: Medium - Used for BOX orders only
- **Current Values**: 1 = Match, 2 = Improvement, 3 = Transparent
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum AuctionStrategy {
      Match = 1,
      Improvement = 2,
      Transparent = 3,
  }
  ```
- **Implementation Notes**:
  - Simple integer enum like TriggerMethod
  - Needs `ToField`, `From<i32>`, `From<AuctionStrategy> for i32`
  - Update `Order::default()` to use `None` (optional field)

## Medium Priority 🟡

### 3. OcaType Enum
- **Location**: `src/orders/mod.rs:133`
- **Current**: `pub oca_type: i32`
- **Impact**: Medium - Used for OCA (One-Cancels-All) groups
- **Current Values**: 1, 2, 3
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum OcaType {
      None = 0,                    // Not part of OCA group
      CancelWithBlock = 1,         // Cancel all remaining orders with block
      ReduceWithBlock = 2,          // Proportionally reduce with block
      ReduceWithoutBlock = 3,       // Proportionally reduce without block
  }
  ```
- **Documentation**: "With block" means overfill protection - only one order routed at a time

### 4. OrderOrigin Enum
- **Location**: `src/orders/mod.rs:220`
- **Current**: `pub origin: i32`
- **Impact**: Low - Institutional customers mainly
- **Current Values**: 0 = Customer, 1 = Firm
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub enum OrderOrigin {
      #[default]
      Customer = 0,
      Firm = 1,
  }
  ```

### 5. ShortSaleSlot Enum
- **Location**: `src/orders/mod.rs:226`
- **Current**: `pub short_sale_slot: i32`
- **Impact**: Low - Institutional short sales
- **Current Values**: 1 = Broker holds shares, 2 = Third party holds shares
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ShortSaleSlot {
      #[default]
      None = 0,              // Not a short sale
      Broker = 1,            // Broker holds shares
      ThirdParty = 2,        // Shares come from elsewhere
  }
  ```
- **Related Field**: `designated_location` (String) - specifies where third party holds shares

### 6. VolatilityType Enum
- **Location**: `src/orders/mod.rs:265`
- **Current**: `pub volatility_type: Option<i32>` with typo "FIXM enum"
- **Impact**: Low - VOL orders only
- **Current Values**: 1 = Daily, 2 = Annual
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum VolatilityType {
      Daily = 1,
      Annual = 2,
  }
  ```

### 7. ReferencePriceType Enum
- **Location**: `src/orders/mod.rs:274`
- **Current**: `pub reference_price_type: Option<i32>`
- **Impact**: Low - VOL orders only
- **Current Values**: 1 = Average of NBBO, 2 = NBB or NBO
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ReferencePriceType {
      AverageOfNBBO = 1,     // Average of National Best Bid/Offer
      NBBO = 2,              // NBB or NBO depending on action/right
  }
  ```

## Lower Priority 🟢

### 8. OptionRight Enum
- **Location**: `src/contracts/mod.rs:154`
- **Current**: `pub right: String`
- **Impact**: Medium - All option contracts
- **Current Values**: "P", "PUT", "C", "CALL"
- **Proposed Enum**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum OptionRight {
      Put,
      Call,
  }
  ```
- **Implementation Notes**:
  - Needs `ToField` that converts to "P" or "C"
  - Needs `From<String>` that accepts "P", "PUT", "C", "CALL" (case insensitive)
  - Used in contract builders - update `ContractBuilder.right()` method
  - May affect existing code more broadly than other changes

### 9. Rule80A Enum
- **Location**: `src/orders/mod.rs` (already exists as enum!)
- **Status**: ✅ Already implemented
- **Note**: This shows the pattern is already established in the codebase

### 10. OrderOpenClose Enum
- **Location**: `src/orders/mod.rs` (already exists as enum!)
- **Status**: ✅ Already implemented
- **Note**: Another example of existing enum pattern

## Implementation Pattern

Based on `TriggerMethod` implementation, follow this pattern for new enums:

1. **Define the enum** with explicit discriminants matching protocol values:
   ```rust
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
   pub enum MyEnum {
       #[default]
       Variant1 = 0,
       Variant2 = 1,
   }
   ```

2. **Implement conversions**:
   ```rust
   impl From<MyEnum> for i32 {
       fn from(e: MyEnum) -> i32 {
           e as i32
       }
   }

   impl From<i32> for MyEnum {
       fn from(value: i32) -> Self {
           match value {
               0 => MyEnum::Variant1,
               1 => MyEnum::Variant2,
               _ => MyEnum::default(),  // Fallback for unknown values
           }
       }
   }
   ```

3. **Implement ToField** (if used in messages):
   ```rust
   impl crate::ToField for MyEnum {
       fn to_field(&self) -> String {
           i32::from(*self).to_string()
       }
   }
   ```

4. **Update usages**:
   - Change field type in struct
   - Update `Default` implementation
   - Update decoders: `field = message.next_int()?.into();`
   - Encoders work automatically via `ToField`

5. **Update tests**:
   - Replace integer literals with enum variants
   - Add imports where needed

6. **Update documentation**:
   - Replace "Valid values" comments with enum reference
   - Add doc comments to enum variants

## Benefits Summary

✅ **Type Safety**: Invalid values caught at compile time
✅ **Self-Documenting**: No need to look up magic numbers
✅ **IDE Support**: Auto-completion shows all valid options
✅ **Refactoring Safety**: Compiler catches all uses when changing enum
✅ **Better Error Messages**: Clear enum names in error messages
✅ **Pattern Matching**: Exhaustive match checking by compiler

## Migration Strategy

1. **Phase 1** (Completed): TriggerMethod for conditions and orders
2. **Phase 2** (Recommended): TimeInForce and AuctionStrategy (both have FIXME comments)
3. **Phase 3**: Remaining order enums (OCA, origin, short sale, volatility)
4. **Phase 4**: Contract enums (OptionRight) - requires more careful migration

## Notes

- All enum implementations should include `#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]`
- Use `#[default]` attribute on the default variant
- Provide bidirectional conversion with protocol values (i32 or String)
- Handle unknown values gracefully in `From<i32>` by falling back to default
- Update all related tests to use enum variants instead of magic numbers
- Consider backward compatibility if this becomes a published crate (currently pre-release)
