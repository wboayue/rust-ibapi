use super::algo_builders::AlgoParams;
use super::types::*;
use super::validation;
use crate::contracts::Contract;
use crate::market_data::TradingHours;
use crate::orders::{Action, Order, OrderComboLeg, OrderCondition, TagValue};

#[cfg(test)]
mod tests;

/// Builder for creating orders with a fluent interface
///
/// All validation is deferred to the build() method to ensure
/// no silent failures occur during order construction.
pub struct OrderBuilder<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) contract: &'a Contract,
    action: Option<Action>,
    quantity: Option<f64>, // Store raw value, validate in build()
    order_type: Option<OrderType>,
    limit_price: Option<f64>, // Store raw value, validate in build()
    stop_price: Option<f64>,  // Store raw value, validate in build()
    time_in_force: TimeInForce,
    outside_rth: bool,
    hidden: bool,
    transmit: bool,
    parent_id: Option<i32>,
    oca_group: Option<String>,
    oca_type: Option<i32>,
    account: Option<String>,
    good_after_time: Option<String>,
    good_till_date: Option<String>,
    conditions: Vec<OrderCondition>,
    algo_strategy: Option<String>,
    algo_params: Vec<TagValue>,
    pub(crate) what_if: bool,

    // Advanced fields
    discretionary_amt: Option<f64>,
    trailing_percent: Option<f64>,
    trail_stop_price: Option<f64>,
    limit_price_offset: Option<f64>,
    volatility: Option<f64>,
    volatility_type: Option<i32>,
    delta: Option<f64>,
    aux_price: Option<f64>,

    // Special order flags
    sweep_to_fill: bool,
    block_order: bool,
    not_held: bool,
    all_or_none: bool,

    // Pegged order fields
    min_trade_qty: Option<i32>,
    min_compete_size: Option<i32>,
    compete_against_best_offset: Option<f64>,
    mid_offset_at_whole: Option<f64>,
    mid_offset_at_half: Option<f64>,

    // Reference contract fields
    reference_contract_id: Option<i32>,
    reference_exchange: Option<String>,
    stock_ref_price: Option<f64>,
    stock_range_lower: Option<f64>,
    stock_range_upper: Option<f64>,
    reference_change_amount: Option<f64>,
    pegged_change_amount: Option<f64>,
    is_pegged_change_amount_decrease: bool,

    // Combo order fields
    order_combo_legs: Vec<OrderComboLeg>,
    smart_combo_routing_params: Vec<TagValue>,

    // Cash quantity for FX orders
    cash_qty: Option<f64>,

    // Manual order time
    manual_order_time: Option<String>,

    // Auction strategy
    auction_strategy: Option<i32>,

    // Starting price
    starting_price: Option<f64>,

    // Hedge type
    hedge_type: Option<String>,
}

impl<'a, C> OrderBuilder<'a, C> {
    /// Creates a new OrderBuilder
    pub fn new(client: &'a C, contract: &'a Contract) -> Self {
        Self {
            client,
            contract,
            action: None,
            quantity: None,
            order_type: None,
            limit_price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            outside_rth: false,
            hidden: false,
            transmit: true,
            parent_id: None,
            oca_group: None,
            oca_type: None,
            account: None,
            good_after_time: None,
            good_till_date: None,
            conditions: Vec::new(),
            algo_strategy: None,
            algo_params: Vec::new(),
            what_if: false,
            discretionary_amt: None,
            trailing_percent: None,
            trail_stop_price: None,
            limit_price_offset: None,
            volatility: None,
            volatility_type: None,
            delta: None,
            aux_price: None,
            sweep_to_fill: false,
            block_order: false,
            not_held: false,
            all_or_none: false,
            min_trade_qty: None,
            min_compete_size: None,
            compete_against_best_offset: None,
            mid_offset_at_whole: None,
            mid_offset_at_half: None,
            reference_contract_id: None,
            reference_exchange: None,
            stock_ref_price: None,
            stock_range_lower: None,
            stock_range_upper: None,
            reference_change_amount: None,
            pegged_change_amount: None,
            is_pegged_change_amount_decrease: false,
            order_combo_legs: Vec::new(),
            smart_combo_routing_params: Vec::new(),
            cash_qty: None,
            manual_order_time: None,
            auction_strategy: None,
            starting_price: None,
            hedge_type: None,
        }
    }

    // Action methods

