//! Decoders for Wall Street Horizon messages. Proto-only; text framing
//! surfaces as `Error::UnexpectedWireFormat` via `require_proto()`.

use prost::Message;

use crate::messages::ResponseMessage;
use crate::wsh::{WshEventData, WshMetadata};
use crate::Error;

pub(crate) fn decode_wsh_metadata(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    decode_wsh_metadata_proto(Message::decode(message.require_proto()?)?)
}

pub(crate) fn decode_wsh_event_data(message: &ResponseMessage) -> Result<WshEventData, Error> {
    decode_wsh_event_data_proto(Message::decode(message.require_proto()?)?)
}

pub(crate) fn decode_wsh_metadata_proto(p: crate::proto::WshMetaData) -> Result<WshMetadata, Error> {
    Ok(WshMetadata {
        data_json: p.data_json.unwrap_or_default(),
    })
}

pub(crate) fn decode_wsh_event_data_proto(p: crate::proto::WshEventData) -> Result<WshEventData, Error> {
    Ok(WshEventData {
        data_json: p.data_json.unwrap_or_default(),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
