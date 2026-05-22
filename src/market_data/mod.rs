//! Market data types and functionality

/// Request builders shared by market data workflows.
pub mod builder;
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

impl SmartDepth {
    /// Returns true when smart depth is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, SmartDepth::Yes)
    }
}

/// Market data type for switching between real-time and frozen/delayed.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketDataType {
    /// Sentinel for values not recognized by this client (forward compatibility).
    /// Decode-only in spirit — encoding sends `0`, which TWS will reject.
    Unknown = 0,
    /// Live market data
    Realtime = 1,
    /// Frozen market data (for when market is closed)
    Frozen = 2,
    /// Delayed market data (usually 15-20 minutes)
    Delayed = 3,
    /// Delayed frozen market data
    DelayedFrozen = 4,
}

impl From<i32> for MarketDataType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Realtime,
            2 => Self::Frozen,
            3 => Self::Delayed,
            4 => Self::DelayedFrozen,
            _ => Self::Unknown,
        }
    }
}

pub(crate) mod encoders {
    use crate::Error;

    use super::MarketDataType;

    pub(crate) fn encode_request_market_data_type(market_data_type: MarketDataType) -> Result<Vec<u8>, Error> {
        use prost::Message;
        let request = crate::proto::MarketDataTypeRequest {
            market_data_type: Some(market_data_type as i32),
        };
        Ok(crate::messages::encode_protobuf_message(
            crate::messages::OutgoingMessages::RequestMarketDataType as i32,
            &request.encode_to_vec(),
        ))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
