use std::fmt::Debug;
use std::string::ToString;

use anyhow::{anyhow, Result};
use log::{error, info};

use crate::client::{Client, RequestPacket, ResponsePacket};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::server_versions;

// Models

#[derive(Debug, PartialEq, Eq, Default)]
/// SecurityType enumerates available security types
pub enum SecurityType {
    /// Stock (or ETF)
    #[default]
    STK,
    /// Option
    OPT,
    /// Future
    FUT,
    /// Index
    IND,
    /// Futures option
    FOP,
    /// Forex pair
    CASH,
    /// Combo
    BAG,
    ///  Warrant
    WAR,
    /// Bond
    BOND,
    /// Commodity
    CMDTY,
    /// News
    NEWS,
    /// Mutual fund
    FUND,
}

impl ToString for SecurityType {
    fn to_string(&self) -> String {
        match self {
            SecurityType::STK => "STK".to_string(),
            SecurityType::OPT => "OPT".to_string(),
            SecurityType::FUT => "FUT".to_string(),
            SecurityType::IND => "IND".to_string(),
            SecurityType::FOP => "FOP".to_string(),
            SecurityType::CASH => "CASH".to_string(),
            SecurityType::BAG => "BAG".to_string(),
            SecurityType::WAR => "WAR".to_string(),
            SecurityType::BOND => "BOND".to_string(),
            SecurityType::CMDTY => "CMDTY".to_string(),
            SecurityType::NEWS => "NEWS".to_string(),
            SecurityType::FUND => "FUND".to_string(),
        }
    }
}

impl SecurityType {
    pub fn from(name: &str) -> SecurityType {
        match name {
            "STK" => SecurityType::STK,
            "OPT" => SecurityType::OPT,
            "FUT" => SecurityType::FUT,
            "IND" => SecurityType::IND,
            "FOP" => SecurityType::FOP,
            "CASH" => SecurityType::CASH,
            "BAG" => SecurityType::BAG,
            "WAR" => SecurityType::WAR,
            "BOND" => SecurityType::BOND,
            "CMDTY" => SecurityType::CMDTY,
            "NEWS" => SecurityType::NEWS,
            "FUND" => SecurityType::FUND,
            &_ => todo!(),
        }
    }
}

#[derive(Debug, Default)]
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
    pub delta_neutral_contract: DeltaNeutralContract,

    pub issuer_id: String,
    pub description: String,
}

impl Contract {
    /// Creates stock contract from specified symbol
    pub fn stock(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: SecurityType::STK,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
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
    /// For instituational customers to determine if this order is to open or close a position.
    pub open_close: OpenClose,
    /// For stock legs when doing short selling. Set to 1 = clearing broker, 2 = third party.
    pub short_sale_slot: i32,
    /// When ShortSaleSlot is 2, this field shall contain the designated location.
    pub designated_location: String,
    // DOC_TODO.
    pub exempt_code: i32,
}

#[derive(Debug, Default)]
/// OpenClose specifies whether an order is an open or closing order.
pub enum OpenClose {
    /// 0 - Same as the parent security. This is the only option for retail customers.
    Same,
    /// 1 - Open. This value is only valid for institutional customers.
    Open,
    /// 2 - Close. This value is only valid for institutional customers.
    Close,
    /// 3 - Unknown.
    #[default]
    Unknown,
}

#[derive(Debug, Default)]
/// Delta and underlying price for Delta-Neutral combo orders.
/// Underlying (STK or FUT), delta and underlying price goes into this attribute.
pub struct DeltaNeutralContract {
    /// The unique contract identifier specifying the security. Used for Delta-Neutral Combo contracts.
    pub contract_id: String,
    /// The underlying stock or future delta. Used for Delta-Neutral Combo contracts.
    pub delta: f64,
    /// The price of the underlying. Used for Delta-Neutral Combo contracts.
    pub price: f64,
}

/// ContractDetails provides extended contract details.
#[derive(Debug, Default)]
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
    pub order_types: String,
    /// Valid exchange fields when placing an order for this contract.
    /// The list of exchanges will is provided in the same order as the corresponding MarketRuleIds list.
    pub valid_exchanges: String,
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
    pub trading_hours: String,
    /// The liquid hours of the product. This value will contain the liquid hours (regular trading hours) of the contract on the specified exchange. Format for TWS versions until 969: 20090507:0700-1830,1830-2330;20090508:CLOSED. In TWS versions 965+ there is an option in the Global Configuration API settings to return 1 month of trading hours. In TWS v970 and above, the format includes the date of the closing time to clarify potential ambiguity, e.g. 20180323:0930-20180323:1600;20180326:0930-20180326:1600.
    pub liquid_hours: String,
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
    pub market_rule_ids: String,
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
#[derive(Clone, Debug)]
pub struct TagValue {
    pub tag: String,
    pub value: String,
}

// API

/// Requests contract information.
///
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::BasicClient;
/// use ibapi::contracts::{self, Contract};
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = BasicClient::connect("localhost:4002")?;
///
///     let contract = Contract::stock("TSLA");
///     let results = contracts::find_contract_details(&mut client, &contract)?;
///
///     for contract_detail in &results {
///         println!("contract: {:?}", contract_detail);
///     }
///
///     Ok(())
/// }
/// ```
pub fn find_contract_details<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
) -> Result<Vec<ContractDetails>> {
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

    let request_id = client.next_request_id();
    let packet = encode_request_contract_data(client.server_version(), request_id, contract)?;

    let responses = client.send_message(request_id, packet)?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();

    for mut message in responses {
        match message.message_type() {
            IncomingMessages::ContractData => {
                let decoded = decode_contract_details(client.server_version(), &mut message)?;
                contract_details.push(decoded);
            }
            IncomingMessages::ContractDataEnd => {
                break;
            }
            IncomingMessages::Error => {
                error!("error: {:?}", message);
                return Err(anyhow!("contract_details {:?}", message));
            }
            _ => {
                error!("unexpected message: {:?}", message);
            }
        }
    }

    Ok(contract_details)
}

