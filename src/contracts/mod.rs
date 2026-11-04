//! Contract definitions and related functionality for trading instruments.
//!
//! This module provides data structures for representing various financial instruments
//! including stocks, options, futures, and complex securities. It includes contract
//! creation helpers, validation, and conversion utilities.

use std::convert::From;
use std::fmt::Debug;
use std::string::ToString;

use serde::Deserialize;
use serde::Serialize;
use tick_types::TickType;

use crate::encode_option_field;
use crate::ToField;

// Re-export builder and contract types
/// The largest `buffer_limit`.
#[deprecated(since = "5.1.0", note = "use `ibapi::subscriptions::MAX_BUFFER_LIMIT`")]
pub const MAX_BUFFER_LIMIT: usize = crate::subscriptions::MAX_BUFFER_LIMIT;
pub use builders::*;
pub use common::contract_builder::ContractBuilder;
pub use contract_details_builder::ContractDetailsBuilder;
pub use option_chain_builder::OptionChainBuilder;
pub use types::*;

// Common implementation modules
mod common;
mod contract_details_builder;
mod option_chain_builder;

// Internal grouping; their `pub` items are re-exported above
// via `pub use builders::*;` / `pub use types::*;`. Users reach the types as
// `ibapi::contracts::*`, not via these submodule paths.
pub(crate) mod builders;
pub(crate) mod types;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// Tick type constants used in option computations and market data.
pub mod tick_types;

// Models

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
/// SecurityType enumerates available security types
///
/// `Other` preserves a wire value this crate does not model; `Display` writes
/// it back unchanged. Parse a wire string with `s.parse::<SecurityType>()`.
pub enum SecurityType {
    /// Stock (or ETF)
    #[default]
    Stock,
    /// Option
    Option,
    /// Future
    Future,
    /// Continuous Future
    ContinuousFuture,
    /// Index
    Index,
    /// Futures option
    FuturesOption,
    /// Forex pair
    ForexPair,
    /// Combo
    Spread,
    ///  Warrant
    Warrant,
    /// Bond
    Bond,
    /// Commodity
    Commodity,
    /// News
    News,
    /// Mutual fund
    MutualFund,
    /// Crypto currency
    Crypto,
    /// Contract for difference
    CFD,
    /// A security type this crate does not model, holding the raw wire value
    Other(String),
}

impl ToField for Option<SecurityType> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl SecurityType {
    /// Return the IBKR wire string; for `Other` it borrows the raw value.
    pub fn as_str(&self) -> &str {
        match self {
            SecurityType::Stock => "STK",
            SecurityType::Option => "OPT",
            SecurityType::Future => "FUT",
            SecurityType::ContinuousFuture => "CONTFUT",
            SecurityType::Index => "IND",
            SecurityType::FuturesOption => "FOP",
            SecurityType::ForexPair => "CASH",
            SecurityType::Spread => "BAG",
            SecurityType::Warrant => "WAR",
            SecurityType::Bond => "BOND",
            SecurityType::Commodity => "CMDTY",
            SecurityType::News => "NEWS",
            SecurityType::MutualFund => "FUND",
            SecurityType::Crypto => "CRYPTO",
            SecurityType::CFD => "CFD",
            SecurityType::Other(name) => name,
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "STK" => Some(Self::Stock),
            "OPT" => Some(Self::Option),
            "FUT" => Some(Self::Future),
            "CONTFUT" => Some(Self::ContinuousFuture),
            "IND" => Some(Self::Index),
            "FOP" => Some(Self::FuturesOption),
            "CASH" => Some(Self::ForexPair),
            "BAG" => Some(Self::Spread),
            "WAR" => Some(Self::Warrant),
            "BOND" => Some(Self::Bond),
            "CMDTY" => Some(Self::Commodity),
            "NEWS" => Some(Self::News),
            "FUND" => Some(Self::MutualFund),
            "CRYPTO" => Some(Self::Crypto),
            "CFD" => Some(Self::CFD),
            _ => None,
        }
    }
}

