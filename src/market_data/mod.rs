//! Market data types and functionality

/// Historical market data models and client APIs.
pub mod historical;
/// Real-time streaming market data helpers.
pub mod realtime;

mod common;
mod ticks;
pub use ticks::{QuoteSide, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint};

use serde::{Deserialize, Serialize};

/// Specifies whether to include only regular trading hours or extended hours
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TradingHours {
    /// Regular Trading Hours only (RTH)
    #[default]
    Regular,
    /// Include extended hours (pre-market and after-hours)
    Extended,
}

impl TradingHours {
    /// Returns true if only regular trading hours should be used
    pub fn use_rth(&self) -> bool {
        matches!(self, TradingHours::Regular)
    }

    /// Creates TradingHours from a boolean use_rth value
    pub fn from_use_rth(use_rth: bool) -> Self {
        if use_rth {
            TradingHours::Regular
        } else {
            TradingHours::Extended
        }
    }
}

/// Whether a bid/ask tick request ignores size changes.
///
/// IBKR's bid/ask tick requests, tick-by-tick and historical, honor this flag;
/// the other tick types (`Last` / `AllLast` / `MidPoint`) ignore it on the wire.
/// Exposed only on the `.bid_ask(...)` terminals of
/// [`HistoricalTicksBuilder`](crate::market_data::historical::HistoricalTicksBuilder)
/// and [`TickByTickBuilder`](crate::market_data::realtime::TickByTickBuilder).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreSize {
    /// Updates that change only a size are not sent; each update still carries its sizes
    /// (`docs/wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize`).
    Yes,
    /// Every update is sent, including those that change only a size.
    No,
}

/// Whether a market-depth subscription aggregates rows across exchanges.
///
/// `Yes` requests smart depth — TWS aggregates the order book across all
/// reporting exchanges. `No` requests single-exchange depth (the default).
/// Used by
/// [`MarketDepthBuilder`](crate::market_data::realtime::MarketDepthBuilder).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SmartDepth {
    /// Aggregate the order book across exchanges.
    Yes,
    /// Single-exchange depth (default).
    #[default]
    No,
}

/// Market data type for switching between real-time and frozen/delayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketDataType {
    /// Live market data. Wire code `1`.
    Realtime,
    /// Frozen market data (for when market is closed). Wire code `2`.
    Frozen,
    /// Delayed market data (usually 15-20 minutes). Wire code `3`.
    Delayed,
    /// Delayed frozen market data. Wire code `4`.
    DelayedFrozen,
    /// Market data type code not modeled by this version of the API, carried as received.
    /// An absent type decodes as `Unknown(0)`. Sent as-is by `switch_market_data_type`.
    Unknown(i32),
}

impl_wire_code!(MarketDataType {
    Realtime = 1,
    Frozen = 2,
    Delayed = 3,
    DelayedFrozen = 4,
});

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
