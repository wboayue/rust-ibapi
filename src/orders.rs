use std::fmt::Debug;

use anyhow::{anyhow, Result};

use crate::client::{Client, RequestPacket, ResponsePacket};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::server_versions;

/// New description
pub use crate::contracts::TagValue;

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
    pub tif: String, // FIXME create enum
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
    /// Overrides TWS constraints.
    /// Precautionary constraints are defined on the TWS Presets page, and help ensure tha tyour price and size order values are reasonable. Orders sent from the API are also validated against these safety constraints, and may be rejected if any constraint is violated. To override validation, set this parameter’s value to True.
    pub override_percentage_constraints: bool,
    /// Individual = 'I', Agency = 'A', AgentOtherMember = 'W', IndividualPTIA = 'J', AgencyPTIA = 'U', AgentOtherMemberPTIA = 'M', IndividualPT = 'K', AgencyPT = 'Y', AgentOtherMemberPT = 'N'
    pub rule_80_a: Rule80A,
    /// Indicates whether or not all the order has to be filled on a single execution.
    pub all_or_none: bool,
    /// Identifies a minimum quantity order type.
    pub min_qty: i32,
    /// The percent offset amount for relative orders.
    pub percent_offset: f64,
    /// Trail stop price for TRAIL LIMIT orders.
    pub trail_stop_price: f64,
    /// Specifies the trailing amount of a trailing stop order as a percentage.
    /// Observe the following guidelines when using the trailingPercent field:
    ///
    /// This field is mutually exclusive with the existing trailing amount. That is, the API client can send one or the other but not both.
    /// This field is read AFTER the stop price (barrier price) as follows: deltaNeutralAuxPrice stopPrice, trailingPercent, scale order attributes
    /// The field will also be sent to the API in the openOrder message if the API client version is >= 56. It is sent after the stopPrice field as follows: stopPrice, trailingPct, basisPoint.    
    pub trailing_percent: f64,
    /// The Financial Advisor group the trade will be allocated to. Use an empty string if not applicable.
    pub fa_group: String,
    /// The Financial Advisor allocation profile the trade will be allocated to. Use an empty string if not applicable.
    pub fa_profile: String,
    /// The Financial Advisor allocation method the trade will be allocated to. Use an empty string if not applicable.
    pub fa_method: String,
    /// The Financial Advisor percentage concerning the trade's allocation. Use an empty string if not applicable.
    pub fa_percentage: String,
    /// For institutional customers only. Valid values are O (open) and C (close).
    /// Available for institutional clients to determine if this order is to open or close a position.
    /// When Action = "BUY" and OpenClose = "O" this will open a new position.
    /// When Action = "BUY" and OpenClose = "C" this will close and existing short position.    
    pub open_close: String,
    /// The order's origin. Same as TWS "Origin" column. Identifies the type of customer from which the order originated.
    /// Valid values are:
    /// 0 - Customer
    /// 1 - Firm.
    pub origin: i32,
    /// For institutions only.
    /// Valid values are:
    /// 1 - Broker holds shares
    /// 2 - Shares come from elsewhere.    
    pub short_sale_slot: i32,
    /// For institutions only. Indicates the location where the shares to short come from. Used only when short sale slot is set to 2 (which means that the shares to short are held elsewhere and not with IB).
    pub designated_location: String,
    /// Only available with IB Execution-Only accounts with applicable securities.
    /// Mark order as exempt from short sale uptick rule.
    pub exempt_code: i32,
    /// The amount off the limit price allowed for discretionary orders.
    pub discretionary_amt: f64,
    /// Use to opt out of default SmartRouting for orders routed directly to ASX.
    /// This attribute defaults to false unless explicitly set to true.
    /// When set to false, orders routed directly to ASX will NOT use SmartRouting.
    /// When set to true, orders routed directly to ASX orders WILL use SmartRouting.
    pub opt_out_smart_routing: bool,
    /// For BOX orders only.
    /// Values include:
    /// 1 - Match
    /// 2 - Improvement
    /// 3 - Transparent.
    pub auction_strategy: i32, // FIXME enum
    /// The auction's starting price. For BOX orders only.
    pub starting_price: f64,
    /// The stock's reference price.
    /// The reference price is used for VOL orders to compute the limit price sent to an exchange (whether or not Continuous Update is selected), and for price range monitoring.    
    pub stock_ref_price: f64,
    /// The stock's Delta. For orders on BOX only.
    pub delta: f64,
    /// The lower value for the acceptable underlying stock price range.
    /// For price improvement option orders on BOX and VOL orders with dynamic management.    
    pub stock_range_lower: f64,
    /// The upper value for the acceptable underlying stock price range.
    /// For price improvement option orders on BOX and VOL orders with dynamic management.
    pub stock_range_upper: f64,
    /// The option price in volatility, as calculated by TWS' Option Analytics.
    /// This value is expressed as a percent and is used to calculate the limit price sent to the exchange.
    pub volatility: f64,
    /// Values include:
    /// 1 - Daily Volatility
    /// 2 - Annual Volatility.
    pub volatility_type: i32, // FIXM enum
    /// Specifies whether TWS will automatically update the limit price of the order as the underlying price moves. VOL orders only.
    pub continuous_update: bool,
    /// Specifies how you want TWS to calculate the limit price for options, and for stock range price monitoring.
    /// VOL orders only.
    /// Valid values include:
    /// 1 - Average of NBBO
    /// 2 - NBB or the NBO depending on the action and right.
    pub reference_price_type: i32,
    /// Enter an order type to instruct TWS to submit a delta neutral trade on full or partial execution of the VOL order. VOL orders only. For no hedge delta order to be sent, specify NONE.
    pub delta_neutral_order_type: String,
    /// Use this field to enter a value if the value in the deltaNeutralOrderType field is an order type that requires an Aux price, such as a REL order. VOL orders only.
    pub delta_neutral_aux_price: f64,
    /// The unique contract identifier specifying the security in Delta Neutral order.
    pub delta_neutral_con_id: i32,
    /// Indicates the firm which will settle the Delta Neutral trade. Institutions only.
    pub delta_neutral_settling_firm: String,
    /// Specifies the beneficiary of the Delta Neutral order.
    pub delta_neutral_clearing_account: String,
    /// Specifies where the clients want their shares to be cleared at. Must be specified by execution-only clients.
    /// Valid values are:
    /// IB, Away, and PTA (post trade allocation).
    pub delta_neutral_clearing_intent: String,
    /// Specifies whether the order is an Open or a Close order and is used when the hedge involves a CFD and and the order is clearing away.
    pub delta_neutral_open_close: String,
    /// Used when the hedge involves a stock and indicates whether or not it is sold short.
    pub delta_neutral_short_sale: bool,
    /// Indicates a short sale Delta Neutral order. Has a value of 1 (the clearing broker holds shares) or 2 (delivered from a third party). If you use 2, then you must specify a deltaNeutralDesignatedLocation.
    pub delta_neutral_short_sale_slot: i32,
    /// Identifies third party order origin. Used only when deltaNeutralShortSaleSlot = 2.
    pub delta_neutral_designated_location: String,
    /// Specifies Basis Points for EFP order. The values increment in 0.01% = 1 basis point. For EFP orders only.
    pub basis_points: f64,
    /// Specifies the increment of the Basis Points. For EFP orders only.
    pub basis_points_type: i32,
    /// Defines the size of the first, or initial, order component. For Scale orders only.
    pub scale_init_level_size: i32,
    /// Defines the order size of the subsequent scale order components. For Scale orders only. Used in conjunction with scaleInitLevelSize().
    pub scale_subs_level_size: i32,
    /// Defines the price increment between scale components. For Scale orders only. This value is compulsory.
    pub scale_price_increment: f64,
    /// Modifies the value of the Scale order. For extended Scale orders.
    pub scale_price_adjust_value: f64,
    /// Specifies the interval when the price is adjusted. For extended Scale orders.
    pub scale_price_adjust_interval: i32,
    /// Specifies the offset when to adjust profit. For extended scale orders.
    pub scale_profit_offset: f64,
    /// Restarts the Scale series if the order is cancelled. For extended scale orders.
    pub scale_auto_reset: bool,
    /// The initial position of the Scale order. For extended scale orders.
    pub scale_init_position: i32,
    /// Specifies the initial quantity to be filled. For extended scale orders.
    pub scale_init_fill_qty: i32,
    /// Defines the random percent by which to adjust the position. For extended scale orders.
    pub scale_random_percent: bool,
    /// For hedge orders.
    /// Possible values include:
    /// D - Delta
    /// B - Beta
    /// F - FX
    /// P - Pair
    pub hedge_type: String,
    /// For hedge orders.
    /// Beta = x for Beta hedge orders, ratio = y for Pair hedge order
    pub hedge_param: String,
    /// The account the trade will be allocated to.    
    pub account: String,
    /// Indicates the firm which will settle the trade. Institutions only.
    pub settling_firm: String,
    /// Specifies the true beneficiary of the order.
    /// For IBExecution customers. This value is required for FUT/FOP orders for reporting to the exchange.
    pub clearing_account: String,
    /// For execution-only clients to know where do they want their shares to be cleared at.
    /// Valid values are:
    /// IB, Away, and PTA (post trade allocation).
    pub clearing_intent: String,
    /// The algorithm strategy.
    /// As of API verion 9.6, the following algorithms are supported:
    /// ArrivalPx - Arrival Price
    /// DarkIce - Dark Ice
    /// PctVol - Percentage of Volume
    /// Twap - TWAP (Time Weighted Average Price)
    /// Vwap - VWAP (Volume Weighted Average Price)
    /// For more information about IB's API algorithms, refer to [https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm](https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm)
    pub algo_strategy: String,
    /// The list of parameters for the IB algorithm.
    /// For more information about IB's API algorithms, refer to [https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm](https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm)
    pub algo_params: Vec<TagValue>,
    /// Allows to retrieve the commissions and margin information.
    /// When placing an order with this attribute set to true, the order will not be placed as such. Instead it will used to request the commissions and margin information that would result from this order.
    pub what_if: bool,
    /// Identifies orders generated by algorithmic trading.
    pub algo_id: String,
    /// Orders routed to IBDARK are tagged as “post only” and are held in IB's order book, where incoming SmartRouted orders from other IB customers are eligible to trade against them.
    /// For IBDARK orders only.
    pub not_held: bool,
    /// Advanced parameters for Smart combo routing.
    /// These features are for both guaranteed and nonguaranteed combination orders routed to Smart, and are available based on combo type and order type. SmartComboRoutingParams is similar to AlgoParams in that it makes use of tag/value pairs to add parameters to combo orders.
    /// Make sure that you fully understand how Advanced Combo Routing works in TWS itself first: <https://guides.interactivebrokers.com/tws/twsguide.htm#usersguidebook/specializedorderentry/advanced_combo_routing.htm>
    /// The parameters cover the following capabilities:
    ///
    /// * Non-Guaranteed - Determine if the combo order is Guaranteed or Non-Guaranteed.
    ///   <br/>Tag = NonGuaranteed
    ///   <br/>Value = 0: The order is guaranteed
    ///   <br/>Value = 1: The order is non-guaranteed
    ///
    /// * Select Leg to Fill First - User can specify which leg to be executed first.
    ///   <br/>Tag = LeginPrio
    ///   <br/>Value = -1: No priority is assigned to either combo leg
    ///   <br/>Value = 0: Priority is assigned to the first leg being added to the comboLeg
    ///   <br/>Value = 1: Priority is assigned to the second leg being added to the comboLeg
    ///   <br/>Note: The LeginPrio parameter can only be applied to two-legged combo.
    ///
    /// * Maximum Leg-In Combo Size - Specify the maximum allowed leg-in size per segment
    ///   <br/>Tag = MaxSegSize
    ///   <br/>Value = Unit of combo size
    ///
    /// * Do Not Start Next Leg-In if Previous Leg-In Did Not Finish - Specify whether or not the system should attempt to fill the next segment before the current segment fills.
    ///   <br/>Tag = DontLeginNext
    ///   <br/>Value = 0: Start next leg-in even if previous leg-in did not finish
    ///   <br/>Value = 1: Do not start next leg-in if previous leg-in did not finish
    ///
    /// * Price Condition - Combo order will be rejected or cancelled if the leg market price is outside of the specified price range [CondPriceMin, CondPriceMax]
    ///   <br/>Tag = PriceCondConid: The ContractID of the combo leg to specify price condition on
    ///   <br/>Value = The ContractID
    ///   <br/>Tag = CondPriceMin: The lower price range of the price condition
    ///   <br/>Value = The lower price
    ///   <br/>Tag = CondPriceMax: The upper price range of the price condition
    ///   <br/>Value = The upper price    
    pub smart_combo_routing_params: Vec<TagValue>,
    /// List of Per-leg price following the same sequence combo legs are added. The combo price must be left unspecified when using per-leg prices.
    pub order_combo_legs: Vec<OrderComboLeg>,
    /// For internal use only. Use the default value XYZ.
    pub order_misc_options: Vec<TagValue>,
    /// Defines the start time of GTC orders.
    pub active_start_time: String,
    /// Defines the stop time of GTC orders.
    pub active_stop_time: String,
    /// The list of scale orders. Used for scale orders.
    pub scale_table: String,
    /// Is used to place an order to a model. For example, "Technology" model can be used for tech stocks first created in TWS.
    pub model_code: String,
    /// This is a regulartory attribute that applies to all US Commodity (Futures) Exchanges, provided to allow client to comply with CFTC Tag 50 Rules.
    pub ext_operator: String,
    /// The native cash quantity.
    pub cash_qty: f64,
    /// Identifies a person as the responsible party for investment decisions within the firm. Orders covered by MiFID 2 (Markets in Financial Instruments Directive 2) must include either Mifid2DecisionMaker or Mifid2DecisionAlgo field (but not both). Requires TWS 969+.
    pub mifid2_decision_maker: String,
    /// Identifies the algorithm responsible for investment decisions within the firm. Orders covered under MiFID 2 must include either Mifid2DecisionMaker or Mifid2DecisionAlgo, but cannot have both. Requires TWS 969+.
    pub mifid2_decision_algo: String,
    /// For MiFID 2 reporting; identifies a person as the responsible party for the execution of a transaction within the firm. Requires TWS 969+.
    pub mifid2_execution_trader: String,
    /// For MiFID 2 reporting; identifies the algorithm responsible for the execution of a transaction within the firm. Requires TWS 969+.
    pub mifid2_execution_algo: String,
    /// Don't use auto price for hedge.
    pub dont_use_auto_price_for_hedge: bool,
    /// Specifies the date to auto cancel the order.
    pub auto_cancel_date: String, // TODO date object
    /// Specifies the initial order quantity to be filled.
    pub filled_quantity: f64,
    /// Identifies the reference future conId.
    pub ref_futures_con_id: i32,
    /// Cancels the parent order if child order was cancelled.
    pub auto_cancel_parent: bool,
    /// Identifies the Shareholder.
    pub shareholder: String,
    /// Used to specify "imbalance only open orders" or "imbalance only closing orders".
    pub imbalance_only: bool,
    /// Routes market order to Best Bid Offer.
    pub route_marketable_to_bbo: bool,
    /// Parent order Id.
    pub parent_perm_id: i32,
    /// Accepts a list with parameters obtained from advancedOrderRejectJson.
    pub advanced_error_override: String,
    /// Used by brokers and advisors when manually entering, modifying or cancelling orders at the direction of a client. Only used when allocating orders to specific groups or accounts. Excluding "All" group.
    pub manual_order_time: String,
    /// Defines the minimum trade quantity to fill. For IBKRATS orders.
    pub min_trade_qty: Option<i32>,
    /// Defines the minimum size to compete. For IBKRATS orders.
    pub min_complete_size: Option<i32>,
    /// Specifies the offset off the midpoint that will be applied to the order. For IBKRATS orders.
    pub compete_against_best_offet: Option<f64>,
    /// his offset is applied when the spread is an even number of cents wide. This offset must be in whole-penny increments or zero. For IBKRATS orders.
    pub mid_offset_at_whole: Option<f64>,
    /// This offset is applied when the spread is an odd number of cents wide. This offset must be in half-penny increments. For IBKRATS orders.
    pub mid_offset_at_half: Option<f64>,
    /// Randomizes the order's size. Only for Volatility and Pegged to Volatility orders.
    pub randomize_size: bool,
    /// Randomizes the order's price. Only for Volatility and Pegged to Volatility orders.
    pub randomize_price: bool,
    /// Pegged-to-benchmark orders: this attribute will contain the conId of the contract against which the order will be pegged.
    pub reference_contract_id: i32,
    /// Pegged-to-benchmark orders: indicates whether the order's pegged price should increase or decreases.
    pub is_pegged_change_amount_decrease: bool,
    /// Pegged-to-benchmark orders: amount by which the order's pegged price should move.
    pub pegged_change_amount: f64,
    /// Pegged-to-benchmark orders: the amount the reference contract needs to move to adjust the pegged order.
    pub reference_change_amount: f64,
    /// Pegged-to-benchmark orders: the exchange against which we want to observe the reference contract.
    pub reference_exchange: String,
    /// Adjusted Stop orders: the parent order will be adjusted to the given type when the adjusted trigger price is penetrated.
    pub adjusted_order_type: String,
    /// Adjusted Stop orders: specifies the trigger price to execute.
    pub trigger_price: f64,
    /// Adjusted Stop orders: specifies the price offset for the stop to move in increments.
    pub lmt_price_offset: f64,
    /// Adjusted Stop orders: specifies the stop price of the adjusted (STP) parent.
    pub adjusted_stop_price: f64,
    /// Adjusted Stop orders: specifies the stop limit price of the adjusted (STPL LMT) parent.
    pub adjusted_stop_limit_price: f64,
    /// Adjusted Stop orders: specifies the trailing amount of the adjusted (TRAIL) parent.
    pub adjusted_trailing_amount: f64,
    /// Adjusted Stop orders: specifies where the trailing unit is an amount (set to 0) or a percentage (set to 1)
    pub adjustable_trailing_unit: i32,
    /// Conditions determining when the order will be activated or canceled.
    pub conditions: Vec<OrderCondition>,
    /// Indicates whether or not conditions will also be valid outside Regular Trading Hours.
    pub conditions_ignore_rth: bool,
    /// Conditions can determine if an order should become active or canceled.
    pub conditions_cancel_order: bool,
    /// Define the Soft Dollar Tier used for the order. Only provided for registered professional advisors and hedge and mutual funds.
    pub soft_dollar_tier: SoftDollarTier,
    /// Set to true to create tickets from API orders when TWS is used as an OMS.
    pub is_oms_container: bool,
    /// Set to true to convert order of type 'Primary Peg' to 'D-Peg'.
    pub discretionary_up_to_limit_price: bool,
    /// Specifies wether to use Price Management Algo. CTCI users only.
    pub use_price_mgmt_algo: bool,
    /// Specifies the duration of the order. Format: yyyymmdd hh:mm:ss TZ. For GTD orders.
    pub duration: i32, // TODO date object?
    /// Value must be positive, and it is number of seconds that SMART order would be parked for at IBKRATS before being routed to exchange.
    pub post_to_ats: i32,
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