    /// Set order to buy the specified quantity
    pub fn buy(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::Buy);
        self.quantity = Some(quantity.into());
        self
    }

    /// Set order to sell the specified quantity
    pub fn sell(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::Sell);
        self.quantity = Some(quantity.into());
        self
    }

    // Order type methods

    /// Create a market order
    pub fn market(mut self) -> Self {
        self.order_type = Some(OrderType::Market);
        self
    }

    /// Create a limit order at the specified price
    pub fn limit(mut self, price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(price.into());
        self
    }

    /// Create a stop order at the specified stop price
    pub fn stop(mut self, stop_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Stop);
        self.stop_price = Some(stop_price.into());
        self
    }

    /// Create a stop-limit order
    pub fn stop_limit(mut self, stop_price: impl Into<f64>, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::StopLimit);
        self.stop_price = Some(stop_price.into());
        self.limit_price = Some(limit_price.into());
        self
    }

    /// Create a trailing stop order
    pub fn trailing_stop(mut self, trailing_percent: f64, stop_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::TrailingStop);
        self.trailing_percent = Some(trailing_percent);
        self.trail_stop_price = Some(stop_price.into());
        self
    }

    /// Set a custom order type
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    /// Create a trailing stop limit order
    pub fn trailing_stop_limit(mut self, trailing_percent: f64, stop_price: impl Into<f64>, limit_offset: f64) -> Self {
        self.order_type = Some(OrderType::TrailingStopLimit);
        self.trailing_percent = Some(trailing_percent);
        self.trail_stop_price = Some(stop_price.into());
        self.limit_price_offset = Some(limit_offset);
        self
    }

    /// Market if Touched - triggers market order when price is touched
    pub fn market_if_touched(mut self, trigger_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::MarketIfTouched);
        self.aux_price = Some(trigger_price.into());
        self
    }

    /// Limit if Touched - triggers limit order when price is touched
    pub fn limit_if_touched(mut self, trigger_price: impl Into<f64>, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::LimitIfTouched);
        self.aux_price = Some(trigger_price.into());
        self.limit_price = Some(limit_price.into());
        self
    }

    /// Market to Limit - starts as market order, remainder becomes limit
    pub fn market_to_limit(mut self) -> Self {
        self.order_type = Some(OrderType::MarketToLimit);
        self
    }

    /// Discretionary order - limit order with hidden discretionary amount
    pub fn discretionary(mut self, limit_price: impl Into<f64>, discretionary_amt: f64) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.discretionary_amt = Some(discretionary_amt);
        self
    }

    /// Sweep to Fill - prioritizes speed of execution over price
    pub fn sweep_to_fill(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.sweep_to_fill = true;
        self
    }

    /// Block order - for large volume option orders (min 50 contracts)
    pub fn block(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.block_order = true;
        self
    }

    /// Midprice order - fills at midpoint between bid/ask or better
    pub fn midprice(mut self, price_cap: Option<f64>) -> Self {
        self.order_type = Some(OrderType::Midprice);
        self.limit_price = price_cap;
        self
    }

    /// Relative/Pegged-to-Primary - seeks more aggressive price than NBBO
    pub fn relative(mut self, offset: f64, price_cap: Option<f64>) -> Self {
        self.order_type = Some(OrderType::Relative);
        self.aux_price = Some(offset);
        self.limit_price = price_cap;
        self
    }

    /// Passive Relative - seeks less aggressive price than NBBO
    pub fn passive_relative(mut self, offset: f64) -> Self {
        self.order_type = Some(OrderType::PassiveRelative);
        self.aux_price = Some(offset);
        self
    }

    /// At Auction - for pre-market opening period execution
    pub fn at_auction(mut self, price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::AtAuction);
        self.limit_price = Some(price.into());
        self.time_in_force = TimeInForce::Auction;
        self
    }

    /// Market on Close - executes as market order at or near closing price
    pub fn market_on_close(mut self) -> Self {
        self.order_type = Some(OrderType::MarketOnClose);
        self
    }

    /// Limit on Close - executes as limit order at close if price is met
    pub fn limit_on_close(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::LimitOnClose);
        self.limit_price = Some(limit_price.into());
        self
    }

    /// Market on Open - executes as market order at market open
    pub fn market_on_open(mut self) -> Self {
        self.order_type = Some(OrderType::Market);
        self.time_in_force = TimeInForce::OpeningAuction;
        self
    }

    /// Limit on Open - executes as limit order at market open
    pub fn limit_on_open(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.time_in_force = TimeInForce::OpeningAuction;
        self
    }

    /// Market with Protection - market order with protection against extreme price movements (futures only)
    pub fn market_with_protection(mut self) -> Self {
        self.order_type = Some(OrderType::MarketWithProtection);
        self
    }

    /// Stop with Protection - stop order with protection against extreme price movements (futures only)
    pub fn stop_with_protection(mut self, stop_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::StopWithProtection);
        self.stop_price = Some(stop_price.into());
        self
    }

    // Time in force methods

    /// Set time in force for the order
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }

    /// Order valid for the day only
    pub fn day_order(mut self) -> Self {
        self.time_in_force = TimeInForce::Day;
        self
    }

    /// Good till cancelled order
    pub fn good_till_cancel(mut self) -> Self {
        self.time_in_force = TimeInForce::GoodTillCancel;
        self
    }

    /// Good till specific date
    pub fn good_till_date(mut self, date: impl Into<String>) -> Self {
        let date_str = date.into();
        self.time_in_force = TimeInForce::GoodTillDate { date: date_str.clone() };
        self.good_till_date = Some(date_str);
        self
    }

    /// Fill or kill order
    pub fn fill_or_kill(mut self) -> Self {
        self.time_in_force = TimeInForce::FillOrKill;
        self
    }

    /// Immediate or cancel order
    pub fn immediate_or_cancel(mut self) -> Self {
        self.time_in_force = TimeInForce::ImmediateOrCancel;
        self
    }

    // Trading hours

    /// Allow order execution outside regular trading hours
    pub fn outside_rth(mut self) -> Self {
        self.outside_rth = true;
        self
    }

    /// Restrict order to regular trading hours only
    pub fn regular_hours_only(mut self) -> Self {
        self.outside_rth = false;
        self
    }

    /// Set trading hours preference
    pub fn trading_hours(mut self, hours: TradingHours) -> Self {
        self.outside_rth = matches!(hours, TradingHours::Extended);
        self
    }

    // Order attributes

    /// Hide order from market depth (only works for NASDAQ-routed orders)
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Set account for order
    pub fn account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Set parent order ID for attached orders
    pub fn parent(mut self, parent_id: i32) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set OCA group
    pub fn oca_group(mut self, group: impl Into<String>, oca_type: i32) -> Self {
        self.oca_group = Some(group.into());
        self.oca_type = Some(oca_type);
        self
    }

    /// Do not transmit order immediately
    pub fn do_not_transmit(mut self) -> Self {
        self.transmit = false;
        self
    }

    /// Create bracket orders with take profit and stop loss
    pub fn bracket(self) -> BracketOrderBuilder<'a, C> {
        BracketOrderBuilder::new(self)
    }

    // Conditional orders

    /// Add a condition to the order.
    ///
    /// The first condition is always treated as AND. Use `and_condition()` or `or_condition()`
    /// for subsequent conditions to specify the logical relationship.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::builder::price;
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .condition(price(265598, "SMART").greater_than(150.0))
    ///     .submit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn condition(mut self, condition: impl Into<OrderCondition>) -> Self {
        let mut cond = condition.into();
        set_conjunction(&mut cond, true);
        self.conditions.push(cond);
        self
    }

    /// Add a condition that must be met along with previous conditions (AND logic).
    ///
    /// # Example
    ///
    /// ```ignore
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::builder::{price, margin};
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .condition(price(265598, "SMART").greater_than(150.0))
    ///     .and_condition(margin().greater_than(30))
    ///     .submit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_condition(mut self, condition: impl Into<OrderCondition>) -> Self {
        if let Some(prev) = self.conditions.last_mut() {
            set_conjunction(prev, true);
        }
        self.conditions.push(condition.into());
        self
    }

    /// Add a condition where either this OR previous conditions trigger the order (OR logic).
    ///
    /// # Example
    ///
    /// ```ignore
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::builder::{price, volume};
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .condition(price(265598, "SMART").less_than(100.0))
    ///     .or_condition(volume(265598, "SMART").greater_than(50_000_000))
    ///     .submit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_condition(mut self, condition: impl Into<OrderCondition>) -> Self {
        if let Some(prev) = self.conditions.last_mut() {
            set_conjunction(prev, false);
        }
        self.conditions.push(condition.into());
        self
    }

    // Algorithmic trading

    /// Set algorithm strategy and parameters.
    ///
    /// Accepts either a strategy name (string) or an algo builder.
    ///
    /// # Example with builder
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::builder::vwap;
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(1000)
    ///     .limit(150.0)
    ///     .algo(vwap()
    ///         .max_pct_vol(0.2)
    ///         .start_time("09:00:00 US/Eastern")
    ///         .end_time("16:00:00 US/Eastern")
    ///         .build()?)
    ///     .submit().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example with string (for custom strategies)
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// let order_id = client.order(&contract)
    ///     .buy(1000)
    ///     .limit(150.0)
    ///     .algo("Vwap")
    ///     .algo_param("maxPctVol", "0.2")
    ///     .submit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn algo(mut self, algo: impl Into<AlgoParams>) -> Self {
        let params = algo.into();
        self.algo_strategy = Some(params.strategy);
        self.algo_params.extend(params.params);
        self
    }

    /// Add algorithm parameter.
    ///
    /// Use this to add individual parameters when using a strategy name string.
    /// When using algo builders, parameters are set via the builder methods.
    pub fn algo_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.algo_params.push(TagValue {
            tag: key.into(),
            value: value.into(),
        });
        self
    }

    // What-if orders

    /// Mark as what-if order for margin/commission calculation
    pub fn what_if(mut self) -> Self {
        self.what_if = true;
        self
    }

    // Additional order attributes

    /// Set volatility for volatility orders
    pub fn volatility(mut self, volatility: f64) -> Self {
        self.volatility = Some(volatility);
        self
    }

    /// Mark order as not held
    pub fn not_held(mut self) -> Self {
        self.not_held = true;
        self
    }

    /// Mark order as all or none
    pub fn all_or_none(mut self) -> Self {
        self.all_or_none = true;
        self
    }

    /// Set good after time
    pub fn good_after_time(mut self, time: impl Into<String>) -> Self {
        self.good_after_time = Some(time.into());
        self
    }

    /// Set good till time (stored in good_till_date field)
    pub fn good_till_time(mut self, time: impl Into<String>) -> Self {
        self.good_till_date = Some(time.into());
        self
    }

    // Pegged order configuration

    /// Set minimum trade quantity for pegged orders
    pub fn min_trade_qty(mut self, qty: i32) -> Self {
        self.min_trade_qty = Some(qty);
        self
    }

    /// Set minimum compete size for pegged orders
    pub fn min_compete_size(mut self, size: i32) -> Self {
        self.min_compete_size = Some(size);
        self
    }

    /// Set compete against best offset for pegged orders
    pub fn compete_against_best_offset(mut self, offset: f64) -> Self {
        self.compete_against_best_offset = Some(offset);
        self
    }

    /// Set mid offset at whole for pegged orders
    pub fn mid_offset_at_whole(mut self, offset: f64) -> Self {
        self.mid_offset_at_whole = Some(offset);
        self
    }

    /// Set mid offset at half for pegged orders
    pub fn mid_offset_at_half(mut self, offset: f64) -> Self {
        self.mid_offset_at_half = Some(offset);
        self
    }

    // Build methods

    /// Build the Order struct with full validation
    pub fn build(self) -> Result<Order, ValidationError> {
        // Validate required fields
        let action = self.action.ok_or(ValidationError::MissingRequiredField("action"))?;
        let quantity_raw = self.quantity.ok_or(ValidationError::MissingRequiredField("quantity"))?;
        let order_type = self.order_type.ok_or(ValidationError::MissingRequiredField("order_type"))?;

        // Validate quantity
        let quantity = Quantity::new(quantity_raw)?;

        // Validate prices based on order type
        let limit_price = if order_type.requires_limit_price() {
            let price_raw = self.limit_price.ok_or(ValidationError::MissingRequiredField("limit_price"))?;
            Some(Price::new(price_raw)?)
        } else if let Some(price_raw) = self.limit_price {
            Some(Price::new(price_raw)?)
        } else {
            None
        };

        let stop_price = match order_type {
            OrderType::Stop | OrderType::StopLimit => {
                let price_raw = self.stop_price.ok_or(ValidationError::MissingRequiredField("stop_price"))?;
                Some(Price::new(price_raw)?)
            }
            _ => {
                if let Some(price_raw) = self.stop_price {
                    Some(Price::new(price_raw)?)
                } else {
                    None
                }
            }
        };

        let trail_stop_price = match order_type {
            OrderType::TrailingStop | OrderType::TrailingStopLimit => {
                if self.trailing_percent.is_none() && self.trail_stop_price.is_none() {
                    return Err(ValidationError::MissingRequiredField("trailing amount or stop price"));
                }
                if let Some(price_raw) = self.trail_stop_price {
                    Some(Price::new(price_raw)?)
                } else {
                    None
                }
            }
            _ => None,
        };

        // Validate volatility for volatility orders
        if order_type == OrderType::Volatility && self.volatility.is_none() {
            return Err(ValidationError::MissingRequiredField("volatility"));
        }

        // Validate time in force specific requirements
        if let TimeInForce::GoodTillDate { .. } = &self.time_in_force {
            if self.good_till_date.is_none() {
                return Err(ValidationError::MissingRequiredField("good_till_date"));
            }
        }

        // Build the order
        let mut order = Order {
            action,
            total_quantity: quantity.value(),
            order_type: order_type.as_str().to_string(),
            ..Default::default()
        };

        // Set prices
        if let Some(price) = limit_price {
            order.limit_price = Some(price.value());
        }

        if let Some(price) = stop_price {
            order.aux_price = Some(price.value());
        }

        if let Some(price) = trail_stop_price {
            order.trail_stop_price = Some(price.value());
        }

        if let Some(percent) = self.trailing_percent {
            order.trailing_percent = Some(percent);
        }

        if let Some(offset) = self.limit_price_offset {
            order.limit_price_offset = Some(offset);
        }

        // Set time in force
        order.tif = crate::orders::TimeInForce::from(self.time_in_force.as_str());
        if let TimeInForce::GoodTillDate { date } = &self.time_in_force {
            order.good_till_date = date.clone();
        }

        // Set other fields
        order.outside_rth = self.outside_rth;
        order.hidden = self.hidden;
        order.transmit = self.transmit;

        if let Some(parent_id) = self.parent_id {
            order.parent_id = parent_id;
        }

        if let Some(group) = self.oca_group {
            order.oca_group = group;
            order.oca_type = self.oca_type.unwrap_or(0).into();
        }

        if let Some(account) = self.account {
            order.account = account;
        }

        if let Some(time) = self.good_after_time {
            order.good_after_time = time;
        }

        // Set good_till_date if set via good_till_time method
        if let Some(date_time) = self.good_till_date {
            if !matches!(self.time_in_force, TimeInForce::GoodTillDate { .. }) {
                // If not already set via time_in_force
                order.good_till_date = date_time;
            }
        }

        if let Some(strategy) = self.algo_strategy {
            order.algo_strategy = strategy;
            order.algo_params = self.algo_params;
        }

        order.what_if = self.what_if;

        // Set advanced fields
        if let Some(amt) = self.discretionary_amt {
            order.discretionary_amt = amt;
        }

        if let Some(vol) = self.volatility {
            order.volatility = Some(vol);
            order.volatility_type = self.volatility_type.map(|v| v.into());
        }

        if let Some(delta) = self.delta {
            order.delta = Some(delta);
        }

        if let Some(aux) = self.aux_price {
            // Only set if not already set by stop price
            if order.aux_price.is_none() {
                order.aux_price = Some(aux);
            }
        }

        // Set special flags
        order.sweep_to_fill = self.sweep_to_fill;
        order.block_order = self.block_order;
        order.not_held = self.not_held;
        order.all_or_none = self.all_or_none;

        // Set pegged order fields
        if let Some(qty) = self.min_trade_qty {
            order.min_trade_qty = Some(qty);
        }

        if let Some(size) = self.min_compete_size {
            order.min_compete_size = Some(size);
        }

        if let Some(offset) = self.compete_against_best_offset {
            order.compete_against_best_offset = Some(offset);
        }

        if let Some(offset) = self.mid_offset_at_whole {
            order.mid_offset_at_whole = Some(offset);
        }

        if let Some(offset) = self.mid_offset_at_half {
            order.mid_offset_at_half = Some(offset);
        }

        // Set conditions
        if !self.conditions.is_empty() {
            order.conditions = self.conditions;
        }

        // Set reference contract fields for pegged to benchmark orders
        if let Some(id) = self.reference_contract_id {
            order.reference_contract_id = id;
        }

        if let Some(exchange) = self.reference_exchange {
            order.reference_exchange = exchange;
        }

        if let Some(price) = self.stock_ref_price {
            order.stock_ref_price = Some(price);
        }

        if let Some(lower) = self.stock_range_lower {
            order.stock_range_lower = Some(lower);
        }

        if let Some(upper) = self.stock_range_upper {
            order.stock_range_upper = Some(upper);
        }

        if let Some(amount) = self.reference_change_amount {
            order.reference_change_amount = Some(amount);
        }

        if let Some(amount) = self.pegged_change_amount {
            order.pegged_change_amount = Some(amount);
        }

        if self.is_pegged_change_amount_decrease {
            order.is_pegged_change_amount_decrease = true;
        }

        // Set combo order fields
        if !self.order_combo_legs.is_empty() {
            order.order_combo_legs = self.order_combo_legs;
        }

        if !self.smart_combo_routing_params.is_empty() {
            order.smart_combo_routing_params = self.smart_combo_routing_params;
        }

        // Set cash quantity for FX orders
        if let Some(qty) = self.cash_qty {
            order.cash_qty = Some(qty);
        }

        // Set manual order time
        if let Some(time) = self.manual_order_time {
            order.manual_order_time = time;
        }

        // Set auction strategy
        if let Some(strategy) = self.auction_strategy {
            order.auction_strategy = Some(strategy.into());
        }

        // Set starting price
        if let Some(price) = self.starting_price {
            order.starting_price = Some(price);
        }

        // Set hedge type
        if let Some(hedge_type) = self.hedge_type {
            order.hedge_type = hedge_type;
        }

        Ok(order)
    }
}

