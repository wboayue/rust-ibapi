//! # Order Management
//!
//! This module provides comprehensive order management capabilities including
//! order creation, modification, cancellation, and execution tracking.
//! It supports various order types and advanced trading features.
//!
//! ## Features
//!
//! - **Order Placement**: Submit market, limit, stop, and other order types
//! - **Order Modification**: Modify existing orders with new parameters
//! - **Order Cancellation**: Cancel individual orders or all open orders
//! - **Execution Tracking**: Monitor order fills and execution details
//! - **Commission Reports**: Track commission charges for executed trades
//! - **Order Status Updates**: Real-time updates on order state changes
//!
//! ## Order Types
//!
//! The module supports various order types including:
//! - Market orders
//! - Limit orders
//! - Stop orders
//! - Stop-limit orders
//! - Trailing stop orders
//! - VWAP/TWAP algorithmic orders
//! - And many more specialized order types
//!
//! ## Usage
//!
//! Orders are created using the `Order` struct and can be customized with various
//! parameters. The `order_builder` module provides a fluent API for constructing
//! complex orders.

// Common implementation modules
pub(crate) mod common;

/// Fluent builder APIs for constructing orders.
pub mod builder;

/// Order condition types for conditional orders.
pub mod conditions;

/// Convenience re-export for low-level order builder helpers.
pub use common::order_builder;

// Re-export builder types
pub use builder::{BracketOrderBuilder, BracketOrderIds, OrderBuilder, OrderId};

// Re-export condition types and builders
pub use conditions::{
    ExecutionCondition, ExecutionConditionBuilder, MarginCondition, MarginConditionBuilder, PercentChangeCondition, PercentChangeConditionBuilder,
    PriceCondition, PriceConditionBuilder, TimeCondition, TimeConditionBuilder, VolumeCondition, VolumeConditionBuilder,
};

use std::convert::From;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::contracts::Contract;
use crate::{encode_option_field, ToField};

// Public types - always available regardless of feature flags

/// Make sure to test using only your paper trading account when applicable. A good way of finding out if an order type/exchange combination
/// is possible is by trying to place such order manually using the TWS.
/// Before contacting our API support team please refer to the available documentation.
pub use crate::contracts::TagValue;

