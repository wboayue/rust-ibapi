//! Contract definitions and related functionality for trading instruments.
//!
//! This module provides data structures for representing various financial instruments
//! including stocks, options, futures, and complex securities. It includes contract
//! creation helpers, validation, and conversion utilities.

use std::convert::From;
use std::fmt::Debug;
use std::string::ToString;

use log::warn;
use log::{error, info};
use serde::Deserialize;
use serde::Serialize;
use tick_types::TickType;

use crate::client::DataStream;
use crate::client::ResponseContext;
use crate::client::Subscription;
use crate::encode_option_field;
use crate::messages::IncomingMessages;
use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::messages::ResponseMessage;
use crate::Client;
use crate::{server_versions, Error, ToField};

pub(crate) mod decoders;
pub(crate) mod encoders;
pub mod tick_types;

#[cfg(test)]
pub(crate) mod contract_samples;
#[cfg(test)]
mod tests;

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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Contract describes an instrument's definition
pub struct Contract {
    /// The unique IB contract identifier.
    pub contract_id: i32,
    /// The underlying's asset symbol.
    pub symbol: String,
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
    pub exchange: String,
    /// The underlying's currency.
    pub currency: String,
    /// The contract's symbol within its primary exchange For options, this will be the OCC symbol.
    pub local_symbol: String,
    /// The contract's primary exchange.
    /// For smart routed contracts, used to define contract in case of ambiguity.
    /// Should be defined as native exchange of contract, e.g. ISLAND for MSFT For exchanges which contain a period in name, will only be part of exchange name prior to period, i.e. ENEXT for ENEXT.BE.
    pub primary_exchange: String,
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
    pub combo_legs: Vec<ComboLeg>,
    /// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
    pub delta_neutral_contract: Option<DeltaNeutralContract>,

    pub issuer_id: String,
    pub description: String,
}

impl Contract {
    /// Creates stock contract from specified symbol
    /// currency defaults to USD and SMART exchange.
    pub fn stock(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: SecurityType::Stock,
            currency: "USD".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        }
    }

    /// Creates futures contract from specified symbol
    pub fn futures(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: SecurityType::Future,
            currency: "USD".to_string(),
            ..Default::default()
        }
    }

    /// Creates Crypto contract from specified symbol
    pub fn crypto(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: SecurityType::Crypto,
            currency: "USD".to_string(),
            exchange: "PAXOS".to_string(),
            ..Default::default()
        }
    }

    /// Creates News contract from specified provider code.
    pub fn news(provider_code: &str) -> Contract {
        Contract {
            symbol: format!("{}:{}_ALL", provider_code, provider_code),
            security_type: SecurityType::News,
            exchange: provider_code.to_string(),
            ..Default::default()
        }
    }

    /// Creates option contract from specified symbol, expiry date, strike price and option type.
    /// Defaults currency to USD and exchange to SMART.
    ///
    /// # Arguments
    /// * `symbol` - Symbols of the underlying asset.
    /// * `expiration_date` - Expiration date of option contract (YYYYMMDD)
    /// * `strike` - Strike price of the option contract.
    /// * `right` - Option type: "C" for Call, "P" for Put
    pub fn option(symbol: &str, expiration_date: &str, strike: f64, right: &str) -> Contract {
        Contract {
            symbol: symbol.into(),
            security_type: SecurityType::Option,
            exchange: "SMART".into(),
            currency: "USD".into(),
            last_trade_date_or_contract_month: expiration_date.into(), // Expiry date (YYYYMMDD)
            strike,
            right: right.into(), // Option type: "C" for Call, "P" for Put
            ..Default::default()
        }
    }

    /// Is Bag request
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
// ComboLeg represents a leg within combo orders.
pub struct ComboLeg {
    /// The Contract's IB's unique id.
    pub contract_id: i32,
    /// Select the relative number of contracts for the leg you are constructing. To help determine the ratio for a specific combination order, refer to the Interactive Analytics section of the User's Guide.
    pub ratio: i32,
    /// The side (buy or sell) of the leg:
    pub action: String,
    // The destination exchange to which the order will be routed.
    pub exchange: String,
    /// Specifies whether an order is an open or closing order.
    /// For institutional customers to determine if this order is to open or close a position.
    pub open_close: ComboLegOpenClose,
    /// For stock legs when doing short selling. Set to 1 = clearing broker, 2 = third party.
    pub short_sale_slot: i32,
    /// When ShortSaleSlot is 2, this field shall contain the designated location.
    pub designated_location: String,
    // DOC_TODO.
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
    pub tag: String,
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

impl DataStream<OptionComputation> for OptionComputation {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickOptionComputation];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickOptionComputation => Ok(decoders::decode_option_computation(client.server_version, message)?),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("request id required to cancel option calculations");
        match context.request_type {
            Some(OutgoingMessages::ReqCalcImpliedVolat) => {
                encoders::encode_cancel_option_computation(OutgoingMessages::CancelImpliedVolatility, request_id)
            }
            Some(OutgoingMessages::ReqCalcOptionPrice) => encoders::encode_cancel_option_computation(OutgoingMessages::CancelOptionPrice, request_id),
            _ => panic!("Unsupported request message type option computation cancel: {:?}", context.request_type),
        }
    }
}

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

