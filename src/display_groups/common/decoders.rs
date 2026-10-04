//! Decoders for display group messages.

use crate::messages::ResponseMessage;
use crate::Error;

use super::stream_decoders::DisplayGroupUpdate;

/// Decodes a DisplayGroupUpdated message.
pub(crate) fn decode_display_group_updated(message: &ResponseMessage) -> Result<DisplayGroupUpdate, Error> {
    decode_display_group_updated_proto(message.proto()?)
}

pub(crate) fn decode_display_group_updated_proto(p: crate::proto::DisplayGroupUpdated) -> Result<DisplayGroupUpdate, Error> {
    Ok(DisplayGroupUpdate::new(p.contract_info.unwrap_or_default()))
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