pub(crate) const COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID: Option<f64> = Some(f64::INFINITY);

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
    /// The time in force - specifies how long the order remains active.
    ///
    /// See [`TimeInForce`] for available options and their behavior.
    pub tif: TimeInForce,
    /// One-Cancels-All group identifier.
    pub oca_group: String,
    /// Tells how to handle remaining orders in an OCA group when one order or part of an order executes.
    ///
    /// See [`OcaType`] for available options. "With block" provides overfill protection by ensuring
    /// only one order in the group is routed at a time.
    pub oca_type: OcaType,
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
    ///
    /// See [`conditions::TriggerMethod`] for available options.
    pub trigger_method: conditions::TriggerMethod,
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
    /// Precautionary constraints are defined on the TWS Presets page, and help ensure that your price and size order values are reasonable. Orders sent from the API are also validated against these safety constraints, and may be rejected if any constraint is violated. To override validation, set this parameter's value to True.
    pub override_percentage_constraints: bool,
    /// NYSE Rule 80A designation values:
    /// - Individual = `I`
    /// - Agency = `A`
    /// - AgentOtherMember = `W`
    /// - IndividualPTIA = `J`
    /// - AgencyPTIA = `U`
    /// - AgentOtherMemberPTIA = `M`
    /// - IndividualPT = `K`
    /// - AgencyPT = `Y`
    /// - AgentOtherMemberPT = `N`
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
    ///
    /// Observe the following guidelines when using the trailingPercent field:
    /// - This field is mutually exclusive with the existing trailing amount. That is, the API client can send one
    ///   or the other but not both.
    /// - This field is read AFTER the stop price (barrier price) as follows: deltaNeutralAuxPrice stopPrice,
    ///   trailingPercent, scale order attributes.
    /// - The field will also be sent to the API in the openOrder message if the API client version is >= 56.
    ///   It is sent after the stopPrice field as follows: stopPrice, trailingPct, basisPoint.    
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
    ///
    /// Available for institutional clients to determine if this order is to open or close a position.
    /// - When Action = "BUY" and OpenClose = "O" this will open a new position.
    /// - When Action = "BUY" and OpenClose = "C" this will close an existing short position.
    pub open_close: Option<OrderOpenClose>,
    /// The order's origin. Same as TWS "Origin" column. Identifies the type of customer from which the order originated.
    ///
    /// See [`OrderOrigin`] for available options.
    pub origin: OrderOrigin,
    /// For institutions only. Specifies the short sale slot.
    ///
    /// See [`ShortSaleSlot`] for available options.
    pub short_sale_slot: ShortSaleSlot,
    /// For institutions only. Indicates the location where the shares to short come from.
    /// Used only when short sale slot is set to `ThirdParty`.
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
    ///
    /// See [`AuctionStrategy`] for available options.
    pub auction_strategy: Option<AuctionStrategy>,
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
    /// VOL orders only. See [`VolatilityType`] for available options.
    pub volatility_type: Option<VolatilityType>,
    /// Specifies whether TWS will automatically update the limit price of the order as the underlying price moves. VOL orders only.
    pub continuous_update: bool,
    /// Specifies how you want TWS to calculate the limit price for options, and for stock range price monitoring.
    /// VOL orders only.
    ///
    /// See [`ReferencePriceType`] for available options.
    pub reference_price_type: Option<ReferencePriceType>,
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
    ///
    /// Valid values are:
    /// - `IB`
    /// - `Away`
    /// - `PTA` (post trade allocation)
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
    ///
    /// Possible values include:
    /// - `D` - Delta
    /// - `B` - Beta
    /// - `F` - FX
    /// - `P` - Pair
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
    ///
    /// Valid values are:
    /// - `IB`
    /// - `Away`
    /// - `PTA` (post trade allocation)
    pub clearing_intent: String,
    /// The algorithm strategy.
    ///
    /// As of API version 9.6, the following algorithms are supported:
    /// - `ArrivalPx` - Arrival Price
    /// - `DarkIce` - Dark Ice
    /// - `PctVol` - Percentage of Volume
    /// - `Twap` - TWAP (Time Weighted Average Price)
    /// - `Vwap` - VWAP (Volume Weighted Average Price)
    ///
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
    /// Orders routed to IBDARK are tagged as "post only" and are held in IB's order book, where incoming SmartRouted orders from other IB customers are eligible to trade against them.
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
    /// Customer account information for completed orders.
    pub customer_account: String,
    /// Indicates if this is a professional customer order.
    pub professional_customer: bool,
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
            tif: TimeInForce::Day,
            oca_group: "".to_owned(),
            oca_type: OcaType::None,
            order_ref: "".to_owned(),
            transmit: true,
            parent_id: 0,
            block_order: false,
            sweep_to_fill: false,
            display_size: Some(0), // TODO - default to None?
            trigger_method: conditions::TriggerMethod::Default,
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
            origin: OrderOrigin::Customer,
            short_sale_slot: ShortSaleSlot::None,
            designated_location: "".to_owned(),
            exempt_code: -1,
            discretionary_amt: 0.0,
            opt_out_smart_routing: false,
            auction_strategy: None,
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
            customer_account: String::new(),
            professional_customer: false,
        }
    }
}

impl Order {
    /// Returns `true` if delta-neutral parameters are configured.
    pub fn is_delta_neutral(&self) -> bool {
        !self.delta_neutral_order_type.is_empty()
    }

    /// Returns `true` if scale order parameters are configured.
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
    /// Buy-side order.
    Buy,
    /// Sell-side order.
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
    /// Return the logical opposite action (buy ↔ sell).
    pub fn reverse(self) -> Action {
        match self {
            Action::Buy => Action::Sell,
            Action::Sell => Action::Buy,
            Action::SellShort => Action::SellLong,
            Action::SellLong => Action::SellShort,
        }
    }

