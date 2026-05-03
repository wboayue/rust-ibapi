//! Common StreamDecoder implementations for WSH module
//!
//! This module contains the StreamDecoder trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::wsh::*;
use crate::Error;

use super::decoders;

impl StreamDecoder<WshMetadata> for WshMetadata {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::WshMetaData, IncomingMessages::Error];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::WshMetaData => decoders::decode_wsh_metadata(message.clone()),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel wsh metadata message")?;
        super::encoders::encode_cancel_wsh_metadata(request_id)
    }
}

impl StreamDecoder<WshEventData> for WshEventData {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::WshEventData, IncomingMessages::Error];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::WshEventData => decoders::decode_event_data_message(message.clone()),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel wsh event data message")?;
        super::encoders::encode_cancel_wsh_event_data(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_tws_error_message;

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    fn error_message() -> ResponseMessage {
        ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|")
    }

    #[test]
    fn test_wsh_metadata_decode_error_message() {
        // Error on the wsh request_id channel surfaces as Error::Message (#434).
        let mut message = error_message();
        let err = WshMetadata::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }

    #[test]
    fn test_wsh_event_data_decode_error_message() {
        let mut message = error_message();
        let err = WshEventData::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}