fn encode_request_contract_data(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
) -> Result<RequestPacket> {
    const VERSION: i32 = 8;

    let mut packet = RequestPacket::default();

    packet.add_field(&OutgoingMessages::RequestContractData);
    packet.add_field(&VERSION);

    if server_version >= server_versions::CONTRACT_DATA_CHAIN {
        packet.add_field(&request_id);
    }

    if server_version >= server_versions::CONTRACT_CONID {
        packet.add_field(&contract.contract_id);
    }

    packet.add_field(&contract.symbol);
    packet.add_field(&contract.security_type);
    packet.add_field(&contract.last_trade_date_or_contract_month);
    packet.add_field(&contract.strike);
    packet.add_field(&contract.right);

    if server_version >= 15 {
        packet.add_field(&contract.multiplier);
    }

    if server_version >= server_versions::PRIMARYEXCH {
        packet.add_field(&contract.exchange);
        packet.add_field(&contract.primary_exchange);
    } else if server_version >= server_versions::LINKING {
        if !contract.primary_exchange.is_empty()
            && (contract.exchange == "BEST" || contract.exchange == "SMART")
        {
            packet.add_field(&format!(
                "{}:{}",
                contract.exchange, contract.primary_exchange
            ));
        } else {
            packet.add_field(&contract.exchange);
        }
    }

    packet.add_field(&contract.currency);
    packet.add_field(&contract.local_symbol);

    if server_version >= server_versions::TRADING_CLASS {
        packet.add_field(&contract.trading_class);
    }
    if server_version >= 31 {
        packet.add_field(&contract.include_expired);
    }
    if server_version >= server_versions::SEC_ID_TYPE {
        packet.add_field(&contract.security_id_type);
        packet.add_field(&contract.security_id);
    }
    if server_version >= server_versions::BOND_ISSUERID {
        packet.add_field(&contract.issuer_id);
    }

    Ok(packet)
}

fn decode_contract_details(
    server_version: i32,
    message: &mut ResponsePacket,
) -> Result<ContractDetails> {
    message.skip(); // message type

    let mut message_version = 8;
    if server_version < server_versions::SIZE_RULES {
        message_version = message.next_int()?;
    }

    let mut request_id = -1;
    if message_version >= 3 {
        request_id = message.next_int()?;
    }

    info!(
        "request_id: {}, server_version: {}, message_version: {}",
        request_id, server_version, message_version
    );

    let mut contract = ContractDetails::default();

    contract.contract.symbol = message.next_string()?;
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, &message.next_string()?, false)?;
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = message.next_string()?;
    contract.contract.currency = message.next_string()?;
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version)
    {
        message.next_int()?; // mdSizeMultiplier no longer used
    }
    contract.contract.multiplier = message.next_string()?;
    contract.order_types = message.next_string()?;
    contract.valid_exchanges = message.next_string()?;
    if message_version >= 2 {
        contract.price_magnifier = message.next_int()?;
    }
    if message_version >= 4 {
        contract.under_contract_id = message.next_int()?;
    }
    if message_version >= 5 {
        //        https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EDecoder.cs#L1626
        contract.long_name = message.next_string()?;
        contract.contract.primary_exchange = message.next_string()?;
    }
    if message_version >= 6 {
        contract.contract_month = message.next_string()?;
        contract.industry = message.next_string()?;
        contract.category = message.next_string()?;
        contract.subcategory = message.next_string()?;
        contract.time_zone_id = message.next_string()?;
        contract.trading_hours = message.next_string()?;
        contract.liquid_hours = message.next_string()?;
    }
    if message_version >= 8 {
        contract.ev_rule = message.next_string()?;
        contract.ev_multiplier = message.next_double()?;
    }
    if message_version >= 7 {
        let sec_id_list_count = message.next_int()?;
        for _ in 0..sec_id_list_count {
            let tag = message.next_string()?;
            let value = message.next_string()?;
            contract.sec_id_list.push(TagValue { tag, value });
        }
    }
    if server_version > server_versions::AGG_GROUP {
        contract.agg_group = message.next_int()?;
    }
    if server_version > server_versions::UNDERLYING_INFO {
        contract.under_symbol = message.next_string()?;
        contract.under_security_type = message.next_string()?;
    }
    if server_version > server_versions::MARKET_RULES {
        contract.market_rule_ids = message.next_string()?;
    }
    if server_version > server_versions::REAL_EXPIRATION_DATE {
        contract.real_expiration_date = message.next_string()?;
    }
    if server_version > server_versions::STOCK_TYPE {
        contract.stock_type = message.next_string()?;
    }
    if (server_versions::FRACTIONAL_SIZE_SUPPORT..server_versions::SIZE_RULES)
        .contains(&server_version)
    {
        message.next_double()?; // size min tick -- no longer used
    }
    if server_version >= server_versions::SIZE_RULES {
        contract.min_size = message.next_double()?;
        contract.size_increment = message.next_double()?;
        contract.suggested_size_increment = message.next_double()?;
    }

    Ok(contract)
}

