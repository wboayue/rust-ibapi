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
    pub fn new(symbol: impl Into<Symbol>) -> StockBuilder<Symbol> {
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

    pub fn trading_class(mut self, class: impl Into<String>) -> Self {
        self.trading_class = Some(class.into());
        self
    }

    /// Build the contract - cannot fail for stocks
    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol.to_string(),
            security_type: SecurityType::Stock,
            exchange: self.exchange.to_string(),
            currency: self.currency.to_string(),
            primary_exchange: self.primary_exchange.map(|e| e.to_string()).unwrap_or_default(),
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
}

impl OptionBuilder<Missing, Missing, Missing> {
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
            strike: Strike::new_unchecked(price),
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

// Optional setters available at any stage when symbol is present
impl<S, E> OptionBuilder<Symbol, S, E> {
    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    pub fn in_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    pub fn multiplier(mut self, multiplier: u32) -> Self {
        self.multiplier = multiplier;
        self
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
    pub fn new(symbol: impl Into<Symbol>) -> FuturesBuilder<Symbol, Missing> {
        FuturesBuilder {
            symbol: symbol.into(),
            contract_month: Missing,
            exchange: Exchange::Globex,
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

    pub fn front_month(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::front())
    }

    pub fn next_quarter(self) -> FuturesBuilder<Symbol, ContractMonth> {
        self.expires_in(ContractMonth::next_quarter())
    }
}

impl<M> FuturesBuilder<Symbol, M> {
    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    pub fn in_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    pub fn multiplier(mut self, value: u32) -> Self {
        self.multiplier = Some(value);
        self
    }
}

impl FuturesBuilder<Symbol, ContractMonth> {
    pub fn build(self) -> Contract {
        // Auto-set multiplier based on symbol if not specified
        let multiplier = self.multiplier.unwrap_or_else(|| match self.symbol.as_str() {
            "ES" | "NQ" => 50,
            "YM" => 5,
            "CL" => 1000,
            _ => 1,
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

/// Forex pair builder
#[derive(Debug, Clone)]
pub struct ForexBuilder {
    pair: String,
    exchange: Exchange,
    amount: u32,
}

impl ForexBuilder {
    pub fn new(base: Currency, quote: Currency) -> Self {
        ForexBuilder {
            pair: format!("{}.{}", base, quote),
            exchange: Exchange::Idealpro,
            amount: 20_000,
        }
    }

    pub fn amount(mut self, amount: u32) -> Self {
        self.amount = amount;
        self
    }

    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    pub fn build(self) -> Contract {
        Contract {
            symbol: self.pair,
            security_type: SecurityType::ForexPair,
            exchange: self.exchange.to_string(),
            currency: "USD".to_string(), // Quote currency
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
    pub fn new(symbol: impl Into<Symbol>) -> Self {
        CryptoBuilder {
            symbol: symbol.into(),
            exchange: Exchange::Paxos,
            currency: Currency::USD,
        }
    }

    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    pub fn in_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    pub fn build(self) -> Contract {
        Contract {
            symbol: self.symbol.to_string(),
            security_type: SecurityType::Crypto,
            exchange: self.exchange.to_string(),
            currency: self.currency.to_string(),
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

#[derive(Debug, Clone)]
pub struct Leg {
    contract_id: i32,
    action: Action,
    ratio: i32,
    exchange: Option<Exchange>,
}

impl SpreadBuilder {
    pub fn new() -> Self {
        SpreadBuilder {
            legs: Vec::new(),
            currency: Currency::USD,
            exchange: Exchange::Smart,
        }
    }
}

impl Default for SpreadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SpreadBuilder {
    pub fn add_leg(self, contract_id: i32, action: Action) -> LegBuilder {
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
        self.add_leg(near_id, Action::Buy).done().add_leg(far_id, Action::Sell).done()
    }

    /// Vertical spread convenience method
    pub fn vertical(self, long_id: i32, short_id: i32) -> Self {
        self.add_leg(long_id, Action::Buy).done().add_leg(short_id, Action::Sell).done()
    }

    /// Iron condor spread convenience method
    pub fn iron_condor(self, long_put_id: i32, short_put_id: i32, short_call_id: i32, long_call_id: i32) -> Self {
        self.add_leg(long_put_id, Action::Buy)
            .done()
            .add_leg(short_put_id, Action::Sell)
            .done()
            .add_leg(short_call_id, Action::Sell)
            .done()
            .add_leg(long_call_id, Action::Buy)
            .done()
    }

    pub fn in_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    pub fn on_exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

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
            currency: self.currency.to_string(),
            exchange: self.exchange.to_string(),
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

#[cfg(test)]
mod tests;
