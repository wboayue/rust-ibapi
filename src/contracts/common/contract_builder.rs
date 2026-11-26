use super::super::{ComboLeg, Contract, Currency, DeltaNeutralContract, Exchange, Expiry, OptionRight, SecurityIdType, SecurityType, Strike, Symbol};
use crate::contracts::ContractId;
use crate::Error;

/// Builder for creating and validating [Contract] instances
///
/// The [ContractBuilder] provides a fluent interface for constructing contracts with validation.
/// It ensures that contracts are properly configured for their security type and prevents
/// common errors through compile-time and runtime validation.
///
/// # Examples
///
/// ## Creating a Stock Contract
///
/// ```no_run
/// use ibapi::contracts::{ContractBuilder, SecurityType};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Using the builder pattern
/// let contract = ContractBuilder::new()
///     .symbol("AAPL")
///     .security_type(SecurityType::Stock)
///     .exchange("SMART")
///     .currency("USD")
///     .build()?;
///
/// // Using the convenience method
/// let contract = ContractBuilder::stock("AAPL", "SMART", "USD").build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating an Option Contract
///
/// ```no_run
/// use ibapi::contracts::{ContractBuilder, OptionRight, SecurityType};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
///     .strike(150.0)
///     .right(OptionRight::Call)
///     .last_trade_date_or_contract_month("20241220")
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a Futures Contract
///
/// ```no_run
/// use ibapi::contracts::{ContractBuilder, SecurityType};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let contract = ContractBuilder::futures("ES", "CME", "USD")
///     .last_trade_date_or_contract_month("202412")
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a Crypto Contract
///
/// ```no_run
/// use ibapi::contracts::{ContractBuilder, SecurityType};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let contract = ContractBuilder::crypto("BTC", "PAXOS", "USD").build()?;
/// # Ok(())
/// # }
/// ```
///
/// # Validation
///
/// The builder performs validation when [build](ContractBuilder::build) is called:
/// - A symbol, local symbol, contract id or security id identifies the contract
/// - A security type is required unless a contract id or security id identifies the contract
/// - Option contracts require strike, right, and expiration date, and futures contracts a
///   contract month, unless a contract id or local symbol identifies the contract
/// - Strike prices cannot be negative
#[derive(Clone, Debug, Default)]
#[must_use = "ContractBuilder does nothing until you call .build()"]
pub struct ContractBuilder {
    pub(crate) contract_id: Option<i32>,
    pub(crate) symbol: Option<String>,
    pub(crate) security_type: Option<SecurityType>,
    pub(crate) last_trade_date_or_contract_month: Option<ExpiryInput>,
    pub(crate) strike: Option<f64>,
    pub(crate) right: Option<OptionRight>,
    pub(crate) multiplier: Option<String>,
    pub(crate) exchange: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) local_symbol: Option<String>,
    pub(crate) primary_exchange: Option<String>,
    pub(crate) trading_class: Option<String>,
    pub(crate) include_expired: Option<bool>,
    pub(crate) security_id_type: Option<SecurityIdType>,
    pub(crate) security_id: Option<String>,
    pub(crate) combo_legs_description: Option<String>,
    pub(crate) combo_legs: Option<Vec<ComboLeg>>,
    pub(crate) delta_neutral_contract: Option<DeltaNeutralContract>,
    pub(crate) last_trade_date: Option<time::Date>,
    pub(crate) issuer_id: Option<String>,
    pub(crate) description: Option<String>,
}

/// What [`ContractBuilder::last_trade_date_or_contract_month`] or
/// [`ContractBuilder::expiry`] stored; text is parsed in `build()`, which can
/// return the error.
#[derive(Clone, Debug)]
pub(crate) enum ExpiryInput {
    Text(String),
    Typed(Expiry),
}

impl ContractBuilder {
    /// Creates a new [ContractBuilder]
    ///
    /// A field left unset builds empty or zero: security type [`SecurityType::Stock`], and an
    /// empty exchange and currency. A `Contract` struct literal ending in `..Default::default()`
    /// differs in one of these: its exchange is `SMART`. Set [`exchange`](Self::exchange) for
    /// smart routing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::{ContractBuilder, SecurityType};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::new()
    ///     .symbol("MSFT")
    ///     .security_type(SecurityType::Stock)
    ///     .exchange("SMART")
    ///     .currency("USD")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the contract ID
    ///
    /// The unique IB contract identifier. When specified, other contract details may be optional.
    pub fn contract_id(mut self, contract_id: impl Into<ContractId>) -> Self {
        self.contract_id = Some(contract_id.into().value());
        self
    }

