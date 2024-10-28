use std::convert::From;
use std::fmt::Debug;

use log::{error, info};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::{ResponseContext, Subscribable, Subscription};
use crate::contracts::{ComboLeg, ComboLegOpenClose, Contract, DeltaNeutralContract, SecurityType};
use crate::messages::{IncomingMessages, Notice, OutgoingMessages};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::transport::InternalSubscription;
use crate::Client;
use crate::{encode_option_field, ToField};
use crate::{server_versions, Error};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

/// Make sure to test using only your paper trading account when applicable. A good way of finding out if an order type/exchange combination
/// is possible is by trying to place such order manually using the TWS.
/// Before contacting our API support team please refer to the available documentation.
pub mod order_builder;

/// New description
pub use crate::contracts::TagValue;

const COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID: Option<f64> = Some(f64::INFINITY);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub limit_price: Option<f64>,
    /// Generic field to contain the stop price for STP LMT orders, trailing amount, etc.
    pub aux_price: Option<f64>,
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
    pub display_size: Option<i32>,
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
    pub rule_80_a: Option<Rule80A>,
    /// Indicates whether or not all the order has to be filled on a single execution.
    pub all_or_none: bool,
    /// Identifies a minimum quantity order type.
    pub min_qty: Option<i32>,
    /// The percent offset amount for relative orders.
    pub percent_offset: Option<f64>,
    /// Trail stop price for TRAIL LIMIT orders.
    pub trail_stop_price: Option<f64>,
    /// Specifies the trailing amount of a trailing stop order as a percentage.
    /// Observe the following guidelines when using the trailingPercent field:
    ///
    /// This field is mutually exclusive with the existing trailing amount. That is, the API client can send one or the other but not both.
    /// This field is read AFTER the stop price (barrier price) as follows: deltaNeutralAuxPrice stopPrice, trailingPercent, scale order attributes
    /// The field will also be sent to the API in the openOrder message if the API client version is >= 56. It is sent after the stopPrice field as follows: stopPrice, trailingPct, basisPoint.    
    pub trailing_percent: Option<f64>,
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
    pub open_close: Option<OrderOpenClose>,
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
    pub auction_strategy: Option<i32>, // FIXME enum
    /// The auction's starting price. For BOX orders only.
    pub starting_price: Option<f64>,
    /// The stock's reference price.
    /// The reference price is used for VOL orders to compute the limit price sent to an exchange (whether or not Continuous Update is selected), and for price range monitoring.    
    pub stock_ref_price: Option<f64>,
    /// The stock's Delta. For orders on BOX only.
    pub delta: Option<f64>,
    /// The lower value for the acceptable underlying stock price range.
    /// For price improvement option orders on BOX and VOL orders with dynamic management.    
    pub stock_range_lower: Option<f64>,
    /// The upper value for the acceptable underlying stock price range.
    /// For price improvement option orders on BOX and VOL orders with dynamic management.
    pub stock_range_upper: Option<f64>,
    /// The option price in volatility, as calculated by TWS' Option Analytics.
    /// This value is expressed as a percent and is used to calculate the limit price sent to the exchange.
    pub volatility: Option<f64>,
    /// Values include:
    /// 1 - Daily Volatility
    /// 2 - Annual Volatility.
    pub volatility_type: Option<i32>, // FIXM enum
    /// Specifies whether TWS will automatically update the limit price of the order as the underlying price moves. VOL orders only.
    pub continuous_update: bool,
    /// Specifies how you want TWS to calculate the limit price for options, and for stock range price monitoring.
    /// VOL orders only.
    /// Valid values include:
    /// 1 - Average of NBBO
    /// 2 - NBB or the NBO depending on the action and right.
    pub reference_price_type: Option<i32>,
    /// Enter an order type to instruct TWS to submit a delta neutral trade on full or partial execution of the VOL order. VOL orders only. For no hedge delta order to be sent, specify NONE.
    pub delta_neutral_order_type: String,
    /// Use this field to enter a value if the value in the deltaNeutralOrderType field is an order type that requires an Aux price, such as a REL order. VOL orders only.
    pub delta_neutral_aux_price: Option<f64>,
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
    pub basis_points: Option<f64>,
    /// Specifies the increment of the Basis Points. For EFP orders only.
    pub basis_points_type: Option<i32>,
    /// Defines the size of the first, or initial, order component. For Scale orders only.
    pub scale_init_level_size: Option<i32>,
    /// Defines the order size of the subsequent scale order components. For Scale orders only. Used in conjunction with scaleInitLevelSize().
    pub scale_subs_level_size: Option<i32>,
    /// Defines the price increment between scale components. For Scale orders only. This value is compulsory.
    pub scale_price_increment: Option<f64>,
    /// Modifies the value of the Scale order. For extended Scale orders.
    pub scale_price_adjust_value: Option<f64>,
    /// Specifies the interval when the price is adjusted. For extended Scale orders.
    pub scale_price_adjust_interval: Option<i32>,
    /// Specifies the offset when to adjust profit. For extended scale orders.
    pub scale_profit_offset: Option<f64>,
    /// Restarts the Scale series if the order is cancelled. For extended scale orders.
    pub scale_auto_reset: bool,
    /// The initial position of the Scale order. For extended scale orders.
    pub scale_init_position: Option<i32>,
    /// Specifies the initial quantity to be filled. For extended scale orders.
    pub scale_init_fill_qty: Option<i32>,
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
    /// As of API version 9.6, the following algorithms are supported:
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
    /// These features are for both guaranteed and non-guaranteed combination orders routed to Smart, and are available based on combo type and order type. SmartComboRoutingParams is similar to AlgoParams in that it makes use of tag/value pairs to add parameters to combo orders.
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
    /// This is a regulatory attribute that applies to all US Commodity (Futures) Exchanges, provided to allow client to comply with CFTC Tag 50 Rules.
    pub ext_operator: String,
    /// The native cash quantity.
    pub cash_qty: Option<f64>,
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
    pub ref_futures_con_id: Option<i32>,
    /// Cancels the parent order if child order was cancelled.
    pub auto_cancel_parent: bool,
    /// Identifies the Shareholder.
    pub shareholder: String,
    /// Used to specify "imbalance only open orders" or "imbalance only closing orders".
    pub imbalance_only: bool,
    /// Routes market order to Best Bid Offer.
    pub route_marketable_to_bbo: bool,
    /// Parent order Id.
    pub parent_perm_id: Option<i64>,
    /// Accepts a list with parameters obtained from advancedOrderRejectJson.
    pub advanced_error_override: String,
    /// Used by brokers and advisors when manually entering, modifying or cancelling orders at the direction of a client. Only used when allocating orders to specific groups or accounts. Excluding "All" group.
    pub manual_order_time: String,
    /// Defines the minimum trade quantity to fill. For IBKRATS orders.
    pub min_trade_qty: Option<i32>,
    /// Defines the minimum size to compete. For IBKRATS orders.
    pub min_compete_size: Option<i32>,
    /// Specifies the offset off the midpoint that will be applied to the order. For IBKRATS orders.
    pub compete_against_best_offset: Option<f64>,
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
    pub pegged_change_amount: Option<f64>,
    /// Pegged-to-benchmark orders: the amount the reference contract needs to move to adjust the pegged order.
    pub reference_change_amount: Option<f64>,
    /// Pegged-to-benchmark orders: the exchange against which we want to observe the reference contract.
    pub reference_exchange: String,
    /// Adjusted Stop orders: the parent order will be adjusted to the given type when the adjusted trigger price is penetrated.
    pub adjusted_order_type: String,
    /// Adjusted Stop orders: specifies the trigger price to execute.
    pub trigger_price: Option<f64>,
    /// Adjusted Stop orders: specifies the price offset for the stop to move in increments.
    pub limit_price_offset: Option<f64>,
    /// Adjusted Stop orders: specifies the stop price of the adjusted (STP) parent.
    pub adjusted_stop_price: Option<f64>,
    /// Adjusted Stop orders: specifies the stop limit price of the adjusted (STPL LMT) parent.
    pub adjusted_stop_limit_price: Option<f64>,
    /// Adjusted Stop orders: specifies the trailing amount of the adjusted (TRAIL) parent.
    pub adjusted_trailing_amount: Option<f64>,
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
    pub duration: Option<i32>, // TODO date object?
    /// Value must be positive, and it is number of seconds that SMART order would be parked for at IBKRATS before being routed to exchange.
    pub post_to_ats: Option<i32>,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            order_id: 0,
            solicited: false,
            client_id: 0,
            perm_id: 0,
            action: Action::Buy,
            total_quantity: 0.0,
            order_type: "".to_owned(),
            limit_price: None,
            aux_price: None,
            tif: "".to_owned(),
            oca_group: "".to_owned(),
            oca_type: 0,
            order_ref: "".to_owned(),
            transmit: true,
            parent_id: 0,
            block_order: false,
            sweep_to_fill: false,
            display_size: Some(0), // TODO - default to None?
            trigger_method: 0,
            outside_rth: false,
            hidden: false,
            good_after_time: "".to_owned(),
            good_till_date: "".to_owned(),
            override_percentage_constraints: false,
            rule_80_a: None,
            all_or_none: false,
            min_qty: None,
            percent_offset: None,
            trail_stop_price: None,
            trailing_percent: None,
            fa_group: "".to_owned(),
            fa_profile: "".to_owned(),
            fa_method: "".to_owned(),
            fa_percentage: "".to_owned(),
            open_close: None,
            origin: 0,
            short_sale_slot: 0,
            designated_location: "".to_owned(),
            exempt_code: -1,
            discretionary_amt: 0.0,
            opt_out_smart_routing: false,
            auction_strategy: Some(0), // TODO - use enum
            starting_price: None,
            stock_ref_price: None,
            delta: None,
            stock_range_lower: None,
            stock_range_upper: None,
            volatility: None,
            volatility_type: None,
            continuous_update: false,
            reference_price_type: None,
            delta_neutral_order_type: "".to_owned(),
            delta_neutral_aux_price: None,
            delta_neutral_con_id: 0,
            delta_neutral_settling_firm: "".to_owned(),
            delta_neutral_clearing_account: "".to_owned(),
            delta_neutral_clearing_intent: "".to_owned(),
            delta_neutral_open_close: "".to_owned(),
            delta_neutral_short_sale: false,
            delta_neutral_short_sale_slot: 0,
            delta_neutral_designated_location: "".to_owned(),
            basis_points: Some(0.0),
            basis_points_type: Some(0),
            scale_init_level_size: None,
            scale_subs_level_size: None,
            scale_price_increment: None,
            scale_price_adjust_value: None,
            scale_price_adjust_interval: None,
            scale_profit_offset: None,
            scale_auto_reset: false,
            scale_init_position: None,
            scale_init_fill_qty: None,
            scale_random_percent: false,
            hedge_type: "".to_owned(),
            hedge_param: "".to_owned(),
            account: "".to_owned(),
            settling_firm: "".to_owned(),
            clearing_account: "".to_owned(),
            clearing_intent: "".to_owned(),
            algo_strategy: "".to_owned(),
            algo_params: vec![],
            what_if: false,
            algo_id: "".to_owned(),
            not_held: false,
            smart_combo_routing_params: vec![],
            order_combo_legs: vec![],
            order_misc_options: vec![],
            active_start_time: "".to_owned(),
            active_stop_time: "".to_owned(),
            scale_table: "".to_owned(),
            model_code: "".to_owned(),
            ext_operator: "".to_owned(),
            cash_qty: None,
            mifid2_decision_maker: "".to_owned(),
            mifid2_decision_algo: "".to_owned(),
            mifid2_execution_trader: "".to_owned(),
            mifid2_execution_algo: "".to_owned(),
            dont_use_auto_price_for_hedge: false,
            auto_cancel_date: "".to_owned(),
            filled_quantity: 0.0,
            ref_futures_con_id: Some(0),
            auto_cancel_parent: false,
            shareholder: "".to_owned(),
            imbalance_only: false,
            route_marketable_to_bbo: false,
            parent_perm_id: None,
            advanced_error_override: "".to_owned(),
            manual_order_time: "".to_owned(),
            min_trade_qty: None,
            min_compete_size: None,
            compete_against_best_offset: None,
            mid_offset_at_whole: None,
            mid_offset_at_half: None,
            randomize_size: false,
            randomize_price: false,
            reference_contract_id: 0,
            is_pegged_change_amount_decrease: false,
            pegged_change_amount: Some(0.0),
            reference_change_amount: Some(0.0),
            reference_exchange: "".to_owned(),
            adjusted_order_type: "".to_owned(),
            trigger_price: None,
            limit_price_offset: None,
            adjusted_stop_price: None,
            adjusted_stop_limit_price: None,
            adjusted_trailing_amount: None,
            adjustable_trailing_unit: 0,
            conditions: vec![],
            conditions_ignore_rth: false,
            conditions_cancel_order: false,
            soft_dollar_tier: SoftDollarTier::default(),
            is_oms_container: false,
            discretionary_up_to_limit_price: false,
            use_price_mgmt_algo: false,
            duration: None,
            post_to_ats: None,
        }
    }
}