impl_wire_enum!(SecurityType, fallback Other);

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Contract describes an instrument's definition.
///
/// Prefer the typed entry points (`Contract::stock`, `Contract::call`, `Contract::put`,
/// `Contract::futures`, `Contract::forex`, `Contract::crypto`, `Contract::index`,
/// `Contract::bond_cusip`, `Contract::bond_isin`, `Contract::spread`) or the
/// field-minimal [`ContractBuilder::new`] — they set the correct field types and
/// sensible defaults for you. A bare struct literal with `..Default::default()` also
/// works when you need full control over individual fields.
///
/// # Examples
///
/// ```
/// use ibapi::contracts::Contract;
///
/// let contract = Contract::stock("AAPL").build();
/// ```
pub struct Contract {
    /// The unique IB contract identifier.
    pub contract_id: i32,
    /// The underlying's asset symbol.
    pub symbol: Symbol,
    /// Type of security (stock, option, future, etc.).
    pub security_type: SecurityType,
    /// The contract's last trading day or contract month (for Options and Futures).
    /// Strings with format YYYYMM will be interpreted as the Contract Month whereas YYYYMMDD will be interpreted as Last Trading Day.
    pub last_trade_date_or_contract_month: String,
    /// The option's strike price.
    pub strike: f64,
    /// Option type (only meaningful when `security_type == SecurityType::Option`).
    /// `None` on non-option contracts. Wire values are `"C"` (Call) and `"P"` (Put).
    pub right: Option<OptionRight>,
    /// The instrument's multiplier (i.e. options, futures).
    pub multiplier: String,
    /// The destination exchange.
    pub exchange: Exchange,
    /// The underlying's currency.
    pub currency: Currency,
    /// The contract's symbol within its primary exchange For options, this will be the OCC symbol.
    pub local_symbol: String,
    /// The contract's primary exchange.
    /// For smart routed contracts, used to define contract in case of ambiguity.
    /// Should be defined as native exchange of contract, e.g. ISLAND for MSFT For exchanges which contain a period in name, will only be part of exchange name prior to period, i.e. ENEXT for ENEXT.BE.
    pub primary_exchange: Exchange,
    /// The trading class name for this contract. Available in TWS contract description window as well. For example, GBL Dec '13 future's trading class is "FGBL".
    pub trading_class: String,
    /// If set to true, contract details requests and historical data queries can be performed pertaining to expired futures contracts. Expired options or other instrument types are not available.
    pub include_expired: bool,
    /// `None` when no external identifier; otherwise paired with [`security_id`](Self::security_id) (e.g. `Some(SecurityIdType::Isin)` with `security_id = "US0378331005"`).
    pub security_id_type: Option<SecurityIdType>,
    /// Identifier of the security type.
    pub security_id: String,
    /// Description of the combo legs.
    pub combo_legs_description: String,
    /// Individual legs composing a combo contract.
    pub combo_legs: Vec<ComboLeg>,
    /// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
    pub delta_neutral_contract: Option<DeltaNeutralContract>,

    /// The last trade date of the contract, returned by the server for derivatives
    /// (OPT/FUT/FOP). `None` for STK/BOND, or when the server value isn't `YYYYMMDD`.
    pub last_trade_date: Option<time::Date>,

    /// Identifier of the issuer for bonds and structured products.
    pub issuer_id: String,
    /// Human-readable description provided by TWS.
    pub description: String,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            contract_id: 0,
            symbol: Symbol::default(),
            security_type: SecurityType::default(),
            last_trade_date_or_contract_month: String::new(),
            strike: 0.0,
            right: None,
            multiplier: String::new(),
            exchange: Exchange::default(), // "SMART"
            currency: Currency::default(),
            local_symbol: String::new(),
            primary_exchange: Exchange::from(""), // Empty, not "SMART"
            trading_class: String::new(),
            include_expired: false,
            security_id_type: None,
            security_id: String::new(),
            combo_legs_description: String::new(),
            combo_legs: Vec::new(),
            delta_neutral_contract: None,
            last_trade_date: None,
            issuer_id: String::new(),
            description: String::new(),
        }
    }
}

impl Contract {
    /// Creates a stock contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, Exchange, Currency};
    ///
    /// // Simple stock
    /// let aapl = Contract::stock("AAPL").build();
    ///
    /// // Stock with customization
    /// let toyota = Contract::stock("7203")
    ///     .on_exchange("TSEJ")
    ///     .in_currency("JPY")
    ///     .build();
    /// ```
    pub fn stock(symbol: impl Into<Symbol>) -> StockBuilder<Symbol> {
        StockBuilder::new(symbol)
    }

