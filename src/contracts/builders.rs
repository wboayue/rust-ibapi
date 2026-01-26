//! Type-safe builders for different contract types.

use super::types::*;
use super::{ComboLeg, Contract, SecurityType};
use crate::Error;

/// Stock contract builder with type-safe API
#[derive(Debug, Clone)]
pub struct StockBuilder<S = Missing> {
    symbol: S,
    exchange: Exchange,
    currency: Currency,
    primary_exchange: Option<Exchange>,
    trading_class: Option<String>,
}

impl StockBuilder<Missing> {
    /// Start building a stock contract for the provided symbol.
    pub fn new(symbol: impl Into<Symbol>) -> StockBuilder<Symbol> {
        StockBuilder {
            symbol: symbol.into(),
            exchange: "SMART".into(),
            currency: "USD".into(),
            primary_exchange: None,
            trading_class: None,
        }
    }
}

impl StockBuilder<Symbol> {
    /// Route the order to the specified exchange instead of the default.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Quote the contract in a different currency.
    pub fn in_currency(mut self, currency: impl Into<Currency>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Prefer a specific primary exchange when resolving the contract.
    pub fn primary(mut self, exchange: impl Into<Exchange>) -> Self {
        self.primary_exchange = Some(exchange.into());
        self
    }

    /// Hint the trading class for venues that require it.
    pub fn trading_class(mut self, class: impl Into<String>) -> Self {
        self.trading_class = Some(class.into());
        self
    }

    /// Build the contract - cannot fail for stocks
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol,
            security_type: SecurityType::Stock,
            exchange: self.exchange,
            currency: self.currency,
            primary_exchange: self.primary_exchange.unwrap_or_else(|| Exchange::from("")),
            trading_class: self.trading_class.unwrap_or_default(),
            ..Default::default()
        }
    }
}

/// Option contract builder with type states for required fields
#[derive(Debug, Clone)]
pub struct OptionBuilder<Symbol = Missing, Strike = Missing, Expiry = Missing> {
    symbol: Symbol,
    right: OptionRight,
    strike: Strike,
    expiry: Expiry,
    exchange: Exchange,
    currency: Currency,
    multiplier: u32,
    primary_exchange: Option<Exchange>,
    trading_class: Option<String>,
}

impl OptionBuilder<Missing, Missing, Missing> {
    /// Begin constructing a call option contract for the provided symbol.
    pub fn call(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder {
            symbol: symbol.into(),
            right: OptionRight::Call,
            strike: Missing,
            expiry: Missing,
            exchange: "SMART".into(),
            currency: "USD".into(),
            multiplier: 100,
            primary_exchange: None,
            trading_class: None,
        }
    }

    /// Begin constructing a put option contract for the provided symbol.
    pub fn put(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder {
            symbol: symbol.into(),
            right: OptionRight::Put,
            strike: Missing,
            expiry: Missing,
            exchange: "SMART".into(),
            currency: "USD".into(),
            multiplier: 100,
            primary_exchange: None,
            trading_class: None,
        }
    }
}

// Can only set strike when symbol is present
impl<E> OptionBuilder<Symbol, Missing, E> {
    /// Specify the option strike price.
    pub fn strike(self, price: f64) -> OptionBuilder<Symbol, Strike, E> {
        OptionBuilder {
            symbol: self.symbol,
            right: self.right,
            strike: Strike::new_unchecked(price),
            expiry: self.expiry,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
            primary_exchange: self.primary_exchange,
            trading_class: self.trading_class,
        }
    }
}

// Can only set expiry when symbol is present
impl<S> OptionBuilder<Symbol, S, Missing> {
    /// Provide an explicit expiration date.
    pub fn expires(self, date: ExpirationDate) -> OptionBuilder<Symbol, S, ExpirationDate> {
        OptionBuilder {
            symbol: self.symbol,
            right: self.right,
            strike: self.strike,
            expiry: date,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
            primary_exchange: self.primary_exchange,
            trading_class: self.trading_class,
        }
    }

    /// Convenience helper to set a specific calendar date.
    pub fn expires_on(self, year: u16, month: u8, day: u8) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::new(year, month, day))
    }

    /// Set the expiry to the next Friday weekly contract.
    pub fn expires_weekly(self) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::next_friday())
    }

    /// Set the expiry to the standard monthly contract.
    pub fn expires_monthly(self) -> OptionBuilder<Symbol, S, ExpirationDate> {
        self.expires(ExpirationDate::third_friday_of_month())
    }
}

