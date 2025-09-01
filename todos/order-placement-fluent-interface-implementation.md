# Order Placement Fluent Interface - Implementation Guide

## Executive Summary

This document provides a comprehensive design and implementation plan for adding a fluent interface to the order placement API in rust-ibapi. The fluent interface will provide a more intuitive, type-safe, and discoverable API for creating and submitting orders while maintaining full backward compatibility with the existing API.

## Design Goals

1. **Better Discoverability**: IDE autocomplete guides users through available options
2. **Type Safety**: Invalid combinations prevented at compile time
3. **Cleaner Code**: No mutable variables or default structs required
4. **Optional Parameters**: Clear distinction between required and optional parameters
5. **Validation**: Orders validated before submission (all validation deferred to build() to avoid silent failures)
6. **Backwards Compatibility**: Coexist with existing API
7. **Shared Code**: Maximize code reuse between sync and async implementations

## Proposed API Design

### Basic Usage Examples

```rust
// Simple market order
let order_id: OrderId = client.order(&contract)
    .buy(100)
    .market()
    .submit()?;

// Limit order with time in force
let order_id: OrderId = client.order(&contract)
    .sell(200)
    .limit(150.50)
    .time_in_force(TimeInForce::GoodTillCancel)
    .submit()?;

// Complex order with multiple conditions
let order_id: OrderId = client.order(&contract)
    .buy(100)
    .limit(50.0)
    .outside_rth()
    .hidden()
    .submit()?;

// Bracket order (creates parent + take profit + stop loss orders)
let bracket_ids: BracketOrderIds = client.order(&contract)
    .buy(100)
    .bracket()  // Must use .bracket() to create bracket orders
    .entry_limit(50.0)
    .take_profit(55.0)
    .stop_loss(45.0)
    .submit_all()?;

// Access individual bracket order IDs
println!("Parent order: {}", bracket_ids.parent);
println!("Take profit order: {}", bracket_ids.take_profit);
println!("Stop loss order: {}", bracket_ids.stop_loss);

// What-if order (for margin/commission calculation)
let analysis = client.order(&contract)
    .buy(100)
    .limit(50.0)
    .what_if()
    .analyze()?;
```

## Implementation Architecture

### Module Structure

```
src/orders/
├── mod.rs                    # Existing module definitions
├── builder/
│   ├── mod.rs               # Builder module exports
│   ├── common.rs            # Shared builder logic
│   ├── order_builder.rs     # Main OrderBuilder struct
│   ├── types.rs             # NewType wrappers and enums
│   ├── validation.rs        # Order validation logic
│   └── tests.rs             # Comprehensive test suite
├── common/                   # Existing common module
├── sync.rs                   # Sync-specific implementations
└── async.rs                  # Async-specific implementations
```

### Core Components

#### 1. NewType Wrappers and Enums

