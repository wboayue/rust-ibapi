use std::str::FromStr;
use strum_macros::EnumString;
use time::OffsetDateTime;

#[derive(Debug, PartialEq, EnumString)]
/// SecurityType enumerates available security types
pub enum SecurityType {
    /// Stock (or ETF)
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

#[derive(Debug)]
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
    pub combo_legs: Box<[ComboLeg]>,
    /// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
    pub delta_neutral_contract: DeltaNeutralContract,
}

#[derive(Debug)]
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

#[derive(Debug)]
/// OpenClose specifies whether an order is an open or closing order.
pub enum OpenClose {
    /// 0 - Same as the parent security. This is the only option for retail customers.
    Same,
    /// 1 - Open. This value is only valid for institutional customers.
    Open,
    /// 2 - Close. This value is only valid for institutional customers.
    Close,
    /// 3 - Unknown.
    Unknown,
}

#[derive(Debug)]
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
    pub ev_multiplier: i32,
    /// Aggregated group Indicates the smart-routing group to which a contract belongs. contracts which cannot be smart-routed have aggGroup = -1.
    pub agg_group: i32,
    /// A list of contract identifiers that the customer is allowed to view. CUSIP/ISIN/etc. For US stocks, receiving the ISIN requires the CUSIP market data subscription. For Bonds, the CUSIP or ISIN is input directly into the symbol field of the Contract class.
    pub sec_id_list: Box<[TagValue]>,
    /// For derivatives, the symbol of the underlying contract.
    pub under_symbol: String,
    /// For derivatives, returns the underlying security type.
    pub under_sec_type: String,
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

/// Bar describes the historical data bar.
pub struct Bar {
    /// The bar's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// The bar's open price.
    pub open: f64,
    /// The bar's high price.
    pub high: f64,
    /// The bar's low price.
    pub low: f64,
    /// The bar's close price.
    pub close: f64,
    /// The bar's traded volume if available (only available for TRADES)
    pub volume: i64,
    /// The bar's Weighted Average Price (only available for TRADES)
    pub wap: f64,
    /// The number of trades during the bar's timespan (only available for TRADES)
    pub count: i32,
}

pub struct Trade {
    /// Tick type: "Last" or "AllLast"
    pub tick_type: String,
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// Tick last price
    pub price: f64,
    /// Tick last size
    pub size: i64,
    /// Tick attribs (bit 0 - past limit, bit 1 - unreported)
    pub trade_attribute: TradeAttribute,
    /// Tick exchange
    pub exchange: String,
    /// Tick special conditions
    pub special_conditions: String,
}

pub struct TradeAttribute {
    pub past_limit: bool,
    pub unreported: bool,
}

pub struct BidAsk {
    /// The spread's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// tick-by-tick real-time tick bid price
    pub bid_price: f64,
    /// tick-by-tick real-time tick ask price
    pub ask_price: f64,
    /// tick-by-tick real-time tick bid size
    pub bid_size: i64,
    /// tick-by-tick real-time tick ask size
    pub ask_size: i64,
    /// tick-by-tick real-time bid/ask tick attribs (bit 0 - bid past low, bit 1 - ask past high)
    pub bid_ask_attribute: BidAskAttribute,
}