// Optional setters available at any stage when symbol is present
impl<S, E> OptionBuilder<Symbol, S, E> {
    /// Route the option to a specific exchange.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Quote the option in a different currency.
    pub fn in_currency(mut self, currency: impl Into<Currency>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Override the contract multiplier (defaults to 100).
    pub fn multiplier(mut self, multiplier: u32) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Prefer a specific primary exchange when resolving the option.
    pub fn primary(mut self, exchange: impl Into<Exchange>) -> Self {
        self.primary_exchange = Some(exchange.into());
        self
    }

    /// Hint the trading class used by this contract.
    pub fn trading_class(mut self, class: impl Into<String>) -> Self {
        self.trading_class = Some(class.into());
        self
    }
}

// Build only available when all required fields are set
impl OptionBuilder<Symbol, Strike, ExpirationDate> {
    /// Finalize the option contract once symbol, strike, and expiry are set.
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol,
            security_type: SecurityType::Option,
            strike: self.strike.value(),
            right: self.right.to_string(),
            last_trade_date_or_contract_month: self.expiry.to_string(),
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier.to_string(),
            primary_exchange: self.primary_exchange.unwrap_or_else(|| Exchange::from("")),
            trading_class: self.trading_class.unwrap_or_default(),
            ..Default::default()
        }
    }
}

/// Futures contract builder with type states
#[derive(Debug, Clone)]
pub struct FuturesBuilder<Symbol = Missing, Month = Missing> {
    symbol: Symbol,
    contract_month: Month,
    exchange: Exchange,
    currency: Currency,
    multiplier: Option<u32>,
}

impl FuturesBuilder<Missing, Missing> {
    /// Start building a futures contract for the given symbol.
    pub fn new(symbol: impl Into<Symbol>) -> FuturesBuilder<Symbol, Missing> {
        FuturesBuilder {
            symbol: symbol.into(),
            contract_month: Missing,
            exchange: "GLOBEX".into(),
            currency: "USD".into(),
            multiplier: None,
        }
    }
}

impl FuturesBuilder<Symbol, Missing> {
    /// Specify the contract month to target for the future.
    pub fn expires_in(self, month: ContractMonth) -> FuturesBuilder<Symbol, ContractMonth> {
        FuturesBuilder {
            symbol: self.symbol,
            contract_month: month,
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier,
        }
    }

    /// Shortcut for selecting the current front-month contract.
    pub fn front_month(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::front())
    }

    /// Shortcut for selecting the next quarterly contract.
    pub fn next_quarter(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::next_quarter())
    }
}

impl<M> FuturesBuilder<Symbol, M> {
    /// Route the futures contract to a specific exchange.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Quote the future in a different currency.
    pub fn in_currency(mut self, currency: impl Into<Currency>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Set a custom multiplier value for the contract.
    pub fn multiplier(mut self, value: u32) -> Self {
        self.multiplier = Some(value);
        self
    }
}

impl FuturesBuilder<Symbol, ContractMonth> {
    /// Finalize the futures contract once the contract month is chosen.
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol,
            security_type: SecurityType::Future,
            last_trade_date_or_contract_month: self.contract_month.to_string(),
            exchange: self.exchange,
            currency: self.currency,
            multiplier: self.multiplier.map(|m| m.to_string()).unwrap_or_default(),
            ..Default::default()
        }
    }
}

/// Forex pair builder
#[derive(Debug, Clone)]
pub struct ForexBuilder {
    base: Currency,
    quote: Currency,
    exchange: Exchange,
}

impl ForexBuilder {
    /// Create a forex contract using the given base and quote currencies.
    pub fn new(base: impl Into<Currency>, quote: impl Into<Currency>) -> Self {
        ForexBuilder {
            base: base.into(),
            quote: quote.into(),
            exchange: "IDEALPRO".into(),
        }
    }

    /// Route the trade to a different forex venue.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Complete the forex contract definition.
    pub fn build(self) -> Contract {
        Contract {
            symbol: Symbol::new(self.base.0),
            security_type: SecurityType::ForexPair,
            exchange: self.exchange,
            currency: self.quote,
            ..Default::default()
        }
    }
}

