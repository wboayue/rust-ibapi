use serde::{Deserialize, Serialize};
use std::fmt;

use crate::builder::ValidationError;

/// Order IDs from [`AttachedOrdersBuilder::submit`](crate::orders::AttachedOrdersBuilder).
///
/// A leg is `Some` when it was requested, which doesn't mean TWS created it — see
/// [`Order::preset_stop_loss_order_id`](crate::orders::Order::preset_stop_loss_order_id).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachedOrderIds {
    /// The parent order ID
    pub parent: i32,
    /// The preset stop-loss order ID, if requested
    pub stop_loss: Option<i32>,
    /// The preset profit-taker order ID, if requested
    pub profit_taker: Option<i32>,
}

/// Represents the order IDs for a bracket order
///
/// Converts from `[i32; 3]` with `From`, and from `Vec<i32>` with `TryFrom`, which fails unless the `Vec` holds exactly three ids.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BracketOrderIds {
    /// The parent order ID
    pub parent: i32,
    /// The take profit order ID
    pub take_profit: i32,
    /// The stop loss order ID
    pub stop_loss: i32,
}

impl BracketOrderIds {
    /// Creates a new BracketOrderIds
    pub fn new(parent: i32, take_profit: i32, stop_loss: i32) -> Self {
        Self {
            parent,
            take_profit,
            stop_loss,
        }
    }

    /// Returns all order IDs as a vector
    pub fn as_vec(&self) -> Vec<i32> {
        vec![self.parent, self.take_profit, self.stop_loss]
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

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
