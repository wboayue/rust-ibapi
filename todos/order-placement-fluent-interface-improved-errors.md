# Improved Error Messages for Order Placement Fluent Interface

## Enhanced ValidationError with Context

```rust
// src/orders/builder/types.rs

use std::fmt;

/// Validation errors with detailed context
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    InvalidQuantity {
        value: f64,
        reason: QuantityErrorReason,
    },
    InvalidPrice {
        field: PriceField,
        value: f64,
        reason: PriceErrorReason,
    },
    MissingRequiredField {
        field: &'static str,
        context: &'static str,
    },
    InvalidCombination {
        fields: Vec<&'static str>,
        reason: String,
    },
    InvalidStopPrice {
        stop: f64,
        current: f64,
        action: String,
        reason: &'static str,
    },
    InvalidLimitPrice {
        limit: f64,
        current: f64,
        action: String,
        reason: &'static str,
    },
    InvalidBracketOrder {
        field: BracketField,
        entry: f64,
        take_profit: f64,
        stop_loss: f64,
        action: String,
        reason: String,
    },
    InvalidOrderType {
        order_type: String,
        missing_fields: Vec<&'static str>,
        reason: String,
    },
    InvalidTimeInForce {
        tif: String,
        missing_field: &'static str,
        reason: String,
    },
}

/// Reason for quantity validation failure
#[derive(Debug, Clone, PartialEq)]
pub enum QuantityErrorReason {
    Zero,
    Negative,
    NotANumber,
    Infinite,
    ExceedsMaximum { max: f64 },
}

/// Which price field failed validation
#[derive(Debug, Clone, PartialEq)]
pub enum PriceField {
    LimitPrice,
    StopPrice,
    TrailStopPrice,
    AuxPrice,
    EntryPrice,
    TakeProfitPrice,
    StopLossPrice,
}

/// Reason for price validation failure
#[derive(Debug, Clone, PartialEq)]
pub enum PriceErrorReason {
    Negative,
    NotANumber,
    Infinite,
    ExceedsMaximum { max: f64 },
    BelowMinimum { min: f64 },
}

/// Which bracket order field failed validation
#[derive(Debug, Clone, PartialEq)]
pub enum BracketField {
    TakeProfit,
    StopLoss,
    Entry,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity { value, reason } => {
                write!(f, "Invalid quantity {}: {}", value, reason)
            }
            Self::InvalidPrice { field, value, reason } => {
                write!(f, "Invalid {} value {}: {}", field, value, reason)
            }
            Self::MissingRequiredField { field, context } => {
                write!(f, "Missing required field '{}' for {}", field, context)
            }
            Self::InvalidCombination { fields, reason } => {
                write!(f, "Invalid combination of fields [{}]: {}", fields.join(", "), reason)
            }
            Self::InvalidStopPrice { stop, current, action, reason } => {
                write!(
                    f,
                    "Invalid stop price {} for {} order (current price: {}): {}",
                    stop, action, current, reason
                )
            }
            Self::InvalidLimitPrice { limit, current, action, reason } => {
                write!(
                    f,
                    "Invalid limit price {} for {} order (current price: {}): {}",
                    limit, action, current, reason
                )
            }
            Self::InvalidBracketOrder { field, entry, take_profit, stop_loss, action, reason } => {
                write!(
                    f,
                    "Invalid {} for {} bracket order (entry: {}, TP: {}, SL: {}): {}",
                    field, action, entry, take_profit, stop_loss, reason
                )
            }
            Self::InvalidOrderType { order_type, missing_fields, reason } => {
                write!(
                    f,
                    "Invalid {} order - missing required fields [{}]: {}",
                    order_type,
                    missing_fields.join(", "),
                    reason
                )
            }
            Self::InvalidTimeInForce { tif, missing_field, reason } => {
                write!(
                    f,
                    "Invalid time in force '{}' - missing '{}': {}",
                    tif, missing_field, reason
                )
            }
        }
    }
}

impl fmt::Display for QuantityErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => write!(f, "quantity must be greater than zero"),
            Self::Negative => write!(f, "quantity cannot be negative"),
            Self::NotANumber => write!(f, "quantity must be a valid number"),
            Self::Infinite => write!(f, "quantity cannot be infinite"),
            Self::ExceedsMaximum { max } => {
                write!(f, "quantity exceeds maximum allowed value of {}", max)
            }
        }
    }
}

impl fmt::Display for PriceField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitPrice => write!(f, "limit price"),
            Self::StopPrice => write!(f, "stop price"),
            Self::TrailStopPrice => write!(f, "trail stop price"),
            Self::AuxPrice => write!(f, "auxiliary price"),
            Self::EntryPrice => write!(f, "entry price"),
            Self::TakeProfitPrice => write!(f, "take profit price"),
            Self::StopLossPrice => write!(f, "stop loss price"),
        }
    }
}

impl fmt::Display for PriceErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Negative => write!(f, "price cannot be negative"),
            Self::NotANumber => write!(f, "price must be a valid number"),
            Self::Infinite => write!(f, "price cannot be infinite"),
            Self::ExceedsMaximum { max } => {
                write!(f, "price exceeds maximum allowed value of {}", max)
            }
            Self::BelowMinimum { min } => {
                write!(f, "price is below minimum allowed value of {}", min)
            }
        }
    }
}

impl fmt::Display for BracketField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TakeProfit => write!(f, "take profit price"),
            Self::StopLoss => write!(f, "stop loss price"),
            Self::Entry => write!(f, "entry price"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Updated Quantity validation with better error context
impl Quantity {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value == 0.0 {
            return Err(ValidationError::InvalidQuantity {
                value,
                reason: QuantityErrorReason::Zero,
            });
        }
        if value < 0.0 {
            return Err(ValidationError::InvalidQuantity {
                value,
                reason: QuantityErrorReason::Negative,
            });
        }
        if value.is_nan() {
            return Err(ValidationError::InvalidQuantity {
                value,
                reason: QuantityErrorReason::NotANumber,
            });
        }
        if value.is_infinite() {
            return Err(ValidationError::InvalidQuantity {
                value,
                reason: QuantityErrorReason::Infinite,
            });
        }
        // Optional: Add maximum quantity check
        const MAX_QUANTITY: f64 = 1_000_000_000.0;
        if value > MAX_QUANTITY {
            return Err(ValidationError::InvalidQuantity {
                value,
                reason: QuantityErrorReason::ExceedsMaximum { max: MAX_QUANTITY },
            });
        }
        Ok(Self(value))
    }
}

/// Updated Price validation with field context
impl Price {
    pub fn new_with_context(value: f64, field: PriceField) -> Result<Self, ValidationError> {
        if value < 0.0 {
            return Err(ValidationError::InvalidPrice {
                field,
                value,
                reason: PriceErrorReason::Negative,
            });
        }
        if value.is_nan() {
            return Err(ValidationError::InvalidPrice {
                field,
                value,
                reason: PriceErrorReason::NotANumber,
            });
        }
        if value.is_infinite() {
            return Err(ValidationError::InvalidPrice {
                field,
                value,
                reason: PriceErrorReason::Infinite,
            });
        }
        // Optional: Add maximum price check
        const MAX_PRICE: f64 = 1_000_000.0;
        if value > MAX_PRICE {
            return Err(ValidationError::InvalidPrice {
                field,
                value,
                reason: PriceErrorReason::ExceedsMaximum { max: MAX_PRICE },
            });
        }
        Ok(Self(value))
    }
    
    // Keep backward compatible constructor
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        Self::new_with_context(value, PriceField::LimitPrice)
    }
}
```