pub struct BidAskAttribute {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

pub struct HistogramData {
    pub price: f64,
    pub count: i32,
}

pub struct DepthMktDataDescription {
    pub exchange: String,
    pub sec_type: String,
    pub listing_exch: String,
    pub service_data_type: String,
    pub agg_group: i32,
}

pub struct SmartComponent {
    pub bit_number: i32,
    pub exchange: String,
    pub exchange_letter: String,
}

pub struct TickAttrib {
    pub can_auto_execute: bool,
    pub past_limit: bool,
    pub pre_open: bool,
}

pub struct TickAttribBidAsk {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

pub struct TickAttribLast {
    pub past_limit: bool,
    pub unreported: bool,
}

pub struct FamilyCode {
    pub account_id: String,
    pub family_code_str: String,
}

pub struct PriceIncrement {
    pub low_edge: f64,
    pub increment: f64,
}

pub struct HistoricalTick {
    pub time: i32,
    pub price: f64,
    pub size: i32,
}

pub struct HistoricalTickBidAsk {
    pub time: i32,
    pub tick_attrib_bid_ask: TickAttribBidAsk,
    pub price_bid: f64,
    pub price_ask: f64,
    pub size_bid: i32,
    pub size_ask: i32,
}

pub struct NewsProvider {
    pub code: String,
    pub name: String,
}

pub enum ComboParam {
    NonGuaranteed,
    PriceCondConid,
    CondPriceMax,
    CondPriceMin,
    ChangeToMktTime1,
    ChangeToMktTime2,
    DiscretionaryPct,
    DontLeginNext,
    LeginPrio,
    MaxSegSize,
}

pub enum HedgeType {
    None,
    Delta,
    Beta,
    Fx,
    Pair,
}

pub enum Right {
    None,
    Put,
    Call,
}

pub enum VolatilityType {
    None,
    Daily,
    Annual,
}

pub enum ReferencePriceType {
    None,
    Midpoint,
    BidOrAsk,
}

#[derive(Clone, Debug)]
/// Order describes the order.
pub struct Order {
    /// The API client's order id.
    pub order_id: i32,
    /// The Solicited field should be used for orders initiated or recommended by the broker or adviser that were approved by the client (by phone, email, chat, verbally, etc.) prior to entry. Please note that orders that the adviser or broker placed without specifically discussing with the client are discretionary orders, not solicited.
    pub solicited: bool,
    /// The API client id which placed the order.
    pub client_id: i32,
    /// The Host order identifier.
    pub perm_id: i32,
    /// Identifies the side.
    /// Generally available values are BUY and SELL.
    /// Additionally, SSHORT and SLONG are available in some institutional-accounts only.
    /// For general account types, a SELL order will be able to enter a short position automatically if the order quantity is larger than your current long position.
    /// SSHORT is only supported for institutional account configured with Long/Short account segments or clearing with a separate account.
    /// SLONG is available in specially-configured institutional accounts to indicate that long position not yet delivered is being sold.
    pub action: Action,
    /// The number of positions being bought/sold.
    pub total_quantity: f64,
    /// The order's type.
    pub order_type: String,
    /// The LIMIT price.
    /// Used for limit, stop-limit and relative orders. In all other cases specify zero. For relative orders with no limit price, also specify zero.
    pub lmt_price: f64,
    /// Generic field to contain the stop price for STP LMT orders, trailing amount, etc.
    pub aux_price: f64,
    /// The time in force.
    /// Valid values are:
    /// DAY - Valid for the day only.
    /// GTC - Good until canceled. The order will continue to work within the system and in the marketplace until it executes or is canceled. GTC orders will be automatically be cancelled under the following conditions:
    /// If a corporate action on a security results in a stock split (forward or reverse), exchange for shares, or distribution of shares. If you do not log into your IB account for 90 days.
    /// At the end of the calendar quarter following the current quarter. For example, an order placed during the third quarter of 2011 will be canceled at the end of the first quarter of 2012. If the last day is a non-trading day, the cancellation will occur at the close of the final trading day of that quarter. For example, if the last day of the quarter is Sunday, the orders will be cancelled on the preceding Friday.
    /// Orders that are modified will be assigned a new “Auto Expire” date consistent with the end of the calendar quarter following the current quarter.
    /// Orders submitted to IB that remain in force for more than one day will not be reduced for dividends. To allow adjustment to your order price on ex-dividend date, consider using a Good-Til-Date/Time (GTD) or Good-after-Time/Date (GAT) order type, or a combination of the two.
    /// IOC - Immediate or Cancel. Any portion that is not filled as soon as it becomes available in the market is canceled.
    /// GTD - Good until Date. It will remain working within the system and in the marketplace until it executes or until the close of the market on the date specified
    /// OPG - Use OPG to send a market-on-open (MOO) or limit-on-open (LOO) order.
    /// FOK - If the entire Fill-or-Kill order does not execute as soon as it becomes available, the entire order is canceled.
    /// DTC - Day until Canceled.
    pub tif: String,
    /// One-Cancels-All group identifier.
    pub oca_group: String,
    /// Tells how to handle remaining orders in an OCA group when one order or part of an order executes.
    /// Valid values are:
    /// 1 - Cancel all remaining orders with block.
    /// 2 - Remaining orders are proportionately reduced in size with block.
    /// 3 - Remaining orders are proportionately reduced in size with no block.
    /// If you use a value "with block" it gives the order overfill protection. This means that only one order in the group will be routed at a time to remove the possibility of an overfill.
    pub oca_type: i32,
    /// The order reference.
    /// Intended for institutional customers only, although all customers may use it to identify the API client that sent the order when multiple API clients are running.
    pub order_ref: String,
    /// Specifies whether the order will be transmitted by TWS. If set to false, the order will be created at TWS but will not be sent.
    pub transmit: bool,
    /// The order ID of the parent order, used for bracket and auto trailing stop orders.
    pub parent_id: i32,
    /// If set to true, specifies that the order is an ISE Block order.
    pub block_order: bool,
    /// If set to true, specifies that the order is a Sweep-to-Fill order.
    pub sweep_to_fill: bool,
    /// The publicly disclosed order size, used when placing Iceberg orders.
    pub display_size: i32,
    /// Specifies how Simulated Stop, Stop-Limit and Trailing Stop orders are triggered.
    /// Valid values are:
    /// 0 - The default value. The "double bid/ask" function will be used for orders for OTC stocks and US options. All other orders will used the "last" function.
    /// 1 - use "double bid/ask" function, where stop orders are triggered based on two consecutive bid or ask prices.
    /// 2 - "last" function, where stop orders are triggered based on the last price.
    /// 3 - double last function.
    /// 4 - bid/ask function.
    /// 7 - last or bid/ask function.
    /// 8 - mid-point function.    
    pub trigger_method: i32,
    /// If set to true, allows orders to also trigger or fill outside of regular trading hours.
    pub outside_rth: bool,
    /// If set to true, the order will not be visible when viewing the market depth. This option only applies to orders routed to the NASDAQ exchange.
    pub hidden: bool,
    /// Specifies the date and time after which the order will be active.
    /// Format: yyyymmdd hh:mm:ss {optional Timezone}.
    pub good_after_time: String,
    /// The date and time until the order will be active.
    /// You must enter GTD as the time in force to use this string. The trade's "Good Till Date," format "yyyyMMdd HH:mm:ss (optional time zone)" or UTC "yyyyMMdd-HH:mm:ss".
    pub good_till_date: String,

