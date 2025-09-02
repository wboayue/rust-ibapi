# Contract Builder Fluent API - Implementation Guide

## Overview

This document outlines an enhanced fluent API design for the contract builder in rust-ibapi. The design focuses on improving type safety, ergonomics, and discoverability while maintaining backward compatibility with the existing `ContractBuilder`.

## Design Goals

1. **Type Safety**: Use Rust's type system to prevent invalid contract configurations at compile time
2. **Ergonomics**: Provide intuitive method chains that guide users to valid contracts
3. **Discoverability**: Make the API self-documenting through clear method names and builder states
4. **Flexibility**: Support both simple convenience methods and advanced configuration
5. **Backward Compatibility**: Enhance existing builder without breaking current code

## Enhanced API Design

### 1. Type-State Pattern for Contract Types

Introduce marker types to enforce correct field requirements at compile time:

```rust
// Marker types for contract states
pub struct Unspecified;
pub struct StockContract;
pub struct OptionContract;
pub struct FuturesContract;
pub struct ForexContract;
pub struct BondContract;
pub struct ComboContract;
pub struct CryptoContract;
pub struct NewsContract;

// Marker types for required fields
pub struct WithSymbol;
pub struct WithoutSymbol;
pub struct WithStrike;
pub struct WithoutStrike;
pub struct WithExpiry;
pub struct WithoutExpiry;

pub struct ContractBuilder<T = Unspecified, S = WithoutSymbol, K = WithoutStrike, E = WithoutExpiry> {
    // fields...
    _phantom: PhantomData<(T, S, K, E)>,
}
```

### 2. Progressive Builder Methods

#### Stock Contracts

```rust
impl ContractBuilder {
    // Entry point - simple stock
    pub fn stock(symbol: &str) -> ContractBuilder<StockContract, WithSymbol> {
        // Sets symbol, security_type=Stock, exchange=SMART, currency=USD
    }
    
    // Entry point - specific market stock
    pub fn stock_on(symbol: &str, exchange: &str) -> ContractBuilder<StockContract, WithSymbol> {
        // Sets symbol, security_type=Stock, specific exchange, currency=USD
    }
}

impl<S, K, E> ContractBuilder<StockContract, S, K, E> {
    // Stock-specific methods
    pub fn primary_exchange(self, exchange: &str) -> Self { }
    pub fn trading_class(self, class: &str) -> Self { }
}
```

#### Option Contracts

```rust
impl ContractBuilder {
    // Entry point - equity option
    pub fn call(symbol: &str) -> ContractBuilder<OptionContract, WithSymbol, WithoutStrike, WithoutExpiry> {
        // Sets symbol, right=C, security_type=Option, exchange=SMART
    }
    
    pub fn put(symbol: &str) -> ContractBuilder<OptionContract, WithSymbol, WithoutStrike, WithoutExpiry> {
        // Sets symbol, right=P, security_type=Option, exchange=SMART
    }
}

impl<E> ContractBuilder<OptionContract, WithSymbol, WithoutStrike, E> {
    pub fn strike(self, price: f64) -> ContractBuilder<OptionContract, WithSymbol, WithStrike, E> {
        // Transitions to WithStrike state
    }
}

impl<K> ContractBuilder<OptionContract, WithSymbol, K, WithoutExpiry> {
    pub fn expires(self, date: &str) -> ContractBuilder<OptionContract, WithSymbol, K, WithExpiry> {
        // Transitions to WithExpiry state
    }
    
    pub fn expires_on(self, year: u16, month: u8, day: u8) -> ContractBuilder<OptionContract, WithSymbol, K, WithExpiry> {
        // Convenience method with date parts
    }
}

// Build only available when all required fields are set
impl ContractBuilder<OptionContract, WithSymbol, WithStrike, WithExpiry> {
    pub fn build(self) -> Result<Contract, Error> { }
}
```

#### Futures Contracts

