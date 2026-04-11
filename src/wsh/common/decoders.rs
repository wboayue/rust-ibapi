//! Decoders for Wall Street Horizon messages

use prost::Message;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::wsh::{WshEventData, WshMetadata};
use crate::Error;

pub(in crate::wsh) fn decode_wsh_metadata(mut message: ResponseMessage) -> Result<WshMetadata, Error> {
    message.skip(); // skip message type
    message.skip(); // skip request id

    Ok(WshMetadata {
        data_json: message.next_string()?,
    })
}

pub(in crate::wsh) fn decode_wsh_event_data(mut message: ResponseMessage) -> Result<WshEventData, Error> {
    message.skip(); // skip message type
    message.skip(); // skip request id

    Ok(WshEventData {
        data_json: message.next_string()?,
    })
}

#[allow(dead_code)]
pub(crate) fn decode_wsh_metadata_proto(bytes: &[u8]) -> Result<WshMetadata, Error> {
    let p = crate::proto::WshMetaData::decode(bytes)?;
    Ok(WshMetadata {
        data_json: p.data_json.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_wsh_event_data_proto(bytes: &[u8]) -> Result<WshEventData, Error> {
    let p = crate::proto::WshEventData::decode(bytes)?;
    Ok(WshEventData {
        data_json: p.data_json.unwrap_or_default(),
    })
}

/// Helper function to decode event data messages with error handling
pub(in crate::wsh) fn decode_event_data_message(message: ResponseMessage) -> Result<WshEventData, Error> {
    match message.message_type() {
        IncomingMessages::WshEventData => decode_wsh_event_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::UnexpectedResponse(message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn test_decode_wsh_metadata_proto() {
        let proto_msg = crate::proto::WshMetaData {
            req_id: Some(1),
            data_json: Some(r#"{"key":"value"}"#.into()),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_wsh_metadata_proto(&bytes).unwrap();
        assert_eq!(result.data_json, r#"{"key":"value"}"#);
    }
}
