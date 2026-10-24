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

/// Decodes a DisplayGroupList message into the visible group ids, most used first.
pub(crate) fn decode_display_group_list_proto(p: crate::proto::DisplayGroupList) -> Result<Vec<i32>, Error> {
    p.groups
        .unwrap_or_default()
        .split('|')
        .filter(|id| !id.is_empty())
        .map(|id| id.parse().map_err(|e| Error::parse_field(id, format!("invalid display group id: {e}"))))
        .collect()
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
