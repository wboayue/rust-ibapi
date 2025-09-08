//! Market data types and functionality

pub mod builder;
pub mod historical;
pub mod realtime;

use crate::{server_versions, Error};

use serde::{Deserialize, Serialize};

/// Specifies whether to include only regular trading hours or extended hours
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

/// Market data type for switching between real-time and frozen/delayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketDataType {
    /// Live market data
    Realtime = 1,
    /// Frozen market data (for when market is closed)
    Frozen = 2,
    /// Delayed market data (usually 15-20 minutes)
    Delayed = 3,
    /// Delayed frozen market data
    DelayedFrozen = 4,
}

#[cfg(feature = "sync")]
pub(crate) fn switch_market_data_type(client: &crate::Client, market_data_type: MarketDataType) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

    let message = encoders::encode_request_market_data_type(market_data_type)?;
    let _ = client.send_shared_request(crate::messages::OutgoingMessages::RequestMarketDataType, message)?;

    Ok(())
}

#[cfg(feature = "async")]
pub(crate) async fn switch_market_data_type(client: &crate::client::r#async::Client, market_data_type: MarketDataType) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

    let message = encoders::encode_request_market_data_type(market_data_type)?;
    client.send_message(message).await?;

    Ok(())
}

mod encoders {
    use crate::messages::{OutgoingMessages, RequestMessage};
    use crate::Error;

    use super::MarketDataType;

    pub(super) fn encode_request_market_data_type(market_data_type: MarketDataType) -> Result<RequestMessage, Error> {
        const VERSION: i32 = 1;

        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestMarketDataType);
        message.push_field(&VERSION);
        message.push_field(&(market_data_type as i32));

        Ok(message)
    }
}