    /// Sets the underlying asset symbol
    ///
    /// Required field for all contracts.
    ///
    /// # Examples
    /// - Stocks: "AAPL", "MSFT", "TSLA"
    /// - Futures: "ES", "NQ", "CL"
    /// - Crypto: "BTC", "ETH"
    pub fn symbol<S: Into<String>>(mut self, symbol: S) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Sets the security type
    ///
    /// Defines what type of instrument this contract represents.
    /// See [SecurityType] for available options. Optional when a
    /// [`contract_id`](Self::contract_id) or [`security_id`](Self::security_id) identifies the
    /// contract: the contract's `security_type` is then `None` and no `secType` is sent.
    /// Required otherwise; [`build`](Self::build) rejects a symbol or local-symbol lookup
    /// without one.
    pub fn security_type(mut self, security_type: SecurityType) -> Self {
        self.security_type = Some(security_type);
        self
    }

    /// Sets the last trade date or contract month
    ///
    /// For futures and options, this field is required:
    /// - Format YYYYMM for contract month (e.g., "202412")
    /// - Format YYYYMMDD for specific expiration date (e.g., "20241220")
    ///
    /// [`build`](Self::build) parses the string and returns [`Error::Validation`]
    /// with [`ValidationError::InvalidDate`](crate::builder::ValidationError::InvalidDate) if it is neither. An empty string clears the
    /// field, as in 5.x, so an option or future then fails the required check.
    /// For a typed value use [`expiry`](Self::expiry).
    pub fn last_trade_date_or_contract_month<S: Into<String>>(mut self, date: S) -> Self {
        let date = date.into();
        self.last_trade_date_or_contract_month = (!date.is_empty()).then_some(ExpiryInput::Text(date));
        self
    }

    /// Sets the last trade date or contract month from a typed value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{ContractBuilder, ContractMonth, Expiry, SecurityType};
    ///
    /// let contract = ContractBuilder::new()
    ///     .symbol("ES")
    ///     .security_type(SecurityType::Future)
    ///     .exchange("CME")
    ///     .expiry(ContractMonth::new(2026, 12).unwrap())
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(contract.last_trade_date_or_contract_month, Some(Expiry::Month(ContractMonth::new(2026, 12).unwrap())));
    /// ```
    pub fn expiry(mut self, expiry: impl Into<Expiry>) -> Self {
        self.last_trade_date_or_contract_month = Some(ExpiryInput::Typed(expiry.into()));
        self
    }

    /// Sets the option's strike price
    ///
    /// Required for option contracts. Must be a positive value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::{ContractBuilder, OptionRight};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
    ///     .strike(150.0)
    ///     .right(OptionRight::Call)
    ///     .last_trade_date_or_contract_month("20241220")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn strike(mut self, strike: f64) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Sets the option right (Call or Put).
    ///
    /// Required for option contracts.
    pub fn right(mut self, right: OptionRight) -> Self {
        self.right = Some(right);
        self
    }

    /// Sets the contract multiplier
    ///
    /// For example, options on futures often have a multiplier that differs from
    /// the underlying future's multiplier.
    pub fn multiplier<S: Into<String>>(mut self, multiplier: S) -> Self {
        self.multiplier = Some(multiplier.into());
        self
    }

    /// Sets the exchange for routing orders
    ///
    /// Common values include:
    /// - "SMART" for IB's smart routing
    /// - "NASDAQ", "NYSE", "AMEX" for US equities
    /// - "CME", "NYMEX" for futures
    pub fn exchange<S: Into<String>>(mut self, exchange: S) -> Self {
        self.exchange = Some(exchange.into());
        self
    }

    /// Sets the contract's currency
    ///
    /// Typically "USD" for US markets, "EUR" for European markets, etc.
    pub fn currency<S: Into<String>>(mut self, currency: S) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Sets the local symbol
    ///
    /// The symbol within the exchange. Often the same as symbol but can differ
    /// for futures and options contracts.
    pub fn local_symbol<S: Into<String>>(mut self, local_symbol: S) -> Self {
        self.local_symbol = Some(local_symbol.into());
        self
    }

    /// Sets the primary exchange
    ///
    /// The primary listing exchange. Used with SMART routing to define
    /// the contract unambiguously.
    pub fn primary_exchange<S: Into<String>>(mut self, primary_exchange: S) -> Self {
        self.primary_exchange = Some(primary_exchange.into());
        self
    }

    /// Sets the trading class
    ///
    /// For example, options traded on different exchanges may have different
    /// trading class names despite being otherwise identical.
    pub fn trading_class<S: Into<String>>(mut self, trading_class: S) -> Self {
        self.trading_class = Some(trading_class.into());
        self
    }