## Updated OrderBuilder build() Method with Context

```rust
// src/orders/builder/order_builder.rs

impl<'a, C> OrderBuilder<'a, C> {
    /// Build the Order struct with full validation and detailed error context
    pub fn build(self) -> Result<Order, ValidationError> {
        // Validate required fields with context
        let action = self.action.ok_or(ValidationError::MissingRequiredField {
            field: "action",
            context: "order creation - must specify buy() or sell()",
        })?;
        
        let quantity_raw = self.quantity.ok_or(ValidationError::MissingRequiredField {
            field: "quantity",
            context: "order creation - quantity must be specified in buy() or sell()",
        })?;
        
        let order_type = self.order_type.ok_or(ValidationError::MissingRequiredField {
            field: "order_type",
            context: "order creation - must specify order type (market(), limit(), stop(), etc.)",
        })?;
        
        // Validate quantity with detailed error
        let quantity = Quantity::new(quantity_raw)?;
        
        // Validate prices based on order type with field context
        let limit_price = match order_type {
            OrderType::Limit | OrderType::StopLimit | OrderType::LimitOnClose | OrderType::LimitOnOpen => {
                let price_raw = self.limit_price.ok_or(ValidationError::InvalidOrderType {
                    order_type: order_type.as_str().to_string(),
                    missing_fields: vec!["limit_price"],
                    reason: format!("{} orders require a limit price", order_type.as_str()),
                })?;
                Some(Price::new_with_context(price_raw, PriceField::LimitPrice)?)
            }
            _ => {
                if let Some(price_raw) = self.limit_price {
                    Some(Price::new_with_context(price_raw, PriceField::LimitPrice)?)
                } else {
                    None
                }
            }
        };
        
        let stop_price = match order_type {
            OrderType::Stop | OrderType::StopLimit => {
                let price_raw = self.stop_price.ok_or(ValidationError::InvalidOrderType {
                    order_type: order_type.as_str().to_string(),
                    missing_fields: vec!["stop_price"],
                    reason: format!("{} orders require a stop price", order_type.as_str()),
                })?;
                Some(Price::new_with_context(price_raw, PriceField::StopPrice)?)
            }
            _ => {
                if let Some(price_raw) = self.stop_price {
                    Some(Price::new_with_context(price_raw, PriceField::StopPrice)?)
                } else {
                    None
                }
            }
        };
        
        let trail_stop_price = match order_type {
            OrderType::TrailingStop | OrderType::TrailingStopLimit => {
                if self.trailing_percent.is_none() && self.trail_stop_price.is_none() {
                    return Err(ValidationError::InvalidOrderType {
                        order_type: order_type.as_str().to_string(),
                        missing_fields: vec!["trailing_percent or trail_stop_price"],
                        reason: "Trailing orders require either a trailing percentage or stop price".to_string(),
                    });
                }
                if let Some(price_raw) = self.trail_stop_price {
                    Some(Price::new_with_context(price_raw, PriceField::TrailStopPrice)?)
                } else {
                    None
                }
            }
            _ => None,
        };
        
        // Validate volatility for volatility orders with context
        if order_type == OrderType::Volatility && self.volatility.is_none() {
            return Err(ValidationError::InvalidOrderType {
                order_type: "Volatility".to_string(),
                missing_fields: vec!["volatility"],
                reason: "Volatility orders require a volatility parameter".to_string(),
            });
        }
        
        // Validate time in force specific requirements with context
        if let TimeInForce::GoodTillDate { .. } = &self.time_in_force {
            if self.good_till_date.is_none() {
                return Err(ValidationError::InvalidTimeInForce {
                    tif: "GoodTillDate".to_string(),
                    missing_field: "good_till_date",
                    reason: "GoodTillDate orders require a specific date to be set".to_string(),
                });
            }
        }
        
        // Build the order (rest remains the same)
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
        
        // Set other fields (rest remains the same)
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
```

