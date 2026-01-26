//! Contract definitions and related functionality for trading instruments.
//!
//! This module provides data structures for representing various financial instruments
//! including stocks, options, futures, and complex securities. It includes contract
//! creation helpers, validation, and conversion utilities.

use std::convert::From;
use std::fmt::Debug;
use std::string::ToString;

use log::warn;
use serde::Deserialize;
use serde::Serialize;
use tick_types::TickType;

use crate::encode_option_field;
use crate::messages::RequestMessage;
use crate::messages::ResponseMessage;
use crate::{Error, ToField};

// Re-export V2 API types
pub use builders::*;
pub use types::*;

// Common implementation modules
mod common;

// V2 API modules
pub mod builders;
pub mod types;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// Tick type constants used in option computations and market data.
pub mod tick_types;

// Models

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
/// SecurityType enumerates available security types
pub enum SecurityType {
    /// Stock (or ETF)
    #[default]
    Stock,
    /// Option
    Option,
    /// Future
    Future,
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
    /// Other
    Other(String),
}

impl ToField for SecurityType {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<SecurityType> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl std::fmt::Display for SecurityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityType::Stock => write!(f, "STK"),
            SecurityType::Option => write!(f, "OPT"),
            SecurityType::Future => write!(f, "FUT"),
            SecurityType::Index => write!(f, "IND"),
            SecurityType::FuturesOption => write!(f, "FOP"),
            SecurityType::ForexPair => write!(f, "CASH"),
            SecurityType::Spread => write!(f, "BAG"),
            SecurityType::Warrant => write!(f, "WAR"),
            SecurityType::Bond => write!(f, "BOND"),
            SecurityType::Commodity => write!(f, "CMDTY"),
            SecurityType::News => write!(f, "NEWS"),
            SecurityType::MutualFund => write!(f, "FUND"),
            SecurityType::Crypto => write!(f, "CRYPTO"),
            SecurityType::CFD => write!(f, "CFD"),
            SecurityType::Other(name) => write!(f, "{name}"),
        }
    }
}

