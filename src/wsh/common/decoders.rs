//! Decoders for Wall Street Horizon messages

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

/// Helper function to decode event data messages with error handling
pub(in crate::wsh) fn decode_event_data_message(message: ResponseMessage) -> Result<WshEventData, Error> {
    match message.message_type() {
        IncomingMessages::WshEventData => decode_wsh_event_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::UnexpectedResponse(message)),
    }
}
