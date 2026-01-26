# Contract Builder API

The V2 Contract Builder API provides a type-safe, ergonomic way to create contracts for various financial instruments. It leverages Rust's type system to prevent invalid contracts at compile time while offering an intuitive, discoverable interface.

## Table of Contents

- [Key Features](#key-features)
- [Quick Start](#quick-start)
- [Contract Types](#contract-types)
  - [Stocks](#stocks)
  - [Options](#options)
  - [Futures](#futures)
  - [Forex](#forex)
  - [Cryptocurrency](#cryptocurrency)
  - [Index](#index)
  - [Bonds](#bonds)
  - [Spreads and Combos](#spreads-and-combos)
- [Strong Types](#strong-types)
- [Type-State Pattern](#type-state-pattern)
- [Error Handling](#error-handling)
- [Migration from V1](#migration-from-v1)
- [Best Practices](#best-practices)

## Key Features

- **Type Safety**: Required fields are enforced at compile time
- **Zero Invalid States**: Impossible to build incomplete contracts
- **Smart Defaults**: Sensible defaults for exchanges, currencies, and multipliers
- **Discoverable API**: IDE autocomplete guides you through the process
- **Strongly Typed**: No more string typos for exchanges, currencies, or option rights

## Quick Start

```rust
use ibapi::contracts::Contract;

// Simple stock contract
let aapl = Contract::stock("AAPL").build();

// Stock with customization
let toyota = Contract::stock("7203")
    .on_exchange("TSEJ")
    .in_currency("JPY")
    .build();
```

## Contract Types

### Stocks

The simplest contract type. Defaults to SMART routing and USD currency.

```rust
use ibapi::contracts::Contract;

// Basic stock - uses SMART routing and USD
let stock = Contract::stock("AAPL").build();

// European stock - disambiguate which listing we want
let european_stock = Contract::stock("SAN")
    .on_exchange("SMART")  // Use SMART routing for best execution
    .primary("IBIS")       // But we want the IBIS listing (not Madrid, etc.)
    .in_currency("EUR")
    .build();

// Stock with trading class
let stock_with_class = Contract::stock("IBKR")
    .trading_class("NMS")
    .build();
```

#### Exchange vs Primary Exchange

- **`on_exchange()`** - Where to route your order (e.g., SMART, NASDAQ, NYSE)
- **`primary()`** - Which listing of the stock you want (for disambiguation)

Common patterns:
```rust
// US stock - usually no primary_exchange needed
let us_stock = Contract::stock("AAPL")
    .on_exchange("SMART")  // Route via SMART (default)
    .build();

// Dual-listed stock - specify which listing
let dual_listed = Contract::stock("BMW")
    .on_exchange("SMART")  // Route via SMART for best price
    .primary("IBIS")       // We want the German listing, not another
    .in_currency("EUR")
    .build();

// Direct routing to specific exchange
let direct_route = Contract::stock("BMW")
    .on_exchange("IBIS")   // Route directly to IBIS
    .primary("IBIS")       // And it's the IBIS listing we want
    .in_currency("EUR")
    .build();
```

### Options

Options require strike price and expiration date. The builder enforces these at compile time.

```rust
use ibapi::contracts::{Contract, ExpirationDate};

// Call option
let call = Contract::call("AAPL")
    .strike(150.0)  // Validates positive strike price
    .expires_on(2024, 12, 20)
    .build();

// Put option with custom exchange
let put = Contract::put("SPY")
    .strike(450.0)
    .expires(ExpirationDate::new(2024, 3, 15))
    .on_exchange("CBOE")
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

Futures contracts with flexible expiration options.

```rust
use ibapi::contracts::{Contract, ContractMonth};

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
    .on_exchange("ECBOT")
    .build();
```

**Note:** The futures builder leaves the multiplier field empty by default, allowing TWS to determine the correct value. Use `.multiplier()` only when needed for non-standard contracts.

### Forex

Foreign exchange pairs with automatic pair formatting.

```rust
use ibapi::contracts::Contract;

// EUR/USD pair
let eur_usd = Contract::forex("EUR", "USD").build();

// GBP/JPY with custom exchange
let gbp_jpy = Contract::forex("GBP", "JPY")
    .on_exchange("IDEALPRO")
    .build();
```

### Cryptocurrency

Digital assets with Paxos as the default exchange.

```rust
use ibapi::contracts::Contract;

// Bitcoin
let btc = Contract::crypto("BTC")
    .build();  // Defaults to PAXOS exchange and USD

// Ethereum with custom settings
let eth = Contract::crypto("ETH")
    .in_currency("EUR")
    .on_exchange("PAXOS")
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

// FTSE - automatically uses FTSE exchange and GBP  
let ftse = Contract::index("FTSE");

// Custom index
let custom = Contract::index("VIX");  // Defaults to SMART/USD
```

### Bonds

Bond contracts can be created using CUSIP or ISIN identifiers.

```rust
use ibapi::contracts::Contract;

// US Treasury bond by CUSIP
let treasury = Contract::bond_cusip("912810RN0");

// European bond by ISIN
let euro_bond = Contract::bond_isin("DE0001102309");

// Corporate bond by CUSIP
let corporate = Contract::bond_cusip("037833100");  // Apple bond
```

The bond builder automatically:
- Sets the correct security ID type (CUSIP or ISIN)
- Determines currency based on ISIN country code
- Uses SMART exchange routing

### Spreads and Combos

Complex multi-leg strategies with type-safe leg construction.

```rust
use ibapi::contracts::{Contract, LegAction};

// Vertical spread
let vertical = Contract::spread()
    .vertical(11111, 22222)  // Long and short contract IDs
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

// Custom butterfly spread
let butterfly = Contract::spread()
    .add_leg(30001, LegAction::Buy)   // Buy 1 lower strike
        .ratio(1)
        .done()
    .add_leg(30002, LegAction::Sell)  // Sell 2 middle strike
        .ratio(2)
        .done()
    .add_leg(30003, LegAction::Buy)   // Buy 1 higher strike
        .ratio(1)
        .done()
    .build()?;
```

## Strong Types

The V2 API uses strong types instead of strings to prevent errors:

### Exchanges

IBKR supports 160+ exchanges worldwide. The Exchange type provides constants for common exchanges and supports any exchange code.

```rust
use ibapi::contracts::Exchange;

// Create exchanges from string literals
let exchange = Exchange::from("SMART");        // Smart routing
let exchange = Exchange::from("NASDAQ");       // NASDAQ
let exchange = Exchange::from("NYSE");         // NYSE
let exchange = Exchange::from("CBOE");         // CBOE
let exchange = Exchange::from("GLOBEX");       // CME Globex
let exchange = Exchange::from("IDEALPRO");     // Forex
let exchange = Exchange::from("PAXOS");        // Crypto

// Any other exchange code
let exchange = Exchange::from("ARCA");         // NYSE Arca
let exchange = Exchange::from("IBIS");         // XETRA
let exchange = Exchange::from("SEHK");         // Hong Kong

// Builder methods accept string literals directly
let contract = Contract::stock("AAPL")
    .on_exchange("NASDAQ")  // String literal converted automatically
    .build();
```

### Currencies

IBKR supports trading in many currencies. The Currency type provides constants for major currencies and supports any currency code.

```rust
use ibapi::contracts::Currency;

// Create currencies from string literals
let currency = Currency::from("USD");          // US Dollar
let currency = Currency::from("EUR");          // Euro
let currency = Currency::from("GBP");          // British Pound
let currency = Currency::from("JPY");          // Japanese Yen
let currency = Currency::from("CHF");          // Swiss Franc
let currency = Currency::from("CAD");          // Canadian Dollar
let currency = Currency::from("AUD");          // Australian Dollar
let currency = Currency::from("HKD");          // Hong Kong Dollar
let currency = Currency::from("CNH");          // Offshore RMB

// Any other currency code
let currency = Currency::from("SEK");          // Swedish Krona
let currency = Currency::from("NOK");          // Norwegian Krone
let currency = Currency::from("INR");          // Indian Rupee

// Builder methods accept string literals directly
let contract = Contract::stock("7203")
    .in_currency("JPY")  // String literal converted automatically
    .build();
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
3. **Strong Types**: Use `Exchange::from("SMART")` for type safety or pass string literals directly to builder methods
4. **Smart Defaults**: Less boilerplate for common cases
5. **No Runtime Validation**: Invalid states prevented by the type system

## Best Practices

1. **Use specific entry points**: Start with `Contract::stock()`, `Contract::call()`, etc. for clarity
2. **Let the compiler help**: Missing required fields will be caught at compile time
3. **Flexible types**: Builder methods accept string literals directly (e.g., `.on_exchange("SMART")`), or use `Exchange::from("SMART")` for explicit type creation
4. **Leverage defaults**: Don't specify values that match the defaults
5. **Handle errors appropriately**: Strike validation and spread building return `Result`