impl DataStream<OptionChain> for OptionChain {
    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<OptionChain, Error> {
        match message.message_type() {
            IncomingMessages::SecurityDefinitionOptionParameter => Ok(decoders::decode_option_chain(message)?),
            IncomingMessages::SecurityDefinitionOptionParameterEnd => Err(Error::EndOfStream),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

// === API ===

// Requests contract information.
//
// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
//
// # Arguments
// * `client` - [Client] with an active connection to gateway.
// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
pub(super) fn contract_details(client: &Client, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
    verify_contract(client, contract)?;

    let request_id = client.next_request_id();
    let packet = encoders::encode_request_contract_data(client.server_version(), request_id, contract)?;

    let responses = client.send_request(request_id, packet)?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();

    while let Some(response) = responses.next() {
        log::debug!("response: {:#?}", response);
        match response {
            Ok(mut message) if message.message_type() == IncomingMessages::ContractData => {
                let decoded = decoders::decode_contract_details(client.server_version(), &mut message)?;
                contract_details.push(decoded);
            }
            Ok(message) if message.message_type() == IncomingMessages::ContractDataEnd => return Ok(contract_details),
            Ok(message) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
            Ok(message) => return Err(Error::UnexpectedResponse(message)),
            Err(e) => return Err(e),
        }
    }

    Err(Error::UnexpectedEndOfStream)
}

fn verify_contract(client: &Client, contract: &Contract) -> Result<(), Error> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(
            server_versions::SEC_ID_TYPE,
            "It does not support security_id_type or security_id attributes",
        )?
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support the trading_class parameter when requesting contract details.",
        )?
    }

    if !contract.primary_exchange.is_empty() {
        client.check_server_version(
            server_versions::LINKING,
            "It does not support primary_exchange parameter when requesting contract details.",
        )?
    }

    if !contract.issuer_id.is_empty() {
        client.check_server_version(
            server_versions::BOND_ISSUERID,
            "It does not support issuer_id parameter when requesting contract details.",
        )?
    }

    Ok(())
}

/// Contract data and list of derivative security types
#[derive(Debug)]
pub struct ContractDescription {
    pub contract: Contract,
    pub derivative_security_types: Vec<String>,
}

// Requests matching stock symbols.
//
// # Arguments
// * `client` - [Client] with an active connection to gateway.
// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
pub(super) fn matching_symbols(client: &Client, pattern: &str) -> Result<Vec<ContractDescription>, Error> {
    client.check_server_version(server_versions::REQ_MATCHING_SYMBOLS, "It does not support matching symbols requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_matching_symbols(request_id, pattern)?;
    let subscription = client.send_request(request_id, request)?;

    if let Some(Ok(mut message)) = subscription.next() {
        match message.message_type() {
            IncomingMessages::SymbolSamples => {
                return decoders::decode_contract_descriptions(client.server_version(), &mut message);
            }
            IncomingMessages::Error => {
                // TODO custom error
                error!("unexpected error: {:?}", message);
                return Err(Error::Simple(format!("unexpected error: {message:?}")));
            }
            _ => {
                info!("unexpected message: {:?}", message);
                return Err(Error::Simple(format!("unexpected message: {message:?}")));
            }
        }
    }

    Ok(Vec::default())
}

#[derive(Debug, Default)]
/// Minimum price increment structure for a particular market rule ID.
pub struct MarketRule {
    /// Market Rule ID requested.
    pub market_rule_id: i32,
    /// Returns the available price increments based on the market rule.
    pub price_increments: Vec<PriceIncrement>,
}

#[derive(Debug, Default)]
pub struct PriceIncrement {
    pub low_edge: f64,
    pub increment: f64,
}

// Requests details about a given market rule
//
// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
// A list of market rule ids can be obtained by invoking [request_contract_details] on a particular contract. The returned market rule ID list will provide the market rule ID for the instrument in the correspond valid exchange list in [ContractDetails].
pub(super) fn market_rule(client: &Client, market_rule_id: i32) -> Result<MarketRule, Error> {
    client.check_server_version(server_versions::MARKET_RULES, "It does not support market rule requests.")?;

    let request = encoders::encode_request_market_rule(market_rule_id)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestMarketRule, request)?;

    match subscription.next() {
        Some(Ok(mut message)) => Ok(decoders::decode_market_rule(&mut message)?),
        Some(Err(e)) => Err(e),
        None => Err(Error::Simple("no market rule found".into())),
    }
}

