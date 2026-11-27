use super::algo_builders::AlgoParams;
use super::types::*;
use super::validation;
use crate::accounts::types::{AccountId, ContractId};
use crate::builder::Missing;
use crate::contracts::Contract;
use crate::contracts::TagValue;
use crate::market_data::TradingHours;
use crate::orders::common::order_builder::non_guaranteed_params;
use crate::orders::conditions::TriggerMethod;
use crate::orders::{
    Action, OcaType, Order, OrderComboLeg, OrderCondition, OrderOpenClose, OrderOrigin, ReferencePriceType, Rule80A, ShortSaleSlot, TimeInForce,
    VolatilityType, WhatIfType, COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID,
};
use crate::orders::{OrderId, OrderType};

#[cfg(test)]
#[path = "order_builder_tests.rs"]
mod tests;

/// Builder for creating orders with a fluent interface
///
/// `T` is the builder's target: [`ClientBound`] for a builder from `Client::order`, which can
/// submit, or [`Detached`] for one from [`Order::builder`], which only builds.
///
/// `S` (side) and `K` (kind) track the two choices every order needs. Both start as
/// [`Missing`]. A side setter ([`buy`](Self::buy), [`sell`](Self::sell),
/// [`sell_short`](Self::sell_short), [`sell_long`](Self::sell_long)) makes `S` an [`Action`];
/// an order-type setter ([`market`](Self::market), [`limit`](Self::limit), ...,
/// [`order_type`](Self::order_type)) makes `K` an [`OrderType`]. They can come in either order,
/// and a later setter of the same kind replaces the earlier one. `build()`, `submit()`,
/// `analyze()` and the preset legs exist only on `OrderBuilder<T, Action, OrderType>`, so an
/// order without a side or a type does not compile. Values (quantity, prices) are checked by
/// `build()`.
///
/// # Examples
///
/// ```
/// use ibapi::orders::{Action, Order, OrderType};
///
/// let order = Order::builder().limit(150.0).buy(100).build()?;
/// assert_eq!(order.action, Action::Buy);
/// assert_eq!(order.order_type, Some(OrderType::Limit));
/// # Ok::<(), ibapi::Error>(())
/// ```
///
/// Without an order type there is no `build()`:
///
/// ```compile_fail,E0599
/// use ibapi::orders::Order;
///
/// let order = Order::builder().buy(100).build();
/// ```
///
/// Nor without a side:
///
/// ```compile_fail,E0599
/// use ibapi::orders::Order;
///
/// let order = Order::builder().limit(150.0).build();
/// ```
#[must_use = "OrderBuilder does nothing until you call .submit() (place it) or .build() (offline construction)"]
pub struct OrderBuilder<T, S = Missing, K = Missing> {
    pub(crate) target: T,
    side: S,
    kind: K,
    pub(crate) fields: OrderFields,
}

/// Everything an [`OrderBuilder`] holds besides its target, side and order type. Raw values,
/// validated in `build()`.
pub(crate) struct OrderFields {
    quantity: f64,
    limit_price: Option<f64>,
    stop_price: Option<f64>,
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
    what_if_type: Option<WhatIfType>,

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

impl Default for OrderFields {
    fn default() -> Self {
        Self {
            quantity: 0.0,
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
            what_if_type: None,
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
}

impl<'a, C, K> OrderBuilder<ClientBound<'a, C>, Action, K> {
    /// Create bracket orders with take profit and stop loss
    ///
    /// Needs the side ([`buy`](Self::buy) / [`sell`](Self::sell) / ...); the entry is set on the
    /// returned builder with [`entry_limit`](BracketOrderBuilder::entry_limit) or
    /// [`entry_market`](BracketOrderBuilder::entry_market), replacing any order type set here.
    ///
    /// The prices are the caller's and the three orders are placed by the client. To have TWS
    /// attach children priced from its order presets instead, see
    /// [`preset_stop_loss`](OrderBuilder::preset_stop_loss) / [`preset_profit_taker`](OrderBuilder::preset_profit_taker).
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
    ///     .bracket()
    ///     .entry_limit(150.0)
    ///     .take_profit(160.0)
    ///     .stop_loss(145.0)
    ///     .submit_all()
    ///     .await?;
    /// println!("parent {} take-profit {} stop-loss {}", ids.parent, ids.take_profit, ids.stop_loss);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// A bracket needs the side first:
    ///
    /// ```compile_fail,E0599
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::OrderBuilder;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let bracket = OrderBuilder::new(&(), &contract).bracket();
    /// ```
    pub fn bracket(self) -> BracketOrderBuilder<'a, C> {
        BracketOrderBuilder::new(self.without_kind())
    }
}

impl<'a, C> OrderBuilder<ClientBound<'a, C>, Action, OrderType> {
    /// Ask TWS to attach a stop-loss to this order, priced from its order presets.
    ///
    /// No price is sent; see [`Order::preset_stop_loss_order_id`] for how TWS resolves the
    /// preset and what happens when none is defined. `submit()` doesn't wait for the outcome,
    /// so watch [`order_update_stream`](crate::Client::order_update_stream).
    ///
    /// Call it after the order's own setters: it returns an [`AttachedOrdersBuilder`], which
    /// only adds the other leg and submits.
    ///
    /// For caller-priced children placed by the client, use [`bracket`](OrderBuilder::bracket).
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