```rust
// src/orders/builder/types.rs

use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents a unique order identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub i32);

impl OrderId {
    /// Creates a new OrderId
    pub fn new(id: i32) -> Self {
        Self(id)
    }
    
    /// Returns the inner i32 value
    pub fn value(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for OrderId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<OrderId> for i32 {
    fn from(id: OrderId) -> i32 {
        id.0
    }
}

/// Represents the order IDs for a bracket order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BracketOrderIds {
    /// The parent order ID
    pub parent: OrderId,
    /// The take profit order ID
    pub take_profit: OrderId,
    /// The stop loss order ID
    pub stop_loss: OrderId,
}

impl BracketOrderIds {
    /// Creates a new BracketOrderIds
    pub fn new(parent: i32, take_profit: i32, stop_loss: i32) -> Self {
        Self {
            parent: OrderId(parent),
            take_profit: OrderId(take_profit),
            stop_loss: OrderId(stop_loss),
        }
    }
    
    /// Returns all order IDs as a vector
    pub fn as_vec(&self) -> Vec<OrderId> {
        vec![self.parent, self.take_profit, self.stop_loss]
    }
    
    /// Returns all order IDs as i32 values
    pub fn as_i32_vec(&self) -> Vec<i32> {
        vec![self.parent.0, self.take_profit.0, self.stop_loss.0]
    }
}

impl fmt::Display for BracketOrderIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BracketOrder(parent: {}, tp: {}, sl: {})", 
               self.parent, self.take_profit, self.stop_loss)
    }
}

impl From<Vec<i32>> for BracketOrderIds {
    fn from(ids: Vec<i32>) -> Self {
        assert_eq!(ids.len(), 3, "BracketOrderIds requires exactly 3 order IDs");
        Self::new(ids[0], ids[1], ids[2])
    }
}

impl From<[i32; 3]> for BracketOrderIds {
    fn from(ids: [i32; 3]) -> Self {
        Self::new(ids[0], ids[1], ids[2])
    }
}

/// Represents a quantity of shares/contracts
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quantity(f64);

impl Quantity {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value <= 0.0 {
            return Err(ValidationError::InvalidQuantity(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidQuantity(value));
        }
        Ok(Self(value))
    }
    
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Represents a price value
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(f64);

impl Price {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value < 0.0 {
            return Err(ValidationError::InvalidPrice(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidPrice(value));
        }
        Ok(Self(value))
    }
    
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Time in force options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GoodTillCancel,
    ImmediateOrCancel,
    GoodTillDate { date: String },
    FillOrKill,
    GoodTillCrossing,
    DayTillCanceled,
    Auction,
}

impl TimeInForce {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Day => "DAY",
            Self::GoodTillCancel => "GTC",
            Self::ImmediateOrCancel => "IOC",
            Self::GoodTillDate { .. } => "GTD",
            Self::FillOrKill => "FOK",
            Self::GoodTillCrossing => "GTX",
            Self::DayTillCanceled => "DTC",
            Self::Auction => "AUC",
        }
    }
}

/// Order types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
    TrailingStop,
    TrailingStopLimit,
    MarketOnClose,
    LimitOnClose,
    MarketOnOpen,
    LimitOnOpen,
    PeggedToMarket,
    PeggedToStock,
    PeggedToMidpoint,
    Volatility,
    BoxTop,
    AuctionLimit,
    AuctionRelative,
}

impl OrderType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Market => "MKT",
            Self::Limit => "LMT",
            Self::Stop => "STP",
            Self::StopLimit => "STP LMT",
            Self::TrailingStop => "TRAIL",
            Self::TrailingStopLimit => "TRAIL LIMIT",
            Self::MarketOnClose => "MOC",
            Self::LimitOnClose => "LOC",
            Self::MarketOnOpen => "MKT",
            Self::LimitOnOpen => "LMT",
            Self::PeggedToMarket => "PEG MKT",
            Self::PeggedToStock => "PEG STK",
            Self::PeggedToMidpoint => "PEG MID",
            Self::Volatility => "VOL",
            Self::BoxTop => "BOX TOP",
            Self::AuctionLimit => "LMT",
            Self::AuctionRelative => "REL",
        }
    }
}

/// Validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    InvalidQuantity(f64),
    InvalidPrice(f64),
    MissingRequiredField(&'static str),
    InvalidCombination(String),
    InvalidStopPrice { stop: f64, current: f64 },
    InvalidLimitPrice { limit: f64, current: f64 },
    InvalidBracketOrder(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity(q) => write!(f, "Invalid quantity: {}", q),
            Self::InvalidPrice(p) => write!(f, "Invalid price: {}", p),
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidCombination(msg) => write!(f, "Invalid combination: {}", msg),
            Self::InvalidStopPrice { stop, current } => {
                write!(f, "Invalid stop price {} for current price {}", stop, current)
            }
            Self::InvalidLimitPrice { limit, current } => {
                write!(f, "Invalid limit price {} for current price {}", limit, current)
            }
            Self::InvalidBracketOrder(msg) => write!(f, "Invalid bracket order: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}
```