// Calculates an option’s price based on the provided volatility and its underlying’s price.
//
// # Arguments
// * `contract`   - The [Contract] object for which the depth is being requested.
// * `volatility` - Hypothetical volatility.
// * `underlying_price` - Hypothetical option’s underlying price.
pub(super) fn calculate_option_price(
    client: &Client,
    contract: &Contract,
    volatility: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    client.check_server_version(server_versions::REQ_CALC_OPTION_PRICE, "It does not support calculation price requests.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_calculate_option_price(client.server_version(), request_id, contract, volatility, underlying_price)?;
    let subscription = client.send_request(request_id, message)?;

    match subscription.next() {
        Some(Ok(mut message)) => OptionComputation::decode(client, &mut message),
        Some(Err(e)) => Err(e),
        None => Err(Error::Simple("no data for option calculation".into())),
    }
}

// Calculates the implied volatility based on hypothetical option and its underlying prices.
//
// # Arguments
// * `contract`   - The [Contract] object for which the depth is being requested.
// * `option_price` - Hypothetical option price.
// * `underlying_price` - Hypothetical option’s underlying price.
pub(super) fn calculate_implied_volatility(
    client: &Client,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    client.check_server_version(
        server_versions::REQ_CALC_IMPLIED_VOLAT,
        "It does not support calculate implied volatility.",
    )?;

    let request_id = client.next_request_id();
    let message = encoders::encode_calculate_implied_volatility(client.server_version(), request_id, contract, option_price, underlying_price)?;
    let subscription = client.send_request(request_id, message)?;

    match subscription.next() {
        Some(Ok(mut message)) => OptionComputation::decode(client, &mut message),
        Some(Err(e)) => Err(e),
        None => Err(Error::Simple("no data for option calculation".into())),
    }
}

pub(super) fn option_chain<'a>(
    client: &'a Client,
    symbol: &str,
    exchange: &str,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<Subscription<'a, OptionChain>, Error> {
    client.check_server_version(
        server_versions::SEC_DEF_OPT_PARAMS_REQ,
        "It does not support security definition option parameters.",
    )?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

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
    contract_id: Option<i32>,
    symbol: Option<String>,
    security_type: Option<SecurityType>,
    last_trade_date_or_contract_month: Option<String>,
    strike: Option<f64>,
    right: Option<String>,
    multiplier: Option<String>,
    exchange: Option<String>,
    currency: Option<String>,
    local_symbol: Option<String>,
    primary_exchange: Option<String>,
    trading_class: Option<String>,
    include_expired: Option<bool>,
    security_id_type: Option<String>,
    security_id: Option<String>,
    combo_legs_description: Option<String>,
    combo_legs: Option<Vec<ComboLeg>>,
    delta_neutral_contract: Option<DeltaNeutralContract>,
    issuer_id: Option<String>,
    description: Option<String>,
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

    /// Sets the option right
    ///
    /// Required for option contracts. Valid values:
    /// - "P" for put options
    /// - "C" for call options
    ///
    /// Case insensitive.
    pub fn right<S: Into<String>>(mut self, right: S) -> Self {
        self.right = Some(right.into().to_uppercase());
        self
    }

    /// Sets the instrument's multiplier
    ///
    /// Defines the contract size multiplier for futures and options.
    /// For most options, this is typically "100".
    pub fn multiplier<S: Into<String>>(mut self, multiplier: S) -> Self {
        self.multiplier = Some(multiplier.into());
        self
    }

    /// Sets the destination exchange
    ///
    /// Common exchanges:
    /// - "SMART" for smart routing
    /// - "NYSE", "NASDAQ" for stocks
    /// - "GLOBEX", "NYMEX" for futures
    /// - "PAXOS" for crypto
    pub fn exchange<S: Into<String>>(mut self, exchange: S) -> Self {
        self.exchange = Some(exchange.into());
        self
    }

    /// Sets the underlying's currency
    ///
    /// Standard 3-letter currency codes (e.g., "USD", "EUR", "GBP").
    pub fn currency<S: Into<String>>(mut self, currency: S) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Sets the local symbol
    pub fn local_symbol<S: Into<String>>(mut self, local_symbol: S) -> Self {
        self.local_symbol = Some(local_symbol.into());
        self
    }

    /// Sets the primary exchange
    pub fn primary_exchange<S: Into<String>>(mut self, primary_exchange: S) -> Self {
        self.primary_exchange = Some(primary_exchange.into());
        self
    }

    /// Sets the trading class
    pub fn trading_class<S: Into<String>>(mut self, trading_class: S) -> Self {
        self.trading_class = Some(trading_class.into());
        self
    }

    /// Sets include expired flag
    pub fn include_expired(mut self, include_expired: bool) -> Self {
        self.include_expired = Some(include_expired);
        self
    }

    /// Sets the security ID type
    pub fn security_id_type<S: Into<String>>(mut self, security_id_type: S) -> Self {
        self.security_id_type = Some(security_id_type.into());
        self
    }

    /// Sets the security ID
    pub fn security_id<S: Into<String>>(mut self, security_id: S) -> Self {
        self.security_id = Some(security_id.into());
        self
    }

    /// Sets the combo legs description
    pub fn combo_legs_description<S: Into<String>>(mut self, description: S) -> Self {
        self.combo_legs_description = Some(description.into());
        self
    }

    /// Sets the combo legs
    pub fn combo_legs(mut self, combo_legs: Vec<ComboLeg>) -> Self {
        self.combo_legs = Some(combo_legs);
        self
    }

    /// Sets the delta neutral contract
    pub fn delta_neutral_contract(mut self, delta_neutral_contract: DeltaNeutralContract) -> Self {
        self.delta_neutral_contract = Some(delta_neutral_contract);
        self
    }

    /// Sets the issuer ID
    pub fn issuer_id<S: Into<String>>(mut self, issuer_id: S) -> Self {
        self.issuer_id = Some(issuer_id.into());
        self
    }

    /// Sets the description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Creates a stock contract builder with symbol, exchange, and currency
    ///
    /// Convenience method for creating stock contracts with common defaults.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "AAPL", "MSFT")
    /// * `exchange` - Exchange (e.g., "SMART", "NYSE")
    /// * `currency` - Currency (e.g., "USD")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::stock("AAPL", "SMART", "USD").build()?;
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

    /// Creates a futures contract builder with symbol, exchange, and currency
    ///
    /// Convenience method for creating futures contracts. Remember to set the contract month.
    ///
    /// # Arguments
    /// * `symbol` - Futures symbol (e.g., "ES", "NQ", "CL")
    /// * `exchange` - Exchange (e.g., "GLOBEX", "NYMEX")
    /// * `currency` - Currency (e.g., "USD")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::futures("ES", "GLOBEX", "USD")
    ///     .last_trade_date_or_contract_month("202412")
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

    /// Creates a crypto contract builder with symbol, exchange, and currency
    ///
    /// Convenience method for creating cryptocurrency contracts.
    ///
    /// # Arguments
    /// * `symbol` - Crypto symbol (e.g., "BTC", "ETH")
    /// * `exchange` - Exchange (e.g., "PAXOS")
    /// * `currency` - Quote currency (e.g., "USD")
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

    /// Creates an option contract builder with symbol, exchange, and currency
    ///
    /// Convenience method for creating option contracts. Remember to set strike, right, and expiration.
    ///
    /// # Arguments
    /// * `symbol` - Underlying symbol (e.g., "AAPL", "SPY")
    /// * `exchange` - Exchange (e.g., "SMART")
    /// * `currency` - Currency (e.g., "USD")
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

    /// Builds and validates the [Contract]
    ///
    /// Performs validation based on the security type and returns a [Contract] instance
    /// or an error if validation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Symbol is missing
    /// - Required fields for the security type are missing
    /// - Strike price is negative
    /// - Option right is not "P" or "C"
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::ContractBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = ContractBuilder::stock("AAPL", "SMART", "USD")
    ///     .build()
    ///     .expect("Failed to build contract");
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Contract, Error> {
        let symbol = self.symbol.ok_or_else(|| Error::Simple("Symbol is required".to_string()))?;
        let security_type = self.security_type.unwrap_or_default();

        // Validate required fields based on security type
        match security_type {
            SecurityType::Option => {
                if self.strike.is_none() {
                    return Err(Error::Simple("Strike price is required for options".to_string()));
                }
                if self.right.is_none() {
                    return Err(Error::Simple("Right (P for PUT or C for CALL) is required for options".to_string()));
                }
                if self.last_trade_date_or_contract_month.is_none() {
                    return Err(Error::Simple("Expiration date is required for options".to_string()));
                }
            }
            SecurityType::Future | SecurityType::FuturesOption => {
                if self.last_trade_date_or_contract_month.is_none() {
                    return Err(Error::Simple("Contract month is required for futures".to_string()));
                }
            }
            _ => {}
        }

        // Validate option right format
        if let Some(ref right) = self.right {
            let right_upper = right.to_uppercase();
            if !["P", "C"].contains(&right_upper.as_str()) {
                return Err(Error::Simple("Option right must be P for PUT or C for CALL".to_string()));
            }
        }

        // Validate strike price
        if let Some(strike) = self.strike {
            if strike < 0.0 {
                return Err(Error::Simple("Strike price cannot be negative".to_string()));
            }
        }

        Ok(Contract {
            contract_id: self.contract_id.unwrap_or(0),
            symbol,
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
