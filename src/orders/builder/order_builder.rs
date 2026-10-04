use super::algo_builders::AlgoParams;
use super::types::*;
use super::validation;
use crate::accounts::types::ContractId;
use crate::contracts::Contract;
use crate::contracts::TagValue;
use crate::market_data::TradingHours;
use crate::orders::common::order_builder::non_guaranteed_params;
use crate::orders::conditions::TriggerMethod;
use crate::orders::OrderId;
use crate::orders::{
    Action, OcaType, Order, OrderComboLeg, OrderCondition, OrderOpenClose, OrderOrigin, ReferencePriceType, Rule80A, ShortSaleSlot, TimeInForce,
    VolatilityType, COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID,
};

#[cfg(test)]
mod tests;

/// Builder for creating orders with a fluent interface
///
/// All validation is deferred to the build() method to ensure
/// no silent failures occur during order construction.
///
/// `T` is the builder's target: [`ClientBound`] for a builder from `Client::order`, which can
/// submit, or [`Detached`] for one from [`Order::builder`], which only builds.
#[must_use = "OrderBuilder does nothing until you call .submit() (place it) or .build() (offline construction)"]
pub struct OrderBuilder<T> {
    pub(crate) target: T,
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
    oca_type: OcaType,
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
    volatility_type: Option<VolatilityType>,
    reference_price_type: Option<ReferencePriceType>,
    delta: Option<f64>,
    aux_price: Option<f64>,

    // Institutional and exchange routing fields
    trigger_method: TriggerMethod,
    origin: OrderOrigin,
    short_sale_slot: ShortSaleSlot,
    designated_location: Option<String>,
    rule_80_a: Option<Rule80A>,
    open_close: Option<OrderOpenClose>,

    // Special order flags
    sweep_to_fill: bool,
    block_order: bool,
    not_held: bool,
    all_or_none: bool,

    // Pegged order fields
    min_trade_qty: Option<i32>,
    min_compete_size: Option<i32>,
    compete: Option<CompeteAgainstBest>,
    mid_offsets: Option<MidOffsets>,

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

    // Starting price
    starting_price: Option<f64>,
}

/// Target of an [`OrderBuilder`] from `Client::order`: the client and contract the order is
/// submitted with.
pub struct ClientBound<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) contract: &'a Contract,
}

/// Target of an [`OrderBuilder`] from [`Order::builder`]: no client or contract, so the
/// builder only builds an [`Order`].
pub struct Detached;

impl<'a, C> OrderBuilder<ClientBound<'a, C>> {
    /// Creates a builder that submits to `client` for `contract`.
    pub fn new(client: &'a C, contract: &'a Contract) -> Self {
        Self::with_target(ClientBound { client, contract })
    }

    /// Create bracket orders with take profit and stop loss
    ///
    /// The prices are the caller's and the three orders are placed by the client. To have TWS
    /// attach children priced from its order presets instead, see
    /// [`preset_stop_loss`](Self::preset_stop_loss) / [`preset_profit_taker`](Self::preset_profit_taker).
    pub fn bracket(self) -> BracketOrderBuilder<'a, C> {
        BracketOrderBuilder::new(self)
    }

    /// Ask TWS to attach a stop-loss to this order, priced from its order presets.
    ///
    /// No price is sent; see [`Order::preset_stop_loss_order_id`] for how TWS resolves the
    /// preset and what happens when none is defined. `submit()` doesn't wait for the outcome,
    /// so watch [`order_update_stream`](crate::Client::order_update_stream).
    ///
    /// Call it after the order's own setters: it returns an [`AttachedOrdersBuilder`], which
    /// only adds the other leg and submits.
    ///
    /// For caller-priced children placed by the client, use [`bracket`](Self::bracket).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::contracts::Contract;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(150.0)
    ///     .preset_stop_loss()
    ///     .preset_profit_taker()
    ///     .submit()
    ///     .await?;
    /// println!("parent {} stop-loss {:?} profit-taker {:?}", ids.parent, ids.stop_loss, ids.profit_taker);
    /// # Ok(())
    /// # }
    /// ```
    pub fn preset_stop_loss(self) -> AttachedOrdersBuilder<'a, C> {
        AttachedOrdersBuilder::new(self).preset_stop_loss()
    }

    /// Ask TWS to attach a profit-taker to this order, priced from its order presets.
    ///
    /// Same behavior as [`preset_stop_loss`](Self::preset_stop_loss), using the profit-taker
    /// preset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::contracts::Contract;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).limit(150.0).preset_profit_taker().submit().await?;
    /// assert!(ids.stop_loss.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub fn preset_profit_taker(self) -> AttachedOrdersBuilder<'a, C> {
        AttachedOrdersBuilder::new(self).preset_profit_taker()
    }
}

