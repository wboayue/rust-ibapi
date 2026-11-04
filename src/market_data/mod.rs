//! Market data types and functionality

/// Historical market data models and client APIs.
pub mod historical;
/// Real-time streaming market data helpers.
pub mod realtime;

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

/// Whether a tick-by-tick subscription should drop tick size information.
///
/// IBKR's bid/ask tick-by-tick request honors this flag; the other tick types
/// (`Last` / `AllLast` / `MidPoint`) ignore it on the wire. Exposed only on
/// the `.bid_ask(...)` terminals of
/// [`HistoricalTicksBuilder`](crate::market_data::historical::HistoricalTicksBuilder)
/// and [`TickByTickBuilder`](crate::market_data::realtime::TickByTickBuilder).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreSize {
    /// Tick sizes are omitted from the response.
    Yes,
    /// Tick sizes are included in the response.
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

impl From<MarketDataType> for i32 {
    fn from(value: MarketDataType) -> i32 {
        match value {
            MarketDataType::Realtime => 1,
            MarketDataType::Frozen => 2,
            MarketDataType::Delayed => 3,
            MarketDataType::DelayedFrozen => 4,
            MarketDataType::Unknown(code) => code,
        }
    }
}

impl From<i32> for MarketDataType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Realtime,
            2 => Self::Frozen,
            3 => Self::Delayed,
            4 => Self::DelayedFrozen,
            code => Self::Unknown(code),
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