    /// Creates a call option contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// let call = Contract::call("AAPL")
    ///     .strike(150.0)
    ///     .expires_on(2024, 12, 20)
    ///     .build();
    /// ```
    pub fn call(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder::call(symbol)
    }

    /// Creates a put option contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// let put = Contract::put("SPY")
    ///     .strike(450.0)
    ///     .expires_on(2024, 12, 20)
    ///     .build();
    /// ```
    pub fn put(symbol: impl Into<Symbol>) -> OptionBuilder<Symbol, Missing, Missing> {
        OptionBuilder::put(symbol)
    }

    /// Creates a futures contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, ContractMonth};
    ///
    /// let es = Contract::futures("ES")
    ///     .expires_in(ContractMonth::new(2024, 3))
    ///     .build();
    /// ```
    ///
    /// For an open `contract_details` query, end with [`any_month`](FuturesBuilder::any_month)
    /// instead of a specific month. A month *terminal* is still required — calling `build()`
    /// straight after `futures(..)` does not compile:
    ///
    /// ```compile_fail,E0599
    /// use ibapi::contracts::Contract;
    ///
    /// let es = Contract::futures("ES").build(); // no `build` without a month terminal
    /// ```
    pub fn futures(symbol: impl Into<Symbol>) -> FuturesBuilder<Symbol, Missing> {
        FuturesBuilder::new(symbol)
    }

    /// Creates a continuous futures contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, ContractMonth};
    ///
    /// let es = Contract::continuous_futures("ES")
    ///     .on_exchange("CME")
    ///     .build();
    /// ```
    pub fn continuous_futures(symbol: impl Into<Symbol>) -> ContinuousFuturesBuilder<Symbol> {
        ContinuousFuturesBuilder::new(symbol)
    }

    /// Creates a forex contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, Currency};
    ///
    /// let eur_usd = Contract::forex("EUR", "USD").build();
    /// ```
    pub fn forex(base: impl Into<Currency>, quote: impl Into<Currency>) -> ForexBuilder {
        ForexBuilder::new(base, quote)
    }

    /// Creates a crypto contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// let btc = Contract::crypto("BTC").build();
    /// ```
    pub fn crypto(symbol: impl Into<Symbol>) -> CryptoBuilder {
        CryptoBuilder::new(symbol)
    }

    /// Creates an index contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// let spx = Contract::index("SPX");
    /// ```
    pub fn index(symbol: &str) -> Contract {
        let (exchange, currency): (Exchange, Currency) = match symbol {
            "SPX" | "NDX" | "DJI" | "RUT" => ("CBOE".into(), "USD".into()),
            "DAX" => ("EUREX".into(), "EUR".into()),
            "FTSE" => ("LSE".into(), "GBP".into()),
            _ => ("SMART".into(), "USD".into()),
        };

        Contract {
            symbol: Symbol::new(symbol),
            security_type: SecurityType::Index,
            exchange,
            currency,
            ..Default::default()
        }
    }

    /// Create a bond contract with CUSIP identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// // US Treasury bond by CUSIP
    /// let bond = Contract::bond_cusip("912810RN0");
    /// ```
    pub fn bond_cusip(cusip: impl Into<String>) -> Contract {
        Contract::bond(BondIdentifier::Cusip(Cusip::new(cusip)))
    }

    /// Create a bond contract with ISIN identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// // European bond by ISIN
    /// let bond = Contract::bond_isin("DE0001102309");
    /// ```
    pub fn bond_isin(isin: impl Into<String>) -> Contract {
        Contract::bond(BondIdentifier::Isin(Isin::new(isin)))
    }

