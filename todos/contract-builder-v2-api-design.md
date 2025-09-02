# Contract Builder V2 API Design

## Executive Summary

Version 2 redesigns the contract building API to be type-safe, ergonomic, and foolproof. By embracing breaking changes, we can create a best-in-class API that prevents runtime errors through compile-time guarantees.

## Core Design Principles

1. **Zero Invalid States**: Impossible to build invalid contracts
2. **Type-Driven Development**: Leverage Rust's type system fully
3. **Fail Fast, Fail at Compile Time**: No runtime validation needed
4. **Discoverable API**: IDE autocomplete guides users naturally
5. **Minimal Cognitive Load**: Intuitive naming and structure

## Breaking Changes from V1

### 1. Remove Generic `ContractBuilder::new()`
- **V1**: `ContractBuilder::new().symbol("AAPL").security_type(SecurityType::Stock)`
- **V2**: `Contract::stock("AAPL")` - Direct, type-safe entry points

### 2. Mandatory Type-State Pattern
- All builders use phantom types to enforce requirements
- No optional fields that are actually required
- Build method only available when contract is valid

### 3. Stronger Types Instead of Strings
```rust
// V1: Strings everywhere
.right("C")  // Could be anything
.exchange("SMART")  // Typo-prone

// V2: Type-safe enums
.right(OptionRight::Call)
.exchange(Exchange::Smart)
```

### 4. Separate Builder Per Contract Type
- No more single mega-builder trying to handle all cases
- Each contract type has its own specialized builder

## V2 API Architecture

### Core Types

```rust
// Strong typing for all domain concepts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionRight {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Exchange {
    Smart,
    Nasdaq,
    Nyse,
    Cboe,
    Globex,
    Idealpro,
    Paxos,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    CAD,
    AUD,
    Custom(String),
}

// Date types that can't be wrong
pub struct ExpirationDate {
    year: u16,
    month: u8,
    day: u8,
}

pub struct ContractMonth {
    year: u16,
    month: u8,
}

// Validated types
pub struct Symbol(String);
pub struct Cusip(String);
pub struct Isin(String);
pub struct Strike(f64);
```

### Stock Contract Builder

```rust
pub struct StockBuilder<S = Missing> {
    symbol: S,
    exchange: Exchange,
    currency: Currency,
    primary_exchange: Option<Exchange>,
    trading_class: Option<String>,
}

impl Contract {
    pub fn stock(symbol: impl Into<Symbol>) -> StockBuilder<Symbol> {
        StockBuilder {
            symbol: symbol.into(),
            exchange: Exchange::Smart,
            currency: Currency::USD,
            primary_exchange: None,
            trading_class: None,
        }
    }
}

impl StockBuilder<Symbol> {
    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }
    
    pub fn in_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }
    
    pub fn primary(mut self, exchange: Exchange) -> Self {
        self.primary_exchange = Some(exchange);
        self
    }
    
    // Direct build - no Result needed, can't fail
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol.to_string(),
            security_type: SecurityType::Stock,
            exchange: self.exchange.to_string(),
            currency: self.currency.to_string(),
            primary_exchange: self.primary_exchange.map(|e| e.to_string()).unwrap_or_default(),
            ..Default::default()
        }
    }
}
```

### Option Contract Builder with Type States