    /// Sets whether to include expired contracts
    ///
    /// If true, contract details requests can return expired contracts.
    pub fn include_expired(mut self, include_expired: bool) -> Self {
        self.include_expired = Some(include_expired);
        self
    }

    /// Sets the security ID scheme paired with [`security_id`](Self::security_id).
    pub fn security_id_type(mut self, security_id_type: SecurityIdType) -> Self {
        self.security_id_type = Some(security_id_type);
        self
    }

    /// Sets the security ID
    ///
    /// The actual security identifier value corresponding to the security_id_type.
    pub fn security_id<S: Into<String>>(mut self, security_id: S) -> Self {
        self.security_id = Some(security_id.into());
        self
    }

    /// Sets the combo legs description
    ///
    /// For combo orders, provides a human-readable description of the legs.
    pub fn combo_legs_description<S: Into<String>>(mut self, combo_legs_description: S) -> Self {
        self.combo_legs_description = Some(combo_legs_description.into());
        self
    }

    /// Sets the combo legs
    ///
    /// Defines the individual legs for combo/spread contracts.
    pub fn combo_legs(mut self, combo_legs: Vec<ComboLeg>) -> Self {
        self.combo_legs = Some(combo_legs);
        self
    }

    /// Sets the delta neutral contract
    ///
    /// Used for delta-neutral combo orders.
    pub fn delta_neutral_contract(mut self, delta_neutral_contract: DeltaNeutralContract) -> Self {
        self.delta_neutral_contract = Some(delta_neutral_contract);
        self
    }

    /// Sets the last trade date.
    ///
    /// Not sent to TWS; populated only on decoded contracts, so most
    /// callers can leave it unset.
    pub fn last_trade_date(mut self, date: time::Date) -> Self {
        self.last_trade_date = Some(date);
        self
    }

    /// Sets the issuer ID
    ///
    /// Primarily used for bond contracts.
    pub fn issuer_id<S: Into<String>>(mut self, issuer_id: S) -> Self {
        self.issuer_id = Some(issuer_id.into());
        self
    }

