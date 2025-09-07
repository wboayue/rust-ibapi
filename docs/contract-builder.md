# Contract Builder API

The V2 Contract Builder API provides a type-safe, ergonomic way to create contracts for various financial instruments. It leverages Rust's type system to prevent invalid contracts at compile time while offering an intuitive, discoverable interface.

## Key Features

- **Type Safety**: Required fields are enforced at compile time
- **Zero Invalid States**: Impossible to build incomplete contracts
- **Smart Defaults**: Sensible defaults for exchanges, currencies, and multipliers
- **Discoverable API**: IDE autocomplete guides you through the process
- **Strongly Typed**: No more string typos for exchanges, currencies, or option rights

## Quick Start

```rust
use ibapi::contracts::{Contract, Exchange, Currency};

// Simple stock contract
let aapl = Contract::stock("AAPL").build();

// Stock with customization
let toyota = Contract::stock("7203")
    .on_exchange(Exchange::Tsej)
    .in_currency(Currency::JPY)
    .build();
```

## Contract Types

### Stocks

The simplest contract type. Defaults to SMART routing and USD currency.

```rust
use ibapi::contracts::{Contract, Exchange, Currency};

// Basic stock - uses SMART routing and USD
let stock = Contract::stock("AAPL").build();

// International stock with specific exchange
let european_stock = Contract::stock("SAN")
    .on_exchange(Exchange::Custom("IBIS".to_string()))
    .in_currency(Currency::EUR)
    .primary(Exchange::Custom("IBIS".to_string()))
    .build();

// Stock with trading class
let stock_with_class = Contract::stock("IBKR")
    .trading_class("NMS")
    .build();
```

### Options

Options require strike price and expiration date. The builder enforces these at compile time.

```rust
use ibapi::contracts::{Contract, Exchange, ExpirationDate};

// Call option
let call = Contract::call("AAPL")
    .strike(150.0)  // Validates positive strike price
    .expires_on(2024, 12, 20)
    .build();

// Put option with custom exchange
let put = Contract::put("SPY")
    .strike(450.0)
    .expires(ExpirationDate::new(2024, 3, 15))
    .on_exchange(Exchange::Cboe)
    .multiplier(100)
    .build();

// Weekly options (expires next Friday)
let weekly_call = Contract::call("QQQ")
    .strike(400.0)
    .expires_weekly()
    .build();

// Monthly options (third Friday of the month)
let monthly_put = Contract::put("IWM")
    .strike(200.0)
    .expires_monthly()
    .build();

// Note: These won't compile (type safety!)
// let invalid = Contract::call("AAPL").build();  // Missing strike and expiry
// let invalid = Contract::call("AAPL").strike(150.0).build();  // Missing expiry
```

### Futures

Futures contracts with automatic multiplier detection for common symbols.

```rust
use ibapi::contracts::{Contract, ContractMonth, Exchange};

// Front month contract (next expiring)
let es_front = Contract::futures("ES")
    .front_month()
    .build();

// Next quarterly expiration (Mar/Jun/Sep/Dec)
let nq_quarter = Contract::futures("NQ")
    .next_quarter()
    .build();

// Futures with specific expiration
let cl_futures = Contract::futures("CL")
    .expires_in(ContractMonth::new(2024, 6))
    .build();

// Custom futures on specific exchange
let zc_futures = Contract::futures("ZC")
    .expires_in(ContractMonth::new(2024, 12))
    .on_exchange(Exchange::Custom("ECBOT".to_string()))
    .build();

// Only specify multiplier when needed for special cases
let custom_contract = Contract::futures("CUSTOM")
    .expires_in(ContractMonth::new(2024, 9))
    .multiplier(100)  // Explicitly set for non-standard contracts
    .build();
```

#### Multiplier Settings

The futures builder leaves the multiplier field empty by default, allowing the TWS API to use the correct multiplier for each contract. Only specify a multiplier explicitly when needed for non-standard contracts using the `.multiplier()` method.

### Forex

Foreign exchange pairs with automatic pair formatting.

```rust
use ibapi::contracts::{Contract, Currency, Exchange};

// EUR/USD pair
let eur_usd = Contract::forex(Currency::EUR, Currency::USD)
    .amount(100_000)
    .build();

// GBP/JPY with custom exchange
let gbp_jpy = Contract::forex(Currency::GBP, Currency::JPY)
    .amount(50_000)
    .on_exchange(Exchange::Idealpro)
    .build();
```

### Cryptocurrency

Digital assets with Paxos as the default exchange.