#### 2. Main OrderBuilder Structure

```rust
// src/orders/builder/order_builder.rs

use crate::contracts::Contract;
use crate::orders::{Order, Action};
use super::types::*;
use crate::market_data::TradingHours;

/// Builder for creating orders with a fluent interface
/// 
/// All validation is deferred to the build() method to ensure
/// no silent failures occur during order construction.
pub struct OrderBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    action: Option<Action>,
    quantity: Option<f64>,  // Store raw value, validate in build()
    order_type: Option<OrderType>,
    limit_price: Option<f64>,  // Store raw value, validate in build()
    stop_price: Option<f64>,   // Store raw value, validate in build()
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
    what_if: bool,
    // Advanced fields
    discretionary_amt: Option<f64>,
    trailing_percent: Option<f64>,
    trail_stop_price: Option<f64>,  // Store raw value, validate in build()
    volatility: Option<f64>,
    volatility_type: Option<i32>,
    delta: Option<f64>,
    aux_price: Option<f64>,
}

impl<'a, C> OrderBuilder<'a, C> {
    /// Creates a new OrderBuilder
    pub(crate) fn new(client: &'a C, contract: &'a Contract) -> Self {
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
            volatility: None,
            volatility_type: None,
            delta: None,
            aux_price: None,
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
    
    /// Hide order from market depth
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
    
    // Bracket orders
    
    /// Create bracket orders with take profit and stop loss
    /// Returns a BracketOrderBuilder for configuring the bracket
    pub fn bracket(mut self) -> BracketOrderBuilder<'a, C> {
        BracketOrderBuilder::new(self)
    }
    
    // Algorithmic trading
    
    /// Set algorithm strategy
    pub fn algo(mut self, strategy: impl Into<String>) -> Self {
        self.algo_strategy = Some(strategy.into());
        self
    }
    
    /// Add algorithm parameter
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
    
    // Order conditions
    
    /// Add a price condition
    pub fn price_condition(mut self, condition: PriceCondition) -> Self {
        self.conditions.push(OrderCondition::Price(condition));
        self
    }
    
    /// Add a time condition
    pub fn time_condition(mut self, condition: TimeCondition) -> Self {
        self.conditions.push(OrderCondition::Time(condition));
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
        let limit_price = match order_type {
            OrderType::Limit | OrderType::StopLimit | OrderType::LimitOnClose | OrderType::LimitOnOpen => {
                let price_raw = self.limit_price
                    .ok_or(ValidationError::MissingRequiredField("limit_price"))?;
                Some(Price::new(price_raw)?)
            }
            _ => {
                // Optional limit price for other order types
                if let Some(price_raw) = self.limit_price {
                    Some(Price::new(price_raw)?)
                } else {
                    None
                }
            }
        };
        
        let stop_price = match order_type {
            OrderType::Stop | OrderType::StopLimit => {
                let price_raw = self.stop_price
                    .ok_or(ValidationError::MissingRequiredField("stop_price"))?;
                Some(Price::new(price_raw)?)
            }
            _ => {
                // Optional stop price for other order types
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
        let mut order = Order::default();
        
        // Set basic fields
        order.action = action;
        order.total_quantity = quantity.value();
        order.order_type = order_type.as_str().to_string();
        
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
        
        // Set time in force
        order.tif = self.time_in_force.as_str().to_string();
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
            order.oca_type = self.oca_type.unwrap_or(0);
        }
        
        if let Some(account) = self.account {
            order.account = account;
        }
        
        if let Some(time) = self.good_after_time {
            order.good_after_time = time;
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
            order.volatility_type = self.volatility_type;
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
        
        Ok(order)
    }
}

/// Builder for bracket orders
/// Creates a parent order with attached take profit and stop loss child orders
/// 
/// Note: When calling submit_all() on a bracket order, it returns a BracketOrderIds struct
/// containing the parent, take_profit, and stop_loss order IDs
/// 
/// All validation is deferred to the build() method to ensure
/// no silent failures occur during bracket order construction.
pub struct BracketOrderBuilder<'a, C> {
    parent_builder: OrderBuilder<'a, C>,
    entry_price: Option<f64>,      // Store raw value, validate in build()
    take_profit_price: Option<f64>, // Store raw value, validate in build()
    stop_loss_price: Option<f64>,   // Store raw value, validate in build()
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
        let entry_price_raw = self.entry_price
            .ok_or(ValidationError::MissingRequiredField("entry_price"))?;
        let take_profit_raw = self.take_profit_price
            .ok_or(ValidationError::MissingRequiredField("take_profit"))?;
        let stop_loss_raw = self.stop_loss_price
            .ok_or(ValidationError::MissingRequiredField("stop_loss"))?;
        
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
        let mut take_profit_order = Order::default();
        take_profit_order.action = parent.action.reverse();
        take_profit_order.order_type = "LMT".to_string();
        take_profit_order.total_quantity = parent.total_quantity;
        take_profit_order.limit_price = Some(take_profit.value());
        take_profit_order.parent_id = parent.order_id;
        take_profit_order.transmit = false;
        
        // Build stop loss order
        let mut stop_loss_order = Order::default();
        stop_loss_order.action = parent.action.reverse();
        stop_loss_order.order_type = "STP".to_string();
        stop_loss_order.total_quantity = parent.total_quantity;
        stop_loss_order.aux_price = Some(stop_loss.value());
        stop_loss_order.parent_id = parent.order_id;
        stop_loss_order.transmit = true;
        
        Ok(vec![parent, take_profit_order, stop_loss_order])
    }
}
```