```rust
pub struct OptionBuilder<Symbol = Missing, Strike = Missing, Expiry = Missing> {
    symbol: Symbol,
    right: OptionRight,
    strike: Strike,
    expiry: Expiry,
    exchange: Exchange,
    currency: Currency,
    multiplier: u32,
}

impl Contract {
    pub fn call(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder {
            symbol: symbol.into(),
            right: OptionRight::Call,
            strike: Missing,
            expiry: Missing,
            exchange: Exchange::Smart,
            currency: Currency::USD,
            multiplier: 100,
        }
    }
    
    pub fn put(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder {
            symbol: symbol.into(),
            right: OptionRight::Put,
            strike: Missing,
            expiry: Missing,
            exchange: Exchange::Smart,
            currency: Currency::USD,
            multiplier: 100,
        }
    }
}

// Can only set strike when symbol is present
impl<E> OptionBuilder<Symbol, Missing, E> {
    pub fn strike(self, price: f64) -> OptionBuilder<Symbol, Strike, E> {
        OptionBuilder {
            symbol: self.symbol,
            right: self.right,
            strike: Strike::new(price), // Validates positive
            expiry: self.expiry,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
        }
    }
}

// Can only set expiry when symbol is present
impl<S> OptionBuilder<Symbol, S, Missing> {
    pub fn expires(self, date: ExpirationDate) -> OptionBuilder<Symbol, S, ExpirationDate> {
        OptionBuilder {
            symbol: self.symbol,
            right: self.right,
            strike: self.strike,
            expiry: date,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
        }
    }
    
    // Convenience constructors
    pub fn expires_on(self, year: u16, month: u8, day: u8) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::new(year, month, day))
    }
    
    pub fn expires_weekly(self) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::next_friday())
    }
    
    pub fn expires_monthly(self) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::third_friday_of_month())
    }
}

// Build only available when all required fields are set
impl OptionBuilder<Symbol, Strike, ExpirationDate> {
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol.to_string(),
            security_type: SecurityType::Option,
            strike: self.strike.value(),
            right: self.right.to_string(),
            last_trade_date_or_contract_month: self.expiry.to_string(),
            exchange: self.exchange.to_string(),
            currency: self.currency.to_string(),
            multiplier: self.multiplier.to_string(),
            ..Default::default()
        }
    }
}
```

### Futures Contract Builder

```rust
pub struct FuturesBuilder<Symbol = Missing, Month = Missing> {
    symbol: Symbol,
    contract_month: Month,
    exchange: Exchange,
    currency: Currency,
    multiplier: Option<u32>,
}

impl Contract {
    pub fn futures(symbol: impl Into<Symbol>) -> FuturesBuilder<Symbol, Missing> {
        FuturesBuilder {
            symbol: symbol.into(),
            contract_month: Missing,
            exchange: Exchange::Globex, // Smart default for futures
            currency: Currency::USD,
            multiplier: None,
        }
    }
}

impl FuturesBuilder<Symbol, Missing> {
    pub fn expires_in(self, month: ContractMonth) -> FuturesBuilder<Symbol, ContractMonth> {
        FuturesBuilder {
            symbol: self.symbol,
            contract_month: month,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
        }
    }
    
    // Convenience for common patterns
    pub fn front_month(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::front())
    }
    
    pub fn next_quarter(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::next_quarter())
    }
}

impl FuturesBuilder<Symbol, ContractMonth> {
    pub fn multiplier(mut self, value: u32) -> Self {
        self.multiplier = Some(value);
        self
    }
    
    pub fn build(self) -> Contract {
        // Auto-set multiplier based on symbol if not specified
        let multiplier = self.multiplier.unwrap_or_else(|| {
            match self.symbol.as_str() {
                "ES" | "NQ" => 50,
                "YM" => 5,
                "CL" => 1000,
                _ => 1,
            }
        });
        
        Contract {
            symbol: self.symbol.to_string(),
            security_type: SecurityType::Future,
            last_trade_date_or_contract_month: self.contract_month.to_string(),
            exchange: self.exchange.to_string(),
            currency: self.currency.to_string(),
            multiplier: multiplier.to_string(),
            ..Default::default()
        }
    }
}
```

### Spread/Combo Builder