    /// Parse an action from the TWS string identifier.
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

/// Time in force specifies how long an order remains active.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Valid for the day only.
    #[default]
    Day,
    /// Good until canceled. The order will continue to work within the system and in the marketplace
    /// until it executes or is canceled. GTC orders will be automatically cancelled under certain conditions.
    GoodTilCanceled,
    /// Immediate or Cancel. Any portion that is not filled as soon as it becomes available in the
    /// market is canceled.
    ImmediateOrCancel,
    /// Good until Date. It will remain working within the system and in the marketplace until it
    /// executes or until the close of the market on the date specified.
    GoodTilDate,
    /// Market-on-open (MOO) or limit-on-open (LOO) order.
    OnOpen,
    /// Fill-or-Kill. If the entire order does not execute as soon as it becomes available, the entire
    /// order is canceled.
    FillOrKill,
    /// Day until Canceled.
    DayTilCanceled,
    /// Auction - for auction orders.
    Auction,
}

impl ToField for TimeInForce {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            TimeInForce::Day => "DAY",
            TimeInForce::GoodTilCanceled => "GTC",
            TimeInForce::ImmediateOrCancel => "IOC",
            TimeInForce::GoodTilDate => "GTD",
            TimeInForce::OnOpen => "OPG",
            TimeInForce::FillOrKill => "FOK",
            TimeInForce::DayTilCanceled => "DTC",
            TimeInForce::Auction => "AUC",
        };
        write!(f, "{text}")
    }
}

impl From<String> for TimeInForce {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&str> for TimeInForce {
    fn from(value: &str) -> Self {
        match value {
            "DAY" => TimeInForce::Day,
            "GTC" => TimeInForce::GoodTilCanceled,
            "IOC" => TimeInForce::ImmediateOrCancel,
            "GTD" => TimeInForce::GoodTilDate,
            "OPG" => TimeInForce::OnOpen,
            "FOK" => TimeInForce::FillOrKill,
            "DTC" => TimeInForce::DayTilCanceled,
            "AUC" => TimeInForce::Auction,
            _ => TimeInForce::Day, // Default fallback
        }
    }
}

/// Tells how to handle remaining orders in an OCA group when one order or part of an order executes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcaType {
    /// Not part of OCA group.
    #[default]
    None = 0,
    /// Cancel all remaining orders with block (overfill protection - only one order routed at a time).
    CancelWithBlock = 1,
    /// Proportionally reduce remaining orders with block.
    ReduceWithBlock = 2,
    /// Proportionally reduce remaining orders without block.
    ReduceWithoutBlock = 3,
}

impl ToField for OcaType {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl From<OcaType> for i32 {
    fn from(value: OcaType) -> i32 {
        value as i32
    }
}

impl From<i32> for OcaType {
    fn from(value: i32) -> Self {
        match value {
            0 => OcaType::None,
            1 => OcaType::CancelWithBlock,
            2 => OcaType::ReduceWithBlock,
            3 => OcaType::ReduceWithoutBlock,
            _ => OcaType::None,
        }
    }
}

/// The order's origin. Identifies the type of customer from which the order originated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderOrigin {
    /// Customer order.
    #[default]
    Customer = 0,
    /// Firm order (institutional customers only).
    Firm = 1,
}

impl ToField for OrderOrigin {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl From<OrderOrigin> for i32 {
    fn from(value: OrderOrigin) -> i32 {
        value as i32
    }
}

impl From<i32> for OrderOrigin {
    fn from(value: i32) -> Self {
        match value {
            0 => OrderOrigin::Customer,
            1 => OrderOrigin::Firm,
            _ => OrderOrigin::Customer,
        }
    }
}

/// Specifies the short sale slot (for institutional short sales).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortSaleSlot {
    /// Not a short sale.
    #[default]
    None = 0,
    /// Broker holds shares.
    Broker = 1,
    /// Shares come from elsewhere (third party). Use with `designated_location` field.
    ThirdParty = 2,
}

