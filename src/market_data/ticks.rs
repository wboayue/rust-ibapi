//! Tick payloads shared by `tick_by_tick` and `historical_ticks`: TWS sends
//! both as the same proto messages.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A last-trade tick: from `tick_by_tick` `last` / `all_last`, or `historical_ticks` `last`
/// (whatToShow = TRADES).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickLast {
    /// Time of the tick.
    pub time: OffsetDateTime,
    /// Last price.
    pub price: f64,
    /// Last size. `0.0` if TWS sends none; every trade and bid/ask tick probed, live and
    /// historical, carried one (`docs/wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize`).
    pub size: f64,
    /// Tick attributes.
    pub attributes: TickAttributeLast,
    /// Source exchange.
    pub exchange: String,
    /// Conditions of the tick. Refer to Trade Conditions page for more details: <https://www.interactivebrokers.com/en/index.php?f=7235>.
    pub special_conditions: String,
}

impl Default for TickLast {
    /// A zero trade with empty exchange and conditions, timestamped at the Unix epoch.
    fn default() -> Self {
        Self {
            time: OffsetDateTime::UNIX_EPOCH,
            price: 0.0,
            size: 0.0,
            attributes: TickAttributeLast::default(),
            exchange: String::new(),
            special_conditions: String::new(),
        }
    }
}

/// Attributes of a [`TickLast`].
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TickAttributeLast {
    /// `true` if the trade occurred outside exchange limits.
    pub past_limit: bool,
    /// `true` if the trade is from an unreported trade source.
    pub unreported: bool,
}

/// A bid/ask tick: from `tick_by_tick` `bid_ask`, or `historical_ticks` `bid_ask`
/// (whatToShow = BID_ASK).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickBidAsk {
    /// Time of the tick.
    pub time: OffsetDateTime,
    /// Bid price.
    pub price_bid: f64,
    /// Ask price.
    pub price_ask: f64,
    /// Bid size. `0.0` if TWS sends none; every trade and bid/ask tick probed, live and
    /// historical, carried one (`docs/wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize`).
    pub size_bid: f64,
    /// Ask size. `0.0` if TWS sends none; every trade and bid/ask tick probed, live and
    /// historical, carried one (`docs/wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize`).
    pub size_ask: f64,
    /// Tick attributes.
    pub attributes: TickAttributeBidAsk,
}

impl Default for TickBidAsk {
    /// Zero prices and sizes, timestamped at the Unix epoch.
    fn default() -> Self {
        Self {
            time: OffsetDateTime::UNIX_EPOCH,
            price_bid: 0.0,
            price_ask: 0.0,
            size_bid: 0.0,
            size_ask: 0.0,
            attributes: TickAttributeBidAsk::default(),
        }
    }
}

/// Attributes of a [`TickBidAsk`].
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TickAttributeBidAsk {
    /// Indicates whether the bid is past the lower price band.
    pub bid_past_low: bool,
    /// Indicates whether the ask is past the upper price band.
    pub ask_past_high: bool,
}

/// A midpoint tick: from `tick_by_tick` `mid_point`, or `historical_ticks` `mid_point`
/// (whatToShow = MIDPOINT).
///
/// A midpoint has no size. The wire message has a size field, but live tick-by-tick midpoint
/// frames leave it out and historical midpoint ticks send `0`
/// (`docs/wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize`), so it is not decoded.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickMidpoint {
    /// Time of the tick.
    pub time: OffsetDateTime,
    /// Midpoint price.
    pub price: f64,
}

impl Default for TickMidpoint {
    /// A zero midpoint, timestamped at the Unix epoch.
    fn default() -> Self {
        Self {
            time: OffsetDateTime::UNIX_EPOCH,
            price: 0.0,
        }
    }
}

#[cfg(test)]
#[path = "ticks_tests.rs"]
mod tests;