    /// Build the order and return it without submitting
    /// Useful for batch operations or custom submission logic
    ///
    /// # Errors
    /// Same as [`build`](OrderBuilder::build).
    pub fn build_order(self) -> Result<Order, crate::Error> {
        self.build()
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
            side: Missing,
            kind: Missing,
            fields: OrderFields::default(),
        }
    }
}

impl<T, S, K> OrderBuilder<T, S, K> {
    fn with_side(self, action: Action) -> OrderBuilder<T, Action, K> {
        OrderBuilder {
            target: self.target,
            side: action,
            kind: self.kind,
            fields: self.fields,
        }
    }

    fn without_kind(self) -> OrderBuilder<T, S, Missing> {
        OrderBuilder {
            target: self.target,
            side: self.side,
            kind: Missing,
            fields: self.fields,
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
    pub fn buy(mut self, quantity: impl Into<f64>) -> OrderBuilder<T, Action, K> {
        self.fields.quantity = quantity.into();
        self.with_side(Action::Buy)
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
    pub fn sell(mut self, quantity: impl Into<f64>) -> OrderBuilder<T, Action, K> {
        self.fields.quantity = quantity.into();
        self.with_side(Action::Sell)
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
    pub fn sell_short(mut self, quantity: impl Into<f64>) -> OrderBuilder<T, Action, K> {
        self.fields.quantity = quantity.into();
        self.with_side(Action::SellShort)
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
    pub fn sell_long(mut self, quantity: impl Into<f64>) -> OrderBuilder<T, Action, K> {
        self.fields.quantity = quantity.into();
        self.with_side(Action::SellLong)
    }

    // Order type methods

    /// Create a market order
    pub fn market(self) -> OrderBuilder<T, S, OrderType> {
        self.order_type(OrderType::Market)
    }

    /// Create a limit order at the specified price
    pub fn limit(mut self, price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(price.into());
        self.order_type(OrderType::Limit)
    }

    /// Create a stop order at the specified stop price
    pub fn stop(mut self, stop_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.stop_price = Some(stop_price.into());
        self.order_type(OrderType::Stop)
    }

    /// Create a stop-limit order
    pub fn stop_limit(mut self, stop_price: impl Into<f64>, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.stop_price = Some(stop_price.into());
        self.fields.limit_price = Some(limit_price.into());
        self.order_type(OrderType::StopLimit)
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
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn trailing_stop(mut self, trail: TrailBy, stop_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.set_trail(trail);
        self.fields.trail_stop_price = Some(stop_price.into());
        self.order_type(OrderType::TrailingStop)
    }

    /// Set the order type without setting any price.
    ///
    /// `build()` sends only the price fields `order_type` uses, so a price set earlier for a type
    /// that doesn't take it is dropped: `.limit(100.0).order_type(OrderType::PeggedToStock)` sends
    /// no limit price. Prefer the named setter (`.relative(..)`, `.pegged_to_midpoint(..)`, ...).
    ///
    /// [`OrderType::Unknown`] is the exception: the crate doesn't know which prices an unlisted
    /// code takes, so `build()` sends every one set earlier (limit, aux — or the stop price when
    /// no aux was set — trail and limit offset) without checking it, and TWS validates them.
    ///
    /// A market-on-open, limit-on-open or auction order is `Market`, `Limit` or `MarketToLimit`
    /// with the time in force that makes it; `market_on_open()`, `limit_on_open(..)` and
    /// `at_auction(..)` set both.
    ///
    /// The type alone satisfies the builder's typestate, but not a type's required prices:
    /// `build()` fails with [`Error::InvalidArgument`](crate::Error::InvalidArgument) when the
    /// type needs a price no earlier setter wrote, such as `.order_type(OrderType::Limit)` with
    /// no limit price.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::OrderType;
    /// use ibapi::orders::Order;
    ///
    /// let order = Order::builder().buy(100).limit(150.0).order_type(OrderType::PegBest).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PegBest));
    /// assert_eq!(order.limit_price, Some(150.0));
    ///
    /// let no_price = Order::builder().buy(100).order_type(OrderType::Limit).build();
    /// assert!(matches!(no_price, Err(ibapi::Error::InvalidArgument(_))));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn order_type(self, order_type: OrderType) -> OrderBuilder<T, S, OrderType> {
        OrderBuilder {
            target: self.target,
            side: self.side,
            kind: order_type,
            fields: self.fields,
        }
    }

    /// Create a trailing stop limit order that trails the market by `trail`, starting from
    /// `stop_price`. When it triggers, the limit price is the stop price minus `limit_offset`
    /// for a sell (plus for a buy).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::builder::TrailBy;
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().sell(100).trailing_stop_limit(TrailBy::Amount(2.0), 148.0, 0.5).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::TrailingStopLimit));
    /// assert_eq!(order.aux_price, Some(2.0));
    /// assert_eq!(order.limit_price_offset, Some(0.5));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn trailing_stop_limit(mut self, trail: TrailBy, stop_price: impl Into<f64>, limit_offset: f64) -> OrderBuilder<T, S, OrderType> {
        self.set_trail(trail);
        self.fields.trail_stop_price = Some(stop_price.into());
        self.fields.limit_price_offset = Some(limit_offset);
        self.order_type(OrderType::TrailingStopLimit)
    }

    fn set_trail(&mut self, trail: TrailBy) {
        match trail {
            TrailBy::Amount(amount) => {
                self.fields.aux_price = Some(amount);
                self.fields.trailing_percent = None;
            }
            TrailBy::Percent(percent) => {
                self.fields.trailing_percent = Some(percent);
                self.fields.aux_price = None;
            }
        }
    }

    /// Market if Touched - triggers market order when price is touched
    pub fn market_if_touched(mut self, trigger_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(trigger_price.into());
        self.order_type(OrderType::MarketIfTouched)
    }

    /// Limit if Touched - triggers limit order when price is touched
    pub fn limit_if_touched(mut self, trigger_price: impl Into<f64>, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(trigger_price.into());
        self.fields.limit_price = Some(limit_price.into());
        self.order_type(OrderType::LimitIfTouched)
    }

    /// Market to Limit - starts as market order, remainder becomes limit
    pub fn market_to_limit(mut self) -> OrderBuilder<T, S, OrderType> {
        // `build()` sends `limit_price` for MTL (an auction order carries one), so clear one an
        // earlier setter wrote; C# `OrderSamples.MarketToLimit` sets none.
        self.fields.limit_price = None;
        self.order_type(OrderType::MarketToLimit)
    }

    /// Discretionary order - limit order with hidden discretionary amount
    pub fn discretionary(mut self, limit_price: impl Into<f64>, discretionary_amt: f64) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        self.fields.discretionary_amt = Some(discretionary_amt);
        self.order_type(OrderType::Limit)
    }

    /// Sweep to Fill - prioritizes speed of execution over price
    pub fn sweep_to_fill(mut self, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        self.fields.sweep_to_fill = true;
        self.order_type(OrderType::Limit)
    }

    /// Block order - for large volume option orders (min 50 contracts)
    pub fn block(mut self, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        self.fields.block_order = true;
        self.order_type(OrderType::Limit)
    }

    /// Midprice order - fills at midpoint between bid/ask or better
    pub fn midprice(mut self, price_cap: Option<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = price_cap;
        self.order_type(OrderType::Midprice)
    }

    /// Relative/Pegged-to-Primary - seeks more aggressive price than NBBO
    pub fn relative(mut self, offset: f64, price_cap: Option<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(offset);
        self.fields.limit_price = price_cap;
        self.order_type(OrderType::Relative)
    }

    /// Passive Relative - seeks less aggressive price than NBBO
    pub fn passive_relative(mut self, offset: f64) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(offset);
        self.order_type(OrderType::PassiveRelative)
    }