    /// Create a bond contract with CUSIP or ISIN identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, BondIdentifier, Cusip, Isin};
    ///
    /// // US Treasury bond by CUSIP
    /// let bond = Contract::bond(BondIdentifier::Cusip(Cusip::new("912810RN0")));
    ///
    /// // European bond by ISIN
    /// let bond = Contract::bond(BondIdentifier::Isin(Isin::new("DE0001102309")));
    /// ```
    pub fn bond(identifier: BondIdentifier) -> Contract {
        let (security_id_type, currency, security_id) = match identifier {
            BondIdentifier::Cusip(cusip) => (SecurityIdType::Cusip, "USD", cusip.0),
            BondIdentifier::Isin(isin) => (SecurityIdType::Isin, isin.implied_currency(), isin.0),
        };

        Contract {
            symbol: Symbol::new(security_id.clone()),
            security_type: SecurityType::Bond,
            security_id_type: Some(security_id_type),
            security_id,
            exchange: "SMART".into(),
            currency: currency.into(),
            ..Default::default()
        }
    }

    /// Creates a spread/combo contract builder for the combo `symbol`.
    ///
    /// The symbol is usually the underlying. For a combo across underlyings, TWS's samples join
    /// them with a comma (`"IBKR,MCD"`). TWS rejects a combo without one with 321, so
    /// [`build`](SpreadBuilder::build) rejects an empty symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// let spread = Contract::spread("AAPL").long_calendar(12345, 67890).build()?;
    /// assert_eq!(spread.symbol.to_string(), "AAPL");
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn spread(symbol: impl Into<Symbol>) -> SpreadBuilder {
        SpreadBuilder::new(symbol)
    }

    /// Creates a news contract from the specified provider code.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, Symbol, Exchange};
    ///
    /// let news = Contract::news("BRFG");
    /// assert_eq!(news.symbol, Symbol::from("BRFG:BRFG_ALL"));
    /// assert_eq!(news.exchange, Exchange::from("BRFG"));
    /// ```
    pub fn news(provider_code: &str) -> Contract {
        Contract {
            symbol: Symbol::new(format!("{provider_code}:{provider_code}_ALL")),
            security_type: SecurityType::News,
            exchange: Exchange::from(provider_code),
            ..Default::default()
        }
    }

    /// Creates a simple option contract from the specified parameters.
    /// Currency defaults to USD and exchange defaults to SMART.
    ///
    /// For new code, prefer the `Contract::call()` / `Contract::put()` builders.
    ///
    /// # Arguments
    /// * `symbol` - Symbol of the underlying asset
    /// * `expiration_date` - Expiration date of option contract (YYYYMMDD)
    /// * `strike` - Strike price of the option contract
    /// * `right` - `OptionRight::Call` or `OptionRight::Put`
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, OptionRight, Symbol};
    ///
    /// let call = Contract::option("AAPL", "20240119", 150.0, OptionRight::Call);
    /// assert_eq!(call.symbol, Symbol::from("AAPL"));
    /// assert_eq!(call.strike, 150.0);
    /// assert_eq!(call.right, Some(OptionRight::Call));
    /// ```
    // Legacy all-required convenience kept flat; `call()` / `put()` are the builder path
    // (docs/rules/style/param-budget.md).
    pub fn option(symbol: &str, expiration_date: &str, strike: f64, right: OptionRight) -> Contract {
        Contract {
            symbol: symbol.into(),
            security_type: SecurityType::Option,
            exchange: "SMART".into(),
            currency: "USD".into(),
            last_trade_date_or_contract_month: expiration_date.into(),
            strike,
            right: Some(right),
            ..Default::default()
        }
    }

    /// Returns true if this contract represents a bag/combo order.
    pub fn is_bag(&self) -> bool {
        self.security_type == SecurityType::Spread
    }
}

/// A single component within a combo contract.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComboLeg {
    /// The Contract's IB's unique id.
    pub contract_id: i32,
    /// Select the relative number of contracts for the leg you are constructing. To help determine the ratio for a specific combination order, refer to the Interactive Analytics section of the User's Guide.
    pub ratio: i32,
    /// The side of the leg (`Buy` / `Sell` / `SellShort`). Combo legs do not
    /// accept `SLONG` — for long-undelivered semantics use the outer
    /// `Order.action: Action::SellLong`.
    pub action: LegAction,
    /// The destination exchange to which the order will be routed.
    pub exchange: String,
    /// Specifies whether an order is an open or closing order.
    /// For institutional customers to determine if this order is to open or close a position.
    pub open_close: ComboLegOpenClose,
    /// For stock legs when doing short selling. Set to 1 = clearing broker, 2 = third party.
    pub short_sale_slot: i32,
    /// When ShortSaleSlot is 2, this field shall contain the designated location.
    pub designated_location: String,
    /// Regulation SHO code for the leg (0 = none).
    pub exempt_code: i32,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