impl Order {
    /// Start a fluent [`OrderBuilder`] with no client attached.
    ///
    /// `build()` returns the [`Order`], which you place with `place_order` or `submit_order`.
    /// It's the same builder `Client::order` returns, without `submit()`, `analyze()`, `bracket()` or preset legs: those
    /// allocate order ids, which only a client can do.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::Order;
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let order = Order::builder().buy(100).limit(150.0).build()?;
    /// let order_id = client.next_order_id();
    /// client.submit_order(order_id, &contract, &order).await?;
    /// # Ok(()) }
    /// ```
    pub fn builder() -> OrderBuilder<Detached> {
        OrderBuilder::with_target(Detached)
    }
}

impl<T> OrderBuilder<T> {
    fn with_target(target: T) -> Self {
        Self {
            target,
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
            oca_type: OcaType::None,
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
            reference_price_type: None,
            trigger_method: TriggerMethod::Default,
            origin: OrderOrigin::Customer,
            short_sale_slot: ShortSaleSlot::None,
            designated_location: None,
            rule_80_a: None,
            open_close: None,
            delta: None,
            aux_price: None,
            sweep_to_fill: false,
            block_order: false,
            not_held: false,
            all_or_none: false,
            min_trade_qty: None,
            min_compete_size: None,
            compete: None,
            mid_offsets: None,
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
            starting_price: None,
        }
    }

    // Action methods