```rust
impl ContractBuilder {
    pub fn futures(symbol: &str) -> ContractBuilder<FuturesContract, WithSymbol, WithoutStrike, WithoutExpiry> {
        // Sets symbol, security_type=Future
    }
}

impl ContractBuilder<FuturesContract, WithSymbol, WithoutStrike, WithoutExpiry> {
    pub fn contract_month(self, month: &str) -> ContractBuilder<FuturesContract, WithSymbol, WithoutStrike, WithExpiry> {
        // Sets contract month (YYYYMM format)
    }
    
    pub fn expires_in(self, year: u16, month: u8) -> ContractBuilder<FuturesContract, WithSymbol, WithoutStrike, WithExpiry> {
        // Convenience method
    }
}
```

#### Combo/Spread Contracts

```rust
impl ContractBuilder {
    pub fn spread() -> ComboBuilder {
        ComboBuilder::new()
    }
}

pub struct ComboBuilder {
    legs: Vec<ComboLeg>,
    description: Option<String>,
}

impl ComboBuilder {
    pub fn buy_leg(self, contract_id: i32) -> ComboLegBuilder {
        ComboLegBuilder::new(self, "BUY", contract_id)
    }
    
    pub fn sell_leg(self, contract_id: i32) -> ComboLegBuilder {
        ComboLegBuilder::new(self, "SELL", contract_id)
    }
    
    pub fn calendar_spread(self, near_contract: i32, far_contract: i32) -> Self {
        // Convenience for calendar spreads
    }
    
    pub fn vertical_spread(self, long_strike: i32, short_strike: i32) -> Self {
        // Convenience for vertical spreads
    }
    
    pub fn build(self) -> Result<Contract, Error> { }
}

pub struct ComboLegBuilder {
    parent: ComboBuilder,
    leg: ComboLeg,
}

impl ComboLegBuilder {
    pub fn ratio(mut self, ratio: i32) -> Self { }
    pub fn exchange(mut self, exchange: &str) -> Self { }
    pub fn add_leg(mut self) -> ComboBuilder {
        // Returns to parent builder
    }
}
```

### 3. Convenience Methods for Common Patterns

```rust
impl ContractBuilder {
    // Market indices
    pub fn index(symbol: &str) -> ContractBuilder<StockContract, WithSymbol> {
        // e.g., "SPX", "NDX", "DJI"
    }
    
    // Forex pairs
    pub fn forex(base: &str, quote: &str) -> ContractBuilder<ForexContract, WithSymbol> {
        // e.g., forex("EUR", "USD") -> "EUR.USD"
    }
    
    // Bonds
    pub fn bond(cusip: &str) -> ContractBuilder<BondContract, WithSymbol> {
        // Using CUSIP as identifier
    }
    
    // Weekly options
    pub fn weekly_option(symbol: &str) -> OptionChainBuilder {
        // Builder for weekly option chains
    }
}
```

### 4. Smart Defaults and Exchange Routing

```rust
impl ContractBuilder {
    // Smart routing with fallback
    pub fn smart_routed(mut self) -> Self {
        self.exchange("SMART")
    }
    
    // Regional preferences
    pub fn us_markets(mut self) -> Self {
        self.currency("USD").exchange("SMART")
    }
    
    pub fn european_markets(mut self) -> Self {
        self.currency("EUR").exchange("SMART")
    }
    
    pub fn asian_markets(mut self, currency: &str) -> Self {
        self.currency(currency)
    }
}
```

### 5. Validation and Error Handling

```rust
pub enum ContractBuilderError {
    MissingRequiredField(&'static str),
    InvalidStrike(f64),
    InvalidExpiration(String),
    InvalidRight(String),
    InvalidSymbol(String),
    InvalidExchange(String),
    IncompatibleConfiguration(String),
}

impl ContractBuilder {
    fn validate(&self) -> Result<(), ContractBuilderError> {
        // Comprehensive validation
    }
}
```

### 6. Builder Extensions for Advanced Features

