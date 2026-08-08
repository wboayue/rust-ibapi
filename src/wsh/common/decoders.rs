//! Decoders for Wall Street Horizon messages. Proto-only; text framing
//! surfaces as `Error::UnexpectedWireFormat` via `require_proto()`.

use prost::Message;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::wsh::{WshEventData, WshMetadata};
use crate::Error;

pub(crate) fn decode_wsh_metadata(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    decode_wsh_metadata_proto(message.require_proto()?)
}

pub(crate) fn decode_wsh_event_data(message: &ResponseMessage) -> Result<WshEventData, Error> {
    decode_wsh_event_data_proto(message.require_proto()?)
}

/// Dispatch a WSH frame to its typed decoder. Shared by the `StreamDecoder`
/// impls and the one-shot request path; narrowing rationale lives on
/// [`ResponseMessage::expect_type`].
pub(in crate::wsh) fn decode_metadata_message(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    decode_wsh_metadata(message.expect_type(IncomingMessages::WshMetaData)?)
}

pub(in crate::wsh) fn decode_event_data_message(message: &ResponseMessage) -> Result<WshEventData, Error> {
    decode_wsh_event_data(message.expect_type(IncomingMessages::WshEventData)?)
}

pub(crate) fn decode_wsh_metadata_proto(bytes: &[u8]) -> Result<WshMetadata, Error> {
    let p = crate::proto::WshMetaData::decode(bytes)?;
    Ok(WshMetadata {
        data_json: p.data_json.unwrap_or_default(),
    })
}

pub(crate) fn decode_wsh_event_data_proto(bytes: &[u8]) -> Result<WshEventData, Error> {
    let p = crate::proto::WshEventData::decode(bytes)?;
    Ok(WshEventData {
        data_json: p.data_json.unwrap_or_default(),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
