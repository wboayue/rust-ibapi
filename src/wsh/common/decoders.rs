//! Decoders for Wall Street Horizon messages.
//!
//! WSH metadata + event-data are proto-only at floor 213; TWS encodes them as
//! protobuf since `MIN_SERVER_VER_PROTOBUF_NEWS_DATA` (209). Text framing is
//! rejected via `require_proto()`, which yields `Error::UnexpectedResponse` —
//! the dispatcher skip-classifies that variant (per CLAUDE.md rule 20).

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
mod tests {
    use super::*;

    #[test]
    fn test_decode_wsh_metadata_proto() {
        let bytes = crate::proto::WshMetaData {
            req_id: Some(1),
            data_json: Some(r#"{"key":"value"}"#.into()),
        }
        .encode_to_vec();

        let result = decode_wsh_metadata_proto(&bytes).unwrap();
        assert_eq!(result.data_json, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_decode_wsh_event_data_proto() {
        let bytes = crate::proto::WshEventData {
            req_id: Some(1),
            data_json: Some(r#"{"event":"earnings"}"#.into()),
        }
        .encode_to_vec();

        let result = decode_wsh_event_data_proto(&bytes).unwrap();
        assert_eq!(result.data_json, r#"{"event":"earnings"}"#);
    }

    #[test]
    fn test_decode_wsh_metadata_rejects_text_framing() {
        // Text-framed arrival at a proto-only decoder must surface
        // UnexpectedResponse (rule 20).
        let message = ResponseMessage::from("104\09000\0{\"hi\":1}\0");
        match decode_wsh_metadata(&message) {
            Err(Error::UnexpectedResponse(_)) => {}
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }

    #[test]
    fn test_decode_wsh_event_data_rejects_text_framing() {
        let message = ResponseMessage::from("105\09000\0{\"event\":\"e\"}\0");
        match decode_wsh_event_data(&message) {
            Err(Error::UnexpectedResponse(_)) => {}
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }
}
