//! Decoders for Wall Street Horizon messages. Proto-only; text framing
//! surfaces as `Error::UnexpectedResponse` via `require_proto()`.

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

/// Dispatch on incoming message type and forward to the typed decoder. Routes
/// `Error` frames into `Error::Notice` and any other variant into
/// `Error::UnexpectedResponse`.
pub(in crate::wsh) fn decode_metadata_message(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    match message.message_type() {
        IncomingMessages::WshMetaData => decode_wsh_metadata(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

pub(in crate::wsh) fn decode_event_data_message(message: &ResponseMessage) -> Result<WshEventData, Error> {
    match message.message_type() {
        IncomingMessages::WshEventData => decode_wsh_event_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
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
