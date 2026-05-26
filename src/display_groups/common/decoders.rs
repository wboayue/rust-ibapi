//! Decoders for display group messages.

use prost::Message;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use super::stream_decoders::DisplayGroupUpdate;

/// Decodes a DisplayGroupUpdated message.
pub(crate) fn decode_display_group_updated(message: &ResponseMessage) -> Result<DisplayGroupUpdate, Error> {
    if message.message_type() != IncomingMessages::DisplayGroupUpdated {
        return Err(Error::unexpected_response(message));
    }
    decode_display_group_updated_proto(message.require_proto()?)
}

pub(crate) fn decode_display_group_updated_proto(bytes: &[u8]) -> Result<DisplayGroupUpdate, Error> {
    let p = crate::proto::DisplayGroupUpdated::decode(bytes)?;
    Ok(DisplayGroupUpdate::new(p.contract_info.unwrap_or_default()))
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