#[derive(Clone, Debug)]
pub struct OrderComboLeg {
    price: f64,
}

impl Default for OrderComboLeg {
    fn default() -> Self {
        Self { price: f64::MAX }
    }
}

#[derive(Clone, Debug)]
pub enum OrderCondition {
    Price,
    Time,
    Margin,
    Execution,
    Volume,
    PercentChange,
}

// Price = 1,
// Time = 3,
// Margin = 4,
// Execution = 5,
// Volume = 6,
// PercentCange = 7

#[derive(Clone, Debug, Default)]
pub struct SoftDollarTier {
    name: String,
    value: String,
    display_name: String,
}

pub struct OrderState {
    /// The order's current status
    status: String,
    /// The account's current initial margin.
    initial_margin_before: String,
    /// The account's current maintenance margin
    maintenance_margin_before: String,
    /// The account's current equity with loan
    equity_with_loan_before: String,
    /// The change of the account's initial margin.
    initial_margin_change: String,
    /// The change of the account's maintenance margin
    maintenance_margin_change: String,
    /// The change of the account's equity with loan
    equity_with_loan_change: String,
    /// The order's impact on the account's initial margin.
    initial_margin_after: String,
    /// The order's impact on the account's maintenance margin
    maintenance_margin_after: String,
    /// Shows the impact the order would have on the account's equity with loan
    equity_with_loan_after: String,
    /// The order's generated commission.
    commission: f64,
    // The execution's minimum commission.
    minimum_commission: f64,
    /// The executions maximum commission.
    maximum_commission: f64,
    /// The generated commission currency
    commission_currency: String,
    /// If the order is warranted, a descriptive message will be provided.
    warning_text: String,
    completed_time: String,
    completed_status: String,
}