#### 3. Validation Module

```rust
// src/orders/builder/validation.rs

use super::*;
use crate::orders::Action;

// The validate_order_builder function has been removed since all validation
// is now done directly in the OrderBuilder::build() method to avoid silent failures

/// Validates bracket order prices
pub fn validate_bracket_prices(
    action: Option<&Action>,
    entry: f64,
    take_profit: f64,
    stop_loss: f64,
) -> Result<(), ValidationError> {
    let action = action.ok_or(ValidationError::MissingRequiredField("action"))?;
    
    match action {
        Action::Buy => {
            if take_profit <= entry {
                return Err(ValidationError::InvalidBracketOrder(
                    format!("Take profit ({}) must be above entry ({}) for buy orders", take_profit, entry)
                ));
            }
            if stop_loss >= entry {
                return Err(ValidationError::InvalidBracketOrder(
                    format!("Stop loss ({}) must be below entry ({}) for buy orders", stop_loss, entry)
                ));
            }
        }
        Action::Sell | Action::SellShort => {
            if take_profit >= entry {
                return Err(ValidationError::InvalidBracketOrder(
                    format!("Take profit ({}) must be below entry ({}) for sell orders", take_profit, entry)
                ));
            }
            if stop_loss <= entry {
                return Err(ValidationError::InvalidBracketOrder(
                    format!("Stop loss ({}) must be above entry ({}) for sell orders", stop_loss, entry)
                ));
            }
        }
        _ => {}
    }
    
    Ok(())
}

/// Validates stop price relative to current market price
pub fn validate_stop_price(
    action: &Action,
    stop_price: f64,
    current_price: Option<f64>,
) -> Result<(), ValidationError> {
    if let Some(current) = current_price {
        match action {
            Action::Buy => {
                if stop_price <= current {
                    return Err(ValidationError::InvalidStopPrice {
                        stop: stop_price,
                        current,
                    });
                }
            }
            Action::Sell | Action::SellShort => {
                if stop_price >= current {
                    return Err(ValidationError::InvalidStopPrice {
                        stop: stop_price,
                        current,
                    });
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

#### 4. Client Extension Methods

```rust
// src/orders/builder/common.rs

use crate::contracts::Contract;

/// Extension trait for Client to provide fluent order API
pub trait OrderBuilderExt {
    type Builder;
    
