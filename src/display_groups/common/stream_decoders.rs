//! StreamDecoder implementations for display group subscriptions

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::{decoders, encoders};

/// Represents a display group update event from TWS.
///
/// When subscribed to a display group, this type is returned whenever the user
/// changes the contract displayed in that group within TWS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayGroupUpdate {
    /// Contract information string (e.g., "265598@SMART")
    pub contract_info: String,
}

impl DisplayGroupUpdate {
    /// Creates a new DisplayGroupUpdate
    pub fn new(contract_info: String) -> Self {
        Self { contract_info }
    }
}

impl StreamDecoder<DisplayGroupUpdate> for DisplayGroupUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::DisplayGroupUpdated];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::DisplayGroupUpdated => decoders::decode_display_group_updated(message),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "unsubscribe from group events")?;
        encoders::encode_unsubscribe_from_group_events(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(fields: &[&str]) -> ResponseMessage {
        let raw = fields.join("\0") + "\0";
        ResponseMessage::from(&raw)
    }

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    #[test]
    fn test_decode_display_group_update() {
        let mut message = make_response(&["68", "1", "9000", "265598@SMART"]);

        let result = DisplayGroupUpdate::decode(&test_context(), &mut message).expect("decoding failed");

        assert_eq!(result.contract_info, "265598@SMART");
    }

    #[test]
    fn test_decode_display_group_update_empty() {
        let mut message = make_response(&["68", "1", "9000"]);

        let result = DisplayGroupUpdate::decode(&test_context(), &mut message).expect("decoding failed");

        assert_eq!(result.contract_info, "");
    }

    #[test]
    fn test_decode_wrong_message_type() {
        let mut message = make_response(&["67", "1", "9000", "data"]);

        let result = DisplayGroupUpdate::decode(&test_context(), &mut message);

        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_message() {
        let message = DisplayGroupUpdate::cancel_message(176, Some(9000), None).expect("cancel message failed");

        assert_eq!(message[0], "70"); // UnsubscribeFromGroupEvents
        assert_eq!(message[1], "1"); // version
        assert_eq!(message[2], "9000"); // request_id
    }

    #[test]
    fn test_cancel_message_no_request_id() {
        let result = DisplayGroupUpdate::cancel_message(176, None, None);

        assert!(result.is_err());
    }
}