// place_order
pub fn place_order<C: Client + Debug>(
    client: &mut C,
    order_id: i32,
    contract: &Contract,
    order: &Order,
) -> Result<()> {
    verify_order(client, order, order_id)?;
    verify_order_contract(contract, order_id)?;

    let request_id = client.next_request_id();
    let message = encode_place_order(
        client.server_version(),
        request_id,
        order_id,
        contract,
        order,
    )?;

    // let responses = client.send_message(request_id, packet)?;

    // let mut contract_details: Vec<ContractDetails> = Vec::default();

    // for mut message in responses {
    //     match message.message_type() {
    //         IncomingMessages::ContractData => {
    //             let decoded = decode_contract_details(client.server_version(), &mut message)?;
    //             contract_details.push(decoded);
    //         }
    //         IncomingMessages::ContractDataEnd => {
    //             break;
    //         }
    //         IncomingMessages::Error => {
    //             error!("error: {:?}", message);
    //             return Err(anyhow!("contract_details {:?}", message));
    //         }
    //         _ => {
    //             error!("unexpected message: {:?}", message);
    //         }
    //     }
    // }

    Ok(())
}

fn verify_order<C: Client + Debug>(client: &mut C, order: &Order, order_id: i32) -> Result<()> {
    let is_bag_order: bool = true; // StringsAreEqual(Constants.BagSecType, contract.SecType)

    if order.scale_init_level_size != i32::MAX || order.scale_price_increment != f64::MAX {
        client.check_server_version(
            server_versions::SCALE_ORDERS,
            "It does not support Scale orders.",
        )?
    }

    if order.what_if {
        client.check_server_version(
            server_versions::WHAT_IF_ORDERS,
            "It does not support what-if orders.",
        )?
    }

    if order.scale_subs_level_size != i32::MAX {
        client.check_server_version(
            server_versions::SCALE_ORDERS2,
            "It does not support Subsequent Level Size for Scale orders.",
        )?
    }

    if !order.algo_strategy.is_empty() {
        client.check_server_version(
            server_versions::ALGO_ORDERS,
            "It does not support algo orders.",
        )?
    }

    if order.not_held {
        client.check_server_version(
            server_versions::NOT_HELD,
            "It does not support notHeld parameter.",
        )?
    }

    if order.exempt_code != -1 {
        client.check_server_version(
            server_versions::SSHORTX,
            "It does not support exemptCode parameter.",
        )?
    }

    if !order.hedge_type.is_empty() {
        client.check_server_version(
            server_versions::HEDGE_ORDERS,
            "It does not support hedge orders.",
        )?
    }

    if order.opt_out_smart_routing {
        client.check_server_version(
            server_versions::OPT_OUT_SMART_ROUTING,
            "It does not support optOutSmartRouting parameter.",
        )?
    }

    if order.delta_neutral_con_id > 0
        || !order.delta_neutral_settling_firm.is_empty()
        || !order.delta_neutral_clearing_account.is_empty()
        || !order.delta_neutral_clearing_intent.is_empty()
    {
        client.check_server_version(
                server_versions::DELTA_NEUTRAL_CONID,
                "It does not support deltaNeutral parameters: ConId, SettlingFirm, ClearingAccount, ClearingIntentIt does not support deltaNeutral parameters: ConId, SettlingFirm, ClearingAccount, ClearingIntentIt does not support deltaNeutral parameters: ConId, SettlingFirm, ClearingAccount, ClearingIntent",
            )?
    }

    if !order.delta_neutral_open_close.is_empty()
        || order.delta_neutral_short_sale
        || order.delta_neutral_short_sale_slot > 0
        || !order.delta_neutral_designated_location.is_empty()
    {
        client.check_server_version(
                server_versions::DELTA_NEUTRAL_OPEN_CLOSE,
                "It does not support deltaNeutral parameters: OpenClose, ShortSale, ShortSaleSlot, DesignatedLocation",
            )?
    }

    if (order.scale_price_increment > 0.0 && order.scale_price_increment != f64::MAX)
        && (order.scale_price_adjust_value != f64::MAX
            || order.scale_price_adjust_interval != i32::MAX
            || order.scale_profit_offset != f64::MAX
            || order.scale_auto_reset
            || order.scale_init_position != i32::MAX
            || order.scale_init_fill_qty != i32::MAX
            || order.scale_random_percent)
    {
        client.check_server_version(
                server_versions::SCALE_ORDERS3,
                "It does not support Scale order parameters: PriceAdjustValue, PriceAdjustInterval, ProfitOffset, AutoReset, InitPosition, InitFillQty and RandomPercent",
            )?
    }

    if is_bag_order && order.order_combo_legs.iter().any(|l| l.price != f64::MAX) {
        client.check_server_version(
            server_versions::ORDER_COMBO_LEGS_PRICE,
            "It does not support per-leg prices for order combo legs.",
        )?
    }

    if order.trailing_percent != f64::MAX {
        client.check_server_version(
            server_versions::TRAILING_PERCENT,
            "It does not support trailing percent parameter.",
        )?
    }

    if !order.algo_id.is_empty() {
        client.check_server_version(
            server_versions::ALGO_ID,
            "It does not support algo_id parameter",
        )?
    }

    if !order.scale_table.is_empty()
        || !order.active_start_time.is_empty()
        || !order.active_stop_time.is_empty()
    {
        client.check_server_version(
            server_versions::SCALE_TABLE,
            "It does not support scale_table, active_start_time nor active_stop_time parameters.",
        )?
    }

    if !order.ext_operator.is_empty() {
        client.check_server_version(
            server_versions::EXT_OPERATOR,
            "It does not support ext_operator parameter",
        )?
    }

    if order.cash_qty != f64::MAX {
        client.check_server_version(
            server_versions::CASH_QTY,
            "It does not support cash_qty parameter",
        )?
    }

    if !order.mifid2_execution_trader.is_empty() || !order.mifid2_execution_algo.is_empty() {
        client.check_server_version(
            server_versions::DECISION_MAKER,
            "It does not support MIFID II execution parameters",
        )?
    }

    if order.dont_use_auto_price_for_hedge {
        client.check_server_version(
            server_versions::AUTO_PRICE_FOR_HEDGE,
            "It does not support don't use auto price for hedge parameter",
        )?
    }

    if order.is_oms_container {
        client.check_server_version(
            server_versions::ORDER_CONTAINER,
            "It does not support oms container parameter",
        )?
    }

    if order.discretionary_up_to_limit_price {
        client.check_server_version(
            server_versions::D_PEG_ORDERS,
            "It does not support D-Peg orders",
        )?
    }

    if order.use_price_mgmt_algo {
        client.check_server_version(
            server_versions::PRICE_MGMT_ALGO,
            "It does not support Use Price Management Algo requests",
        )?
    }

    if order.duration != i32::MAX {
        client.check_server_version(
            server_versions::DURATION,
            "It does not support duration attribute",
        )?
    }

    if order.post_to_ats != i32::MAX {
        client.check_server_version(
            server_versions::POST_TO_ATS,
            "It does not support post_to_ats attribute",
        )?
    }

    if order.auto_cancel_parent {
        client.check_server_version(
            server_versions::AUTO_CANCEL_PARENT,
            "It does not support auto_cancel_parent attribute",
        )?
    }

    if !order.advanced_error_override.is_empty() {
        client.check_server_version(
            server_versions::ADVANCED_ORDER_REJECT,
            "It does not support advanced error override attribute",
        )?
    }

    if !order.manual_order_time.is_empty() {
        client.check_server_version(
            server_versions::MANUAL_ORDER_TIME,
            "It does not support manual order time attribute",
        )?
    }

    if order.min_trade_qty.is_some()
        || order.min_complete_size.is_some()
        || order.compete_against_best_offet.is_some()
        || order.mid_offset_at_whole.is_some()
        || order.mid_offset_at_half.is_some()
    {
        client.check_server_version(
            server_versions::PEGBEST_PEGMID_OFFSETS,
            "It does not support PEG BEST / PEG MID order parameters: minTradeQty, minCompeteSize, competeAgainstBestOffset, midOffsetAtWhole and midOffsetAtHalf",
        )?
    }

    Ok(())
}

fn verify_order_contract(contract: &Contract, order_id: i32) -> Result<()> {
    Ok(())
}

fn encode_place_order(
    server_version: i32,
    request_id: i32,
    order_id: i32,
    contract: &Contract,
    order: &Order,
) -> Result<RequestPacket> {
    let mut message = RequestPacket::default();
    let message_version = if server_version < server_versions::NOT_HELD {
        27
    } else {
        45
    };

    message.add_field(&OutgoingMessages::PlaceOrder);

    if server_version >= server_versions::ORDER_CONTAINER {
        message.add_field(&message_version);
    }

    message.add_field(&order_id);

    // https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EClient.cs#L783

    Ok(message)
}

// cancel_order
pub fn cancel_order<C: Client + Debug>(client: &mut C, order_id: i32) -> Result<()> {
    Ok(())
}

pub fn check_order_status<C: Client + Debug>() {}