    /// Sets the contract description
    ///
    /// Human-readable description of the contract.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Creates a stock contract builder with common defaults
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::stock("AAPL", "SMART", "USD")
    ///     .primary_exchange("NASDAQ")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stock<S: Into<String>>(symbol: S, exchange: S, currency: S) -> Self {
        Self::new()
            .symbol(symbol)
            .security_type(SecurityType::Stock)
            .exchange(exchange)
            .currency(currency)
    }

    /// Creates an option contract builder with common defaults
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::{ContractBuilder, OptionRight};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
    ///     .strike(150.0)
    ///     .right(OptionRight::Call)
    ///     .last_trade_date_or_contract_month("20241220")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn option<S: Into<String>>(symbol: S, exchange: S, currency: S) -> Self {
        Self::new()
            .symbol(symbol)
            .security_type(SecurityType::Option)
            .exchange(exchange)
            .currency(currency)
    }

    /// Creates a futures contract builder with common defaults
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::futures("ES", "CME", "USD")
    ///     .last_trade_date_or_contract_month("202412")
    ///     .multiplier("50")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn futures<S: Into<String>>(symbol: S, exchange: S, currency: S) -> Self {
        Self::new()
            .symbol(symbol)
            .security_type(SecurityType::Future)
            .exchange(exchange)
            .currency(currency)
    }

    /// Creates a continuous futures contract builder with common defaults
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::continuous_futures("ES", "CME", "USD")
    ///     .multiplier("50")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn continuous_futures<S: Into<String>>(symbol: S, exchange: S, currency: S) -> Self {
        Self::new()
            .symbol(symbol)
            .security_type(SecurityType::ContinuousFuture)
            .exchange(exchange)
            .currency(currency)
    }

    /// Creates a crypto contract builder with common defaults
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::crypto("BTC", "PAXOS", "USD").build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn crypto<S: Into<String>>(symbol: S, exchange: S, currency: S) -> Self {
        Self::new()
            .symbol(symbol)
            .security_type(SecurityType::Crypto)
            .exchange(exchange)
            .currency(currency)
    }

    /// Builds the final [Contract] instance with validation
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] if:
    /// - None of symbol, local symbol, contract id or security id (with its
    ///   [`security_id_type`](Self::security_id_type)) is provided
    /// - [`security_type`](Self::security_type) is not set and neither a contract id nor a
    ///   security id identifies the contract. TWS rejects such a lookup with error 321
    /// - Option contracts are missing required fields (strike, right, expiration), or
    ///   futures contracts the contract month, unless a contract id or local symbol
    ///   identifies the contract
    ///
    /// [`Error::Validation`] if:
    /// - An option's strike is not positive and finite ([`ValidationError::InvalidPrice`](crate::builder::ValidationError::InvalidPrice)),
    ///   as [`OptionBuilder::build`](crate::contracts::OptionBuilder::build) checks. A
    ///   strike of `0.0` passes when a non-zero `contract_id` or a `local_symbol` identifies
    ///   the contract: TWS reads it as unset
    /// - The [`last_trade_date_or_contract_month`](Self::last_trade_date_or_contract_month)
    ///   text is neither `YYYYMMDD` nor `YYYYMM` ([`ValidationError::InvalidDate`](crate::builder::ValidationError::InvalidDate))
    pub fn build(self) -> Result<Contract, Error> {
        // 0 is the unset id: the encoder omits it.
        let has_contract_id = self.contract_id.is_some_and(|id| id != 0);
        let has_security_id = self.security_id_type.is_some() && self.security_id.as_deref().is_some_and(|id| !id.is_empty());

        if self.symbol.is_none() && self.local_symbol.is_none() && !has_contract_id && !has_security_id {
            return Err(Error::InvalidArgument(
                "Symbol, local_symbol, contract_id, or security_id is required".into(),
            ));
        }

        // TWS resolves a contract id or security id without a type, and rejects a symbol or
        // local symbol lookup without one (error 321; docs/wire-facts/contracts.md).
        if self.security_type.is_none() && !has_contract_id && !has_security_id {
            return Err(Error::InvalidArgument(
                "security_type is required unless contract_id or security_id identifies the contract".into(),
            ));
        }

        let security_type = self.security_type;

        // A contract id or local symbol names one contract, and TWS resolves it with no
        // expiry, strike or right (docs/wire-facts/contracts.md).
        let identified = has_contract_id || self.local_symbol.as_deref().is_some_and(|symbol| !symbol.is_empty());

        // Validate option-specific requirements
        if matches!(security_type, Some(SecurityType::Option | SecurityType::FuturesOption)) {
            match self.strike {
                None if identified => {}
                None => return Err(Error::InvalidArgument("Strike price is required for options".into())),
                // Strike 0 is TWS's "unset" (docs/wire-facts/contracts.md, ZC futures option).
                Some(strike) if strike == 0.0 && identified => {}
                Some(strike) => {
                    Strike::new(strike)?;
                }
            }

            if !identified && self.right.is_none() {
                return Err(Error::InvalidArgument(
                    "Right (OptionRight::Call or OptionRight::Put) is required for options".into(),
                ));
            }

            if !identified && self.last_trade_date_or_contract_month.is_none() {
                return Err(Error::InvalidArgument("Expiration date is required for options".into()));
            }
        }

        // Validate futures-specific requirements
        if !identified
            && matches!(security_type, Some(SecurityType::Future | SecurityType::FuturesOption))
            && self.last_trade_date_or_contract_month.is_none()
        {
            return Err(Error::InvalidArgument("Contract month is required for futures".into()));
        }

        let expiry = match self.last_trade_date_or_contract_month {
            None => None,
            Some(ExpiryInput::Typed(expiry)) => Some(expiry),
            Some(ExpiryInput::Text(text)) => Some(text.parse::<Expiry>()?),
        };

        Ok(Contract {
            contract_id: self.contract_id.unwrap_or(0),
            symbol: Symbol::from(self.symbol.unwrap_or_default()),
            security_type,
            last_trade_date_or_contract_month: expiry,
            strike: self.strike.unwrap_or(0.0),
            right: self.right,
            multiplier: self.multiplier.unwrap_or_default(),
            exchange: Exchange::from(self.exchange.unwrap_or_default()),
            currency: Currency::from(self.currency.unwrap_or_default()),
            local_symbol: self.local_symbol.unwrap_or_default(),
            primary_exchange: Exchange::from(self.primary_exchange.unwrap_or_default()),
            trading_class: self.trading_class.unwrap_or_default(),
            include_expired: self.include_expired.unwrap_or(false),
            security_id_type: self.security_id_type,
            security_id: self.security_id.unwrap_or_default(),
            combo_legs_description: self.combo_legs_description.unwrap_or_default(),
            combo_legs: self.combo_legs.unwrap_or_default(),
            delta_neutral_contract: self.delta_neutral_contract,
            last_trade_date: self.last_trade_date,
            issuer_id: self.issuer_id.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
#[path = "contract_builder_tests.rs"]
mod tests;
