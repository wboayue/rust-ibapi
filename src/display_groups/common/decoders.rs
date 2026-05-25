//! Decoders for display group messages.

use log::warn;
use prost::Message;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use super::stream_decoders::DisplayGroupUpdate;

/// Decodes a DisplayGroupUpdated message.
pub(crate) fn decode_display_group_updated(message: &mut ResponseMessage) -> Result<DisplayGroupUpdate, Error> {
    if message.message_type() != IncomingMessages::DisplayGroupUpdated {
        return Err(Error::unexpected_response(message));
    }
    message.decode_proto_or_text(decode_display_group_updated_proto, |msg| {
        // text layout: message_type, version, request_id, contract_info
        let contract_info = msg.peek_string(3).unwrap_or_else(|_| {
            warn!("DisplayGroupUpdated message has fewer fields than expected (len={})", msg.len());
            String::new()
        });
        Ok(DisplayGroupUpdate::new(contract_info))
    })
}

pub(crate) fn decode_display_group_updated_proto(bytes: &[u8]) -> Result<DisplayGroupUpdate, Error> {
    let p = crate::proto::DisplayGroupUpdated::decode(bytes)?;
    Ok(DisplayGroupUpdate::new(p.contract_info.unwrap_or_default()))
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

        assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "got {result:?}");
    }

    #[test]
    fn test_decode_display_group_updated_proto() {
        use prost::Message;

        let proto_msg = crate::proto::DisplayGroupUpdated {
            req_id: Some(1),
            contract_info: Some("265598@SMART".into()),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_display_group_updated_proto(&bytes).unwrap();
        assert_eq!(result.contract_info, "265598@SMART");
    }

    #[test]
    fn test_decode_display_group_updated_dispatches_proto() {
        use prost::Message;

        let proto_msg = crate::proto::DisplayGroupUpdated {
            req_id: Some(9000),
            contract_info: Some("265598@SMART".into()),
        };
        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let mut message = ResponseMessage::from_protobuf(IncomingMessages::DisplayGroupUpdated as i32, bytes, 213);

        let result = decode_display_group_updated(&mut message).expect("decoding failed");
        assert_eq!(result.contract_info, "265598@SMART");
    }
}