    // Clearing info
    pub account: String,
    //used only when short_sale_slot=2
    pub open_close: String,
    // O=Open, C=Close
    // pub origin: Origin,

    // "Time in Force" - DAY, GTC, etc.
    pub active_start_time: String,
    // for GTC orders
    pub active_stop_time: String,
    pub discretionary_amt: f64,
    pub fa_group: String,
    pub fa_method: String,
    pub fa_percentage: String,
    pub fa_profile: String,
    // models
    pub model_code: String,
    // 0=Customer, 1=Firm
    pub short_sale_slot: i32,
    // institutional (ie non-cleared) only
    pub designated_location: String,
    // type: int; 1 if you hold the shares, 2 if they will be delivered from elsewhere.  Only for Action=SSHORT
    pub exempt_code: i32,
    // Format: 20060505 08:00:00 {time zone}
    pub rule80a: Rule80A,
    // Individual = 'I', Agency = 'A', AgentOtherMember = 'W', IndividualPTIA = 'J', AgencyPTIA = 'U', AgentOtherMemberPTIA = 'M', IndividualPT = 'K', AgencyPT = 'Y', AgentOtherMemberPT = 'N'
    pub settling_firm: String,
    pub all_or_none: bool,
    pub min_qty: i32,
    //type: int
    pub percent_offset: f64,
    // SMART routing only
    pub e_trade_only: bool,
    pub firm_quote_only: bool,
    pub nbbo_price_cap: f64,
    // pub auction_strategy: AuctionStrategy,
    // type: int; AuctionMatch, AuctionImprovement, AuctionTransparent
    pub starting_price: f64,
    // type: float
    pub stock_ref_price: f64,
    // type: float
    pub delta: f64, // type: float
    // pegged to stock and VOL orders only
    pub stock_range_lower: f64,
    // type: float
    pub stock_range_upper: f64, // type: float
    // type: float; REL orders only
    pub override_percentage_constraints: bool,
    // VOLATILITY ORDERS ONLY
    pub volatility: f64,
    // type: float
    pub volatility_type: i32,
    // type: int   // 1=daily, 2=annual
    pub delta_neutral_order_type: String,
    pub delta_neutral_aux_price: f64,
    // type: float
    pub delta_neutral_con_id: i32,
    pub delta_neutral_settling_firm: String,
    pub delta_neutral_clearing_account: String,
    pub delta_neutral_clearing_intent: String,
    pub delta_neutral_open_close: String,
    pub delta_neutral_short_sale: bool,
    pub delta_neutral_short_sale_slot: i32,
    pub delta_neutral_designated_location: String,
    pub continuous_update: bool,
    pub reference_price_type: i32, // type: int; 1=Average, 2 = BidOrAsk
    pub trail_stop_price: f64,
    // type: float
    pub trailing_percent: f64, // type: float; TRAILLIMIT orders only