impl Order {
    pub fn is_delta_neutral(&self) -> bool {
        !self.delta_neutral_order_type.is_empty()
    }

    pub fn is_scale_order(&self) -> bool {
        match self.scale_price_increment {
            Some(price_increment) => price_increment > 0.0,
            _ => false,
        }
    }
}

/// Identifies the side.
/// Generally available values are BUY and SELL.
/// Additionally, SSHORT and SLONG are available in some institutional-accounts only.
/// For general account types, a SELL order will be able to enter a short position automatically if the order quantity is larger than your current long position.
/// SSHORT is only supported for institutional account configured with Long/Short account segments or clearing with a separate account.
/// SLONG is available in specially-configured institutional accounts to indicate that long position not yet delivered is being sold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum Action {
    #[default]
    Buy,
    Sell,
    /// SSHORT is only supported for institutional account configured with Long/Short account segments or clearing with a separate account.
    SellShort,
    /// SLONG is available in specially-configured institutional accounts to indicate that long position not yet delivered is being sold.
    SellLong,
}

impl ToField for Action {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Action::Buy => "BUY",
            Action::Sell => "SELL",
            Action::SellShort => "SSHORT",
            Action::SellLong => "SLONG",
        };

        write!(f, "{text}")
    }
}

impl Action {
    pub fn reverse(self) -> Action {
        match self {
            Action::Buy => Action::Sell,
            Action::Sell => Action::Buy,
            Action::SellShort => Action::SellLong,
            Action::SellLong => Action::SellShort,
        }
    }