```rust
use ibapi::contracts::{Contract, Currency, Exchange};

// Bitcoin
let btc = Contract::crypto("BTC")
    .build();  // Defaults to PAXOS exchange and USD

// Ethereum with custom settings
let eth = Contract::crypto("ETH")
    .in_currency(Currency::EUR)
    .on_exchange(Exchange::Paxos)
    .build();
```

### Index

Market indices with smart exchange and currency defaults.

```rust
use ibapi::contracts::Contract;

// S&P 500 - automatically uses CBOE exchange and USD
let spx = Contract::index("SPX");

// DAX - automatically uses EUREX exchange and EUR
let dax = Contract::index("DAX");

// FTSE - automatically uses LSE exchange and GBP  
let ftse = Contract::index("FTSE");

// Custom index
let custom = Contract::index("VIX");  // Defaults to SMART/USD
```

### Bonds

Bond contracts can be created using CUSIP or ISIN identifiers.

```rust
use ibapi::contracts::{Contract, BondIdentifier, Cusip, Isin};

// US Treasury bond by CUSIP
let treasury = Contract::bond(BondIdentifier::Cusip(Cusip::new("912810RN0")));

// European bond by ISIN
let euro_bond = Contract::bond(BondIdentifier::Isin(Isin::new("DE0001102309")));

// Corporate bond by CUSIP
let corporate = Contract::bond(BondIdentifier::Cusip(Cusip::new("037833100")));  // Apple bond
```

The bond builder automatically:
- Sets the correct security ID type (CUSIP or ISIN)
- Determines currency based on ISIN country code
- Uses SMART exchange routing

### Spreads and Combos

Complex multi-leg strategies with type-safe leg construction.

```rust
use ibapi::contracts::{Contract, Action, Currency, Exchange};

// Calendar spread (buy near, sell far)
let calendar = Contract::spread()
    .calendar(12345, 67890)  // Contract IDs for near and far legs
    .build()?;

// Vertical spread
let vertical = Contract::spread()
    .vertical(11111, 22222)  // Long and short contract IDs
    .in_currency(Currency::USD)
    .build()?;

// Iron condor using convenience method
let iron_condor = Contract::spread()
    .iron_condor(
        10001,  // Long put
        10002,  // Short put
        10003,  // Short call
        10004   // Long call
    )
    .build()?;

// Custom multi-leg spread
let butterfly = Contract::spread()
    .add_leg(30001, Action::Buy)   // Buy 1 lower strike
        .ratio(1)
        .done()
    .add_leg(30002, Action::Sell)  // Sell 2 middle strike
        .ratio(2)
        .done()
    .add_leg(30003, Action::Buy)   // Buy 1 higher strike
        .ratio(1)
        .done()
    .on_exchange(Exchange::Smart)
    .build()?;

// Ratio spread with different quantities
let ratio_spread = Contract::spread()
    .add_leg(20001, Action::Buy)
        .ratio(1)
        .done()
    .add_leg(20002, Action::Sell)
        .ratio(2)
        .on_exchange(Exchange::Cboe)
        .done()
    .build()?;
```

## Strong Types

The V2 API uses strong types instead of strings to prevent errors:

### Exchanges

```rust
use ibapi::contracts::Exchange;

let exchange = Exchange::Smart;        // Smart routing
let exchange = Exchange::Nasdaq;       // NASDAQ
let exchange = Exchange::Cboe;         // CBOE
let exchange = Exchange::Globex;       // CME Globex
let exchange = Exchange::Idealpro;     // Forex
let exchange = Exchange::Paxos;        // Crypto
let exchange = Exchange::Custom("ARCA".to_string());  // Custom exchange
```

### Currencies

```rust
use ibapi::contracts::Currency;

let currency = Currency::USD;
let currency = Currency::EUR;
let currency = Currency::GBP;
let currency = Currency::JPY;
let currency = Currency::Custom("SEK".to_string());  // Custom currency
```

### Option Rights

```rust
use ibapi::contracts::OptionRight;

let right = OptionRight::Call;  // Converts to "C"
let right = OptionRight::Put;   // Converts to "P"
```

## Type-State Pattern

The builders use Rust's type system to track required fields:

```rust
use ibapi::contracts::{Contract, Missing, Symbol, Strike, ExpirationDate};

// The option builder tracks which fields are set using phantom types
let builder1: OptionBuilder<Symbol, Missing, Missing> = Contract::call("AAPL");
let builder2: OptionBuilder<Symbol, Strike, Missing> = builder1.strike(150.0);
let builder3: OptionBuilder<Symbol, Strike, ExpirationDate> = builder2.expires_on(2024, 12, 20);
let contract = builder3.build();  // Build only available when all required fields are set
```

## Error Handling

The API validates inputs and returns errors for invalid data:

```rust
use ibapi::contracts::{Contract, Strike};

// Strike price validation
let result = Strike::new(-10.0);
assert!(result.is_err());  // Negative strikes are invalid

// Spread validation
let empty_spread = Contract::spread().build();
assert!(empty_spread.is_err());  // Spreads need at least one leg

// Option validation happens at compile time
// This won't compile:
// let invalid = Contract::call("AAPL").build();  // Missing required fields
```

## Migration from V1

If you're upgrading from the old ContractBuilder API:

### Old V1 API
```rust
use ibapi::contracts::{ContractBuilder, SecurityType};

let contract = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Stock)
    .exchange("SMART")
    .currency("USD")
    .build()?;
```

### New V2 API
```rust
use ibapi::contracts::Contract;

let contract = Contract::stock("AAPL").build();
```

### Key Differences

1. **Entry Points**: Use `Contract::stock()`, `Contract::call()`, etc. instead of `ContractBuilder::new()`
2. **Type Safety**: Required fields enforced at compile time
3. **Strong Types**: Use `Exchange::Smart` instead of `"SMART"`
4. **Smart Defaults**: Less boilerplate for common cases
5. **No Runtime Validation**: Invalid states prevented by the type system

## Complete Example

Here's a comprehensive example showing various contract types:

```rust
use ibapi::contracts::{
    Contract, Exchange, Currency, OptionRight, 
    ExpirationDate, ContractMonth, Action
};

fn create_contracts() -> Result<(), Box<dyn std::error::Error>> {
    // Equity
    let stock = Contract::stock("MSFT")
        .on_exchange(Exchange::Nasdaq)
        .build();
    
    // Option chain
    let call = Contract::call("MSFT")
        .strike(400.0)
        .expires_on(2024, 12, 20)
        .build();
    
    let put = Contract::put("MSFT")
        .strike(380.0)
        .expires_on(2024, 12, 20)
        .build();
    
    // Futures
    let futures = Contract::futures("NQ")
        .expires_in(ContractMonth::new(2024, 6))
        .build();
    
    // Forex
    let forex = Contract::forex(Currency::EUR, Currency::USD)
        .amount(100_000)
        .build();
    
    // Crypto
    let crypto = Contract::crypto("ETH").build();
    
    // Index
    let index = Contract::index("NDX");
    
    // Complex spread
    let butterfly = Contract::spread()
        .add_leg(50001, Action::Buy)
            .ratio(1)
            .done()
        .add_leg(50002, Action::Sell)
            .ratio(2)
            .done()
        .add_leg(50003, Action::Buy)
            .ratio(1)
            .done()
        .build()?;
    
    Ok(())
}
```

## Best Practices

1. **Use specific entry points**: Start with `Contract::stock()`, `Contract::call()`, etc. for clarity
2. **Let the compiler help**: Missing required fields will be caught at compile time
3. **Use strong types**: Prefer `Exchange::Smart` over `Exchange::Custom("SMART".to_string())`
4. **Leverage defaults**: Don't specify values that match the defaults
5. **Handle errors appropriately**: Strike validation and spread building return `Result`

## Recent V2 Improvements

The V2 Contract Builder API has been enhanced with several production-ready features:

### New Convenience Methods

**Options:**
- `expires_weekly()` - Automatically calculates next Friday expiration
- `expires_monthly()` - Automatically calculates third Friday of the month

**Futures:**
- `front_month()` - Gets the next expiring contract month
- `next_quarter()` - Gets the next quarterly expiration (Mar/Jun/Sep/Dec)

**Spreads:**
- `iron_condor()` - Convenience method for creating iron condor spreads

### New Contract Types

**Bonds:**
- Support for CUSIP and ISIN identifiers
- Automatic currency detection from ISIN country codes
- `Contract::bond(BondIdentifier::Cusip(cusip))` or `Contract::bond(BondIdentifier::Isin(isin))`

### API Improvements

**Strike Price Validation:**
- No longer returns `Result` from the builder method
- Cleaner API: `.strike(150.0)` instead of `.strike(150.0)?`
- Runtime validation ensures positive strike prices

**Date Utilities:**
- Smart date calculations for options expiration
- Automatic handling of weekends and holidays logic
- Time zone aware calculations

### Type Safety Enhancements

All builders now use the type-state pattern consistently:
- Compile-time enforcement of required fields
- No runtime surprises from missing data
- IDE autocomplete shows only valid methods at each step

## API Reference

For complete API documentation, run:
```bash
cargo doc --open
```

The contract builder modules are located at:
- `ibapi::contracts::builders` - Builder implementations
- `ibapi::contracts::types` - Strong type definitions
- `ibapi::contracts` - Contract struct and factory methods