fn read_last_trade_date(
    contract: &mut ContractDetails,
    last_trade_date_or_contract_month: &str,
    is_bond: bool,
) -> Result<()> {
    if last_trade_date_or_contract_month.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = if last_trade_date_or_contract_month.contains('-') {
        last_trade_date_or_contract_month.split('-').collect()
    } else {
        // let re = Regex::new(r"\s+").unwrap();
        last_trade_date_or_contract_month.split(' ').collect()
    };

    if !splitted.is_empty() {
        if is_bond {
            contract.maturity = splitted[0].to_string();
        } else {
            contract.contract.last_trade_date_or_contract_month = splitted[0].to_string();
        }
    }
    if splitted.len() > 1 {
        contract.last_trade_time = splitted[1].to_string();
    }
    if is_bond && splitted.len() > 2 {
        contract.time_zone_id = splitted[2].to_string();
    }

    Ok(())
}

/// Contract data and list of derivative security types
#[derive(Debug)]
pub struct ContractDescription {
    pub contract: Contract,
    pub derivative_security_types: Vec<String>,
}

/// Requests matching stock symbols.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::BasicClient;
/// use ibapi::contracts;
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = BasicClient::connect("localhost:4002")?;
///
///     let contracts = contracts::find_contract_descriptions_matching(&mut client, "IB")?;
///
///     for contract in &contracts {
///         println!("contract: {:?}", contract);
///     }
///
///     Ok(())
/// }
/// ```
pub fn find_contract_descriptions_matching<C: Client + Debug>(
    client: &mut C,
    pattern: &str,
) -> Result<Vec<ContractDescription>> {
    client.check_server_version(
        server_versions::REQ_MATCHING_SYMBOLS,
        "It does not support mathing symbols requests.",
    )?;

    let request_id = client.next_request_id();
    let request = encode_request_matching_symbols(request_id, pattern)?;

    let mut responses = client.send_message(request_id, request)?;

    if let Some(mut message) = responses.next() {
        match message.message_type() {
            IncomingMessages::SymbolSamples => {
                return decode_contract_descriptions(client.server_version(), &mut message);
            }
            IncomingMessages::Error => {
                error!("unexpected error: {:?}", message);
                return Err(anyhow!("unexpected error: {:?}", message));
            }
            _ => {
                info!("unexpected message: {:?}", message);
                return Err(anyhow!("unexpected message: {:?}", message));
            }
        }
    }

    Ok(Vec::default())
}

fn encode_request_matching_symbols(request_id: i32, pattern: &str) -> Result<RequestPacket> {
    let mut message = RequestPacket::default();

    message.add_field(&OutgoingMessages::RequestMatchingSymbols);
    message.add_field(&request_id);
    message.add_field(&pattern);

    Ok(message)
}

fn decode_contract_descriptions(
    server_version: i32,
    message: &mut ResponsePacket,
) -> Result<Vec<ContractDescription>> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let contract_descriptions_count = message.next_int()?;

    if contract_descriptions_count < 1 {
        return Ok(Vec::default());
    }

    let mut contract_descriptions: Vec<ContractDescription> =
        Vec::with_capacity(contract_descriptions_count as usize);

    for _ in 0..contract_descriptions_count {
        let mut contract = Contract {
            contract_id: message.next_int()?,
            symbol: message.next_string()?,
            security_type: SecurityType::from(&message.next_string()?),
            primary_exchange: message.next_string()?,
            currency: message.next_string()?,
            ..Default::default()
        };

        let derivative_security_types_count = message.next_int()?;
        let mut derivative_security_types: Vec<String> =
            Vec::with_capacity(derivative_security_types_count as usize);
        for _ in 0..derivative_security_types_count {
            derivative_security_types.push(message.next_string()?);
        }

        if server_version >= server_versions::BOND_ISSUERID {
            contract.description = message.next_string()?;
            contract.issuer_id = message.next_string()?;
        }

        contract_descriptions.push(ContractDescription {
            contract,
            derivative_security_types,
        });
    }

    Ok(contract_descriptions)
}

#[cfg(test)]
mod tests;