```rust
trait ContractBuilderExt {
    // Delta neutral configurations
    fn with_delta_neutral(self, contract_id: i32, delta: f64, price: f64) -> Self;
    
    // Security identifiers
    fn with_isin(self, isin: &str) -> Self;
    fn with_cusip(self, cusip: &str) -> Self;
    
    // Include expired contracts
    fn include_expired(self) -> Self;
}
```

## Implementation Strategy

### Phase 1: Enhance Existing Builder (Backward Compatible)
- Add convenience constructors (stock, option, futures, etc.)
- Improve validation messages
- Add builder method aliases for better discoverability

### Phase 2: Type-State Pattern (New Module)
- Implement type-state builder in `contracts::builder::typed` module
- Maintain existing `ContractBuilder` for compatibility
- Provide migration guide

### Phase 3: Advanced Features
- Combo/spread builder
- Option chain builder
- Market scanner integration

## Usage Examples

### Simple Stock
```rust
let aapl = Contract::stock("AAPL").build()?;
```

### Stock with Details
```rust
let aapl = Contract::stock("AAPL")
    .primary_exchange("NASDAQ")
    .trading_class("NMS")
    .build()?;
```

### Call Option
```rust
let call = Contract::call("AAPL")
    .strike(150.0)
    .expires("20241220")
    .build()?;
```

### Put Option with Monthly Expiry
```rust
let put = Contract::put("SPY")
    .strike(450.0)
    .expires_on(2024, 12, 20)
    .multiplier("100")
    .build()?;
```

### Futures Contract
```rust
let es = Contract::futures("ES")
    .contract_month("202412")
    .exchange("GLOBEX")
    .multiplier("50")
    .build()?;
```

### Calendar Spread
```rust
let spread = Contract::spread()
    .buy_leg(12345)  // Near month contract ID
        .ratio(1)
        .add_leg()
    .sell_leg(67890)  // Far month contract ID
        .ratio(1)
        .add_leg()
    .description("Calendar Spread")
    .build()?;
```

### Forex Pair
```rust
let eurusd = Contract::forex("EUR", "USD")
    .exchange("IDEALPRO")
    .build()?;
```

### Crypto
```rust
let btc = Contract::crypto("BTC")
    .currency("USD")
    .exchange("PAXOS")
    .build()?;
```

## Benefits

1. **Compile-Time Safety**: Invalid configurations caught at compile time
2. **Intuitive API**: Method names guide users to correct usage
3. **Less Boilerplate**: Smart defaults reduce repetitive code
4. **Progressive Disclosure**: Simple things simple, complex things possible
5. **Better IDE Support**: Type-driven autocomplete suggestions

## Migration Path

Existing code using `ContractBuilder::new()` continues to work. New code can use either:

1. Enhanced convenience methods on existing builder
2. New type-state builder for maximum safety

```rust
// Old way (still works)
let contract = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Stock)
    .exchange("SMART")
    .currency("USD")
    .build()?;

// New way (convenience)
let contract = Contract::stock("AAPL").build()?;

// New way (type-safe)
let contract = typed::Contract::stock("AAPL")
    .smart_routed()
    .build()?;  // Compile-time guaranteed valid
```

## Testing Strategy

1. **Unit Tests**: Each builder state transition
2. **Integration Tests**: Common contract patterns
3. **Property Tests**: Validation logic
4. **Compilation Tests**: Type-state transitions
5. **Backward Compatibility**: Existing API usage

## Documentation

1. **API Docs**: Comprehensive rustdoc with examples
2. **Guide**: Step-by-step contract creation guide
3. **Cookbook**: Common patterns and recipes
4. **Migration Guide**: Moving from old to new API

## Next Steps

1. Review and approve design
2. Implement Phase 1 (backward compatible enhancements)
3. Create proof-of-concept for Phase 2 (type-state)
4. Gather feedback from users
5. Iterate and refine based on usage patterns