## Updated Bracket Order Validation with Context

```rust
// src/orders/builder/validation.rs

/// Validates bracket order prices with detailed context
pub fn validate_bracket_prices(
    action: Option<&Action>,
    entry: f64,
    take_profit: f64,
    stop_loss: f64,
) -> Result<(), ValidationError> {
    let action = action.ok_or(ValidationError::MissingRequiredField {
        field: "action",
        context: "bracket order creation - must specify buy() or sell() before bracket()",
    })?;
    
    let action_str = format!("{:?}", action);
    
    match action {
        Action::Buy => {
            if take_profit <= entry {
                return Err(ValidationError::InvalidBracketOrder {
                    field: BracketField::TakeProfit,
                    entry,
                    take_profit,
                    stop_loss,
                    action: action_str,
                    reason: format!(
                        "take profit must be above entry price for buy orders (TP: {} <= Entry: {})",
                        take_profit, entry
                    ),
                });
            }
            if stop_loss >= entry {
                return Err(ValidationError::InvalidBracketOrder {
                    field: BracketField::StopLoss,
                    entry,
                    take_profit,
                    stop_loss,
                    action: action_str,
                    reason: format!(
                        "stop loss must be below entry price for buy orders (SL: {} >= Entry: {})",
                        stop_loss, entry
                    ),
                });
            }
        }
        Action::Sell | Action::SellShort => {
            if take_profit >= entry {
                return Err(ValidationError::InvalidBracketOrder {
                    field: BracketField::TakeProfit,
                    entry,
                    take_profit,
                    stop_loss,
                    action: action_str,
                    reason: format!(
                        "take profit must be below entry price for sell orders (TP: {} >= Entry: {})",
                        take_profit, entry
                    ),
                });
            }
            if stop_loss <= entry {
                return Err(ValidationError::InvalidBracketOrder {
                    field: BracketField::StopLoss,
                    entry,
                    take_profit,
                    stop_loss,
                    action: action_str,
                    reason: format!(
                        "stop loss must be above entry price for sell orders (SL: {} <= Entry: {})",
                        stop_loss, entry
                    ),
                });
            }
        }
        _ => {}
    }
    
    Ok(())
}

/// Validates stop price relative to current market price with context
pub fn validate_stop_price(
    action: &Action,
    stop_price: f64,
    current_price: Option<f64>,
) -> Result<(), ValidationError> {
    if let Some(current) = current_price {
        let action_str = format!("{:?}", action);
        
        match action {
            Action::Buy => {
                if stop_price <= current {
                    return Err(ValidationError::InvalidStopPrice {
                        stop: stop_price,
                        current,
                        action: action_str,
                        reason: "buy stop orders must have stop price above current market price",
                    });
                }
            }
            Action::Sell | Action::SellShort => {
                if stop_price >= current {
                    return Err(ValidationError::InvalidStopPrice {
                        stop: stop_price,
                        current,
                        action: action_str,
                        reason: "sell stop orders must have stop price below current market price",
                    });
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Updated BracketOrderBuilder with Context

```rust
// src/orders/builder/order_builder.rs