/// Crypto currency builder
#[derive(Debug, Clone)]
pub struct CryptoBuilder {
    symbol: Symbol,
    exchange: Exchange,
    currency: Currency,
}

impl CryptoBuilder {
    /// Create a crypto contract for the specified symbol (e.g. `BTC`).
    pub fn new(symbol: impl Into<Symbol>) -> Self {
        CryptoBuilder {
            symbol: symbol.into(),
            exchange: "PAXOS".into(),
            currency: "USD".into(),
        }
    }

    /// Route the trade to a specific crypto venue.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Quote the pair in an alternate fiat or stablecoin.
    pub fn in_currency(mut self, currency: impl Into<Currency>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Finish building the crypto contract.
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol,
            security_type: SecurityType::Crypto,
            exchange: self.exchange,
            currency: self.currency,
            ..Default::default()
        }
    }
}

/// Spread/Combo builder
#[derive(Debug, Clone)]
pub struct SpreadBuilder {
    legs: Vec<Leg>,
    currency: Currency,
    exchange: Exchange,
}

/// Internal representation of a spread leg used by [SpreadBuilder].
#[derive(Debug, Clone)]
pub struct Leg {
    contract_id: i32,
    action: LegAction,
    ratio: i32,
    exchange: Option<Exchange>,
}

impl SpreadBuilder {
    /// Create an empty spread builder ready to accept legs.
    pub fn new() -> Self {
        SpreadBuilder {
            legs: Vec::new(),
            currency: "USD".into(),
            exchange: "SMART".into(),
        }
    }
}

impl Default for SpreadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SpreadBuilder {
    /// Begin configuring a new leg for the spread.
    pub fn add_leg(self, contract_id: i32, action: LegAction) -> LegBuilder {
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

    /// Calendar spread convenience method
    pub fn calendar(self, near_id: i32, far_id: i32) -> Self {
        self.add_leg(near_id, LegAction::Buy).done().add_leg(far_id, LegAction::Sell).done()
    }

    /// Vertical spread convenience method
    pub fn vertical(self, long_id: i32, short_id: i32) -> Self {
        self.add_leg(long_id, LegAction::Buy).done().add_leg(short_id, LegAction::Sell).done()
    }

    /// Iron condor spread convenience method
    pub fn iron_condor(self, long_put_id: i32, short_put_id: i32, short_call_id: i32, long_call_id: i32) -> Self {
        self.add_leg(long_put_id, LegAction::Buy)
            .done()
            .add_leg(short_put_id, LegAction::Sell)
            .done()
            .add_leg(short_call_id, LegAction::Sell)
            .done()
            .add_leg(long_call_id, LegAction::Buy)
            .done()
    }

    /// Override the spread currency, useful for non-USD underlyings.
    pub fn in_currency(mut self, currency: impl Into<Currency>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Route the spread order to a specific exchange.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Finalize the spread contract, returning an error if no legs were added.
    pub fn build(self) -> Result<Contract, Error> {
        if self.legs.is_empty() {
            return Err(Error::Simple("Spread must have at least one leg".into()));
        }

        let combo_legs: Vec<ComboLeg> = self
            .legs
            .into_iter()
            .map(|leg| ComboLeg {
                contract_id: leg.contract_id,
                ratio: leg.ratio,
                action: leg.action.to_string(),
                exchange: leg.exchange.map(|e| e.to_string()).unwrap_or_default(),
                ..Default::default()
            })
            .collect();

        Ok(Contract {
            security_type: SecurityType::Spread,
            currency: self.currency,
            exchange: self.exchange,
            combo_legs,
            ..Default::default()
        })
    }
}

/// Builder for individual spread legs
pub struct LegBuilder {
    parent: SpreadBuilder,
    leg: Leg,
}

impl LegBuilder {
    /// Set the contract ratio for the current leg.
    pub fn ratio(mut self, ratio: i32) -> Self {
        self.leg.ratio = ratio;
        self
    }

    /// Target a specific exchange for the leg.
    pub fn on_exchange(mut self, exchange: impl Into<Exchange>) -> Self {
        self.leg.exchange = Some(exchange.into());
        self
    }

    /// Finish the leg and return control to the parent spread builder.
    pub fn done(mut self) -> SpreadBuilder {
        self.parent.legs.push(self.leg);
        self.parent
    }
}

#[cfg(test)]
mod tests;
