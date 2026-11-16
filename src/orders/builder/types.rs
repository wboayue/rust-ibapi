use serde::{Deserialize, Serialize};
use std::fmt;

use crate::orders::OrderId;

/// Order IDs from [`AttachedOrdersBuilder::submit`](crate::orders::AttachedOrdersBuilder).
///
/// A leg is `Some` when it was requested, which doesn't mean TWS created it — see
/// [`Order::preset_stop_loss_order_id`](crate::orders::Order::preset_stop_loss_order_id).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachedOrderIds {
    /// The parent order ID
    pub parent: OrderId,
    /// The preset stop-loss order ID, if requested
    pub stop_loss: Option<OrderId>,
    /// The preset profit-taker order ID, if requested
    pub profit_taker: Option<OrderId>,
}

/// Represents the order IDs for a bracket order
///
/// Converts from `[i32; 3]` with `From`, and from `Vec<i32>` with `TryFrom`, which fails unless the `Vec` holds exactly three ids.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
        write!(
            f,
            "BracketOrder(parent: {}, tp: {}, sl: {})",
            self.parent, self.take_profit, self.stop_loss
        )
    }
}

impl TryFrom<Vec<i32>> for BracketOrderIds {
    type Error = ValidationError;

    /// Fails with [`ValidationError::InvalidBracketOrder`] unless `ids` has exactly three elements.
    fn try_from(ids: Vec<i32>) -> Result<Self, Self::Error> {
        match ids[..] {
            [parent, take_profit, stop_loss] => Ok(Self::new(parent, take_profit, stop_loss)),
            _ => Err(ValidationError::InvalidBracketOrder(format!("expected 3 order ids, got {}", ids.len()))),
        }
    }
}

impl From<[i32; 3]> for BracketOrderIds {
    fn from(ids: [i32; 3]) -> Self {
        Self::new(ids[0], ids[1], ids[2])
    }
}

/// Represents a quantity of shares/contracts
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quantity(f64);

impl Quantity {
    /// Create a validated quantity ensuring it is positive and finite.
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(ValidationError::InvalidQuantity(value));
        }
        Ok(Self(value))
    }

    /// Access the raw quantity value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Represents a price value
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(f64);

impl Price {
    /// Create a validated price ensuring it is finite.
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if !value.is_finite() {
            return Err(ValidationError::InvalidPrice(value));
        }
        Ok(Self(value))
    }

    /// Access the raw price value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// How far a trailing stop trails the market.
///
/// The `Order` fields it sets depend on the path:
///
/// - [`OrderBuilder::trailing_stop`](crate::orders::OrderBuilder::trailing_stop) and
///   [`OrderBuilder::trailing_stop_limit`](crate::orders::OrderBuilder::trailing_stop_limit):
///   `Amount` sets [`Order::aux_price`](crate::orders::Order::aux_price), `Percent` sets
///   [`Order::trailing_percent`](crate::orders::Order::trailing_percent).
/// - [`AdjustTo::Trail`](crate::orders::order_builder::AdjustTo::Trail), the trailing target of
///   [`attach_adjustable_stop`](crate::orders::order_builder::attach_adjustable_stop): either
///   variant sets [`Order::adjusted_trailing_amount`](crate::orders::Order::adjusted_trailing_amount),
///   and [`Order::adjustable_trailing_unit`](crate::orders::Order::adjustable_trailing_unit)
///   records which one.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrailBy {
    /// Trail by a fixed price amount.
    Amount(f64),
    /// Trail by a percentage of the market price.
    Percent(f64),
}

/// How a PEG BEST order competes. See
/// [`OrderBuilder::peg_best`](crate::orders::OrderBuilder::peg_best).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompeteAgainstBest {
    /// Improve on the best bid (buy) or offer (sell) by this offset. Must be finite.
    Offset(f64),
    /// Compete up to the midpoint, offset from it by [`MidOffsets`].
    UpToMid(MidOffsets),
}

/// Offsets from the midpoint for IBKRATS pegs, named so they can't be swapped. See
/// [`OrderBuilder::peg_mid`](crate::orders::OrderBuilder::peg_mid) and
/// [`CompeteAgainstBest::UpToMid`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidOffsets {
    /// Applied when the spread is an even number of cents wide, so the midpoint is a whole
    /// penny. Whole-penny increments or zero.
    pub at_whole: f64,
    /// Applied when the spread is an odd number of cents wide, so the midpoint is a half
    /// penny. Half-penny increments.
    pub at_half: f64,
}

/// Entry, take-profit and stop-loss prices of a bracket order, named so they can't be swapped.
/// See [`order_builder::bracket_order`](crate::orders::order_builder::bracket_order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BracketPrices {
    /// Limit price of the parent order.
    pub entry: f64,
    /// Limit price of the take-profit order.
    pub take_profit: f64,
    /// Stop price of the stop-loss order.
    pub stop_loss: f64,
}

/// Validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Quantity must be positive and finite.
    InvalidQuantity(f64),
    /// Price must be finite (not NaN or infinity).
    InvalidPrice(f64),
    /// Required builder field was not supplied.
    MissingRequiredField(&'static str),
    /// Combination of inputs violates broker rules.
    InvalidCombination(String),
    /// Stop price conflicts with current market context.
    InvalidStopPrice {
        /// Stop trigger price supplied by caller.
        stop: f64,
        /// Reference market price used for validation.
        current: f64,
    },
    /// Limit price conflicts with current market context.
    InvalidLimitPrice {
        /// Limit price supplied by caller.
        limit: f64,
        /// Reference market price used for validation.
        current: f64,
    },
    /// Bracket order configuration is invalid.
    InvalidBracketOrder(String),
    /// Percentage value outside allowed range (10-50%).
    InvalidPercentage {
        /// The field name (e.g., "max_pct_vol", "pct_vol").
        field: &'static str,
        /// The invalid value provided.
        value: f64,
        /// Minimum allowed value.
        min: f64,
        /// Maximum allowed value.
        max: f64,
    },
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
            Self::InvalidPercentage { field, value, min, max } => {
                write!(f, "Invalid {}: {} (must be between {} and {})", field, value, min, max)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