```rust
pub struct SpreadBuilder {
    legs: Vec<Leg>,
    currency: Currency,
    exchange: Exchange,
}

pub struct Leg {
    contract_id: i32,
    action: Action,
    ratio: i32,
    exchange: Option<Exchange>,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Buy,
    Sell,
}

impl Contract {
    pub fn spread() -> SpreadBuilder {
        SpreadBuilder {
            legs: Vec::new(),
            currency: Currency::USD,
            exchange: Exchange::Smart,
        }
    }
}

impl SpreadBuilder {
    pub fn add_leg(mut self, contract_id: i32, action: Action) -> LegBuilder {
        LegBuilder {
            parent: self,
            leg: Leg {
                contract_id,
                action,
                ratio: 1,
                exchange: None,
            },
        }
    }
    
    // Convenience methods for common spreads
    pub fn calendar(self, near_id: i32, far_id: i32) -> Self {
        self.add_leg(near_id, Action::Buy)
            .done()
            .add_leg(far_id, Action::Sell)
            .done()
    }
    
    pub fn vertical(self, long_id: i32, short_id: i32) -> Self {
        self.add_leg(long_id, Action::Buy)
            .done()
            .add_leg(short_id, Action::Sell)
            .done()
    }
    
    pub fn iron_condor(
        self,
        long_put: i32,
        short_put: i32,
        short_call: i32,
        long_call: i32,
    ) -> Self {
        self.add_leg(long_put, Action::Buy)
            .done()
            .add_leg(short_put, Action::Sell)
            .done()
            .add_leg(short_call, Action::Sell)
            .done()
            .add_leg(long_call, Action::Buy)
            .done()
    }
    
    pub fn build(self) -> Result<Contract, SpreadError> {
        if self.legs.is_empty() {
            return Err(SpreadError::NoLegs);
        }
        
        Ok(Contract {
            security_type: SecurityType::Spread,
            currency: self.currency.to_string(),
            exchange: self.exchange.to_string(),
            combo_legs: self.legs.into_iter().map(Into::into).collect(),
            ..Default::default()
        })
    }
}

pub struct LegBuilder {
    parent: SpreadBuilder,
    leg: Leg,
}

impl LegBuilder {
    pub fn ratio(mut self, ratio: i32) -> Self {
        self.leg.ratio = ratio;
        self
    }
    
    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.leg.exchange = Some(exchange);
        self
    }
    
    pub fn done(mut self) -> SpreadBuilder {
        self.parent.legs.push(self.leg);
        self.parent
    }
}
```

### Special Contract Types

```rust
impl Contract {
    // Forex with automatic symbol formatting
    pub fn forex(base: Currency, quote: Currency) -> ForexBuilder {
        ForexBuilder {
            pair: format!("{}.{}", base, quote),
            exchange: Exchange::Idealpro,
            amount: 20_000, // Default minimum
        }
    }
    
    // Crypto with sane defaults
    pub fn crypto(symbol: impl Into<Symbol>) -> CryptoBuilder {
        CryptoBuilder {
            symbol: symbol.into(),
            exchange: Exchange::Paxos,
            currency: Currency::USD,
        }
    }
    
    // Index with proper configuration
    pub fn index(symbol: &str) -> Contract {
        let (exchange, currency) = match symbol {
            "SPX" | "NDX" | "DJI" | "RUT" => (Exchange::Cboe, Currency::USD),
            "DAX" => (Exchange::Eurex, Currency::EUR),
            "FTSE" => (Exchange::Lse, Currency::GBP),
            _ => (Exchange::Smart, Currency::USD),
        };
        
        Contract {
            symbol: symbol.to_string(),
            security_type: SecurityType::Index,
            exchange: exchange.to_string(),
            currency: currency.to_string(),
            ..Default::default()
        }
    }
    
    // Bond by CUSIP/ISIN
    pub fn bond(identifier: BondIdentifier) -> Contract {
        match identifier {
            BondIdentifier::Cusip(cusip) => Contract {
                symbol: cusip.to_string(),
                security_type: SecurityType::Bond,
                security_id_type: "CUSIP".to_string(),
                security_id: cusip.to_string(),
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                ..Default::default()
            },
            BondIdentifier::Isin(isin) => Contract {
                symbol: isin.to_string(),
                security_type: SecurityType::Bond,
                security_id_type: "ISIN".to_string(),
                security_id: isin.to_string(),
                exchange: "SMART".to_string(),
                currency: determine_currency_from_isin(&isin),
                ..Default::default()
            },
        }
    }
}
```

## Usage Examples

### V2 API in Action

```rust
// Stock - clean and simple
let aapl = Contract::stock("AAPL").build();

// Stock with customization
let toyota = Contract::stock("7203")
    .on_exchange(Exchange::Tsej)
    .in_currency(Currency::JPY)
    .build();

// Call option - won't compile without strike and expiry
let call = Contract::call("AAPL")
    .strike(150.0)
    .expires_on(2024, 12, 20)
    .build();

// Put with weekly expiry
let put = Contract::put("SPY")
    .strike(450.0)
    .expires_weekly()
    .build();

// Futures
let es = Contract::futures("ES")
    .front_month()
    .build(); // Multiplier auto-set to 50

// Forex
let eur_usd = Contract::forex(Currency::EUR, Currency::USD)
    .amount(100_000)
    .build();

// Crypto
let btc = Contract::crypto("BTC").build();

// Calendar spread
let spread = Contract::spread()
    .calendar(12345, 67890)
    .build()?;

// Iron condor
let ic = Contract::spread()
    .iron_condor(
        put_long_id,
        put_short_id,
        call_short_id,
        call_long_id,
    )
    .build()?;

// Index
let spx = Contract::index("SPX"); // No build needed, direct creation
```

