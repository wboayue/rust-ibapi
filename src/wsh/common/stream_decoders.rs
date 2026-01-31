//! Common StreamDecoder implementations for WSH module
//!
//! This module contains the StreamDecoder trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::wsh::*;
use crate::Error;

use super::decoders;

impl StreamDecoder<WshMetadata> for WshMetadata {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::WshMetaData];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::WshMetaData => decoders::decode_wsh_metadata(message.clone()),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel wsh metadata message")?;
        super::encoders::encode_cancel_wsh_metadata(request_id)
    }
}

impl StreamDecoder<WshEventData> for WshEventData {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::WshEventData];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_event_data_message(message.clone())
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel wsh event data message")?;
        super::encoders::encode_cancel_wsh_event_data(request_id)
    }
}