/// OpenClose specifies whether an order is an open or closing order.
pub enum ComboLegOpenClose {
    /// Same as the parent security. This is the only option for retail customers. Wire code `0`.
    #[default]
    Same,
    /// Open. This value is only valid for institutional customers. Wire code `1`.
    Open,
    /// Close. This value is only valid for institutional customers. Wire code `2`.
    Close,
    /// Any other code, including IB's own "unknown" position (`3`), carried as received and
    /// sent back unchanged.
    Unknown(i32),
}

impl From<ComboLegOpenClose> for i32 {
    fn from(value: ComboLegOpenClose) -> i32 {
        match value {
            ComboLegOpenClose::Same => 0,
            ComboLegOpenClose::Open => 1,
            ComboLegOpenClose::Close => 2,
            ComboLegOpenClose::Unknown(code) => code,
        }
    }
}

impl From<i32> for ComboLegOpenClose {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Same,
            1 => Self::Open,
            2 => Self::Close,
            code => Self::Unknown(code),
        }
    }
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Delta and underlying price for Delta-Neutral combo orders.
/// Underlying (STK or FUT), delta and underlying price goes into this attribute.
pub struct DeltaNeutralContract {
    /// The unique contract identifier specifying the security. Used for Delta-Neutral Combo contracts.
    pub contract_id: i32,
    /// The underlying stock or future delta. Used for Delta-Neutral Combo contracts.
    pub delta: f64,
    /// The price of the underlying. Used for Delta-Neutral Combo contracts.
    pub price: f64,
}

