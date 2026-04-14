use super::super::{ComboLeg, Contract, DeltaNeutralContract, SecurityType};
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
/// use ibapi::contracts::{ContractBuilder, SecurityType};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
///     .strike(150.0)
///     .right("C")  // Call option
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
/// let contract = ContractBuilder::futures("ES", "GLOBEX", "USD")
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
/// - Symbol is always required
/// - Option contracts require strike, right (P/C), and expiration date
/// - Futures contracts require contract month
/// - Strike prices cannot be negative
/// - Option rights must be "P" or "C" (case insensitive)
#[derive(Clone, Debug, Default)]
pub struct ContractBuilder {
    pub(crate) contract_id: Option<i32>,
    pub(crate) symbol: Option<String>,
    pub(crate) security_type: Option<SecurityType>,
    pub(crate) last_trade_date_or_contract_month: Option<String>,
    pub(crate) strike: Option<f64>,
    pub(crate) right: Option<String>,
    pub(crate) multiplier: Option<String>,
    pub(crate) exchange: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) local_symbol: Option<String>,
    pub(crate) primary_exchange: Option<String>,
    pub(crate) trading_class: Option<String>,
    pub(crate) include_expired: Option<bool>,
    pub(crate) security_id_type: Option<String>,
    pub(crate) security_id: Option<String>,
    pub(crate) combo_legs_description: Option<String>,
    pub(crate) combo_legs: Option<Vec<ComboLeg>>,
    pub(crate) delta_neutral_contract: Option<DeltaNeutralContract>,
    pub(crate) issuer_id: Option<String>,
    pub(crate) description: Option<String>,
}

impl ContractBuilder {
    /// Creates a new [ContractBuilder]
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
    pub fn contract_id(mut self, contract_id: i32) -> Self {
        self.contract_id = Some(contract_id);
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
    /// See [SecurityType] for available options.
    pub fn security_type(mut self, security_type: SecurityType) -> Self {
        self.security_type = Some(security_type);
        self
    }

    /// Sets the last trade date or contract month
    ///
    /// For futures and options, this field is required:
    /// - Format YYYYMM for contract month (e.g., "202412")
    /// - Format YYYYMMDD for specific expiration date (e.g., "20241220")
    pub fn last_trade_date_or_contract_month<S: Into<String>>(mut self, date: S) -> Self {
        self.last_trade_date_or_contract_month = Some(date.into());
        self
    }

    /// Sets the option's strike price
    ///
    /// Required for option contracts. Must be a positive value.
    ///
    /// # Examples
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
    ///     .strike(150.0)
    ///     .right("C")
    ///     .last_trade_date_or_contract_month("20241220")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn strike(mut self, strike: f64) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Sets the option right (Put or Call)
    ///
    /// Required for option contracts. Valid values are:
    /// - "P" or "p" for Put options
    /// - "C" or "c" for Call options
    pub fn right<S: Into<String>>(mut self, right: S) -> Self {
        self.right = Some(right.into());
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
    /// - "GLOBEX", "NYMEX", "CME" for futures
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

    /// Sets the security ID type
    ///
    /// Examples: "ISIN", "CUSIP", "SEDOL", "RIC"
    pub fn security_id_type<S: Into<String>>(mut self, security_id_type: S) -> Self {
        self.security_id_type = Some(security_id_type.into());
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
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::option("AAPL", "SMART", "USD")
    ///     .strike(150.0)
    ///     .right("C")
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
    /// let contract = ContractBuilder::futures("ES", "GLOBEX", "USD")
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
    /// let contract = ContractBuilder::continuous_futures("ES", "GLOBEX", "USD")
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
    /// Returns an error if:
    /// - Symbol is not provided
    /// - Option contracts are missing required fields (strike, right, expiration)
    /// - Futures contracts are missing contract month
    /// - Strike price is negative
    /// - Option right is not "P" or "C"
    pub fn build(self) -> Result<Contract, Error> {
        // Symbol is required unless local_symbol or contract_id is provided
        if self.symbol.is_none() && self.local_symbol.is_none() && self.contract_id.is_none() {
            return Err(Error::Simple("Symbol, local_symbol, or contract_id is required".into()));
        }

        let security_type = self.security_type.clone().unwrap_or_default();

        // Validate option-specific requirements
        if security_type == SecurityType::Option || security_type == SecurityType::FuturesOption {
            if self.strike.is_none() {
                return Err(Error::Simple("Strike price is required for options".into()));
            }

            if let Some(strike) = self.strike {
                if strike < 0.0 {
                    return Err(Error::Simple("Strike price cannot be negative".into()));
                }
            }

            if self.right.is_none() {
                return Err(Error::Simple("Right (P for PUT or C for CALL) is required for options".into()));
            }

            if let Some(ref right) = self.right {
                let right_upper = right.to_uppercase();
                if right_upper != "P" && right_upper != "C" {
                    return Err(Error::Simple("Option right must be P for PUT or C for CALL".into()));
                }
            }

            if self.last_trade_date_or_contract_month.is_none() {
                return Err(Error::Simple("Expiration date is required for options".into()));
            }
        }

        // Validate futures-specific requirements
        if (security_type == SecurityType::Future || security_type == SecurityType::FuturesOption) && self.last_trade_date_or_contract_month.is_none()
        {
            return Err(Error::Simple("Contract month is required for futures".into()));
        }

        Ok(Contract {
            contract_id: self.contract_id.unwrap_or(0),
            symbol: self.symbol.unwrap_or_default(),
            security_type,
            last_trade_date_or_contract_month: self.last_trade_date_or_contract_month.unwrap_or_default(),
            strike: self.strike.unwrap_or(0.0),
            right: self.right.unwrap_or_default(),
            multiplier: self.multiplier.unwrap_or_default(),
            exchange: self.exchange.unwrap_or_default(),
            currency: self.currency.unwrap_or_default(),
            local_symbol: self.local_symbol.unwrap_or_default(),
            primary_exchange: self.primary_exchange.unwrap_or_default(),
            trading_class: self.trading_class.unwrap_or_default(),
            include_expired: self.include_expired.unwrap_or(false),
            security_id_type: self.security_id_type.unwrap_or_default(),
            security_id: self.security_id.unwrap_or_default(),
            combo_legs_description: self.combo_legs_description.unwrap_or_default(),
            combo_legs: self.combo_legs.unwrap_or_default(),
            delta_neutral_contract: self.delta_neutral_contract,
            issuer_id: self.issuer_id.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
        })
    }
}


#[cfg(test)]
mod tests;