impl ToField for ShortSaleSlot {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl From<ShortSaleSlot> for i32 {
    fn from(value: ShortSaleSlot) -> i32 {
        value as i32
    }
}

impl From<i32> for ShortSaleSlot {
    fn from(value: i32) -> Self {
        match value {
            0 => ShortSaleSlot::None,
            1 => ShortSaleSlot::Broker,
            2 => ShortSaleSlot::ThirdParty,
            _ => ShortSaleSlot::None,
        }
    }
}

/// Volatility type for VOL orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityType {
    /// Daily volatility.
    Daily = 1,
    /// Annual volatility.
    Annual = 2,
}

impl ToField for VolatilityType {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl ToField for Option<VolatilityType> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl From<VolatilityType> for i32 {
    fn from(value: VolatilityType) -> i32 {
        value as i32
    }
}

impl From<i32> for VolatilityType {
    fn from(value: i32) -> Self {
        match value {
            1 => VolatilityType::Daily,
            2 => VolatilityType::Annual,
            _ => VolatilityType::Daily,
        }
    }
}

/// Reference price type for VOL orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferencePriceType {
    /// Average of National Best Bid/Offer.
    AverageOfNBBO = 1,
    /// NBB or NBO depending on action and right.
    NBBO = 2,
}

impl ToField for ReferencePriceType {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl ToField for Option<ReferencePriceType> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl From<ReferencePriceType> for i32 {
    fn from(value: ReferencePriceType) -> i32 {
        value as i32
    }
}

impl From<i32> for ReferencePriceType {
    fn from(value: i32) -> Self {
        match value {
            1 => ReferencePriceType::AverageOfNBBO,
            2 => ReferencePriceType::NBBO,
            _ => ReferencePriceType::AverageOfNBBO,
        }
    }
}

/// NYSE Rule 80A designations for institutional trading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rule80A {
    /// Individual customer.
    Individual,
    /// Agency transaction.
    Agency,
    /// Agent for other member.
    AgentOtherMember,
    /// Individual principal transaction in agency cross.
    IndividualPTIA,
    /// Agency principal transaction in agency cross.
    AgencyPTIA,
    /// Agent for other member principal transaction in agency cross.
    AgentOtherMemberPTIA,
    /// Individual principal transaction.
    IndividualPT,
    /// Agency principal transaction.
    AgencyPT,
    /// Agent for other member principal transaction.
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
    /// Parse a rule 80A code from its string representation.
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

/// Auction strategy for BOX orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionStrategy {
    /// Match strategy.
    Match = 1,
    /// Improvement strategy.
    Improvement = 2,
    /// Transparent strategy.
    Transparent = 3,
}

impl ToField for AuctionStrategy {
    fn to_field(&self) -> String {
        i32::from(*self).to_string()
    }
}

impl ToField for Option<AuctionStrategy> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl From<AuctionStrategy> for i32 {
    fn from(value: AuctionStrategy) -> i32 {
        value as i32
    }
}

impl From<i32> for AuctionStrategy {
    fn from(value: i32) -> Self {
        match value {
            1 => AuctionStrategy::Match,
            2 => AuctionStrategy::Improvement,
            3 => AuctionStrategy::Transparent,
            _ => AuctionStrategy::Match,
        }
    }
}

/// Represents the price component of a combo leg order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderComboLeg {
    /// The price for this combo leg.
    pub price: Option<f64>,
}

/// Order condition types for conditional orders.
///
/// Each variant wraps a specific condition type that defines when the order
/// should be activated or canceled based on market conditions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderCondition {
    /// Price-based condition that triggers when a contract reaches a specified price.
    Price(PriceCondition),
    /// Time-based condition that triggers at a specified time.
    Time(TimeCondition),
    /// Margin-based condition that triggers when margin cushion percentage changes.
    Margin(MarginCondition),
    /// Execution-based condition that triggers when a specific contract is executed.
    Execution(ExecutionCondition),
    /// Volume-based condition that triggers when a contract reaches a specified volume.
    Volume(VolumeCondition),
    /// Percent change condition that triggers when a contract's price changes by a specified percentage.
    PercentChange(PercentChangeCondition),
}