    pub fn from(name: &str) -> Self {
        match name {
            "BUY" => Self::Buy,
            "SELL" => Self::Sell,
            "SSHORT" => Self::SellShort,
            "SLONG" => Self::SellLong,
            &_ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rule80A {
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

impl ToField for Rule80A {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<Rule80A> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl std::fmt::Display for Rule80A {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Rule80A::Individual => "I",
            Rule80A::Agency => "A",
            Rule80A::AgentOtherMember => "W",
            Rule80A::IndividualPTIA => "J",
            Rule80A::AgencyPTIA => "U",
            Rule80A::AgentOtherMemberPTIA => "M",
            Rule80A::IndividualPT => "K",
            Rule80A::AgencyPT => "Y",
            Rule80A::AgentOtherMemberPT => "N",
        };

        write!(f, "{text}")
    }
}

impl Rule80A {
    pub fn from(source: &str) -> Option<Self> {
        match source {
            "I" => Some(Rule80A::Individual),
            "A" => Some(Rule80A::Agency),
            "W" => Some(Rule80A::AgentOtherMember),
            "J" => Some(Rule80A::IndividualPTIA),
            "U" => Some(Rule80A::AgencyPTIA),
            "M" => Some(Rule80A::AgentOtherMemberPTIA),
            "K" => Some(Rule80A::IndividualPT),
            "Y" => Some(Rule80A::AgencyPT),
            "N" => Some(Rule80A::AgentOtherMemberPT),
            _ => None,
        }
    }
}

pub enum AuctionStrategy {
    Match,
    Improvement,
    Transparent,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderComboLeg {
    price: Option<f64>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderCondition {
    Price = 1,
    Time = 3,
    Margin = 4,
    Execution = 5,
    Volume = 6,
    PercentChange = 7,
}

impl ToField for OrderCondition {
    fn to_field(&self) -> String {
        (*self as u8).to_string()
    }
}

impl ToField for Option<OrderCondition> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl From<i32> for OrderCondition {
    fn from(val: i32) -> Self {
        match val {
            1 => OrderCondition::Price,
            3 => OrderCondition::Time,
            4 => OrderCondition::Volume,
            5 => OrderCondition::Execution,
            6 => OrderCondition::Volume,
            7 => OrderCondition::PercentChange,
            _ => panic!("OrderCondition({val}) is unsupported"),
        }
    }
}

/// Stores Soft Dollar Tier information.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SoftDollarTier {
    pub name: String,
    pub value: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderData {
    /// The order's unique id
    pub order_id: i32,
    /// The order's Contract.
    pub contract: Contract,
    /// The currently active order
    pub order: Order,
    /// The order's OrderState
    pub order_state: OrderState,
}

/// Provides an active order's current state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderState {
    /// The order's current status
    pub status: String,
    /// The account's current initial margin.
    pub initial_margin_before: Option<f64>,
    /// The account's current maintenance margin
    pub maintenance_margin_before: Option<f64>,
    /// The account's current equity with loan
    pub equity_with_loan_before: Option<f64>,
    /// The change of the account's initial margin.
    pub initial_margin_change: Option<f64>,
    /// The change of the account's maintenance margin
    pub maintenance_margin_change: Option<f64>,
    /// The change of the account's equity with loan
    pub equity_with_loan_change: Option<f64>,
    /// The order's impact on the account's initial margin.
    pub initial_margin_after: Option<f64>,
    /// The order's impact on the account's maintenance margin
    pub maintenance_margin_after: Option<f64>,
    /// Shows the impact the order would have on the account's equity with loan
    pub equity_with_loan_after: Option<f64>,
    /// The order's generated commission.
    pub commission: Option<f64>,
    // The execution's minimum commission.
    pub minimum_commission: Option<f64>,
    /// The executions maximum commission.
    pub maximum_commission: Option<f64>,
    /// The generated commission currency
    pub commission_currency: String,
    /// If the order is warranted, a descriptive message will be provided.
    pub warning_text: String,
    pub completed_time: String,
    pub completed_status: String,
}

/// For institutional customers only. Valid values are O (open) and C (close).
/// Available for institutional clients to determine if this order is to open or close a position.
/// When Action = "BUY" and OpenClose = "O" this will open a new position.
/// When Action = "BUY" and OpenClose = "C" this will close and existing short position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderOpenClose {
    Open,
    Close,
}

impl ToField for OrderOpenClose {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<OrderOpenClose> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl std::fmt::Display for OrderOpenClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            OrderOpenClose::Open => "O",
            OrderOpenClose::Close => "C",
        };

        write!(f, "{text}")
    }
}

impl OrderOpenClose {
    pub fn from(source: &str) -> Option<Self> {
        match source {
            "O" => Some(OrderOpenClose::Open),
            "C" => Some(OrderOpenClose::Close),
            _ => None,
        }
    }
}

/// Represents the commission generated by an execution.
#[derive(Clone, Debug, Default)]
pub struct CommissionReport {
    /// the execution's id this commission belongs to.
    pub execution_id: String,
    /// the commissions cost.
    pub commission: f64,
    /// the reporting currency.
    pub currency: String,
    /// the realized profit and loss
    pub realized_pnl: Option<f64>,
    /// The income return.
    pub yields: Option<f64>,
    /// date expressed in yyyymmdd format.
    pub yield_redemption_date: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Liquidity {
    #[default]
    None = 0,
    AddedLiquidity = 1,
    RemovedLiquidity = 2,
    LiquidityRoutedOut = 3,
}

impl From<i32> for Liquidity {
    fn from(val: i32) -> Self {
        match val {
            1 => Liquidity::AddedLiquidity,
            2 => Liquidity::RemovedLiquidity,
            3 => Liquidity::LiquidityRoutedOut,
            _ => Liquidity::None,
        }
    }
}

/// Describes an order's execution.
#[derive(Clone, Debug, Default)]
pub struct Execution {
    /// The API client's order Id. May not be unique to an account.
    pub order_id: i32,
    /// The API client identifier which placed the order which originated this execution.
    pub client_id: i32,
    /// The execution's identifier. Each partial fill has a separate ExecId.
    /// A correction is indicated by an ExecId which differs from a previous ExecId in only the digits after the final period,
    /// e.g. an ExecId ending in ".02" would be a correction of a previous execution with an ExecId ending in ".01"
    pub execution_id: String,
    /// The execution's server time.
    pub time: String,
    /// The account to which the order was allocated.
    pub account_number: String,
    /// The exchange where the execution took place.
    pub exchange: String,
    /// Specifies if the transaction was buy or sale
    /// BOT for bought, SLD for sold
    pub side: String,
    /// The number of shares filled.
    pub shares: f64,
    /// The order's execution price excluding commissions.
    pub price: f64,
    /// The TWS order identifier. The PermId can be 0 for trades originating outside IB.
    pub perm_id: i32,
    /// Identifies whether an execution occurred because of an IB-initiated liquidation.
    pub liquidation: i32,
    /// Cumulative quantity.
    // Used in regular trades, combo trades and legs of the combo.
    pub cumulative_quantity: f64,
    /// Average price.
    /// Used in regular trades, combo trades and legs of the combo. Does not include commissions.
    pub average_price: f64,
    /// The OrderRef is a user-customizable string that can be set from the API or TWS and will be associated with an order for its lifetime.
    pub order_reference: String,
    /// The Economic Value Rule name and the respective optional argument.
    /// The two values should be separated by a colon. For example, aussieBond:YearsToExpiration=3. When the optional argument is not present, the first value will be followed by a colon.
    pub ev_rule: String,
    /// Tells you approximately how much the market value of a contract would change if the price were to change by 1.
    /// It cannot be used to get market value by multiplying the price by the approximate multiplier.
    pub ev_multiplier: Option<f64>,
    /// model code
    pub model_code: String,
    // The liquidity type of the execution. Requires TWS 968+ and API v973.05+. Python API specifically requires API v973.06+.
    pub last_liquidity: Liquidity,
}

#[derive(Clone, Debug, Default)]
pub struct ExecutionData {
    pub request_id: i32,
    pub contract: Contract,
    pub execution: Execution,
}

#[derive(Clone, Debug)]
pub enum PlaceOrder {
    OrderStatus(OrderStatus),
    OpenOrder(OrderData),
    ExecutionData(ExecutionData),
    CommissionReport(CommissionReport),
    Message(Notice),
}

/// Contains all relevant information on the current status of the order execution-wise (i.e. amount filled and pending, filling price, etc.).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct OrderStatus {
    /// The order's client id.
    pub order_id: i32,
    /// The current status of the order. Possible values:
    ///     ApiPending - indicates order has not yet been sent to IB server, for instance if there is a delay in receiving the security definition. Uncommonly received.
    ///     PendingSubmit - indicates that you have transmitted the order, but have not yet received confirmation that it has been accepted by the order destination.
    ///     PendingCancel - indicates that you have sent a request to cancel the order but have not yet received cancel confirmation from the order destination. At this point, your order is not confirmed canceled. It is not guaranteed that the cancellation will be successful.
    ///     PreSubmitted - indicates that a simulated order type has been accepted by the IB system and that this order has yet to be elected. The order is held in the IB system until the election criteria are met. At that time the order is transmitted to the order destination as specified .
    ///     Submitted - indicates that your order has been accepted by the system.
    ///     ApiCancelled - after an order has been submitted and before it has been acknowledged, an API client client can request its cancelation, producing this state.
    ///     Cancelled - indicates that the balance of your order has been confirmed canceled by the IB system. This could occur unexpectedly when IB or the destination has rejected your order.
    ///     Filled - indicates that the order has been completely filled. Market orders executions will not always trigger a Filled status.
    ///     Inactive - indicates that the order was received by the system but is no longer active because it was rejected or canceled.    
    pub status: String,
    /// Number of filled positions.
    pub filled: f64,
    /// The remnant positions.
    pub remaining: f64,
    /// Average filling price.
    pub average_fill_price: f64,
    /// The order's permId used by the TWS to identify orders.
    pub perm_id: i32,
    /// Parent's id. Used for bracket and auto trailing stop orders.
    pub parent_id: i32,
    /// Price at which the last positions were filled.
    pub last_fill_price: f64,
    /// API client which submitted the order.
    pub client_id: i32,
    /// This field is used to identify an order held when TWS is trying to locate shares for a short sell. The value used to indicate this is 'locate'.
    pub why_held: String,
    /// If an order has been capped, this indicates the current capped price. Requires TWS 967+ and API v973.04+. Python API specifically requires API v973.06+.
    pub market_cap_price: f64,
}

// Submits an Order.
// After the order is submitted correctly, events will be returned concerning the order's activity.
// https://interactivebrokers.github.io/tws-api/order_submission.html
pub(crate) fn place_order<'a>(client: &'a Client, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<'a, PlaceOrder>, Error> {
    verify_order(client, order, order_id)?;
    verify_order_contract(client, contract, order_id)?;