    /// Set order to buy the specified quantity.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// let contract = Contract::stock("AAPL").build();
    /// let _ = client.order(&contract).buy(100).market().submit().await?;
    /// # Ok(()) }
    /// ```
    pub fn buy(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::Buy);
        self.quantity = Some(quantity.into());
        self
    }

    /// Set order to sell the specified quantity.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// let _ = client.order(&contract).sell(100).limit(150.0).submit().await?;
    /// # Ok(()) }
    /// ```
    pub fn sell(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::Sell);
        self.quantity = Some(quantity.into());
        self
    }

    /// Set order to sell short (`SSHORT`) the specified quantity.
    ///
    /// `SSHORT` is only supported for institutional accounts configured with Long/Short
    /// account segments or clearing with a separate account; see [`Action::SellShort`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// let _ = client.order(&contract).sell_short(100).limit(150.0).submit().await?;
    /// # Ok(()) }
    /// ```
    pub fn sell_short(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::SellShort);
        self.quantity = Some(quantity.into());
        self
    }

    /// Set order to sell long (`SLONG`) the specified quantity.
    ///
    /// `SLONG` is available in specially-configured institutional accounts to indicate
    /// that a long position not yet delivered is being sold; see [`Action::SellLong`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// let _ = client.order(&contract).sell_long(100).limit(150.0).submit().await?;
    /// # Ok(()) }
    /// ```
    pub fn sell_long(mut self, quantity: impl Into<f64>) -> Self {
        self.action = Some(Action::SellLong);
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

    /// Create a trailing stop order that trails the market by `trail`, starting from `stop_price`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::TrailBy;
    /// use ibapi::orders::Order;
    ///
    /// let by_amount = Order::builder().sell(100).trailing_stop(TrailBy::Amount(2.0), 148.0).build()?;
    /// assert_eq!(by_amount.aux_price, Some(2.0));
    ///
    /// let by_percent = Order::builder().sell(100).trailing_stop(TrailBy::Percent(1.5), 148.0).build()?;
    /// assert_eq!(by_percent.trailing_percent, Some(1.5));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn trailing_stop(mut self, trail: TrailBy, stop_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::TrailingStop);
        self.set_trail(trail);
        self.trail_stop_price = Some(stop_price.into());
        self
    }

    /// Set the order type without setting any price.
    ///
    /// `build()` sends only the price fields `order_type` uses, so a price set earlier for a type
    /// that doesn't take it is dropped: `.limit(100.0).order_type(OrderType::PeggedToStock)` sends
    /// no limit price. Prefer the named setter (`.relative(..)`, `.pegged_to_midpoint(..)`, ...).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::OrderType;
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(100).limit(150.0).order_type(OrderType::PegBest).build()?;
    /// assert_eq!(order.order_type, "PEG BEST");
    /// assert_eq!(order.limit_price, Some(150.0));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    /// Create a trailing stop limit order that trails the market by `trail`, starting from
    /// `stop_price`. When it triggers, the limit price is the stop price minus `limit_offset`
    /// for a sell (plus for a buy).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::TrailBy;
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().sell(100).trailing_stop_limit(TrailBy::Amount(2.0), 148.0, 0.5).build()?;
    /// assert_eq!(order.order_type, "TRAIL LIMIT");
    /// assert_eq!(order.aux_price, Some(2.0));
    /// assert_eq!(order.limit_price_offset, Some(0.5));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn trailing_stop_limit(mut self, trail: TrailBy, stop_price: impl Into<f64>, limit_offset: f64) -> Self {
        self.order_type = Some(OrderType::TrailingStopLimit);
        self.set_trail(trail);
        self.trail_stop_price = Some(stop_price.into());
        self.limit_price_offset = Some(limit_offset);
        self
    }

    fn set_trail(&mut self, trail: TrailBy) {
        match trail {
            TrailBy::Amount(amount) => {
                self.aux_price = Some(amount);
                self.trailing_percent = None;
            }
            TrailBy::Percent(percent) => {
                self.trailing_percent = Some(percent);
                self.aux_price = None;
            }
        }
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
        self.time_in_force = TimeInForce::OnOpen;
        self
    }

    /// Limit on Open - executes as limit order at market open
    pub fn limit_on_open(mut self, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(limit_price.into());
        self.time_in_force = TimeInForce::OnOpen;
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

    /// Good till canceled order
    pub fn good_till_canceled(mut self) -> Self {
        self.time_in_force = TimeInForce::GoodTillCanceled;
        self
    }

    /// Good till specific date
    pub fn good_till_date(mut self, date: impl Into<String>) -> Self {
        self.time_in_force = TimeInForce::GoodTillDate;
        self.good_till_date = Some(date.into());
        self
    }

    /// Good till crossing order
    pub fn good_till_crossing(mut self) -> Self {
        self.time_in_force = TimeInForce::GoodTillCrossing;
        self
    }

    /// Day till canceled order
    pub fn day_till_canceled(mut self) -> Self {
        self.time_in_force = TimeInForce::DayTillCanceled;
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
    pub fn parent(mut self, parent_id: impl Into<OrderId>) -> Self {
        self.parent_id = Some(parent_id.into().value());
        self
    }

    /// Join a One-Cancels-All group.
    ///
    /// `oca_type` tells TWS what to do with the rest of the group when one order fills;
    /// see [`OcaType`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::OcaType;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(150.0)
    ///     .oca_group("MyOCA", OcaType::CancelWithBlock)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn oca_group(mut self, group: impl Into<String>, oca_type: OcaType) -> Self {
        self.oca_group = Some(group.into());
        self.oca_type = oca_type;
        self
    }

    /// Set how simulated stop, stop-limit and trailing-stop orders are triggered.
    ///
    /// See [`TriggerMethod`]. The default is [`TriggerMethod::Default`], which lets TWS pick
    /// per security type.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::conditions::TriggerMethod;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .sell(100)
    ///     .stop(140.0)
    ///     .trigger_method(TriggerMethod::Last)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn trigger_method(mut self, method: TriggerMethod) -> Self {
        self.trigger_method = method;
        self
    }

    /// Set the order's origin. Institutional customers only; see [`OrderOrigin`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::OrderOrigin;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .origin(OrderOrigin::Firm)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn origin(mut self, origin: OrderOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Set the short sale slot. Institutional short sales only; see [`ShortSaleSlot`].
    ///
    /// [`ShortSaleSlot::ThirdParty`] also needs [`designated_location`](Self::designated_location).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::ShortSaleSlot;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .sell_short(100)
    ///     .market()
    ///     .short_sale_slot(ShortSaleSlot::Broker)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn short_sale_slot(mut self, slot: ShortSaleSlot) -> Self {
        self.short_sale_slot = slot;
        self
    }

    /// Set where the shares to short come from.
    ///
    /// Only meaningful with [`ShortSaleSlot::ThirdParty`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::ShortSaleSlot;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .sell_short(100)
    ///     .market()
    ///     .short_sale_slot(ShortSaleSlot::ThirdParty)
    ///     .designated_location("ABC SECURITIES")
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn designated_location(mut self, location: impl Into<String>) -> Self {
        self.designated_location = Some(location.into());
        self
    }

    /// Set the NYSE Rule 80A designation.
    ///
    /// Institutional trading only; see [`Rule80A`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::Rule80A;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(150.0)
    ///     .rule_80_a(Rule80A::Agency)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn rule_80_a(mut self, rule: Rule80A) -> Self {
        self.rule_80_a = Some(rule);
        self
    }

    /// Set whether the order opens or closes a position.
    ///
    /// Institutional customers only; see [`OrderOpenClose`]. With
    /// [`Action::Buy`], [`Open`](OrderOpenClose::Open) opens a new long position and
    /// [`Close`](OrderOpenClose::Close) closes an existing short one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::OrderOpenClose;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(150.0)
    ///     .open_close(OrderOpenClose::Open)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn open_close(mut self, open_close: OrderOpenClose) -> Self {
        self.open_close = Some(open_close);
        self
    }

    /// Do not transmit order immediately
    pub fn do_not_transmit(mut self) -> Self {
        self.transmit = false;
        self
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
    /// ```ignore
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
    /// ```ignore
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

    /// Set whether the [`volatility`](Self::volatility) figure is daily or annual.
    ///
    /// VOL orders only; see [`VolatilityType`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::VolatilityType;
    /// use ibapi::orders::builder::OrderType;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(1)
    ///     .order_type(OrderType::Volatility)
    ///     .volatility(0.25)
    ///     .volatility_type(VolatilityType::Annual)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn volatility_type(mut self, volatility_type: VolatilityType) -> Self {
        self.volatility_type = Some(volatility_type);
        self
    }

    /// Set how TWS computes the limit price for a volatility order.
    ///
    /// VOL orders only; see [`ReferencePriceType`]. Also drives stock range price monitoring.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// # use ibapi::Client;
    /// # use ibapi::contracts::Contract;
    /// # let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// # let contract = Contract::stock("AAPL").build();
    /// use ibapi::orders::ReferencePriceType;
    /// use ibapi::orders::builder::OrderType;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(1)
    ///     .order_type(OrderType::Volatility)
    ///     .volatility(0.25)
    ///     .reference_price_type(ReferencePriceType::AverageOfNBBO)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn reference_price_type(mut self, reference_price_type: ReferencePriceType) -> Self {
        self.reference_price_type = Some(reference_price_type);
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

    /// Set the minimum quantity per fill, for IBKRATS orders. TWS may reject it with error 10302
    /// ("not allowed for this order"), as it did on a paper account.
    pub fn min_trade_qty(mut self, qty: i32) -> Self {
        self.min_trade_qty = Some(qty);
        self
    }

    /// Set the minimum size of the quotes a [`peg_best`](Self::peg_best) order competes against.
    pub fn min_compete_size(mut self, size: i32) -> Self {
        self.min_compete_size = Some(size);
        self
    }

    /// Pegged to Best (IBKRATS) - competes with the best bid (buy) or offer (sell) as `compete`
    /// says, never beyond `limit_price`. Route the contract to `IBKRATS`.
    ///
    /// Sets `not_held`, which TWS requires for IBKRATS pegs: without it the order goes
    /// `Inactive`. A later order-type setter doesn't undo it. Optionally chain
    /// [`min_compete_size`](Self::min_compete_size). `build()` rejects a non-finite
    /// [`CompeteAgainstBest::Offset`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::{CompeteAgainstBest, MidOffsets};
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder()
    ///     .buy(100)
    ///     .peg_best(150.0, CompeteAgainstBest::Offset(0.01))
    ///     .min_compete_size(200)
    ///     .build()?;
    /// assert_eq!(order.order_type, "PEG BEST");
    /// assert_eq!(order.compete_against_best_offset, Some(0.01));
    /// assert!(order.not_held);
    ///
    /// let up_to_mid = Order::builder()
    ///     .buy(100)
    ///     .peg_best(150.0, CompeteAgainstBest::UpToMid(MidOffsets { at_whole: 0.02, at_half: 0.025 }))
    ///     .build()?;
    /// assert_eq!(up_to_mid.mid_offset_at_whole, Some(0.02));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn peg_best(mut self, limit_price: impl Into<f64>, compete: CompeteAgainstBest) -> Self {
        self.order_type = Some(OrderType::PegBest);
        self.limit_price = Some(limit_price.into());
        self.not_held = true;
        self.compete = Some(compete);
        self
    }

    /// Pegged to Midpoint, IBKRATS form - pegs to the midpoint, offset by `offsets`, never beyond
    /// `limit_price`. Route the contract to `IBKRATS`. For the offset form on other
    /// venues, see [`pegged_to_midpoint`](Self::pegged_to_midpoint).
    ///
    /// Sets `not_held`, which TWS requires for IBKRATS pegs: without it the order goes `Inactive`.
    /// A later order-type setter doesn't undo it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::MidOffsets;
    /// use ibapi::orders::Order;
    ///
    /// let offsets = MidOffsets { at_whole: 0.02, at_half: 0.025 };
    /// let order = Order::builder().buy(100).peg_mid(150.0, offsets).build()?;
    /// assert_eq!(order.order_type, "PEG MID");
    /// assert_eq!(order.mid_offset_at_half, Some(0.025));
    /// assert_eq!(order.aux_price, None);
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn peg_mid(mut self, limit_price: impl Into<f64>, offsets: MidOffsets) -> Self {
        self.order_type = Some(OrderType::PeggedToMidpoint);
        self.limit_price = Some(limit_price.into());
        // Both PEG MID forms share one order type, so `build()` can't tell them apart: each
        // setter clears the other form's field.
        self.aux_price = None;
        self.not_held = true;
        self.mid_offsets = Some(offsets);
        self
    }

    /// Pegged to Market - pegs to the national best offer minus `offset` for a buy, or the
    /// national best bid plus `offset` for a sell. Stocks only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(100).pegged_to_market(0.05).build()?;
    /// assert_eq!(order.order_type, "PEG MKT");
    /// assert_eq!(order.aux_price, Some(0.05));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn pegged_to_market(mut self, offset: f64) -> Self {
        self.order_type = Some(OrderType::PeggedToMarket);
        self.aux_price = Some(offset);
        self
    }

    /// Pegged to Midpoint - pegs to the NBBO midpoint, offset by `offset`, never beyond
    /// `limit_price`. Same argument order as
    /// [`order_builder::pegged_to_midpoint`](crate::orders::order_builder::pegged_to_midpoint).
    /// For the IBKRATS form with whole / half-penny offsets, see [`peg_mid`](Self::peg_mid).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(100).pegged_to_midpoint(0.01, 150.0).build()?;
    /// assert_eq!(order.order_type, "PEG MID");
    /// assert_eq!(order.aux_price, Some(0.01));
    /// assert_eq!(order.limit_price, Some(150.0));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn pegged_to_midpoint(mut self, offset: f64, limit_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::PeggedToMidpoint);
        self.aux_price = Some(offset);
        self.limit_price = Some(limit_price.into());
        // Clears `peg_mid`'s offsets; see there.
        self.mid_offsets = None;
        self
    }

    /// Box Top - executes as a market order at the best price; any unfilled remainder becomes
    /// a limit order at the fill price. Options routed to BOX only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(10).box_top().build()?;
    /// assert_eq!(order.order_type, "BOX TOP");
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn box_top(mut self) -> Self {
        self.order_type = Some(OrderType::BoxTop);
        self
    }

    /// Pegged to Stock - an option order whose price moves by `delta` times the change in the
    /// underlying stock price, starting from `starting_price`.
    ///
    /// The change is measured from the stock reference price, which defaults to the NBBO
    /// midpoint when the order is placed; set it with
    /// [`stock_reference_price`](Self::stock_reference_price). [`stock_range`](Self::stock_range)
    /// cancels the order when the stock leaves the range. Enter `delta` as an absolute
    /// value: TWS treats it as positive for calls and negative for puts.
    ///
    /// Routed to BOX, this is an Auction Pegged to Stock order:
    /// IB may enter it in BOX's price improvement auction, using the delta times the stock
    /// price change as the improvement amount.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder()
    ///     .buy(1)
    ///     .pegged_to_stock(0.5, 2.10)
    ///     .stock_reference_price(150.0)
    ///     .stock_range(140.0, 160.0)
    ///     .build()?;
    /// assert_eq!(order.order_type, "PEG STK");
    /// assert_eq!(order.delta, Some(0.5));
    /// assert_eq!(order.starting_price, Some(2.10));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn pegged_to_stock(mut self, delta: f64, starting_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::PeggedToStock);
        self.delta = Some(delta);
        self.starting_price = Some(starting_price.into());
        self
    }

    /// Set the stock reference price a pegged-to-stock or pegged-to-benchmark order measures
    /// price changes from.
    pub fn stock_reference_price(mut self, price: impl Into<f64>) -> Self {
        self.stock_ref_price = Some(price.into());
        self
    }

    /// Cancel a pegged-to-stock order when the underlying trades outside `lower`..`upper`.
    ///
    /// Writes the same `Order` fields as [`reference_range`](Self::reference_range).
    pub fn stock_range(mut self, lower: impl Into<f64>, upper: impl Into<f64>) -> Self {
        self.stock_range_lower = Some(lower.into());
        self.stock_range_upper = Some(upper.into());
        self
    }

    /// Pegged to Benchmark - an order whose price tracks a different (reference) contract,
    /// starting from `starting_price`.
    ///
    /// [`reference_contract`](Self::reference_contract) is required; `build()` fails without
    /// it. The order price moves by [`pegged_change_amount`](Self::pegged_change_amount) for
    /// every [`reference_change_amount`](Self::reference_change_amount) the reference contract
    /// moves.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder()
    ///     .buy(100)
    ///     .pegged_to_benchmark(50.0)
    ///     .reference_contract(12345, "ISLAND")
    ///     .pegged_change_amount(0.02)
    ///     .reference_change_amount(0.01)
    ///     .stock_reference_price(49.0)
    ///     .reference_range(48.0, 52.0)
    ///     .build()?;
    /// assert_eq!(order.order_type, "PEG BENCH");
    /// assert_eq!(order.reference_contract_id, 12345);
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn pegged_to_benchmark(mut self, starting_price: impl Into<f64>) -> Self {
        self.order_type = Some(OrderType::PeggedToBenchmark);
        self.starting_price = Some(starting_price.into());
        self
    }

    /// Set the contract a pegged-to-benchmark order tracks, by contract id and exchange.
    pub fn reference_contract(mut self, contract_id: impl Into<ContractId>, exchange: impl Into<String>) -> Self {
        self.reference_contract_id = Some(contract_id.into().0);
        self.reference_exchange = Some(exchange.into());
        self
    }

    /// Set how much a pegged-to-benchmark order's price moves per
    /// [`reference_change_amount`](Self::reference_change_amount) of reference-contract movement.
    pub fn pegged_change_amount(mut self, amount: f64) -> Self {
        self.pegged_change_amount = Some(amount);
        self
    }

    /// Move a pegged-to-benchmark order's price opposite the reference contract.
    pub fn pegged_change_amount_decrease(mut self) -> Self {
        self.is_pegged_change_amount_decrease = true;
        self
    }

    /// Set the reference-contract price change that triggers a
    /// [`pegged_change_amount`](Self::pegged_change_amount) adjustment.
    pub fn reference_change_amount(mut self, amount: f64) -> Self {
        self.reference_change_amount = Some(amount);
        self
    }

    /// Keep a pegged-to-benchmark order active only while the reference contract trades
    /// between `lower` and `upper`.
    ///
    /// Writes the same `Order` fields (`stock_range_lower` / `stock_range_upper`) as
    /// [`stock_range`](Self::stock_range).
    pub fn reference_range(self, lower: impl Into<f64>, upper: impl Into<f64>) -> Self {
        self.stock_range(lower, upper)
    }

    /// Mark a SMART-routed combo order non-guaranteed: legs may fill separately.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(1).limit(2.5).non_guaranteed().build()?;
    /// assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn non_guaranteed(mut self) -> Self {
        self.smart_combo_routing_params = non_guaranteed_params();
        self
    }

    /// Set a limit price per combo leg, in the contract's leg order.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(1).market().combo_leg_prices([1.10, 2.25]).build()?;
    /// assert_eq!(order.order_combo_legs.len(), 2);
    /// assert_eq!(order.order_combo_legs[1].price, Some(2.25));
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn combo_leg_prices(mut self, prices: impl IntoIterator<Item = f64>) -> Self {
        self.order_combo_legs = prices.into_iter().map(|price| OrderComboLeg { price: Some(price) }).collect();
        self
    }

    /// Set the manual order time, the time an order was received from a customer, as
    /// `yyyymmdd hh:mm:ss`.
    pub fn manual_order_time(mut self, time: impl Into<String>) -> Self {
        self.manual_order_time = Some(time.into());
        self
    }

    /// Size the order in the quote currency instead of shares or units (forex).
    ///
    /// With a cash quantity, the order quantity may be `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(0).limit(1.10).cash_qty(20_000.0).build()?;
    /// assert_eq!(order.cash_qty, Some(20_000.0));
    /// assert_eq!(order.total_quantity, 0.0);
    /// # Ok::<(), ibapi::orders::builder::ValidationError>(())
    /// ```
    pub fn cash_qty(mut self, cash_qty: f64) -> Self {
        self.cash_qty = Some(cash_qty);
        self
    }

    // Build methods

    /// Build the Order struct with full validation
    pub fn build(self) -> Result<Order, ValidationError> {
        // Validate required fields
        let action = self.action.ok_or(ValidationError::MissingRequiredField("action"))?;
        let quantity_raw = self.quantity.ok_or(ValidationError::MissingRequiredField("quantity"))?;
        let order_type = self.order_type.ok_or(ValidationError::MissingRequiredField("order_type"))?;

        // Validate quantity. A cash-quantity order may leave the quantity at 0.
        let total_quantity = match self.cash_qty {
            Some(cash_qty) => {
                Quantity::new(cash_qty)?;
                if quantity_raw == 0.0 {
                    0.0
                } else {
                    Quantity::new(quantity_raw)?.value()
                }
            }
            None => Quantity::new(quantity_raw)?.value(),
        };

        // Each price field is sent only for the order types that use it (`OrderType::uses_*`),
        // so one left by an earlier order-type setter is dropped instead of riding along.
        let limit_price = if order_type.requires_limit_price() {
            let price_raw = self.limit_price.ok_or(ValidationError::MissingRequiredField("limit_price"))?;
            Some(Price::new(price_raw)?.value())
        } else if order_type.uses_limit_price() {
            self.limit_price.map(Price::new).transpose()?.map(|price| price.value())
        } else {
            None
        };

        // Stop types send the stop price as `aux_price`; the others send the trigger, offset or
        // trail amount their setter wrote.
        let aux_price = if order_type.uses_stop_price() {
            let price_raw = self.stop_price.ok_or(ValidationError::MissingRequiredField("stop_price"))?;
            Some(Price::new(price_raw)?.value())
        } else if order_type.uses_aux_price() {
            self.aux_price
        } else {
            None
        };

        let (trailing_percent, trail_stop_price) = if order_type.uses_trail() {
            if self.trailing_percent.is_none() && aux_price.is_none() {
                return Err(ValidationError::MissingRequiredField("trailing amount or percent"));
            }
            let trail_stop_price = self.trail_stop_price.map(Price::new).transpose()?.map(|price| price.value());
            (self.trailing_percent, trail_stop_price)
        } else {
            (None, None)
        };

        if order_type.requires_aux_price() && !order_type.uses_trail() && aux_price.is_none() {
            return Err(ValidationError::MissingRequiredField("aux_price"));
        }

        let limit_price_offset = match order_type {
            OrderType::TrailingStopLimit => self.limit_price_offset,
            _ => None,
        };

        // Validate volatility for volatility orders
        if order_type == OrderType::Volatility && self.volatility.is_none() {
            return Err(ValidationError::MissingRequiredField("volatility"));
        }

        if order_type == OrderType::PeggedToStock {
            self.delta.ok_or(ValidationError::MissingRequiredField("delta"))?;
            self.starting_price.ok_or(ValidationError::MissingRequiredField("starting_price"))?;
        }

        if order_type == OrderType::PeggedToBenchmark {
            self.starting_price.ok_or(ValidationError::MissingRequiredField("starting_price"))?;
            if self.reference_contract_id.is_none() || self.reference_exchange.is_none() {
                return Err(ValidationError::MissingRequiredField("reference_contract"));
            }
        }

        // Validate time in force specific requirements
        if self.time_in_force == TimeInForce::GoodTillDate && self.good_till_date.is_none() {
            return Err(ValidationError::MissingRequiredField("good_till_date"));
        }

        // Build the order
        let mut order = Order {
            action,
            total_quantity,
            order_type: order_type.as_str().to_string(),
            limit_price,
            aux_price,
            trailing_percent,
            trail_stop_price,
            limit_price_offset,
            ..Default::default()
        };

        // Set time in force
        order.tif = self.time_in_force;

        // Set other fields
        order.outside_rth = self.outside_rth;
        order.hidden = self.hidden;
        order.transmit = self.transmit;

        if let Some(parent_id) = self.parent_id {
            order.parent_id = parent_id;
        }

        if let Some(group) = self.oca_group {
            order.oca_group = group;
            order.oca_type = self.oca_type;
        }

        if let Some(account) = self.account {
            order.account = account;
        }

        if let Some(time) = self.good_after_time {
            order.good_after_time = time;
        }

        if let Some(date_time) = self.good_till_date {
            order.good_till_date = date_time;
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
        }

        order.volatility_type = self.volatility_type;
        order.reference_price_type = self.reference_price_type;

        order.trigger_method = self.trigger_method;
        order.origin = self.origin;
        order.short_sale_slot = self.short_sale_slot;

        if let Some(location) = self.designated_location {
            order.designated_location = location;
        }

        order.rule_80_a = self.rule_80_a;
        order.open_close = self.open_close;

        if let Some(delta) = self.delta {
            order.delta = Some(delta);
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

        // Compete fields are PEG BEST's; mid offsets are PEG MID's, or PEG BEST's when it
        // competes up to the midpoint (the forms C# sends them for). Gated inline rather than by
        // `OrderType::uses_*`: each applies to one type, and up-to-mid depends on the value.
        let mid_offsets = match order_type {
            OrderType::PegBest => {
                order.min_compete_size = self.min_compete_size;
                match self.compete {
                    Some(CompeteAgainstBest::Offset(offset)) => {
                        order.compete_against_best_offset = Some(Price::new(offset)?.value());
                        None
                    }
                    Some(CompeteAgainstBest::UpToMid(offsets)) => {
                        order.compete_against_best_offset = COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID;
                        Some(offsets)
                    }
                    None => None,
                }
            }
            OrderType::PeggedToMidpoint => self.mid_offsets,
            _ => None,
        };
        if let Some(offsets) = mid_offsets {
            order.mid_offset_at_whole = Some(offsets.at_whole);
            order.mid_offset_at_half = Some(offsets.at_half);
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

        // Set starting price
        if let Some(price) = self.starting_price {
            order.starting_price = Some(price);
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
        OrderCondition::Unknown(c) => c.is_conjunction = is_conjunction,
    }
}

/// Builder for an order with preset stop-loss / profit-taker children attached by TWS.
///
/// Created by [`OrderBuilder::preset_stop_loss`] or [`OrderBuilder::preset_profit_taker`].
/// `submit()` allocates the parent and child order ids and sends one place-order request.
/// Set everything else on the [`OrderBuilder`] first; this builder only adds legs.
#[must_use = "AttachedOrdersBuilder does nothing until you call .submit()"]
pub struct AttachedOrdersBuilder<'a, C> {
    pub(crate) parent_builder: OrderBuilder<ClientBound<'a, C>>,
    stop_loss: bool,
    profit_taker: bool,
}

impl<'a, C> AttachedOrdersBuilder<'a, C> {
    fn new(parent_builder: OrderBuilder<ClientBound<'a, C>>) -> Self {
        Self {
            parent_builder,
            stop_loss: false,
            profit_taker: false,
        }
    }

    /// Also attach a preset stop-loss. See [`OrderBuilder::preset_stop_loss`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::contracts::Contract;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).limit(150.0).preset_profit_taker().preset_stop_loss().submit().await?;
    /// println!("stop-loss {:?} profit-taker {:?}", ids.stop_loss, ids.profit_taker);
    /// # Ok(())
    /// # }
    /// ```
    pub fn preset_stop_loss(mut self) -> Self {
        self.stop_loss = true;
        self
    }

    /// Also attach a preset profit-taker. See [`OrderBuilder::preset_profit_taker`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::contracts::Contract;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).limit(150.0).preset_stop_loss().preset_profit_taker().submit().await?;
    /// println!("stop-loss {:?} profit-taker {:?}", ids.stop_loss, ids.profit_taker);
    /// # Ok(())
    /// # }
    /// ```
    pub fn preset_profit_taker(mut self) -> Self {
        self.profit_taker = true;
        self
    }

    /// Builds the parent order and assigns ids in order parent → stop-loss → profit-taker.
    pub(crate) fn build_with_ids(self, mut next_id: impl FnMut() -> i32) -> Result<(Order, AttachedOrderIds), ValidationError> {
        let mut order = self.parent_builder.build()?;
        let parent = next_id();
        order.order_id = parent;
        order.preset_stop_loss_order_id = self.stop_loss.then(&mut next_id);
        order.preset_profit_taker_order_id = self.profit_taker.then(&mut next_id);
        let ids = AttachedOrderIds {
            parent: OrderId(parent),
            stop_loss: order.preset_stop_loss_order_id.map(OrderId),
            profit_taker: order.preset_profit_taker_order_id.map(OrderId),
        };
        Ok((order, ids))
    }
}

/// Entry order type for bracket orders
#[derive(Default)]
enum BracketEntryType {
    #[default]
    None,
    Limit(f64),
    Market,
}

/// Builder for bracket orders
#[must_use = "BracketOrderBuilder does nothing until you call .submit_all()"]
pub struct BracketOrderBuilder<'a, C> {
    pub(crate) parent_builder: OrderBuilder<ClientBound<'a, C>>,
    entry_type: BracketEntryType,
    take_profit_price: Option<f64>,
    stop_loss_price: Option<f64>,
}