impl OrderCondition {
    /// Returns the condition type discriminator as used by the TWS API.
    pub fn condition_type(&self) -> i32 {
        match self {
            Self::Price(_) => 1,
            Self::Time(_) => 3,
            Self::Margin(_) => 4,
            Self::Execution(_) => 5,
            Self::Volume(_) => 6,
            Self::PercentChange(_) => 7,
        }
    }

    /// Returns whether this is a conjunction (AND) condition.
    pub fn is_conjunction(&self) -> bool {
        match self {
            Self::Price(c) => c.is_conjunction,
            Self::Time(c) => c.is_conjunction,
            Self::Margin(c) => c.is_conjunction,
            Self::Execution(c) => c.is_conjunction,
            Self::Volume(c) => c.is_conjunction,
            Self::PercentChange(c) => c.is_conjunction,
        }
    }
}

impl ToField for OrderCondition {
    fn to_field(&self) -> String {
        self.condition_type().to_string()
    }
}

impl ToField for Option<OrderCondition> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl From<i32> for OrderCondition {
    /// Creates an OrderCondition variant with default values from a type discriminator.
    fn from(val: i32) -> Self {
        match val {
            1 => OrderCondition::Price(PriceCondition::default()),
            3 => OrderCondition::Time(TimeCondition::default()),
            4 => OrderCondition::Margin(MarginCondition::default()),
            5 => OrderCondition::Execution(ExecutionCondition::default()),
            6 => OrderCondition::Volume(VolumeCondition::default()),
            7 => OrderCondition::PercentChange(PercentChangeCondition::default()),
            _ => panic!("OrderCondition({val}) is unsupported"),
        }
    }
}

/// Stores Soft Dollar Tier information.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SoftDollarTier {
    /// Soft dollar tier name.
    pub name: String,
    /// Tier identifier value.
    pub value: String,
    /// User-friendly display name.
    pub display_name: String,
}

/// Contains order information including the order, contract, and order state.
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
    /// The execution's minimum commission.
    pub minimum_commission: Option<f64>,
    /// The executions maximum commission.
    pub maximum_commission: Option<f64>,
    /// The generated commission currency
    pub commission_currency: String,
    /// If the order is warranted, a descriptive message will be provided.
    pub warning_text: String,
    /// Timestamp when the order completed execution.
    pub completed_time: String,
    /// Status value after completion (e.g. `Filled`).
    pub completed_status: String,
}

/// For institutional customers only. Valid values are O (open) and C (close).
/// Available for institutional clients to determine if this order is to open or close a position.
/// When Action = "BUY" and OpenClose = "O" this will open a new position.
/// When Action = "BUY" and OpenClose = "C" this will close and existing short position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderOpenClose {
    /// Open a new position.
    Open,
    /// Close an existing position.
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
    /// Parse an `OrderOpenClose` from the wire-format string.
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

/// Liquidity types for executions.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Liquidity {
    /// No liquidity information.
    #[default]
    None = 0,
    /// Added liquidity to the market.
    AddedLiquidity = 1,
    /// Removed liquidity from the market.
    RemovedLiquidity = 2,
    /// Liquidity was routed out.
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
    /// Liquidity type of the execution (requires TWS 968+ / API v973.05+).
    pub last_liquidity: Liquidity,
}

/// Contains execution information including the request ID, contract, and execution details.
#[derive(Clone, Debug, Default)]
pub struct ExecutionData {
    /// The request ID associated with this execution.
    pub request_id: i32,
    /// The contract that was executed.
    pub contract: Contract,
    /// The execution details.
    pub execution: Execution,
}

/// Responses from placing an order.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum PlaceOrder {
    /// Order status update.
    OrderStatus(OrderStatus),
    /// Open order information.
    OpenOrder(OrderData),
    /// Execution data.
    ExecutionData(ExecutionData),
    /// Commission report.
    CommissionReport(CommissionReport),
    /// Notice or error message.
    Message(crate::messages::Notice),
}