    let request = encoders::encode_place_order(client.server_version(), order_id, contract, order)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

impl Subscribable<PlaceOrder> for PlaceOrder {
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<PlaceOrder, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(PlaceOrder::OpenOrder(decoders::decode_open_order(server_version, message.clone())?)),
            IncomingMessages::OrderStatus => Ok(PlaceOrder::OrderStatus(decoders::decode_order_status(server_version, message)?)),
            IncomingMessages::ExecutionData => Ok(PlaceOrder::ExecutionData(decoders::decode_execution_data(server_version, message)?)),
            IncomingMessages::CommissionsReport => Ok(PlaceOrder::CommissionReport(decoders::decode_commission_report(server_version, message)?)),
            IncomingMessages::Error => Ok(PlaceOrder::Message(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

// Verifies that Order is properly formed.
fn verify_order(client: &Client, order: &Order, _order_id: i32) -> Result<(), Error> {
    let is_bag_order: bool = false; // StringsAreEqual(Constants.BagSecType, contract.SecType)

    if order.scale_init_level_size.is_some() || order.scale_price_increment.is_some() {
        client.check_server_version(server_versions::SCALE_ORDERS, "It does not support Scale orders.")?
    }

    if order.what_if {
        client.check_server_version(server_versions::WHAT_IF_ORDERS, "It does not support what-if orders.")?
    }

    if order.scale_subs_level_size.is_some() {
        client.check_server_version(
            server_versions::SCALE_ORDERS2,
            "It does not support Subsequent Level Size for Scale orders.",
        )?
    }

    if !order.algo_strategy.is_empty() {
        client.check_server_version(server_versions::ALGO_ORDERS, "It does not support algo orders.")?
    }

    if order.not_held {
        client.check_server_version(server_versions::NOT_HELD, "It does not support not_held parameter.")?
    }

    if order.exempt_code != -1 {
        client.check_server_version(server_versions::SSHORTX, "It does not support exempt_code parameter.")?
    }

    if !order.hedge_type.is_empty() {
        client.check_server_version(server_versions::HEDGE_ORDERS, "It does not support hedge orders.")?
    }

    if order.opt_out_smart_routing {
        client.check_server_version(
            server_versions::OPT_OUT_SMART_ROUTING,
            "It does not support opt_out_smart_routing parameter.",
        )?
    }

    if order.delta_neutral_con_id > 0
        || !order.delta_neutral_settling_firm.is_empty()
        || !order.delta_neutral_clearing_account.is_empty()
        || !order.delta_neutral_clearing_intent.is_empty()
    {
        client.check_server_version(
            server_versions::DELTA_NEUTRAL_CONID,
            "It does not support delta_neutral parameters: con_id, settling_firm, clearing_account, clearing_intent.",
        )?
    }

    if !order.delta_neutral_open_close.is_empty()
        || order.delta_neutral_short_sale
        || order.delta_neutral_short_sale_slot > 0
        || !order.delta_neutral_designated_location.is_empty()
    {
        client.check_server_version(
            server_versions::DELTA_NEUTRAL_OPEN_CLOSE,
            "It does not support delta_neutral parameters: open_close, short_sale, short_saleSlot, designated_location",
        )?
    }

    if (order.scale_price_increment > Some(0.0))
        && (order.scale_price_adjust_value.is_some()
            || order.scale_price_adjust_interval.is_some()
            || order.scale_profit_offset.is_some()
            || order.scale_auto_reset
            || order.scale_init_position.is_some()
            || order.scale_init_fill_qty.is_some()
            || order.scale_random_percent)
    {
        client.check_server_version(
                server_versions::SCALE_ORDERS3,
                "It does not support Scale order parameters: PriceAdjustValue, PriceAdjustInterval, ProfitOffset, AutoReset, InitPosition, InitFillQty and RandomPercent",
            )?
    }

    if is_bag_order && order.order_combo_legs.iter().any(|combo_leg| combo_leg.price.is_some()) {
        client.check_server_version(
            server_versions::ORDER_COMBO_LEGS_PRICE,
            "It does not support per-leg prices for order combo legs.",
        )?
    }

    if order.trailing_percent.is_some() {
        client.check_server_version(server_versions::TRAILING_PERCENT, "It does not support trailing percent parameter.")?
    }

    if !order.algo_id.is_empty() {
        client.check_server_version(server_versions::ALGO_ID, "It does not support algo_id parameter")?
    }

    if !order.scale_table.is_empty() || !order.active_start_time.is_empty() || !order.active_stop_time.is_empty() {
        client.check_server_version(
            server_versions::SCALE_TABLE,
            "It does not support scale_table, active_start_time nor active_stop_time parameters.",
        )?
    }

    if !order.ext_operator.is_empty() {
        client.check_server_version(server_versions::EXT_OPERATOR, "It does not support ext_operator parameter")?
    }

    if order.cash_qty.is_some() {
        client.check_server_version(server_versions::CASH_QTY, "It does not support cash_qty parameter")?
    }

    if !order.mifid2_execution_trader.is_empty() || !order.mifid2_execution_algo.is_empty() {
        client.check_server_version(server_versions::DECISION_MAKER, "It does not support MIFID II execution parameters")?
    }

    if order.dont_use_auto_price_for_hedge {
        client.check_server_version(
            server_versions::AUTO_PRICE_FOR_HEDGE,
            "It does not support don't use auto price for hedge parameter",
        )?
    }

    if order.is_oms_container {
        client.check_server_version(server_versions::ORDER_CONTAINER, "It does not support oms container parameter")?
    }

    if order.discretionary_up_to_limit_price {
        client.check_server_version(server_versions::D_PEG_ORDERS, "It does not support D-Peg orders")?
    }

    if order.use_price_mgmt_algo {
        client.check_server_version(server_versions::PRICE_MGMT_ALGO, "It does not support Use Price Management Algo requests")?
    }

    if order.duration.is_some() {
        client.check_server_version(server_versions::DURATION, "It does not support duration attribute")?
    }

    if order.post_to_ats.is_some() {
        client.check_server_version(server_versions::POST_TO_ATS, "It does not support post_to_ats attribute")?
    }

    if order.auto_cancel_parent {
        client.check_server_version(server_versions::AUTO_CANCEL_PARENT, "It does not support auto_cancel_parent attribute")?
    }

    if !order.advanced_error_override.is_empty() {
        client.check_server_version(
            server_versions::ADVANCED_ORDER_REJECT,
            "It does not support advanced error override attribute",
        )?
    }

    if !order.manual_order_time.is_empty() {
        client.check_server_version(server_versions::MANUAL_ORDER_TIME, "It does not support manual order time attribute")?
    }

    if order.min_trade_qty.is_some()
        || order.min_compete_size.is_some()
        || order.compete_against_best_offset.is_some()
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

// Verifies that Contract is properly formed.
fn verify_order_contract(client: &Client, contract: &Contract, _order_id: i32) -> Result<(), Error> {
    if contract
        .combo_legs
        .iter()
        .any(|combo_leg| combo_leg.short_sale_slot != 0 || !combo_leg.designated_location.is_empty())
    {
        client.check_server_version(server_versions::SSHORT_COMBO_LEGS, "It does not support SSHORT flag for combo legs")?
    }

    if contract.delta_neutral_contract.is_some() {
        client.check_server_version(server_versions::DELTA_NEUTRAL, "It does not support delta-neutral orders")?
    }

    if contract.contract_id > 0 {
        client.check_server_version(server_versions::PLACE_ORDER_CONID, "It does not support contract_id parameter")?
    }

    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(server_versions::SEC_ID_TYPE, "It does not support sec_id_type and sec_id parameters")?
    }

    if contract.combo_legs.iter().any(|combo_leg| combo_leg.exempt_code != -1) {
        client.check_server_version(server_versions::SSHORTX, "It does not support exempt_code parameter")?
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support trading_class parameters in place_order",
        )?
    }

    Ok(())
}

// Cancels an open [Order].
pub(crate) fn cancel_order<'a>(client: &'a Client, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<'a, CancelOrder>, Error> {
    if !manual_order_cancel_time.is_empty() {
        client.check_server_version(
            server_versions::MANUAL_ORDER_TIME,
            "It does not support manual order cancel time attribute",
        )?
    }

    let request = encoders::encode_cancel_order(client.server_version(), order_id, manual_order_cancel_time)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

/// Enumerates possible results from cancelling an order.
#[derive(Debug)]
pub enum CancelOrder {
    OrderStatus(OrderStatus),
    Notice(Notice),
}

impl Subscribable<CancelOrder> for CancelOrder {
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<CancelOrder, Error> {
        match message.message_type() {
            IncomingMessages::OrderStatus => Ok(CancelOrder::OrderStatus(decoders::decode_order_status(server_version, message)?)),
            IncomingMessages::Error => Ok(CancelOrder::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

// Cancels all open [Order]s.
pub(crate) fn global_cancel(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel requests.")?;

    let message = encoders::encode_global_cancel()?;

    let request_id = client.next_request_id();
    client.send_order(request_id, message)?;

    Ok(())
}

// Gets next valid order id
pub(crate) fn next_valid_order_id(client: &Client) -> Result<i32, Error> {
    let message = encoders::encode_next_valid_order_id()?;

    let subscription = client.send_shared_request(OutgoingMessages::RequestIds, message)?;

    if let Some(Ok(message)) = subscription.next() {
        let order_id_index = 2;
        let next_order_id = message.peek_int(order_id_index)?;

        client.set_next_order_id(next_order_id);

        Ok(next_order_id)
    } else {
        Err(Error::Simple("no response from server".into()))
    }
}

// Requests completed [Order]s.
pub(crate) fn completed_orders(client: &Client, api_only: bool) -> Result<Subscription<Orders>, Error> {
    client.check_server_version(server_versions::COMPLETED_ORDERS, "It does not support completed orders requests.")?;

    let request = encoders::encode_completed_orders(api_only)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestCompletedOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

/// Enumerates possible results from querying an [Order].
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Orders {
    OrderData(OrderData),
    OrderStatus(OrderStatus),
    Notice(Notice),
}

impl Subscribable<Orders> for Orders {
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Orders, Error> {
        match message.message_type() {
            IncomingMessages::CompletedOrder => Ok(Orders::OrderData(decoders::decode_completed_order(server_version, message.clone())?)),
            IncomingMessages::CommissionsReport => Ok(Orders::OrderData(decoders::decode_open_order(server_version, message.clone())?)),
            IncomingMessages::OpenOrder => Ok(Orders::OrderData(decoders::decode_open_order(server_version, message.clone())?)),
            IncomingMessages::OrderStatus => Ok(Orders::OrderStatus(decoders::decode_order_status(server_version, message)?)),
            IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => Err(Error::StreamEnd),
            IncomingMessages::Error => Ok(Orders::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

/// Requests all open orders places by this specific API client (identified by the API client id).
/// For client ID 0, this will bind previous manual TWS orders.
///
/// # Arguments
/// * `client` - [Client] used to communicate with server.
///
pub(crate) fn open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests all *current* open orders in associated accounts at the current moment.
// Open orders are returned once; this function does not initiate a subscription.
pub(crate) fn all_open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_all_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
pub(crate) fn auto_open_orders(client: &Client, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_auto_open_orders(auto_bind)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

#[derive(Debug, Default)]
/// Filter criteria used to determine which execution reports are returned.
pub struct ExecutionFilter {
    /// The API client which placed the order.
    pub client_id: Option<i32>,
    /// The account to which the order was allocated to
    pub account_code: String,
    /// Time from which the executions will be returned yyyymmdd hh:mm:ss
    /// Only those executions reported after the specified time will be returned.
    pub time: String,
    /// The instrument's symbol
    pub symbol: String,
    /// The Contract's security's type (i.e. STK, OPT...)
    pub security_type: String,
    /// The exchange at which the execution was produced
    pub exchange: String,
    /// The Contract's side (BUY or SELL)
    pub side: String,
}

// Requests current day's (since midnight) executions matching the filter.
//
// Only the current day's executions can be retrieved.
// Along with the [ExecutionData], the [CommissionReport] will also be returned.
// When requesting executions, a filter can be specified to receive only a subset of them
//
// # Arguments
// * `filter` - filter criteria used to determine which execution reports are returned
pub(crate) fn executions(client: &Client, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
    let request_id = client.next_request_id();

    let request = encoders::encode_executions(client.server_version(), request_id, &filter)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

/// Enumerates possible results from querying an [Execution].
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Executions {
    ExecutionData(ExecutionData),
    CommissionReport(CommissionReport),
    Notice(Notice),
}

impl Subscribable<Executions> for Executions {
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Executions, Error> {
        match message.message_type() {
            IncomingMessages::ExecutionData => Ok(Executions::ExecutionData(decoders::decode_execution_data(server_version, message)?)),
            IncomingMessages::CommissionsReport => Ok(Executions::CommissionReport(decoders::decode_commission_report(server_version, message)?)),
            IncomingMessages::ExecutionDataEnd => Err(Error::StreamEnd),
            IncomingMessages::Error => Ok(Executions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

#[derive(Debug)]
pub enum ExerciseAction {
    Exercise = 1,
    Lapse = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ExerciseOptions {
    OpenOrder(OrderData),
    OrderStatus(OrderStatus),
    Notice(Notice),
}

impl Subscribable<ExerciseOptions> for ExerciseOptions {
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<ExerciseOptions, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(ExerciseOptions::OpenOrder(decoders::decode_open_order(server_version, message.clone())?)),
            IncomingMessages::OrderStatus => Ok(ExerciseOptions::OrderStatus(decoders::decode_order_status(server_version, message)?)),
            IncomingMessages::Error => Ok(ExerciseOptions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

pub(crate) fn exercise_options<'a>(
    client: &'a Client,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<Subscription<'a, ExerciseOptions>, Error> {
    let request_id = client.next_request_id();

    let request = encoders::encode_exercise_options(
        client.server_version(),
        request_id,
        contract,
        exercise_action,
        exercise_quantity,
        account,
        ovrd,
        manual_order_time,
    )?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}