impl<'a, C> BracketOrderBuilder<'a, C> {
    /// Build bracket orders with full validation and detailed error context
    pub fn build(mut self) -> Result<Vec<Order>, ValidationError> {
        // Validate and convert prices with context
        let entry_price_raw = self.entry_price
            .ok_or(ValidationError::MissingRequiredField {
                field: "entry_price",
                context: "bracket order - use entry_limit() to set entry price",
            })?;
        let take_profit_raw = self.take_profit_price
            .ok_or(ValidationError::MissingRequiredField {
                field: "take_profit",
                context: "bracket order - use take_profit() to set target price",
            })?;
        let stop_loss_raw = self.stop_loss_price
            .ok_or(ValidationError::MissingRequiredField {
                field: "stop_loss",
                context: "bracket order - use stop_loss() to set stop price",
            })?;
        
        let entry_price = Price::new_with_context(entry_price_raw, PriceField::EntryPrice)?;
        let take_profit = Price::new_with_context(take_profit_raw, PriceField::TakeProfitPrice)?;
        let stop_loss = Price::new_with_context(stop_loss_raw, PriceField::StopLossPrice)?;
        
        // Validate bracket order prices with full context
        validation::validate_bracket_prices(
            self.parent_builder.action.as_ref(),
            entry_price.value(),
            take_profit.value(),
            stop_loss.value(),
        )?;
        
        // Rest of the implementation remains the same
        self.parent_builder.order_type = Some(OrderType::Limit);
        self.parent_builder.limit_price = Some(entry_price.value());
        
        let mut parent = self.parent_builder.build()?;
        parent.transmit = false;
        
        let mut take_profit_order = Order::default();
        take_profit_order.action = parent.action.reverse();
        take_profit_order.order_type = "LMT".to_string();
        take_profit_order.total_quantity = parent.total_quantity;
        take_profit_order.limit_price = Some(take_profit.value());
        take_profit_order.parent_id = parent.order_id;
        take_profit_order.transmit = false;
        
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

## Example Error Messages

With these improvements, users will see much more helpful error messages:

```rust
// Before:
// Error: Invalid quantity: -100

// After:
// Error: Invalid quantity -100: quantity cannot be negative

// Before:
// Error: Missing required field: limit_price

// After:
// Error: Missing required field 'limit_price' for order creation - LMT orders require a limit price

// Before:
// Error: Invalid bracket order: Take profit (45) must be above entry (50) for buy orders

// After:
// Error: Invalid take profit price for Buy bracket order (entry: 50, TP: 45, SL: 40): take profit must be above entry price for buy orders (TP: 45 <= Entry: 50)

// Before:
// Error: Invalid stop price 45 for current price 50

// After:
// Error: Invalid stop price 45 for Buy order (current price: 50): buy stop orders must have stop price above current market price
```

## Benefits of Enhanced Error Messages

1. **Clear Field Identification**: Users know exactly which field failed validation
2. **Contextual Information**: Errors explain why the validation failed and what the requirements are
3. **Actionable Guidance**: Error messages suggest how to fix the issue
4. **Complete State Information**: For complex validations like bracket orders, all relevant values are shown
5. **Type-Safe Error Handling**: Enums prevent typos and ensure consistent error reporting
6. **Debugging Aid**: Detailed context makes it easier to diagnose issues in production

These improvements make the API much more user-friendly and reduce the time developers spend debugging validation failures.