/// Updates received when monitoring order activity.
/// This enum is used by `order_update_stream` to deliver real-time order updates.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum OrderUpdate {
    /// Order status update.
    OrderStatus(OrderStatus),
    /// Open order information.
    OpenOrder(OrderData),
    /// Execution data.
    ExecutionData(ExecutionData),
    /// Commission report.
    CommissionReport(CommissionReport),
    /// Notice or error message.
    Message(crate::messages::Notice),
}

/// Contains all relevant information on the current status of the order execution-wise (i.e. amount filled and pending, filling price, etc.).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct OrderStatus {
    /// The order's client id.
    pub order_id: i32,
    /// The current status of the order. Possible values:
    /// * ApiPending - indicates order has not yet been sent to IB server, for instance if there is a delay in receiving the security definition. Uncommonly received.
    /// * PendingSubmit - indicates that you have transmitted the order, but have not yet received confirmation that it has been accepted by the order destination.
    /// * PendingCancel - indicates that you have sent a request to cancel the order but have not yet received cancel confirmation from the order destination. At this point, your order is not confirmed canceled. It is not guaranteed that the cancellation will be successful.
    /// * PreSubmitted - indicates that a simulated order type has been accepted by the IB system and that this order has yet to be elected. The order is held in the IB system until the election criteria are met. At that time the order is transmitted to the order destination as specified .
    /// * Submitted - indicates that your order has been accepted by the system.
    /// * ApiCancelled - after an order has been submitted and before it has been acknowledged, an API client client can request its cancelation, producing this state.
    /// * Cancelled - indicates that the balance of your order has been confirmed canceled by the IB system. This could occur unexpectedly when IB or the destination has rejected your order.
    /// * Filled - indicates that the order has been completely filled. Market orders executions will not always trigger a Filled status.
    /// * Inactive - indicates that the order was received by the system but is no longer active because it was rejected or canceled.
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

/// Enumerates possible results from cancelling an order.
#[derive(Debug)]
pub enum CancelOrder {
    /// Order status information.
    OrderStatus(OrderStatus),
    /// Informational notice.
    Notice(crate::messages::Notice),
}

/// Enumerates possible results from querying an [Order].
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Orders {
    /// Detailed order data.
    OrderData(OrderData),
    /// Order status update.
    OrderStatus(OrderStatus),
    /// Informational notice.
    Notice(crate::messages::Notice),
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
    /// Filter executions from the last N days (0 = no filter).
    pub last_n_days: i32,
    /// Filter executions for specific dates (format: yyyymmdd, e.g., "20260130").
    pub specific_dates: Vec<String>,
}

/// Enumerates possible results from querying an [Execution].
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Executions {
    /// Execution data payload.
    ExecutionData(ExecutionData),
    /// Commission report payload.
    CommissionReport(CommissionReport),
    /// Informational notice.
    Notice(crate::messages::Notice),
}

/// Exercise action for options.
#[derive(Debug, Clone, Copy)]
pub enum ExerciseAction {
    /// Exercise the option.
    Exercise = 1,
    /// Let the option lapse.
    Lapse = 2,
}

/// Responses from exercising options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ExerciseOptions {
    /// Open order information.
    OpenOrder(OrderData),
    /// Order status update.
    OrderStatus(OrderStatus),
    /// Notice or error message.
    Notice(crate::messages::Notice),
}

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
/// Blocking order management API wrappers on top of the synchronous client.
pub mod blocking {
    pub(crate) use super::sync::{
        all_open_orders, auto_open_orders, cancel_order, completed_orders, executions, exercise_options, global_cancel, next_valid_order_id,
        open_orders, order_update_stream, place_order, submit_order,
    };
}

#[cfg(feature = "async")]
pub(crate) use r#async::{
    all_open_orders, auto_open_orders, cancel_order, completed_orders, executions, exercise_options, global_cancel, next_valid_order_id, open_orders,
    order_update_stream, place_order, submit_order,
};