    /// Start building an order for the given contract
    fn order<'a>(&'a self, contract: &'a Contract) -> Self::Builder;
}

// Sync implementation
#[cfg(feature = "sync")]
impl OrderBuilderExt for crate::client::sync::Client {
    type Builder = OrderBuilder<'_, Self>;
    
    fn order<'a>(&'a self, contract: &'a Contract) -> Self::Builder {
        OrderBuilder::new(self, contract)
    }
}

// Async implementation
#[cfg(feature = "async")]
impl OrderBuilderExt for crate::client::r#async::Client {
    type Builder = OrderBuilder<'_, Self>;
    
    fn order<'a>(&'a self, contract: &'a Contract) -> Self::Builder {
        OrderBuilder::new(self, contract)
    }
}

// Sync submit implementation
#[cfg(feature = "sync")]
impl<'a> OrderBuilder<'a, crate::client::sync::Client> {
    /// Submit the order synchronously
    /// Returns the order ID assigned to the submitted order
    pub fn submit(self) -> Result<OrderId, crate::Error> {
        let order = self.build()?;
        let order_id = self.client.next_order_id();
        self.client.submit_order(order_id, self.contract, &order)?;
        Ok(OrderId::new(order_id))
    }
    
    /// Submit bracket orders synchronously
    /// Returns BracketOrderIds containing all three order IDs
    pub fn submit_bracket(self) -> Result<BracketOrderIds, crate::Error> {
        let bracket_builder = self.bracket();
        let orders = bracket_builder.build()?;
        
        let base_id = self.client.next_order_id();
        let mut order_ids = Vec::new();
        
        for (i, order) in orders.iter().enumerate() {
            let order_id = base_id + i as i32;
            order_ids.push(order_id);
            self.client.submit_order(order_id, self.contract, order)?;
        }
        
        Ok(BracketOrderIds::new(order_ids[0], order_ids[1], order_ids[2]))
    }
    
    /// Submit the bracket orders using the bracket builder
    /// Alias for submit_bracket for consistency with submit_all naming
    pub fn submit_all(self) -> Result<BracketOrderIds, crate::Error> {
        self.submit_bracket()
    }
    
    /// Analyze order for margin/commission (what-if)
    pub fn analyze(self) -> Result<OrderAnalysis, crate::Error> {
        let mut order = self.build()?;
        order.what_if = true;
        
        let order_id = self.client.next_order_id();
        // Submit what-if order and collect response
        // Implementation would depend on existing what-if handling
        todo!("Implement what-if order analysis")
    }
}

