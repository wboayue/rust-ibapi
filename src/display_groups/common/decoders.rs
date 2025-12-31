//! Decoders for display group messages.

use log::warn;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use super::stream_decoders::DisplayGroupUpdate;

/// Decodes a DisplayGroupUpdated message.
pub(crate) fn decode_display_group_updated(message: &mut ResponseMessage) -> Result<DisplayGroupUpdate, Error> {
    // Validate message type
    if message.message_type() != IncomingMessages::DisplayGroupUpdated {
        return Err(Error::Simple(format!("unexpected message type: {:?}", message.message_type())));
    }

    // DisplayGroupUpdated: message_type, version, request_id, contract_info
    let contract_info = if message.len() > 3 {
        message.peek_string(3)
    } else {
        warn!("DisplayGroupUpdated message has fewer fields than expected (len={})", message.len());
        String::new()
    };

    Ok(DisplayGroupUpdate::new(contract_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(fields: &[&str]) -> ResponseMessage {
        let raw = fields.join("\0") + "\0";
        ResponseMessage::from(&raw)
    }

    #[test]
    fn test_decode_display_group_updated() {
        // DisplayGroupUpdated (68), version 1, reqId 9000, contractInfo "265598@SMART"
        let mut message = make_response(&["68", "1", "9000", "265598@SMART"]);

        let result = decode_display_group_updated(&mut message).expect("decoding failed");

        assert_eq!(result.contract_info, "265598@SMART");
    }

    #[test]
    fn test_decode_display_group_updated_empty_group() {
        // Short message with no contract info
        let mut message = make_response(&["68", "1", "9000"]);

        let result = decode_display_group_updated(&mut message).expect("decoding failed");

        assert_eq!(result.contract_info, "");
    }

    #[test]
    fn test_decode_display_group_updated_wrong_message_type() {
        // DisplayGroupList (67) instead of DisplayGroupUpdated (68)
        let mut message = make_response(&["67", "1", "9000", "some data"]);

        let result = decode_display_group_updated(&mut message);

        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("unexpected message type"));
    }
}