/// Helper function to set conjunction flag on OrderCondition enum
fn set_conjunction(condition: &mut OrderCondition, is_conjunction: bool) {
    match condition {
        OrderCondition::Price(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Time(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Margin(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Execution(c) => c.is_conjunction = is_conjunction,
        OrderCondition::Volume(c) => c.is_conjunction = is_conjunction,
        OrderCondition::PercentChange(c) => c.is_conjunction = is_conjunction,
    }
}

/// Builder for bracket orders
pub struct BracketOrderBuilder<'a, C> {
    pub(crate) parent_builder: OrderBuilder<'a, C>,
    entry_price: Option<f64>,
    take_profit_price: Option<f64>,
    stop_loss_price: Option<f64>,
}

impl<'a, C> BracketOrderBuilder<'a, C> {
    fn new(parent_builder: OrderBuilder<'a, C>) -> Self {
        Self {
            parent_builder,
            entry_price: None,
            take_profit_price: None,
            stop_loss_price: None,
        }
    }

    /// Set entry limit price
    pub fn entry_limit(mut self, price: impl Into<f64>) -> Self {
        self.entry_price = Some(price.into());
        self
    }

    /// Set take profit price
    pub fn take_profit(mut self, price: impl Into<f64>) -> Self {
        self.take_profit_price = Some(price.into());
        self
    }

    /// Set stop loss price
    pub fn stop_loss(mut self, price: impl Into<f64>) -> Self {
        self.stop_loss_price = Some(price.into());
        self
    }

    /// Build bracket orders with full validation
    pub fn build(mut self) -> Result<Vec<Order>, ValidationError> {
        // Validate and convert prices
        let entry_price_raw = self.entry_price.ok_or(ValidationError::MissingRequiredField("entry_price"))?;
        let take_profit_raw = self.take_profit_price.ok_or(ValidationError::MissingRequiredField("take_profit"))?;
        let stop_loss_raw = self.stop_loss_price.ok_or(ValidationError::MissingRequiredField("stop_loss"))?;

        let entry_price = Price::new(entry_price_raw)?;
        let take_profit = Price::new(take_profit_raw)?;
        let stop_loss = Price::new(stop_loss_raw)?;

        // Validate bracket order prices
        validation::validate_bracket_prices(
            self.parent_builder.action.as_ref(),
            entry_price.value(),
            take_profit.value(),
            stop_loss.value(),
        )?;

        // Set the entry limit price on parent builder
        self.parent_builder.order_type = Some(OrderType::Limit);
        self.parent_builder.limit_price = Some(entry_price.value());

        // Build parent order
        let mut parent = self.parent_builder.build()?;
        parent.transmit = false;

        // Build take profit order
        let take_profit_order = Order {
            action: parent.action.reverse(),
            order_type: "LMT".to_string(),
            total_quantity: parent.total_quantity,
            limit_price: Some(take_profit.value()),
            parent_id: parent.order_id,
            transmit: false,
            outside_rth: parent.outside_rth,
            ..Default::default()
        };

        // Build stop loss order
        let stop_loss_order = Order {
            action: parent.action.reverse(),
            order_type: "STP".to_string(),
            total_quantity: parent.total_quantity,
            aux_price: Some(stop_loss.value()),
            parent_id: parent.order_id,
            transmit: true,
            outside_rth: parent.outside_rth,
            ..Default::default()
        };

        Ok(vec![parent, take_profit_order, stop_loss_order])
    }
}
