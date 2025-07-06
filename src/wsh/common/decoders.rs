//! Decoders for Wall Street Horizon messages

use crate::messages::ResponseMessage;
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