impl<'a, C> BracketOrderBuilder<'a, C> {
    fn new(parent_builder: OrderBuilder<ClientBound<'a, C>>) -> Self {
        Self {
            parent_builder,
            entry_type: BracketEntryType::None,
            take_profit_price: None,
            stop_loss_price: None,
        }
    }

    /// Set entry as market order (immediate execution)
    pub fn entry_market(mut self) -> Self {
        self.entry_type = BracketEntryType::Market;
        self
    }

    /// Set entry limit price
    pub fn entry_limit(mut self, price: impl Into<f64>) -> Self {
        self.entry_type = BracketEntryType::Limit(price.into());
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
    pub(crate) fn build(mut self) -> Result<Vec<Order>, ValidationError> {
        // Validate and convert take profit and stop loss prices
        let take_profit_raw = self.take_profit_price.ok_or(ValidationError::MissingRequiredField("take_profit"))?;
        let stop_loss_raw = self.stop_loss_price.ok_or(ValidationError::MissingRequiredField("stop_loss"))?;

        let take_profit = Price::new(take_profit_raw)?;
        let stop_loss = Price::new(stop_loss_raw)?;

        // Set order type based on entry type
        match self.entry_type {
            BracketEntryType::None => {
                return Err(ValidationError::MissingRequiredField("entry (use entry_limit() or entry_market())"));
            }
            BracketEntryType::Limit(price) => {
                let entry_price = Price::new(price)?;
                // Validate bracket order prices
                let prices = BracketPrices {
                    entry: entry_price.value(),
                    take_profit: take_profit.value(),
                    stop_loss: stop_loss.value(),
                };
                validation::validate_bracket_prices(self.parent_builder.action.as_ref(), &prices)?;
                self.parent_builder.order_type = Some(OrderType::Limit);
                self.parent_builder.limit_price = Some(entry_price.value());
            }
            BracketEntryType::Market => {
                // Skip price relationship validation for market orders
                self.parent_builder.order_type = Some(OrderType::Market);
            }
        }

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
            tif: parent.tif.clone(),
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
            tif: parent.tif.clone(),
            outside_rth: parent.outside_rth,
            ..Default::default()
        };

        Ok(vec![parent, take_profit_order, stop_loss_order])
    }
}