    /// Relative + Limit combo (`REL + LMT`); `limit_price` caps the combo
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().buy(1).relative_limit_combo(2.5).non_guaranteed().build()?;
    /// assert_eq!(order.order_type, Some(OrderType::RelativeLimitCombo));
    /// assert_eq!(order.limit_price, Some(2.5));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn relative_limit_combo(mut self, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        // `build()` sends `aux_price` for the REL combos, so clear any an earlier setter wrote;
        // C# `OrderSamples` sets none.
        self.fields.aux_price = None;
        self.order_type(OrderType::RelativeLimitCombo)
    }

    /// Relative + Market combo (`REL + MKT`), with no limit price
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().buy(1).relative_market_combo().build()?;
    /// assert_eq!(order.order_type, Some(OrderType::RelativeMarketCombo));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn relative_market_combo(mut self) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = None;
        self.order_type(OrderType::RelativeMarketCombo)
    }

    /// At Auction - for pre-market opening period execution: `MTL` with `TimeInForce::Auction`
    pub fn at_auction(mut self, price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.time_in_force = TimeInForce::Auction;
        self.fields.limit_price = Some(price.into());
        self.order_type(OrderType::MarketToLimit)
    }

    /// Market on Close - executes as market order at or near closing price
    pub fn market_on_close(self) -> OrderBuilder<T, S, OrderType> {
        self.order_type(OrderType::MarketOnClose)
    }

    /// Limit on Close - executes as limit order at close if price is met
    pub fn limit_on_close(mut self, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        self.order_type(OrderType::LimitOnClose)
    }

    /// Market on Open - executes as market order at market open: `MKT` with `TimeInForce::OnOpen`
    pub fn market_on_open(mut self) -> OrderBuilder<T, S, OrderType> {
        self.fields.time_in_force = TimeInForce::OnOpen;
        self.order_type(OrderType::Market)
    }

    /// Limit on Open - executes as limit order at market open: `LMT` with `TimeInForce::OnOpen`
    pub fn limit_on_open(mut self, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.time_in_force = TimeInForce::OnOpen;
        self.fields.limit_price = Some(limit_price.into());
        self.order_type(OrderType::Limit)
    }

    /// Volatility order (`VOL`) - TWS derives the limit price from `volatility` (options and futures options)
    ///
    /// `volatility` is a percentage (`40.0` = 40%), per C# `OrderSamples`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType, VolatilityType};
    ///
    /// let order = Order::builder().buy(1).volatility_order(40.0, VolatilityType::Annual).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::Volatility));
    /// assert_eq!(order.volatility, Some(40.0));
    /// assert_eq!(order.volatility_type, Some(VolatilityType::Annual));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn volatility_order(mut self, volatility: f64, volatility_type: VolatilityType) -> OrderBuilder<T, S, OrderType> {
        self.fields.volatility = Some(volatility);
        self.fields.volatility_type = Some(volatility_type);
        self.order_type(OrderType::Volatility)
    }

    /// Market with Protection - market order with protection against extreme price movements (futures only)
    pub fn market_with_protection(self) -> OrderBuilder<T, S, OrderType> {
        self.order_type(OrderType::MarketWithProtection)
    }

    /// Stop with Protection - stop order with protection against extreme price movements (futures only)
    pub fn stop_with_protection(mut self, stop_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.stop_price = Some(stop_price.into());
        self.order_type(OrderType::StopWithProtection)
    }

    // Time in force methods

    /// Set time in force for the order
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.fields.time_in_force = tif;
        self
    }

    /// Order valid for the day only
    pub fn day_order(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::Day;
        self
    }

    /// Good till canceled order
    pub fn good_till_canceled(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::GoodTillCanceled;
        self
    }

    /// Good till specific date
    pub fn good_till_date(mut self, date: impl Into<String>) -> Self {
        self.fields.time_in_force = TimeInForce::GoodTillDate;
        self.fields.good_till_date = Some(date.into());
        self
    }

    /// Good till crossing order
    pub fn good_till_crossing(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::GoodTillCrossing;
        self
    }

    /// Day till canceled order
    pub fn day_till_canceled(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::DayTillCanceled;
        self
    }

    /// Fill or kill order
    pub fn fill_or_kill(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::FillOrKill;
        self
    }

    /// Immediate or cancel order
    pub fn immediate_or_cancel(mut self) -> Self {
        self.fields.time_in_force = TimeInForce::ImmediateOrCancel;
        self
    }

    // Trading hours

    /// Allow order execution outside regular trading hours
    pub fn outside_rth(mut self) -> Self {
        self.fields.outside_rth = true;
        self
    }

    /// Restrict order to regular trading hours only
    pub fn regular_hours_only(mut self) -> Self {
        self.fields.outside_rth = false;
        self
    }

    /// Set trading hours preference
    pub fn trading_hours(mut self, hours: TradingHours) -> Self {
        self.fields.outside_rth = matches!(hours, TradingHours::Extended);
        self
    }

    // Order attributes

    /// Hide order from market depth (only works for NASDAQ-routed orders)
    pub fn hidden(mut self) -> Self {
        self.fields.hidden = true;
        self
    }

    /// Set account for order
    pub fn account(mut self, account: impl Into<AccountId>) -> Self {
        self.fields.account = Some(account.into().0);
        self
    }

    /// Set parent order ID for attached orders
    pub fn parent(mut self, parent_id: impl Into<OrderId>) -> Self {
        self.fields.parent_id = Some(parent_id.into().value());
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
        self.fields.oca_group = Some(group.into());
        self.fields.oca_type = oca_type;
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
        self.fields.trigger_method = method;
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
        self.fields.origin = origin;
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
        self.fields.short_sale_slot = slot;
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
        self.fields.designated_location = Some(location.into());
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
        self.fields.rule_80_a = Some(rule);
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
        self.fields.open_close = Some(open_close);
        self
    }

    /// Do not transmit order immediately
    pub fn do_not_transmit(mut self) -> Self {
        self.fields.transmit = false;
        self
    }

    // Conditional orders

    /// Add a condition to the order.
    ///
    /// The first condition is always treated as AND. Use `and_condition()` or `or_condition()`
    /// for subsequent conditions to specify the logical relationship.
    ///
    /// # Examples
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
    ///
    /// A condition builder becomes a condition only once its comparison is set:
    ///
    /// ```compile_fail,E0277
    /// use ibapi::orders::builder::price;
    /// use ibapi::orders::OrderCondition;
    ///
    /// let condition: OrderCondition = price(265598, "SMART").into();
    /// ```
    pub fn condition(mut self, condition: impl Into<OrderCondition>) -> Self {
        let mut cond = condition.into();
        set_conjunction(&mut cond, true);
        self.fields.conditions.push(cond);
        self
    }

    /// Add a condition that must be met along with previous conditions (AND logic).
    ///
    /// # Examples
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
        if let Some(prev) = self.fields.conditions.last_mut() {
            set_conjunction(prev, true);
        }
        self.fields.conditions.push(condition.into());
        self
    }

    /// Add a condition where either this OR previous conditions trigger the order (OR logic).
    ///
    /// # Examples
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
        if let Some(prev) = self.fields.conditions.last_mut() {
            set_conjunction(prev, false);
        }
        self.fields.conditions.push(condition.into());
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
        self.fields.algo_strategy = Some(params.strategy);
        self.fields.algo_params.extend(params.params);
        self
    }

    /// Add algorithm parameter.
    ///
    /// Use this to add individual parameters when using a strategy name string.
    /// When using algo builders, parameters are set via the builder methods.
    pub fn algo_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.algo_params.push(TagValue {
            tag: key.into(),
            value: value.into(),
        });
        self
    }

    // What-if orders

    /// Mark as what-if order for margin/commission calculation
    pub fn what_if(mut self) -> Self {
        self.fields.what_if = true;
        self
    }

    /// Set what a what-if order checks; see [`WhatIfType`].
    ///
    /// Use it with `.analyze()`, which makes the order what-if. It does not make the order
    /// what-if itself: with `.submit()`, TWS rejects the order with error 321 and nothing is
    /// placed.
    ///
    /// Requires TWS/Gateway server version 217 (`ADDITIONAL_ORDER_PARAMS_2`) or later.
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
    /// use ibapi::orders::WhatIfType;
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(50.0)
    ///     .what_if_type(WhatIfType::MarginRequirements)
    ///     .analyze()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn what_if_type(mut self, what_if_type: WhatIfType) -> Self {
        self.fields.what_if_type = Some(what_if_type);
        self
    }

    // Additional order attributes

    /// Set the volatility figure for a volatility order, as a percentage (`40.0` = 40%).
    ///
    /// Sets the figure only, not the order type; [`volatility_order`](Self::volatility_order)
    /// builds a VOL order in one call.
    pub fn volatility(mut self, volatility: f64) -> Self {
        self.fields.volatility = Some(volatility);
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
    ///     .volatility(40.0)
    ///     .volatility_type(VolatilityType::Annual)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn volatility_type(mut self, volatility_type: VolatilityType) -> Self {
        self.fields.volatility_type = Some(volatility_type);
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
    /// use ibapi::orders::{ReferencePriceType, VolatilityType};
    ///
    /// let _ = client
    ///     .order(&contract)
    ///     .buy(1)
    ///     .volatility_order(40.0, VolatilityType::Annual)
    ///     .reference_price_type(ReferencePriceType::AverageOfNBBO)
    ///     .submit()
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn reference_price_type(mut self, reference_price_type: ReferencePriceType) -> Self {
        self.fields.reference_price_type = Some(reference_price_type);
        self
    }

    /// Mark order as not held
    pub fn not_held(mut self) -> Self {
        self.fields.not_held = true;
        self
    }

    /// Mark order as all or none
    pub fn all_or_none(mut self) -> Self {
        self.fields.all_or_none = true;
        self
    }

    /// Set good after time
    pub fn good_after_time(mut self, time: impl Into<String>) -> Self {
        self.fields.good_after_time = Some(time.into());
        self
    }

    /// Set good till time (stored in good_till_date field)
    pub fn good_till_time(mut self, time: impl Into<String>) -> Self {
        self.fields.good_till_date = Some(time.into());
        self
    }

    // Pegged order configuration

    /// Set the minimum quantity per fill, for IBKRATS orders. TWS may reject it with error 10302
    /// ("not allowed for this order"), as it did on a paper account.
    pub fn min_trade_qty(mut self, qty: i32) -> Self {
        self.fields.min_trade_qty = Some(qty);
        self
    }

    /// Set the minimum size of the quotes a [`peg_best`](Self::peg_best) order competes against.
    pub fn min_compete_size(mut self, size: i32) -> Self {
        self.fields.min_compete_size = Some(size);
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
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder()
    ///     .buy(100)
    ///     .peg_best(150.0, CompeteAgainstBest::Offset(0.01))
    ///     .min_compete_size(200)
    ///     .build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PegBest));
    /// assert_eq!(order.compete_against_best_offset, Some(0.01));
    /// assert!(order.not_held);
    ///
    /// let up_to_mid = Order::builder()
    ///     .buy(100)
    ///     .peg_best(150.0, CompeteAgainstBest::UpToMid(MidOffsets { at_whole: 0.02, at_half: 0.025 }))
    ///     .build()?;
    /// assert_eq!(up_to_mid.mid_offset_at_whole, Some(0.02));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn peg_best(mut self, limit_price: impl Into<f64>, compete: CompeteAgainstBest) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        self.fields.not_held = true;
        self.fields.compete = Some(compete);
        self.order_type(OrderType::PegBest)
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
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let offsets = MidOffsets { at_whole: 0.02, at_half: 0.025 };
    /// let order = Order::builder().buy(100).peg_mid(150.0, offsets).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PeggedToMidpoint));
    /// assert_eq!(order.mid_offset_at_half, Some(0.025));
    /// assert_eq!(order.aux_price, None);
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn peg_mid(mut self, limit_price: impl Into<f64>, offsets: MidOffsets) -> OrderBuilder<T, S, OrderType> {
        self.fields.limit_price = Some(limit_price.into());
        // Both PEG MID forms share one order type, so `build()` can't tell them apart: each
        // setter clears the other form's field.
        self.fields.aux_price = None;
        self.fields.not_held = true;
        self.fields.mid_offsets = Some(offsets);
        self.order_type(OrderType::PeggedToMidpoint)
    }

    /// Pegged to Market - pegs to the national best offer minus `offset` for a buy, or the
    /// national best bid plus `offset` for a sell. Stocks only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().buy(100).pegged_to_market(0.05).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PeggedToMarket));
    /// assert_eq!(order.aux_price, Some(0.05));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pegged_to_market(mut self, offset: f64) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(offset);
        self.order_type(OrderType::PeggedToMarket)
    }

    /// Pegged to Midpoint - pegs to the NBBO midpoint, offset by `offset`, never beyond
    /// `limit_price`. Same argument order as
    /// [`order_builder::pegged_to_midpoint`](crate::orders::order_builder::pegged_to_midpoint).
    /// For the IBKRATS form with whole / half-penny offsets, see [`peg_mid`](Self::peg_mid).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().buy(100).pegged_to_midpoint(0.01, 150.0).build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PeggedToMidpoint));
    /// assert_eq!(order.aux_price, Some(0.01));
    /// assert_eq!(order.limit_price, Some(150.0));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pegged_to_midpoint(mut self, offset: f64, limit_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.aux_price = Some(offset);
        self.fields.limit_price = Some(limit_price.into());
        // Clears `peg_mid`'s offsets; see there.
        self.fields.mid_offsets = None;
        self.order_type(OrderType::PeggedToMidpoint)
    }

    /// Box Top - executes as a market order at the best price; any unfilled remainder becomes
    /// a limit order at the fill price. Options routed to BOX only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder().buy(10).box_top().build()?;
    /// assert_eq!(order.order_type, Some(OrderType::BoxTop));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn box_top(self) -> OrderBuilder<T, S, OrderType> {
        self.order_type(OrderType::BoxTop)
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
    /// use ibapi::orders::{Order, OrderType};
    ///
    /// let order = Order::builder()
    ///     .buy(1)
    ///     .pegged_to_stock(0.5, 2.10)
    ///     .stock_reference_price(150.0)
    ///     .stock_range(140.0, 160.0)
    ///     .build()?;
    /// assert_eq!(order.order_type, Some(OrderType::PeggedToStock));
    /// assert_eq!(order.delta, Some(0.5));
    /// assert_eq!(order.starting_price, Some(2.10));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pegged_to_stock(mut self, delta: f64, starting_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.delta = Some(delta);
        self.fields.starting_price = Some(starting_price.into());
        self.order_type(OrderType::PeggedToStock)
    }

    /// Set the stock reference price a pegged-to-stock or pegged-to-benchmark order measures
    /// price changes from.
    pub fn stock_reference_price(mut self, price: impl Into<f64>) -> Self {
        self.fields.stock_ref_price = Some(price.into());
        self
    }

    /// Cancel a pegged-to-stock order when the underlying trades outside `lower`..`upper`.
    ///
    /// Writes the same `Order` fields as [`reference_range`](Self::reference_range).
    pub fn stock_range(mut self, lower: impl Into<f64>, upper: impl Into<f64>) -> Self {
        self.fields.stock_range_lower = Some(lower.into());
        self.fields.stock_range_upper = Some(upper.into());
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
    /// use ibapi::orders::{Order, OrderType};
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
    /// assert_eq!(order.order_type, Some(OrderType::PeggedToBenchmark));
    /// assert_eq!(order.reference_contract_id, 12345);
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pegged_to_benchmark(mut self, starting_price: impl Into<f64>) -> OrderBuilder<T, S, OrderType> {
        self.fields.starting_price = Some(starting_price.into());
        self.order_type(OrderType::PeggedToBenchmark)
    }

    /// Set the contract a pegged-to-benchmark order tracks, by contract id and exchange.
    pub fn reference_contract(mut self, contract_id: impl Into<ContractId>, exchange: impl Into<String>) -> Self {
        self.fields.reference_contract_id = Some(contract_id.into().0);
        self.fields.reference_exchange = Some(exchange.into());
        self
    }

    /// Set how much a pegged-to-benchmark order's price moves per
    /// [`reference_change_amount`](Self::reference_change_amount) of reference-contract movement.
    pub fn pegged_change_amount(mut self, amount: f64) -> Self {
        self.fields.pegged_change_amount = Some(amount);
        self
    }

    /// Move a pegged-to-benchmark order's price opposite the reference contract.
    pub fn pegged_change_amount_decrease(mut self) -> Self {
        self.fields.is_pegged_change_amount_decrease = true;
        self
    }

    /// Set the reference-contract price change that triggers a
    /// [`pegged_change_amount`](Self::pegged_change_amount) adjustment.
    pub fn reference_change_amount(mut self, amount: f64) -> Self {
        self.fields.reference_change_amount = Some(amount);
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
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn non_guaranteed(mut self) -> Self {
        self.fields.smart_combo_routing_params = non_guaranteed_params();
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
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn combo_leg_prices(mut self, prices: impl IntoIterator<Item = f64>) -> Self {
        self.fields.order_combo_legs = prices.into_iter().map(|price| OrderComboLeg { price: Some(price) }).collect();
        self
    }

    /// Set the manual order time, the time an order was received from a customer, as
    /// `yyyymmdd hh:mm:ss`.
    pub fn manual_order_time(mut self, time: impl Into<String>) -> Self {
        self.fields.manual_order_time = Some(time.into());
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
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn cash_qty(mut self, cash_qty: f64) -> Self {
        self.fields.cash_qty = Some(cash_qty);
        self
    }
}

impl<T> OrderBuilder<T, Action, OrderType> {
    /// Build the [`Order`], validating its values.
    ///
    /// # Errors
    ///
    /// [`Error::Validation`](crate::Error::Validation) for a value a caller may correct: a
    /// quantity or price that is not positive and finite ([`ValidationError::InvalidQuantity`](crate::builder::ValidationError::InvalidQuantity),
    /// [`ValidationError::InvalidPrice`](crate::builder::ValidationError::InvalidPrice)).
    ///
    /// [`Error::InvalidArgument`](crate::Error::InvalidArgument) for a required input the
    /// builder was never given, which the typestate can't see: a type set with
    /// [`order_type`](OrderBuilder::order_type) whose price no earlier setter wrote,
    /// [`pegged_to_benchmark`](OrderBuilder::pegged_to_benchmark) without
    /// [`reference_contract`](OrderBuilder::reference_contract), or
    /// `time_in_force(TimeInForce::GoodTillDate)` without a date.
    pub fn build(self) -> Result<Order, crate::Error> {
        let action = self.side;
        let order_type = self.kind;
        let quantity_raw = self.fields.quantity;

        // Validate quantity. A cash-quantity order may leave the quantity at 0.
        let total_quantity = match self.fields.cash_qty {
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
        // so one left by an earlier order-type setter is dropped instead of riding along. A code
        // this crate doesn't list (`Unknown`) sends every price the caller set, unchecked: the
        // crate can't know which it takes, and dropping them silently is worse than TWS rejecting.
        let unlisted = matches!(order_type, OrderType::Unknown(_));
        let limit_price = if order_type.requires_limit_price() {
            let price_raw = self.fields.limit_price.ok_or_else(|| missing_input(&order_type, "a limit price"))?;
            Some(Price::new(price_raw)?.value())
        } else if order_type.uses_limit_price() || unlisted {
            self.fields.limit_price.map(Price::new).transpose()?.map(|price| price.value())
        } else {
            None
        };

        // Stop types send the stop price as `aux_price`; the others send the trigger, offset or
        // trail amount their setter wrote.
        let aux_price = if order_type.uses_stop_price() {
            let price_raw = self.fields.stop_price.ok_or_else(|| missing_input(&order_type, "a stop price"))?;
            Some(Price::new(price_raw)?.value())
        } else if order_type.uses_aux_price() {
            self.fields.aux_price
        } else if unlisted {
            self.fields.aux_price.or(self.fields.stop_price)
        } else {
            None
        };

        let (trailing_percent, trail_stop_price) = if order_type.uses_trail() {
            if self.fields.trailing_percent.is_none() && aux_price.is_none() {
                return Err(missing_input(&order_type, "trailing amount or percent"));
            }
            let trail_stop_price = self.fields.trail_stop_price.map(Price::new).transpose()?.map(|price| price.value());
            (self.fields.trailing_percent, trail_stop_price)
        } else if unlisted {
            let trail_stop_price = self.fields.trail_stop_price.map(Price::new).transpose()?.map(|price| price.value());
            (self.fields.trailing_percent, trail_stop_price)
        } else {
            (None, None)
        };

        if order_type.requires_aux_price() && !order_type.uses_trail() && aux_price.is_none() {
            return Err(missing_input(&order_type, "aux price (trigger price or offset)"));
        }

        let limit_price_offset = match order_type {
            OrderType::TrailingStopLimit | OrderType::Unknown(_) => self.fields.limit_price_offset,
            _ => None,
        };

        // Validate volatility for volatility orders
        if order_type == OrderType::Volatility && self.fields.volatility.is_none() {
            return Err(missing_input(&order_type, "volatility"));
        }

        if order_type == OrderType::PeggedToStock {
            self.fields.delta.ok_or_else(|| missing_input(&order_type, "delta"))?;
            self.fields.starting_price.ok_or_else(|| missing_input(&order_type, "starting price"))?;
        }

        if order_type == OrderType::PeggedToBenchmark {
            self.fields.starting_price.ok_or_else(|| missing_input(&order_type, "starting price"))?;
            if self.fields.reference_contract_id.is_none() || self.fields.reference_exchange.is_none() {
                return Err(crate::Error::InvalidArgument(
                    "a pegged-to-benchmark order needs the contract it tracks: call .reference_contract(contract_id, exchange)".into(),
                ));
            }
        }

        // Validate time in force specific requirements
        if self.fields.time_in_force == TimeInForce::GoodTillDate && self.fields.good_till_date.is_none() {
            return Err(crate::Error::InvalidArgument(
                "TimeInForce::GoodTillDate needs a date: use .good_till_date(date) or .good_till_time(time)".into(),
            ));
        }

        // Build the order
        let mut order = Order {
            action,
            total_quantity,
            order_type: Some(order_type),
            limit_price,
            aux_price,
            trailing_percent,
            trail_stop_price,
            limit_price_offset,
            ..Default::default()
        };

        // Set time in force
        order.tif = self.fields.time_in_force;

        // Set other fields
        order.outside_rth = self.fields.outside_rth;
        order.hidden = self.fields.hidden;
        order.transmit = self.fields.transmit;

        if let Some(parent_id) = self.fields.parent_id {
            order.parent_id = parent_id;
        }

        if let Some(group) = self.fields.oca_group {
            order.oca_group = group;
            order.oca_type = self.fields.oca_type;
        }

        if let Some(account) = self.fields.account {
            order.account = account;
        }

        if let Some(time) = self.fields.good_after_time {
            order.good_after_time = time;
        }

        if let Some(date_time) = self.fields.good_till_date {
            order.good_till_date = date_time;
        }

        if let Some(strategy) = self.fields.algo_strategy {
            order.algo_strategy = strategy;
            order.algo_params = self.fields.algo_params;
        }

        order.what_if = self.fields.what_if;
        order.what_if_type = self.fields.what_if_type;

        // Set advanced fields
        if let Some(amt) = self.fields.discretionary_amt {
            order.discretionary_amt = amt;
        }

        if let Some(vol) = self.fields.volatility {
            order.volatility = Some(vol);
        }

        order.volatility_type = self.fields.volatility_type;
        order.reference_price_type = self.fields.reference_price_type;

        order.trigger_method = self.fields.trigger_method;
        order.origin = self.fields.origin;
        order.short_sale_slot = self.fields.short_sale_slot;

        if let Some(location) = self.fields.designated_location {
            order.designated_location = location;
        }

        order.rule_80_a = self.fields.rule_80_a;
        order.open_close = self.fields.open_close;

        if let Some(delta) = self.fields.delta {
            order.delta = Some(delta);
        }

        // Set special flags
        order.sweep_to_fill = self.fields.sweep_to_fill;
        order.block_order = self.fields.block_order;
        order.not_held = self.fields.not_held;
        order.all_or_none = self.fields.all_or_none;

        // Set pegged order fields
        if let Some(qty) = self.fields.min_trade_qty {
            order.min_trade_qty = Some(qty);
        }

        // Compete fields are PEG BEST's; mid offsets are PEG MID's, or PEG BEST's when it
        // competes up to the midpoint (the forms C# sends them for). Gated inline rather than by
        // `OrderType::uses_*`: each applies to one type, and up-to-mid depends on the value.
        let mid_offsets = match order.order_type {
            Some(OrderType::PegBest) => {
                order.min_compete_size = self.fields.min_compete_size;
                match self.fields.compete {
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
            Some(OrderType::PeggedToMidpoint) => self.fields.mid_offsets,
            _ => None,
        };
        if let Some(offsets) = mid_offsets {
            order.mid_offset_at_whole = Some(offsets.at_whole);
            order.mid_offset_at_half = Some(offsets.at_half);
        }

        // Set conditions
        if !self.fields.conditions.is_empty() {
            order.conditions = self.fields.conditions;
        }

        // Set reference contract fields for pegged to benchmark orders
        if let Some(id) = self.fields.reference_contract_id {
            order.reference_contract_id = id;
        }

        if let Some(exchange) = self.fields.reference_exchange {
            order.reference_exchange = exchange;
        }

        if let Some(price) = self.fields.stock_ref_price {
            order.stock_ref_price = Some(price);
        }

        if let Some(lower) = self.fields.stock_range_lower {
            order.stock_range_lower = Some(lower);
        }

        if let Some(upper) = self.fields.stock_range_upper {
            order.stock_range_upper = Some(upper);
        }

        if let Some(amount) = self.fields.reference_change_amount {
            order.reference_change_amount = Some(amount);
        }

        if let Some(amount) = self.fields.pegged_change_amount {
            order.pegged_change_amount = Some(amount);
        }

        if self.fields.is_pegged_change_amount_decrease {
            order.is_pegged_change_amount_decrease = true;
        }

        // Set combo order fields
        if !self.fields.order_combo_legs.is_empty() {
            order.order_combo_legs = self.fields.order_combo_legs;
        }

        if !self.fields.smart_combo_routing_params.is_empty() {
            order.smart_combo_routing_params = self.fields.smart_combo_routing_params;
        }

        // Set cash quantity for FX orders
        if let Some(qty) = self.fields.cash_qty {
            order.cash_qty = Some(qty);
        }

        // Set manual order time
        if let Some(time) = self.fields.manual_order_time {
            order.manual_order_time = time;
        }

        // Set starting price
        if let Some(price) = self.fields.starting_price {
            order.starting_price = Some(price);
        }

        Ok(order)
    }
}

/// The error for a price or parameter an order type needs that no setter wrote. Only reachable
/// through [`OrderBuilder::order_type`]: each named setter takes its type's required inputs.
fn missing_input(order_type: &OrderType, what: &str) -> crate::Error {
    crate::Error::InvalidArgument(format!(
        "{order_type:?} order has no {what}: .order_type(..) sets the type alone; use the type's named setter, which takes it"
    ))
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
    pub(crate) parent_builder: OrderBuilder<ClientBound<'a, C>, Action, OrderType>,
    stop_loss: bool,
    profit_taker: bool,
}

impl<'a, C> AttachedOrdersBuilder<'a, C> {
    fn new(parent_builder: OrderBuilder<ClientBound<'a, C>, Action, OrderType>) -> Self {
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
    pub(crate) fn build_with_ids(self, mut next_id: impl FnMut() -> i32) -> Result<(Order, AttachedOrderIds), crate::Error> {
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

/// Builder for bracket orders: an entry, a take-profit limit and a stop-loss stop.
///
/// Created by [`OrderBuilder::bracket`]. `E` (entry), `P` (take-profit price) and `L` (stop-loss
/// price) start as [`Missing`]; [`entry_limit`](Self::entry_limit) or
/// [`entry_market`](Self::entry_market) makes `E` an [`OrderType`], and
/// [`take_profit`](Self::take_profit) / [`stop_loss`](Self::stop_loss) make `P` / `L` an `f64`.
/// `submit_all()` exists only once all three are set, in any order; a second call of a setter
/// replaces the first.
#[must_use = "BracketOrderBuilder does nothing until you call .submit_all()"]
pub struct BracketOrderBuilder<'a, C, E = Missing, P = Missing, L = Missing> {
    pub(crate) parent_builder: OrderBuilder<ClientBound<'a, C>, Action, E>,
    take_profit: P,
    stop_loss: L,
}

impl<'a, C> BracketOrderBuilder<'a, C> {
    fn new(parent_builder: OrderBuilder<ClientBound<'a, C>, Action, Missing>) -> Self {
        Self {
            parent_builder,
            take_profit: Missing,
            stop_loss: Missing,
        }
    }
}

impl<'a, C, E, P, L> BracketOrderBuilder<'a, C, E, P, L> {
    /// Set entry as market order (immediate execution)
    pub fn entry_market(self) -> BracketOrderBuilder<'a, C, OrderType, P, L> {
        BracketOrderBuilder {
            parent_builder: self.parent_builder.market(),
            take_profit: self.take_profit,
            stop_loss: self.stop_loss,
        }
    }

    /// Set entry limit price
    pub fn entry_limit(self, price: impl Into<f64>) -> BracketOrderBuilder<'a, C, OrderType, P, L> {
        BracketOrderBuilder {
            parent_builder: self.parent_builder.limit(price),
            take_profit: self.take_profit,
            stop_loss: self.stop_loss,
        }
    }

    /// Set take profit price
    pub fn take_profit(self, price: impl Into<f64>) -> BracketOrderBuilder<'a, C, E, f64, L> {
        BracketOrderBuilder {
            parent_builder: self.parent_builder,
            take_profit: price.into(),
            stop_loss: self.stop_loss,
        }
    }

    /// Set stop loss price
    pub fn stop_loss(self, price: impl Into<f64>) -> BracketOrderBuilder<'a, C, E, P, f64> {
        BracketOrderBuilder {
            parent_builder: self.parent_builder,
            take_profit: self.take_profit,
            stop_loss: price.into(),
        }
    }
}

impl<C> BracketOrderBuilder<'_, C, OrderType, f64, f64> {
    /// Build bracket orders with full validation
    pub(crate) fn build(self) -> Result<Vec<Order>, crate::Error> {
        let take_profit = Price::new(self.take_profit)?;
        let stop_loss = Price::new(self.stop_loss)?;

        // Build parent order; this validates the quantity and the entry limit price.
        let mut parent = self.parent_builder.build()?;
        parent.transmit = false;

        // A market entry has no price to check the children against.
        if let (Some(OrderType::Limit), Some(entry)) = (&parent.order_type, parent.limit_price) {
            let prices = BracketPrices {
                entry,
                take_profit: take_profit.value(),
                stop_loss: stop_loss.value(),
            };
            validation::validate_bracket_prices(&parent.action, &prices)?;
        }

        // Build take profit order
        let take_profit_order = Order {
            action: parent.action.reverse(),
            order_type: Some(OrderType::Limit),
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
            order_type: Some(OrderType::Stop),
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