/// ContractDetails provides extended contract details.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractDetails {
    /// A fully-defined Contract object.
    pub contract: Contract,
    /// The market name for this product.
    pub market_name: String,
    /// The minimum allowed price variation. Note that many securities vary their minimum tick size according to their price. This value will only show the smallest of the different minimum tick sizes regardless of the product's price. Full information about the minimum increment price structure can be obtained with the reqMarketRule function or the IB Contract and Security Search site.
    pub min_tick: f64,
    /// Allows execution and strike prices to be reported consistently with market data, historical data and the order price, i.e. Z on LIFFE is reported in Index points and not GBP. In TWS versions prior to 972, the price magnifier is used in defining future option strike prices (e.g. in the API the strike is specified in dollars, but in TWS it is specified in cents). In TWS versions 972 and higher, the price magnifier is not used in defining futures option strike prices so they are consistent in TWS and the API.
    pub price_magnifier: i32,
    /// Supported order types for this product.
    pub order_types: Vec<String>,
    /// Valid exchange fields when placing an order for this contract.
    /// The list of exchanges will is provided in the same order as the corresponding MarketRuleIds list.
    pub valid_exchanges: Vec<String>,
    /// For derivatives, the contract ID (conID) of the underlying instrument.
    pub under_contract_id: i32,
    /// Descriptive name of the product.
    pub long_name: String,
    /// Typically the contract month of the underlying for a Future contract.
    pub contract_month: String,
    /// The industry classification of the underlying/product. For example, Financial.
    pub industry: String,
    /// The industry category of the underlying. For example, InvestmentSvc.
    pub category: String,
    /// The industry subcategory of the underlying. For example, Brokerage.
    pub subcategory: String,
    /// The time zone for the trading hours of the product. For example, EST.
    pub time_zone_id: String,
    /// The trading hours of the product. This value will contain the trading hours of the current day as well as the next's. For example, 20090507:0700-1830,1830-2330;20090508:CLOSED. In TWS versions 965+ there is an option in the Global Configuration API settings to return 1 month of trading hours. In TWS version 970+, the format includes the date of the closing time to clarify potential ambiguity, ex: 20180323:0400-20180323:2000;20180326:0400-20180326:2000 The trading hours will correspond to the hours for the product on the associated exchange. The same instrument can have different hours on different exchanges.
    pub trading_hours: Vec<String>,
    /// The liquid hours of the product. This value will contain the liquid hours (regular trading hours) of the contract on the specified exchange. Format for TWS versions until 969: 20090507:0700-1830,1830-2330;20090508:CLOSED. In TWS versions 965+ there is an option in the Global Configuration API settings to return 1 month of trading hours. In TWS v970 and above, the format includes the date of the closing time to clarify potential ambiguity, e.g. 20180323:0930-20180323:1600;20180326:0930-20180326:1600.
    pub liquid_hours: Vec<String>,
    /// Contains the Economic Value Rule name and the respective optional argument. The two values should be separated by a colon. For example, aussieBond:YearsToExpiration=3. When the optional argument is not present, the first value will be followed by a colon.
    pub ev_rule: String,
    /// Tells you approximately how much the market value of a contract would change if the price were to change by 1. It cannot be used to get market value by multiplying the price by the approximate multiplier.
    pub ev_multiplier: f64,
    /// Aggregated group Indicates the smart-routing group to which a contract belongs. contracts which cannot be smart-routed have aggGroup = -1.
    pub agg_group: i32,
    /// A list of contract identifiers that the customer is allowed to view. CUSIP/ISIN/etc. For US stocks, receiving the ISIN requires the CUSIP market data subscription. For Bonds, the CUSIP or ISIN is input directly into the symbol field of the Contract class.
    pub sec_id_list: Vec<TagValue>,
    /// For derivatives, the symbol of the underlying contract.
    pub under_symbol: String,
    /// For derivatives, returns the underlying security type.
    pub under_security_type: String,
    /// The list of market rule IDs separated by comma Market rule IDs can be used to determine the minimum price increment at a given price.
    pub market_rule_ids: Vec<String>,
    /// Real expiration date. Requires TWS 968+ and API v973.04+. Python API specifically requires API v973.06+.
    pub real_expiration_date: String,
    /// Last trade time.
    pub last_trade_time: String,
    /// Stock type.
    pub stock_type: String,
    /// How the product settles on expiry: "Physical" or "Cash". Empty when TWS omits it.
    pub settlement_method: String,
    /// The nine-character bond CUSIP. For Bonds only. Receiving CUSIPs requires a CUSIP market data subscription.
    pub cusip: String,
    /// Identifies the credit rating of the issuer. This field is not currently available from the TWS API. For Bonds only. A higher credit rating generally indicates a less risky investment. Bond ratings are from Moody's and S&P respectively. Not currently implemented due to bond market data restrictions.
    pub ratings: String,
    /// A description string containing further descriptive information about the bond. For Bonds only.
    pub desc_append: String,
    /// The type of bond, such as "CORP.".
    pub bond_type: String,
    /// The type of bond coupon. This field is currently not available from the TWS API. For Bonds only.
    pub coupon_type: String,
    /// If true, the bond can be called by the issuer under certain conditions. This field is currently not available from the TWS API. For Bonds only.
    pub callable: bool,
    /// Values are True or False. If true, the bond can be sold back to the issuer under certain conditions. This field is currently not available from the TWS API. For Bonds only.
    pub putable: bool,
    /// The interest rate used to calculate the amount you will receive in interest payments over the course of the year. This field is currently not available from the TWS API. For Bonds only.
    pub coupon: f64,
    /// Values are True or False. If true, the bond can be converted to stock under certain conditions. This field is currently not available from the TWS API. For Bonds only.
    pub convertible: bool,
    /// The date on which the issuer must repay the face value of the bond. This field is currently not available from the TWS API. For Bonds only. Not currently implemented due to bond market data restrictions.
    pub maturity: String,
    /// The date the bond was issued. This field is currently not available from the TWS API. For Bonds only. Not currently implemented due to bond market data restrictions.
    pub issue_date: String,
    /// Only if bond has embedded options. This field is currently not available from the TWS API. Refers to callable bonds and puttable bonds. Available in TWS description window for bonds.
    pub next_option_date: String,
    /// Type of embedded option. This field is currently not available from the TWS API. Only if bond has embedded options.
    pub next_option_type: String,
    /// Only if bond has embedded options. This field is currently not available from the TWS API. For Bonds only.
    pub next_option_partial: bool,
    /// If populated for the bond in IB's database. For Bonds only.
    pub notes: String,
    /// Order's minimal size.
    ///
    /// `None` when TWS reports no size rules for the contract (field absent,
    /// empty, or an "unset" sentinel); `Some(0.0)` is a real zero.
    pub min_size: Option<f64>,
    /// Order's size increment.
    ///
    /// `None` when TWS reports no size rules for the contract; see [`Self::min_size`].
    pub size_increment: Option<f64>,
    /// Order's suggested size increment.
    ///
    /// `None` when TWS reports no size rules for the contract; see [`Self::min_size`].
    pub suggested_size_increment: Option<f64>,

    // Fund fields (populated only for FUND security type)
    /// Fund name.
    pub fund_name: String,
    /// Fund family.
    pub fund_family: String,
    /// Fund type.
    pub fund_type: String,
    /// Fund front load.
    pub fund_front_load: String,
    /// Fund back load.
    pub fund_back_load: String,
    /// Fund back load time interval.
    pub fund_back_load_time_interval: String,
    /// Fund management fee.
    pub fund_management_fee: String,
    /// Whether the fund is closed.
    pub fund_closed: bool,
    /// Whether the fund is closed for new investors.
    pub fund_closed_for_new_investors: bool,
    /// Whether the fund is closed for new money.
    pub fund_closed_for_new_money: bool,
    /// Fund notify amount.
    pub fund_notify_amount: String,
    /// Fund minimum initial purchase.
    pub fund_minimum_initial_purchase: String,
    /// Fund subsequent minimum purchase.
    pub fund_subsequent_minimum_purchase: String,
    /// Fund blue sky states.
    pub fund_blue_sky_states: String,
    /// Fund blue sky territories.
    pub fund_blue_sky_territories: String,
    /// Fund distribution policy indicator; `None` when not sent.
    pub fund_distribution_policy_indicator: Option<FundDistributionPolicyIndicator>,
    /// Fund asset type; `None` when not sent.
    pub fund_asset_type: Option<FundAssetType>,

    /// Ineligibility reasons for the contract.
    pub ineligibility_reasons: Vec<IneligibilityReason>,
}

