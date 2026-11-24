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

impl TickBidAsk {
    /// The bid, or `None` when the tick reports no bid (see [`QuoteSide::new`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::TickBidAsk;
    ///
    /// // A far out-of-the-money option with only an offer.
    /// let tick = TickBidAsk { price_bid: 0.0, size_bid: 0.0, price_ask: 0.01, size_ask: 8.0, ..Default::default() };
    /// assert_eq!(tick.bid(), None);
    /// assert_eq!(tick.ask().map(|ask| ask.price), Some(0.01));
    /// ```
    pub fn bid(&self) -> Option<QuoteSide> {
        QuoteSide::new(self.price_bid, self.size_bid)
    }

    /// The ask, or `None` when the tick reports no ask (see [`QuoteSide::new`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::TickBidAsk;
    ///
    /// let tick = TickBidAsk { price_bid: 1.0906, size_bid: 2e6, price_ask: 1.0907, size_ask: 1e6, ..Default::default() };
    /// assert_eq!(tick.ask().map(|ask| (ask.price, ask.size)), Some((1.0907, 1e6)));
    /// ```
    pub fn ask(&self) -> Option<QuoteSide> {
        QuoteSide::new(self.price_ask, self.size_ask)
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

/// One side of a quote: a price with the size available at it.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuoteSide {
    /// Price, as TWS sends it. Can be negative: a combo's bid or ask often is.
    pub price: f64,
    /// Size available at [`price`](Self::price). Positive in a side from [`new`](Self::new).
    pub size: f64,
}

impl QuoteSide {
    /// The side TWS reports as `price` and `size`, or `None` when it reports no quote.
    ///
    /// TWS has no quote on a side when its size is zero. It sends such a side with a placeholder
    /// price that varies by contract and source:
    ///
    /// - `-1` on market data for options and indices, streaming and snapshot.
    /// - `-1` times the price magnifier on a magnified contract (`-100` for a CBOT corn option).
    /// - `0` on market data for combos, and on historical bid/ask ticks.
    ///
    /// The placeholder always comes with size `0`, so the size decides. A negative price with a
    /// positive size is a real quote: a calendar spread's bid can be `-71`
    /// (`docs/wire-facts/market-data.md#no-quote-sides`).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::QuoteSide;
    ///
    /// assert_eq!(QuoteSide::new(-1.0, 0.0), None);
    /// assert_eq!(QuoteSide::new(-71.0, 1.0), Some(QuoteSide { price: -71.0, size: 1.0 }));
    /// ```
    pub fn new(price: f64, size: f64) -> Option<Self> {
        (size > 0.0).then_some(Self { price, size })
    }
}

#[cfg(test)]
#[path = "ticks_tests.rs"]
mod tests;
