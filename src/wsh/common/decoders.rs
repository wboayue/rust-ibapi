//! Decoders for Wall Street Horizon messages. Proto-only; text framing
//! surfaces as `Error::UnexpectedWireFormat` via `require_proto()`.

use prost::Message;

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::wsh::{WshEventData, WshMetadata};
use crate::Error;

pub(crate) fn decode_wsh_metadata(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    decode_wsh_metadata_proto(message.require_proto()?)
}

pub(crate) fn decode_wsh_event_data(message: &ResponseMessage) -> Result<WshEventData, Error> {
    decode_wsh_event_data_proto(message.require_proto()?)
}

/// Dispatch on incoming message type and forward to the typed decoder. Any
/// other variant becomes `Error::UnexpectedResponse`.
///
/// There is deliberately no `IncomingMessages::Error` arm. The dispatcher
/// classifies error frames before either caller sees them — `determine_routing`
/// returns `RoutingDecision::Error`, so an error reaches the subscription as
/// `RoutedItem::Error`/`Notice`, never as `RoutedItem::Response`. Both callers
/// consume only the `Response` side: the `StreamDecoder` impls match on it, and
/// the one-shot request path reaches this through `RoutedItem::into_legacy`,
/// which maps errors to `Some(Err(_))` and never runs the processor.
pub(in crate::wsh) fn decode_metadata_message(message: &ResponseMessage) -> Result<WshMetadata, Error> {
    decode_wsh_metadata(error_helpers::expect_message_type(message, IncomingMessages::WshMetaData)?)
}

pub(in crate::wsh) fn decode_event_data_message(message: &ResponseMessage) -> Result<WshEventData, Error> {
    decode_wsh_event_data(error_helpers::expect_message_type(message, IncomingMessages::WshEventData)?)
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