/// Fund distribution policy indicator (`fundDistributionPolicyIndicator`).
///
/// No `Default`: [`ContractDetails::fund_distribution_policy_indicator`] is
/// `None` when TWS sends no value.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FundDistributionPolicyIndicator {
    /// Accumulation fund. Wire value `N`.
    AccumulationFund,
    /// Income fund. Wire value `Y`.
    IncomeFund,
    /// Value not modeled by this version of the API, carried as received.
    Unknown(String),
}

impl FundDistributionPolicyIndicator {
    /// Return the IBKR wire string; for [`Unknown`](Self::Unknown), the raw value as received.
    pub fn as_str(&self) -> &str {
        match self {
            Self::AccumulationFund => "N",
            Self::IncomeFund => "Y",
            Self::Unknown(raw) => raw,
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "N" => Some(Self::AccumulationFund),
            "Y" => Some(Self::IncomeFund),
            _ => None,
        }
    }
}

// Open: decoded on every inbound `ContractDetails`. An unrecognized value must
// keep its code rather than fail the response or read as unset.
impl_wire_enum!(FundDistributionPolicyIndicator, fallback Unknown);

/// Fund asset type (`fundAssetType`).
///
/// No `Default`: [`ContractDetails::fund_asset_type`] is `None` when TWS sends
/// no value.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FundAssetType {
    /// Other asset types. Wire value `000`.
    Others,
    /// Money market fund. Wire value `001`.
    MoneyMarket,
    /// Fixed income fund. Wire value `002`.
    FixedIncome,
    /// Multi-asset fund. Wire value `003`.
    MultiAsset,
    /// Equity fund. Wire value `004`.
    Equity,
    /// Sector fund. Wire value `005`.
    Sector,
    /// Guaranteed fund. Wire value `006`.
    Guaranteed,
    /// Alternative fund. Wire value `007`.
    Alternative,
    /// Value not modeled by this version of the API, carried as received.
    Unknown(String),
}

