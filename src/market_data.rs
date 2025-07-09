//! Market data functionality for real-time and historical data retrieval.
//!
//! This module provides APIs for accessing both real-time market data (ticks, bars)
//! and historical market data. It includes support for various data types,
//! subscription management, and market data type configuration.

use crate::{messages::OutgoingMessages, server_versions, Client, Error};

pub mod historical;
pub mod realtime;

/// By default, only real-time market data sending is enabled.
#[derive(Debug, Clone, Copy)]
pub enum MarketDataType {
    /// Disables frozen, delayed and delayed-frozen market data sending.
    Live = 1,
    /// Enables frozen market data sending.
    Frozen = 2,
    /// Enables delayed and disables delayed-frozen market data sending.
    Delayed = 3,
    /// Enables delayed and delayed-frozen market data.
    DelayedFrozen = 4,
}

#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) fn switch_market_data_type(client: &Client, market_data_type: MarketDataType) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

    let message = encoders::encode_request_market_data_type(market_data_type)?;
    let _ = client.send_shared_request(OutgoingMessages::RequestMarketDataType, message)?;

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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{market_data::MarketDataType, ToField};

        #[test]
        fn test_encode_request_market_data_type() {
            let market_data_types = vec![
                MarketDataType::Live,
                MarketDataType::Frozen,
                MarketDataType::Delayed,
                MarketDataType::DelayedFrozen,
            ];

            for market_data_type in market_data_types {
                let result = encode_request_market_data_type(market_data_type);
                assert!(result.is_ok());

                let message = result.unwrap();

                assert_eq!(message[0], OutgoingMessages::RequestMarketDataType.to_field());
                assert_eq!(message[1], "1"); // VERSION
                assert_eq!(message[2], (market_data_type as i32).to_string());
            }
        }
    }
}

#[cfg(all(test, feature = "sync", not(feature = "async")))]
mod tests {
    use std::sync::{Arc, RwLock};

    use crate::{market_data::MarketDataType, server_versions, stubs::MessageBusStub, Client};

    #[test]
    fn test_switch_market_data_type() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let market_data_type = MarketDataType::Delayed;
        client.switch_market_data_type(market_data_type).expect("switch market data type failed");

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "59|1|3|");
    }
}
