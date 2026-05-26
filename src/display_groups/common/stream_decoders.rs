//! StreamDecoder implementations for display group subscriptions

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::{decoders, encoders};

/// Represents a display group update event from TWS.
///
/// When subscribed to a display group, this type is returned whenever the user
/// changes the contract displayed in that group within TWS.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayGroupUpdate {
    /// Contract information string (e.g., "265598@SMART")
    pub contract_info: String,
}

impl DisplayGroupUpdate {
    /// Creates a new DisplayGroupUpdate
    pub fn new(contract_info: String) -> Self {
        Self { contract_info }
    }
}

impl StreamDecoder<DisplayGroupUpdate> for DisplayGroupUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::DisplayGroupUpdated, IncomingMessages::Error];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::DisplayGroupUpdated => decoders::decode_display_group_updated(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "unsubscribe from group events")?;
        encoders::encode_unsubscribe_from_group_events(request_id)
    }
}

#[cfg(test)]
#[path = "stream_decoders_tests.rs"]
mod tests;