impl FundAssetType {
    /// Return the IBKR wire string; for [`Unknown`](Self::Unknown), the raw value as received.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Others => "000",
            Self::MoneyMarket => "001",
            Self::FixedIncome => "002",
            Self::MultiAsset => "003",
            Self::Equity => "004",
            Self::Sector => "005",
            Self::Guaranteed => "006",
            Self::Alternative => "007",
            Self::Unknown(raw) => raw,
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "000" => Some(Self::Others),
            "001" => Some(Self::MoneyMarket),
            "002" => Some(Self::FixedIncome),
            "003" => Some(Self::MultiAsset),
            "004" => Some(Self::Equity),
            "005" => Some(Self::Sector),
            "006" => Some(Self::Guaranteed),
            "007" => Some(Self::Alternative),
            _ => None,
        }
    }
}

// Open: see `FundDistributionPolicyIndicator`.
impl_wire_enum!(FundAssetType, fallback Unknown);

/// Reason why a contract is ineligible for trading.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IneligibilityReason {
    /// Reason identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
}

/// TagValue is a convenience struct to define key-value pairs.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct TagValue {
    /// Name of the tag.
    pub tag: String,
    /// String representation of the value.
    pub value: String,
}

impl ToField for Vec<TagValue> {
    fn to_field(&self) -> String {
        let mut values = Vec::new();
        for tag_value in self {
            values.push(format!("{}={};", tag_value.tag, tag_value.value))
        }
        values.concat()
    }
}

/// Receives option specific market data.
/// TWS’s options model volatility, prices, and deltas, along with the present value of dividends expected on that options underlier.
///
/// A value is `None` when TWS has not computed it yet.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OptionComputation {
    /// Specifies the type of option computation.
    pub field: TickType,
    /// 0 – return based, 1- price based.
    pub tick_attribute: Option<i32>,
    /// The implied volatility calculated by the TWS option modeler, using the specified tick type value.
    pub implied_volatility: Option<f64>,
    /// The option delta value.
    pub delta: Option<f64>,
    /// The option price.
    pub option_price: Option<f64>,
    /// The present value of dividends expected on the option’s underlying.
    pub present_value_dividend: Option<f64>,
    /// The option gamma value.
    pub gamma: Option<f64>,
    /// The option vega value.
    pub vega: Option<f64>,
    /// The option theta value.
    pub theta: Option<f64>,
    /// The price of the underlying.
    pub underlying_price: Option<f64>,
}

/// Option chain metadata for a specific underlying security.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default)]
pub struct OptionChain {
    /// The contract ID of the underlying security.
    pub underlying_contract_id: i32,
    /// The option trading class.
    pub trading_class: String,
    /// The option multiplier.
    pub multiplier: String,
    /// Exchange for which the derivative is hosted.
    pub exchange: String,
    /// A list of the expiries for the options of this underlying on this exchange.
    pub expirations: Vec<String>,
    /// A list of the possible strikes for options of this underlying on this exchange.
    pub strikes: Vec<f64>,
}

// === API ===

/// Contract data and list of derivative security types
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug)]
pub struct ContractDescription {
    /// Fully qualified contract metadata.
    pub contract: Contract,
    /// Derivative security types available for the contract.
    pub derivative_security_types: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default)]
/// Minimum price increment structure for a particular market rule ID.
pub struct MarketRule {
    /// Market Rule ID requested.
    pub market_rule_id: i32,
    /// Returns the available price increments based on the market rule.
    pub price_increments: Vec<PriceIncrement>,
}

/// Price ladder entry describing the minimum tick between price bands.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default)]
pub struct PriceIncrement {
    /// Lower inclusive edge where the increment applies.
    pub low_edge: f64,
    /// Minimum tick size within this price band.
    pub increment: f64,
}

/// One contributing exchange behind a consolidated BBO feed.
///
/// Returned by `Client::smart_components` for a given BBO exchange token
/// (e.g. `"a6"`). Each entry maps a bit position in the consolidated
/// quote to the underlying exchange and its single-letter abbreviation.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SmartComponent {
    /// Bit position in the consolidated quote.
    pub bit_number: i32,
    /// Full exchange name (e.g. `"NASDAQ"`).
    pub exchange: String,
    /// Single-letter exchange abbreviation (e.g. `"P"` for Pacific).
    pub exchange_letter: String,
}

// ContractBuilder is deprecated - use the new builder methods on Contract instead
// e.g., Contract::stock(), Contract::call(), Contract::put(), etc.

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