## Migration from V1 to V2

### Automated Migration Tool

```rust
// Provide a migration tool that can parse V1 code and suggest V2 replacements
cargo install ibapi-migrate
ibapi-migrate --fix src/
```

### Migration Examples

```rust
// V1 Code
let contract = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Stock)
    .exchange("SMART")
    .currency("USD")
    .build()?;

// V2 Equivalent
let contract = Contract::stock("AAPL").build();

// V1 Option
let option = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Option)
    .strike(150.0)
    .right("C")
    .last_trade_date_or_contract_month("20241220")
    .exchange("SMART")
    .currency("USD")
    .build()?;

// V2 Option
let option = Contract::call("AAPL")
    .strike(150.0)
    .expires_on(2024, 12, 20)
    .build();
```

### Migration Guide Structure

1. **Quick Reference Card**: Side-by-side V1 vs V2 patterns
2. **Common Patterns**: How to translate typical use cases
3. **Breaking Changes List**: What won't compile and why
4. **New Features**: What V2 enables that V1 couldn't do

## Benefits of V2

### 1. Compile-Time Safety
- Invalid contracts are impossible to create
- Required fields enforced by type system
- No runtime validation needed

### 2. Better Developer Experience
- IDE autocomplete shows only valid methods
- Clear progression through builder states
- Self-documenting API

### 3. Performance
- Zero-cost abstractions
- No runtime field checking
- Smaller binary size (no validation code)

### 4. Maintainability
- Each contract type isolated
- Easy to add new contract types
- Clear separation of concerns

### 5. Correctness
- Strong types prevent typos
- Date validation built-in
- Exchange/currency compatibility checked

## Implementation Plan

### Phase 1: Core Types (Week 1)
- [ ] Define strong types (Symbol, Exchange, Currency, etc.)
- [ ] Implement validation for each type
- [ ] Create conversion traits

### Phase 2: Basic Builders (Week 2)
- [ ] Stock builder with type states
- [ ] Option builder with full validation
- [ ] Futures builder with smart defaults

### Phase 3: Advanced Builders (Week 3)
- [ ] Spread/combo builder
- [ ] Forex and crypto builders
- [ ] Special contract types

### Phase 4: Migration Support (Week 4)
- [ ] Migration tool development
- [ ] Documentation and examples
- [ ] Deprecation warnings in V1

### Phase 5: Testing & Polish (Week 5)
- [ ] Comprehensive test suite
- [ ] Performance benchmarks
- [ ] User feedback incorporation

## Risks and Mitigations

### Risk: User Resistance to Breaking Changes
**Mitigation**: 
- Provide excellent migration tooling
- Clear documentation of benefits
- Maintain V1 in separate module during transition

### Risk: Increased API Surface Area
**Mitigation**:
- Use traits to share common behavior
- Generate repetitive code with macros
- Clear module organization

### Risk: Complex Type States Hard to Understand
**Mitigation**:
- Extensive documentation with examples
- Hide complexity behind simple entry points
- Provide cookbook of common patterns

## Success Metrics

1. **Zero Runtime Panics**: No contract validation failures in production
2. **Reduced Support Burden**: Fewer questions about contract creation
3. **Improved Time to First Trade**: New users successful faster
4. **Type Safety Coverage**: 100% of invalid states prevented at compile time

## Conclusion

V2 represents a fundamental improvement in API design, leveraging Rust's type system to create an API that is both powerful and impossible to misuse. The breaking changes are justified by the significant gains in safety, usability, and performance.

The investment in V2 will pay dividends through:
- Reduced bugs in user code
- Better developer experience
- Easier maintenance
- Foundation for future enhancements

This is the API that Rust developers expect and deserve.