// Async submit implementation
#[cfg(feature = "async")]
impl<'a> OrderBuilder<'a, crate::client::r#async::Client> {
    /// Submit the order asynchronously
    /// Returns the order ID assigned to the submitted order
    pub async fn submit(self) -> Result<OrderId, crate::Error> {
        let order = self.build()?;
        let order_id = self.client.next_order_id().await;
        self.client.submit_order(order_id, self.contract, &order).await?;
        Ok(OrderId::new(order_id))
    }
    
    /// Submit bracket orders asynchronously
    /// Returns BracketOrderIds containing all three order IDs
    pub async fn submit_bracket(self) -> Result<BracketOrderIds, crate::Error> {
        let bracket_builder = self.bracket();
        let orders = bracket_builder.build()?;
        
        let base_id = self.client.next_order_id().await;
        let mut order_ids = Vec::new();
        
        for (i, order) in orders.iter().enumerate() {
            let order_id = base_id + i as i32;
            order_ids.push(order_id);
            self.client.submit_order(order_id, self.contract, order).await?;
        }
        
        Ok(BracketOrderIds::new(order_ids[0], order_ids[1], order_ids[2]))
    }
    
    /// Submit the bracket orders using the bracket builder
    /// Alias for submit_bracket for consistency with submit_all naming
    pub async fn submit_all(self) -> Result<BracketOrderIds, crate::Error> {
        self.submit_bracket().await
    }
    
    /// Analyze order for margin/commission (what-if)
    pub async fn analyze(self) -> Result<OrderAnalysis, crate::Error> {
        let mut order = self.build()?;
        order.what_if = true;
        
        let order_id = self.client.next_order_id().await;
        // Submit what-if order and collect response
        // Implementation would depend on existing what-if handling
        todo!("Implement what-if order analysis")
    }
}
```

## Test Strategy

### 1. Unit Tests

```rust
// src/orders/builder/tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::Action;
    
    mod newtypes {
        use super::*;
        
        #[test]
        fn test_quantity_validation() {
            assert!(Quantity::new(100.0).is_ok());
            assert!(Quantity::new(0.0).is_err());
            assert!(Quantity::new(-10.0).is_err());
            assert!(Quantity::new(f64::NAN).is_err());
            assert!(Quantity::new(f64::INFINITY).is_err());
        }
        
        #[test]
        fn test_price_validation() {
            assert!(Price::new(50.0).is_ok());
            assert!(Price::new(0.0).is_ok());
            assert!(Price::new(-10.0).is_err());
            assert!(Price::new(f64::NAN).is_err());
        }
    }
    
    mod builder {
        use super::*;
        
        fn create_test_contract() -> Contract {
            let mut contract = Contract::default();
            contract.symbol = "AAPL".to_string();
            contract.sec_type = "STK".to_string();
            contract.exchange = "SMART".to_string();
            contract.currency = "USD".to_string();
            contract
        }
        
        fn create_test_builder() -> OrderBuilder<'static, ()> {
            let contract = Box::leak(Box::new(create_test_contract()));
            let client = Box::leak(Box::new(()));
            OrderBuilder::new(client, contract)
        }
        
        #[test]
        fn test_simple_market_buy() {
            let builder = create_test_builder()
                .buy(100)
                .market();
                
            let order = builder.build().unwrap();
            assert_eq!(order.action, Action::Buy);
            assert_eq!(order.total_quantity, 100.0);
            assert_eq!(order.order_type, "MKT");
        }
        
        #[test]
        fn test_invalid_quantity() {
            // Test negative quantity
            let builder = create_test_builder()
                .buy(-100)
                .market();
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::InvalidQuantity(_)));
            
            // Test zero quantity
            let builder = create_test_builder()
                .sell(0)
                .market();
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::InvalidQuantity(_)));
            
            // Test NaN quantity
            let builder = create_test_builder()
                .buy(f64::NAN)
                .market();
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::InvalidQuantity(_)));
        }
        
        #[test]
        fn test_invalid_price() {
            // Test negative limit price
            let builder = create_test_builder()
                .buy(100)
                .limit(-50.0);
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::InvalidPrice(_)));
            
            // Test NaN price
            let builder = create_test_builder()
                .sell(100)
                .stop(f64::NAN);
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::InvalidPrice(_)));
        }
        
        #[test]
        fn test_limit_sell_with_tif() {
            let builder = create_test_builder()
                .sell(200)
                .limit(150.50)
                .good_till_cancel();
                
            let order = builder.build().unwrap();
            assert_eq!(order.action, Action::Sell);
            assert_eq!(order.total_quantity, 200.0);
            assert_eq!(order.order_type, "LMT");
            assert_eq!(order.limit_price, Some(150.50));
            assert_eq!(order.tif, "GTC");
        }
        
        #[test]
        fn test_stop_limit_order() {
            let builder = create_test_builder()
                .buy(100)
                .stop_limit(45.0, 45.50)
                .outside_rth();
                
            let order = builder.build().unwrap();
            assert_eq!(order.order_type, "STP LMT");
            assert_eq!(order.aux_price, Some(45.0));
            assert_eq!(order.limit_price, Some(45.50));
            assert!(order.outside_rth);
        }
        
        #[test]
        fn test_trailing_stop_order() {
            let builder = create_test_builder()
                .sell(100)
                .trailing_stop(5.0, 95.0);
                
            let order = builder.build().unwrap();
            assert_eq!(order.order_type, "TRAIL");
            assert_eq!(order.trailing_percent, Some(5.0));
            assert_eq!(order.trail_stop_price, Some(95.0));
        }
        
        #[test]
        fn test_order_with_conditions() {
            let builder = create_test_builder()
                .buy(100)
                .limit(50.0)
                .hidden()
                .account("DU123456")
                .algo("VWAP")
                .algo_param("startTime", "09:30:00")
                .algo_param("endTime", "16:00:00");
                
            let order = builder.build().unwrap();
            assert!(order.hidden);
            assert_eq!(order.account, "DU123456");
            assert_eq!(order.algo_strategy, "VWAP");
            assert_eq!(order.algo_params.len(), 2);
        }
        
        #[test]
        fn test_missing_required_fields() {
            let builder = create_test_builder();
            assert!(builder.build().is_err());
            
            let builder = create_test_builder().buy(100);
            assert!(builder.build().is_err());
            
            let builder = create_test_builder().market();
            assert!(builder.build().is_err());
        }
        
        #[test]
        fn test_invalid_limit_order() {
            let builder = create_test_builder()
                .buy(100)
                .limit(50.0);
                
            // Should have limit price
            let order = builder.build().unwrap();
            assert_eq!(order.limit_price, Some(50.0));
            
            // Test limit order missing price by manually constructing
            // Since we can't directly set order_type without a price through the fluent API,
            // this validates that the build() method properly enforces the requirement
            let mut builder = create_test_builder();
            builder.action = Some(Action::Buy);
            builder.quantity = Some(100.0);
            builder.order_type = Some(OrderType::Limit);
            // Deliberately not setting limit_price
                
            let result = builder.build();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ValidationError::MissingRequiredField("limit_price")));
        }
    }
    
    mod bracket_orders {
        use super::*;
        
        #[test]
        fn test_valid_buy_bracket() {
            let result = validate_bracket_prices(
                Some(&Action::Buy),
                50.0,  // entry
                55.0,  // take profit (above entry)
                45.0,  // stop loss (below entry)
            );
            assert!(result.is_ok());
        }
        
        #[test]
        fn test_valid_sell_bracket() {
            let result = validate_bracket_prices(
                Some(&Action::Sell),
                50.0,  // entry
                45.0,  // take profit (below entry)
                55.0,  // stop loss (above entry)
            );
            assert!(result.is_ok());
        }
        
        #[test]
        fn test_invalid_buy_bracket_take_profit() {
            let result = validate_bracket_prices(
                Some(&Action::Buy),
                50.0,  // entry
                45.0,  // take profit (BELOW entry - invalid)
                45.0,  // stop loss
            );
            assert!(result.is_err());
        }
        
        #[test]
        fn test_invalid_buy_bracket_stop_loss() {
            let result = validate_bracket_prices(
                Some(&Action::Buy),
                50.0,  // entry
                55.0,  // take profit
                55.0,  // stop loss (ABOVE entry - invalid)
            );
            assert!(result.is_err());
        }
        
        #[test]
        fn test_invalid_sell_bracket() {
            let result = validate_bracket_prices(
                Some(&Action::Sell),
                50.0,  // entry
                55.0,  // take profit (ABOVE entry - invalid for sell)
                45.0,  // stop loss (BELOW entry - invalid for sell)
            );
            assert!(result.is_err());
        }
    }
    
    mod validation {
        use super::*;
        
        #[test]
        fn test_stop_price_validation_buy() {
            // Buy stop must be above current price
            assert!(validate_stop_price(&Action::Buy, 55.0, Some(50.0)).is_ok());
            assert!(validate_stop_price(&Action::Buy, 45.0, Some(50.0)).is_err());
        }
        
        #[test]
        fn test_stop_price_validation_sell() {
            // Sell stop must be below current price
            assert!(validate_stop_price(&Action::Sell, 45.0, Some(50.0)).is_ok());
            assert!(validate_stop_price(&Action::Sell, 55.0, Some(50.0)).is_err());
        }
        
        #[test]
        fn test_stop_price_no_current_price() {
            // Should pass if no current price provided
            assert!(validate_stop_price(&Action::Buy, 55.0, None).is_ok());
            assert!(validate_stop_price(&Action::Sell, 45.0, None).is_ok());
        }
    }
}
```

### 2. Integration Test Template

```rust
// tests/order_builder_integration.rs