impl SecurityType {
    /// Create a [SecurityType] from an IB symbol code (e.g. `STK`, `OPT`).
    pub fn from(name: &str) -> SecurityType {
        match name {
            "STK" => SecurityType::Stock,
            "OPT" => SecurityType::Option,
            "FUT" => SecurityType::Future,
            "IND" => SecurityType::Index,
            "FOP" => SecurityType::FuturesOption,
            "CASH" => SecurityType::ForexPair,
            "BAG" => SecurityType::Spread,
            "WAR" => SecurityType::Warrant,
            "BOND" => SecurityType::Bond,
            "CMDTY" => SecurityType::Commodity,
            "NEWS" => SecurityType::News,
            "FUND" => SecurityType::MutualFund,
            "CRYPTO" => SecurityType::Crypto,
            "CFD" => SecurityType::CFD,
            other => {
                warn!("Unknown security type: {other}. Defaulting to Other");
                SecurityType::Other(other.to_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Contract describes an instrument's definition
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
    /// Either Put or Call (i.e. Options). Valid values are P, PUT, C, CALL.
    pub right: String,
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
    /// Security's identifier when querying contract's details or placing orders ISIN - Example: Apple: US0378331005 CUSIP - Example: Apple: 037833100.
    pub security_id_type: String,
    /// Identifier of the security type.
    pub security_id: String,
    /// Description of the combo legs.
    pub combo_legs_description: String,
    /// Individual legs composing a combo contract.
    pub combo_legs: Vec<ComboLeg>,
    /// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
    pub delta_neutral_contract: Option<DeltaNeutralContract>,

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
            right: String::new(),
            multiplier: String::new(),
            exchange: Exchange::default(), // "SMART"
            currency: Currency::default(),
            local_symbol: String::new(),
            primary_exchange: Exchange::from(""), // Empty, not "SMART"
            trading_class: String::new(),
            include_expired: false,
            security_id_type: String::new(),
            security_id: String::new(),
            combo_legs_description: String::new(),
            combo_legs: Vec::new(),
            delta_neutral_contract: None,
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
    pub fn futures(symbol: impl Into<Symbol>) -> FuturesBuilder<Symbol, Missing> {
        FuturesBuilder::new(symbol)
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
    /// # Example
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// // US Treasury bond by CUSIP
    /// let bond = Contract::bond_cusip("912810RN0");
    /// ```
    pub fn bond_cusip(cusip: impl Into<String>) -> Contract {
        let cusip_str = cusip.into();
        Contract {
            symbol: Symbol::new(cusip_str.clone()),
            security_type: SecurityType::Bond,
            security_id_type: "CUSIP".to_string(),
            security_id: cusip_str,
            exchange: "SMART".into(),
            currency: "USD".into(),
            ..Default::default()
        }
    }

    /// Create a bond contract with ISIN identifier
    ///
    /// # Example
    /// ```
    /// use ibapi::contracts::Contract;
    ///
    /// // European bond by ISIN
    /// let bond = Contract::bond_isin("DE0001102309");
    /// ```
    pub fn bond_isin(isin: impl Into<String>) -> Contract {
        let isin_str = isin.into();
        // Determine currency from ISIN country code (first 2 chars)
        let currency = match isin_str.get(0..2) {
            Some("US") | Some("CA") => "USD",
            Some("GB") => "GBP",
            Some("JP") => "JPY",
            Some("CH") => "CHF",
            Some("AU") => "AUD",
            Some("DE") | Some("FR") | Some("IT") | Some("ES") | Some("NL") | Some("BE") => "EUR",
            _ => "USD", // Default to USD
        };

        Contract {
            symbol: Symbol::new(isin_str.clone()),
            security_type: SecurityType::Bond,
            security_id_type: "ISIN".to_string(),
            security_id: isin_str,
            exchange: "SMART".into(),
            currency: currency.into(),
            ..Default::default()
        }
    }

    /// Create a bond contract with CUSIP or ISIN identifier
    ///
    /// # Example
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
        match identifier {
            BondIdentifier::Cusip(cusip) => Contract {
                symbol: Symbol::new(cusip.to_string()),
                security_type: SecurityType::Bond,
                security_id_type: "CUSIP".to_string(),
                security_id: cusip.to_string(),
                exchange: "SMART".into(),
                currency: "USD".into(),
                ..Default::default()
            },
            BondIdentifier::Isin(isin) => {
                // Determine currency from ISIN country code (first 2 chars)
                let currency = match isin.as_str().get(0..2) {
                    Some("US") | Some("CA") => "USD",
                    Some("GB") => "GBP",
                    Some("JP") => "JPY",
                    Some("CH") => "CHF",
                    Some("AU") => "AUD",
                    Some("DE") | Some("FR") | Some("IT") | Some("ES") | Some("NL") | Some("BE") => "EUR",
                    _ => "USD", // Default to USD
                };

                Contract {
                    symbol: Symbol::new(isin.to_string()),
                    security_type: SecurityType::Bond,
                    security_id_type: "ISIN".to_string(),
                    security_id: isin.to_string(),
                    exchange: "SMART".into(),
                    currency: currency.into(),
                    ..Default::default()
                }
            }
        }
    }

    /// Creates a spread/combo contract builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, LegAction};
    ///
    /// let spread = Contract::spread()
    ///     .calendar(12345, 67890)
    ///     .build();
    /// ```
    pub fn spread() -> SpreadBuilder {
        SpreadBuilder::new()
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

    /// Creates an option contract from the specified parameters.
    ///
    /// Currency defaults to USD and exchange defaults to SMART.
    ///
    /// # Arguments
    /// * `symbol` - Symbol of the underlying asset
    /// * `expiration_date` - Expiration date of option contract (YYYYMMDD)
    /// * `strike` - Strike price of the option contract
    /// * `right` - Option type: "C" for Call, "P" for Put
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, Symbol};
    ///
    /// let call = Contract::option("AAPL", "20240119", 150.0, "C");
    /// assert_eq!(call.symbol, Symbol::from("AAPL"));
    /// assert_eq!(call.strike, 150.0);
    /// assert_eq!(call.right, "C");
    /// ```
    /// Creates a simple option contract (for backward compatibility).
    /// For new code, use Contract::call() or Contract::put() builders instead.
    pub fn option(symbol: &str, expiration_date: &str, strike: f64, right: &str) -> Contract {
        Contract {
            symbol: symbol.into(),
            security_type: SecurityType::Option,
            exchange: "SMART".into(),
            currency: "USD".into(),
            last_trade_date_or_contract_month: expiration_date.into(),
            strike,
            right: right.into(),
            ..Default::default()
        }
    }

    /// Returns true if this contract represents a bag/combo order.
    pub fn is_bag(&self) -> bool {
        self.security_type == SecurityType::Spread
    }

    pub(crate) fn push_fields(&self, message: &mut RequestMessage) {
        message.push_field(&self.contract_id);
        message.push_field(&self.symbol);
        message.push_field(&self.security_type);
        message.push_field(&self.last_trade_date_or_contract_month);
        message.push_field(&self.strike);
        message.push_field(&self.right);
        message.push_field(&self.multiplier);
        message.push_field(&self.exchange);
        message.push_field(&self.primary_exchange);
        message.push_field(&self.currency);
        message.push_field(&self.local_symbol);
        message.push_field(&self.trading_class);
        message.push_field(&self.include_expired);
    }
}

/// A single component within a combo contract.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComboLeg {
    /// The Contract's IB's unique id.
    pub contract_id: i32,
    /// Select the relative number of contracts for the leg you are constructing. To help determine the ratio for a specific combination order, refer to the Interactive Analytics section of the User's Guide.
    pub ratio: i32,
    /// The side (buy or sell) of the leg:
    pub action: String,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
/// OpenClose specifies whether an order is an open or closing order.
pub enum ComboLegOpenClose {
    /// 0 - Same as the parent security. This is the only option for retail customers.
    #[default]
    Same = 0,
    /// 1 - Open. This value is only valid for institutional customers.
    Open = 1,
    /// 2 - Close. This value is only valid for institutional customers.
    Close = 2,
    /// 3 - Unknown.
    Unknown = 3,
}

impl ToField for ComboLegOpenClose {
    fn to_field(&self) -> String {
        (*self as u8).to_string()
    }
}

impl From<i32> for ComboLegOpenClose {
    // TODO - verify these values
    fn from(val: i32) -> Self {
        match val {
            0 => Self::Same,
            1 => Self::Open,
            2 => Self::Close,
            3 => Self::Unknown,
            _ => panic!("unsupported value: {val}"),
        }
    }
}

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
    pub min_size: f64,
    /// Order's size increment.
    pub size_increment: f64,
    /// Order's suggested size increment.
    pub suggested_size_increment: f64,
}

/// TagValue is a convenience struct to define key-value pairs.
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
#[derive(Debug, Default)]
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
#[derive(Debug)]
pub struct ContractDescription {
    /// Fully qualified contract metadata.
    pub contract: Contract,
    /// Derivative security types available for the contract.
    pub derivative_security_types: Vec<String>,
}

#[derive(Debug, Default)]
/// Minimum price increment structure for a particular market rule ID.
pub struct MarketRule {
    /// Market Rule ID requested.
    pub market_rule_id: i32,
    /// Returns the available price increments based on the market rule.
    pub price_increments: Vec<PriceIncrement>,
}

/// Price ladder entry describing the minimum tick between price bands.
#[derive(Debug, Default)]
pub struct PriceIncrement {
    /// Lower inclusive edge where the increment applies.
    pub low_edge: f64,
    /// Minimum tick size within this price band.
    pub increment: f64,
}

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
/// Blocking contract lookup helpers backed by the synchronous transport.
pub mod blocking {
    pub(crate) use super::sync::{
        calculate_implied_volatility, calculate_option_price, contract_details, market_rule, matching_symbols, option_chain,
    };
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub(crate) use sync::{calculate_implied_volatility, calculate_option_price, contract_details, market_rule, matching_symbols, option_chain};

#[cfg(feature = "async")]
pub(crate) use r#async::{calculate_implied_volatility, calculate_option_price, contract_details, market_rule, matching_symbols, option_chain};

// Public function for decoding option computation (used by market_data module)
pub(crate) fn decode_option_computation(server_version: i32, message: &mut ResponseMessage) -> Result<OptionComputation, Error> {
    common::decoders::decode_option_computation(server_version, message)
}

// ContractBuilder is deprecated - use the new builder methods on Contract instead
// e.g., Contract::stock(), Contract::call(), Contract::put(), etc.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_builders() {
        // Test stock builder
        let stock = Contract::stock("AAPL").build();
        assert_eq!(stock.symbol, Symbol::from("AAPL"), "stock.symbol");
        assert_eq!(stock.security_type, SecurityType::Stock, "stock.security_type");
        assert_eq!(stock.currency, Currency::from("USD"), "stock.currency");
        assert_eq!(stock.exchange, Exchange::from("SMART"), "stock.exchange");

        // Test stock with customization
        let toyota = Contract::stock("7203").on_exchange("TSEJ").in_currency("JPY").build();
        assert_eq!(toyota.symbol, Symbol::from("7203"));
        assert_eq!(toyota.exchange, Exchange::from("TSEJ"));
        assert_eq!(toyota.currency, Currency::from("JPY"));

        // Test call option builder
        let call = Contract::call("AAPL").strike(150.0).expires_on(2023, 12, 15).build();
        assert_eq!(call.symbol, Symbol::from("AAPL"));
        assert_eq!(call.security_type, SecurityType::Option);
        assert_eq!(call.strike, 150.0);
        assert_eq!(call.right, "C");
        assert_eq!(call.last_trade_date_or_contract_month, "20231215");

        // Test put option builder
        let put = Contract::put("SPY").strike(450.0).expires_on(2024, 1, 19).build();
        assert_eq!(put.symbol, Symbol::from("SPY"));
        assert_eq!(put.right, "P");
        assert_eq!(put.strike, 450.0);

        // Test crypto builder
        let btc = Contract::crypto("BTC").build();
        assert_eq!(btc.symbol, Symbol::from("BTC"));
        assert_eq!(btc.security_type, SecurityType::Crypto);
        assert_eq!(btc.currency, Currency::from("USD"));
        assert_eq!(btc.exchange, Exchange::from("PAXOS"));

        // Test index
        let spx = Contract::index("SPX");
        assert_eq!(spx.symbol, Symbol::from("SPX"));
        assert_eq!(spx.security_type, SecurityType::Index);
        assert_eq!(spx.exchange, Exchange::from("CBOE"));
        assert_eq!(spx.currency, Currency::from("USD"));

        // Test news constructor (unchanged)
        let news = Contract::news("BZ");
        assert_eq!(news.symbol, Symbol::from("BZ:BZ_ALL"));
        assert_eq!(news.security_type, SecurityType::News);
        assert_eq!(news.exchange, Exchange::from("BZ"));

        // Test backward compatibility with option constructor
        let option = Contract::option("AAPL", "20231215", 150.0, "C");
        assert_eq!(option.symbol, Symbol::from("AAPL"));
        assert_eq!(option.security_type, SecurityType::Option);
        assert_eq!(option.strike, 150.0);
        assert_eq!(option.right, "C");
    }

    #[test]
    fn test_security_type_from() {
        // Test all known security types
        assert_eq!(SecurityType::from("STK"), SecurityType::Stock, "STK should be Stock");
        assert_eq!(SecurityType::from("OPT"), SecurityType::Option, "OPT should be Option");
        assert_eq!(SecurityType::from("FUT"), SecurityType::Future, "FUT should be Future");
        assert_eq!(SecurityType::from("IND"), SecurityType::Index, "IND should be Index");
        assert_eq!(SecurityType::from("FOP"), SecurityType::FuturesOption, "FOP should be FuturesOption");
        assert_eq!(SecurityType::from("CASH"), SecurityType::ForexPair, "CASH should be ForexPair");
        assert_eq!(SecurityType::from("BAG"), SecurityType::Spread, "BAG should be Spread");
        assert_eq!(SecurityType::from("WAR"), SecurityType::Warrant, "WAR should be Warrant");
        assert_eq!(SecurityType::from("BOND"), SecurityType::Bond, "BOND should be Bond");
        assert_eq!(SecurityType::from("CMDTY"), SecurityType::Commodity, "CMDTY should be Commodity");
        assert_eq!(SecurityType::from("NEWS"), SecurityType::News, "NEWS should be News");
        assert_eq!(SecurityType::from("FUND"), SecurityType::MutualFund, "FUND should be MutualFund");
        assert_eq!(SecurityType::from("CRYPTO"), SecurityType::Crypto, "CRYPTO should be Crypto");
        assert_eq!(SecurityType::from("CFD"), SecurityType::CFD, "CFD should be CFD");

        // Test unknown security type
        match SecurityType::from("UNKNOWN") {
            SecurityType::Other(name) => assert_eq!(name, "UNKNOWN", "Other should contain original string"),
            _ => panic!("Expected SecurityType::Other for unknown type"),
        }
    }

    #[test]
    fn test_combo_leg_open_close() {
        // Test From<i32> implementation
        assert_eq!(ComboLegOpenClose::from(0), ComboLegOpenClose::Same, "0 should be Same");
        assert_eq!(ComboLegOpenClose::from(1), ComboLegOpenClose::Open, "1 should be Open");
        assert_eq!(ComboLegOpenClose::from(2), ComboLegOpenClose::Close, "2 should be Close");
        assert_eq!(ComboLegOpenClose::from(3), ComboLegOpenClose::Unknown, "3 should be Unknown");

        // Test ToField implementation
        assert_eq!(ComboLegOpenClose::Same.to_field(), "0", "Same should be 0");
        assert_eq!(ComboLegOpenClose::Open.to_field(), "1", "Open should be 1");
        assert_eq!(ComboLegOpenClose::Close.to_field(), "2", "Close should be 2");
        assert_eq!(ComboLegOpenClose::Unknown.to_field(), "3", "Unknown should be 3");

        // Test Default implementation
        assert_eq!(ComboLegOpenClose::default(), ComboLegOpenClose::Same, "Default should be Same");
    }

    #[test]
    #[should_panic(expected = "unsupported value")]
    fn test_combo_leg_open_close_panic() {
        let _ = ComboLegOpenClose::from(4);
    }

    #[test]
    fn test_tag_value_to_field() {
        // Test with multiple TagValue items
        let tag_values = vec![
            TagValue {
                tag: "TAG1".to_string(),
                value: "VALUE1".to_string(),
            },
            TagValue {
                tag: "TAG2".to_string(),
                value: "VALUE2".to_string(),
            },
            TagValue {
                tag: "TAG3".to_string(),
                value: "VALUE3".to_string(),
            },
        ];

        assert_eq!(
            tag_values.to_field(),
            "TAG1=VALUE1;TAG2=VALUE2;TAG3=VALUE3;",
            "Tag values should be formatted as TAG=VALUE; pairs"
        );

        // Test with a single TagValue
        let single_tag_value = vec![TagValue {
            tag: "SINGLE_TAG".to_string(),
            value: "SINGLE_VALUE".to_string(),
        }];

        assert_eq!(
            single_tag_value.to_field(),
            "SINGLE_TAG=SINGLE_VALUE;",
            "Single tag value should be formatted as TAG=VALUE;"
        );

        // Test with empty vec
        let empty: Vec<TagValue> = vec![];
        assert_eq!(empty.to_field(), "", "Empty vec should result in empty string");

        // Test with empty tag/value
        let empty_fields = vec![TagValue {
            tag: "".to_string(),
            value: "".to_string(),
        }];

        assert_eq!(empty_fields.to_field(), "=;", "Empty tag/value should be formatted as =;");
    }

    #[test]
    fn test_is_bag() {
        // Test with a regular stock contract (not a bag/spread)
        let stock_contract = Contract::stock("AAPL").build();
        assert!(!stock_contract.is_bag(), "Stock contract should not be a bag");

        // Test with a regular option contract (not a bag/spread)
        let option_contract = Contract::option("AAPL", "20231215", 150.0, "C");
        assert!(!option_contract.is_bag(), "Option contract should not be a bag");

        // Test with a futures contract (not a bag/spread)
        // Using the simple factory method for futures that requires adding expiry
        let futures_contract = Contract {
            symbol: Symbol::from("ES"),
            security_type: SecurityType::Future,
            ..Default::default()
        };
        assert!(!futures_contract.is_bag(), "Futures contract should not be a bag");

        // Test with a contract that is a bag/spread
        let spread_contract = Contract {
            security_type: SecurityType::Spread,
            ..Default::default()
        };
        assert!(spread_contract.is_bag(), "Spread contract should be a bag");

        // Test with an explicitly set BAG security type
        let bag_contract = Contract {
            security_type: SecurityType::from("BAG"),
            ..Default::default()
        };
        assert!(bag_contract.is_bag(), "BAG contract should be a bag");

        // Test with combo legs
        let combo_contract = Contract {
            security_type: SecurityType::Spread,
            combo_legs: vec![
                ComboLeg {
                    contract_id: 12345,
                    ratio: 1,
                    action: "BUY".to_string(),
                    exchange: "SMART".to_string(),
                    ..Default::default()
                },
                ComboLeg {
                    contract_id: 67890,
                    ratio: 1,
                    action: "SELL".to_string(),
                    exchange: "SMART".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(combo_contract.is_bag(), "Contract with combo legs should be a bag");
    }
}