    // type: float
    pub opt_out_smart_routing: bool,

    // BOX exchange orders only
    pub randomize_price: bool,
    pub randomize_size: bool,

    // COMBO ORDERS ONLY
    pub basis_points: f64,
    // type: float; EFP orders only
    pub basis_points_type: i32, // type: int;  EFP orders only

    // SCALE ORDERS ONLY
    pub scale_init_level_size: i32,
    // type: int
    pub scale_subs_level_size: i32,
    // type: int
    pub scale_price_increment: f64,
    // type: float
    pub scale_price_adjust_value: f64,
    // type: float
    pub scale_price_adjust_interval: i32,
    // type: int
    pub scale_profit_offset: f64,
    // type: float
    pub scale_auto_reset: bool,
    pub scale_init_position: i32,
    // type: int
    pub scale_init_fill_qty: i32,
    // type: int
    pub scale_random_percent: bool,
    pub scale_table: String,

    // HEDGE ORDERS
    pub hedge_type: String,
    // 'D' - delta, 'B' - beta, 'F' - FX, 'P' - pair
    pub hedge_param: String, // 'beta=X' value for beta hedge, 'ratio=Y' for pair hedge

    // IB account
    pub clearing_account: String,
    //True beneficiary of the order
    pub clearing_intent: String, // "" (Default), "IB", "Away", "PTA" (PostTrade)

    // ALGO ORDERS ONLY
    pub algo_strategy: String,
    pub algo_params: Vec<TagValue>,
    //TagValueList
    pub smart_combo_routing_params: Vec<TagValue>, //TagValueList
    pub algo_id: String,

    // What-if
    pub what_if: bool,

    // Not Held
    pub not_held: bool,

    // order combo legs
    // pub order_combo_legs: Vec<OrderComboLeg>, // OrderComboLegListSPtr

    pub order_misc_options: Vec<TagValue>, // TagValueList

    // VER PEG2BENCH fields:
    pub reference_contract_id: i32,
    pub pegged_change_amount: f64,
    pub is_pegged_change_amount_decrease: bool,
    pub reference_change_amount: f64,
    pub reference_exchange_id: String,
    pub adjusted_order_type: String,

    pub trigger_price: f64,
    pub adjusted_stop_price: f64,
    pub adjusted_stop_limit_price: f64,
    pub adjusted_trailing_amount: f64,
    pub adjustable_trailing_unit: i32,
    pub lmt_price_offset: f64,

    // pub conditions: Vec<OrderConditionEnum>,
    // std::vector<std::shared_ptr<OrderCondition>>
    pub conditions_cancel_order: bool,
    pub conditions_ignore_rth: bool,

    // ext operator
    pub ext_operator: String,

    // pub soft_dollar_tier: SoftDollarTier,
    // native cash quantity
    pub cash_qty: f64,

    pub mifid2decision_maker: String,
    pub mifid2decision_algo: String,
    pub mifid2execution_trader: String,
    pub mifid2execution_algo: String,

    pub dont_use_auto_price_for_hedge: bool,

    pub is_oms_container: bool,

    pub discretionary_up_to_limit_price: bool,

    pub auto_cancel_date: String,
    pub filled_quantity: f64,
    pub ref_futures_con_id: i32,
    pub auto_cancel_parent: bool,
    pub shareholder: String,
    pub imbalance_only: bool,
    pub route_marketable_to_bbo: bool,
    pub parent_perm_id: i32,

    pub use_price_mgmt_algo: bool,
}

/// Identifies the side.
/// Generally available values are BUY and SELL.
/// Additionally, SSHORT and SLONG are available in some institutional-accounts only.
/// For general account types, a SELL order will be able to enter a short position automatically if the order quantity is larger than your current long position.
/// SSHORT is only supported for institutional account configured with Long/Short account segments or clearing with a separate account.
/// SLONG is available in specially-configured institutional accounts to indicate that long position not yet delivered is being sold.
#[derive(Clone, Debug)]
pub enum Action {
    BUY,
    SELL,
    /// SSHORT is only supported for institutional account configured with Long/Short account segments or clearing with a separate account.
    SSHORT,
    /// SLONG is available in specially-configured institutional accounts to indicate that long position not yet delivered is being sold.
    SLONG,
}

#[derive(Clone, Debug)]
pub enum Rule80A {
    None,
    Individual,
    Agency,
    AgentOtherMember,
    IndividualPTIA,
    AgencyPTIA,
    AgentOtherMemberPTIA,
    IndividualPT,
    AgencyPT,
    AgentOtherMemberPT,
}