#[cfg(test)]
mod integration_tests {
    use rust_ibapi::contracts::Contract;
    use rust_ibapi::orders::builder::OrderBuilderExt;
    
    #[test]
    #[cfg(feature = "sync")]
    fn test_sync_order_builder_integration() {
        // This would require a test client setup
        // let client = create_test_client();
        // let contract = create_stock_contract("AAPL");
        
        // let order = client.order(&contract)
        //     .buy(100)
        //     .limit(150.0)
        //     .build()
        //     .unwrap();
        
        // assert_eq!(order.total_quantity, 100.0);
        // assert_eq!(order.limit_price, Some(150.0));
    }
    
    #[tokio::test]
    #[cfg(feature = "async")]
    async fn test_async_order_builder_integration() {
        // Similar async test
    }
}
```

## Implementation Plan

### Phase 1: Core Builder Infrastructure (Week 1)
1. Create module structure
2. Implement NewType wrappers and enums
3. Implement basic OrderBuilder struct
4. Add validation module
5. Write unit tests for types and validation

### Phase 2: Builder Methods (Week 2)
1. Implement action methods (buy/sell)
2. Implement order type methods (market/limit/stop)
3. Implement time-in-force methods
4. Implement order attribute methods
5. Write unit tests for builder methods

### Phase 3: Advanced Features (Week 3)
1. Implement bracket order builder
2. Implement algorithmic order support
3. Implement order conditions
4. Implement what-if analysis
5. Write tests for advanced features

### Phase 4: Client Integration (Week 4)
1. Add OrderBuilderExt trait
2. Implement sync submit methods
3. Implement async submit methods
4. Create integration tests
5. Update documentation and examples

### Phase 5: Documentation and Polish (Week 5)
1. Write comprehensive documentation
2. Create example programs
3. Performance optimization
4. Code review and refactoring
5. Prepare for release

## Migration Guide

### For Existing Users

The new fluent API is purely additive and does not break existing code:

```rust
// Old API still works
let mut order = Order::default();
order.action = Action::Buy;
order.order_type = "LMT".to_string();
order.total_quantity = 100.0;
order.limit_price = Some(50.0);
let order_id = client.next_order_id();
client.submit_order(order_id, &contract, &order)?;

// New fluent API - automatically handles order ID generation and returns OrderId
let order_id: OrderId = client.order(&contract)
    .buy(100)
    .limit(50.0)
    .submit()?;

// Can still get the raw i32 if needed
let raw_id: i32 = order_id.into();
// Or use the value() method
let raw_id = order_id.value();
```

### Gradual Migration

Users can migrate gradually:
1. Start using the fluent API for new code
2. Refactor existing code as needed
3. Both APIs can coexist in the same codebase

## Performance Considerations

1. **Zero-cost abstractions**: NewType wrappers compile to zero overhead
2. **Builder pattern**: Minimal allocations, mostly stack-based
3. **Validation**: Performed once at build time
4. **No runtime overhead**: Same performance as manual Order construction

## Future Enhancements

1. **Macro support**: Declarative order creation with macros
2. **Template orders**: Save and reuse common order configurations
3. **Strategy builders**: Higher-level strategy abstractions
4. **Real-time validation**: Validate against market data
5. **Order modification builder**: Fluent API for modifying existing orders

## Conclusion

This fluent interface design provides a significant improvement in developer experience while maintaining backward compatibility and performance. The implementation is modular, testable, and extensible, setting a foundation for future enhancements to the order management API.