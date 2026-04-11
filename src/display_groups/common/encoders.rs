//! Encoders for display group messages.

use crate::messages::{OutgoingMessages, RequestMessage};
use crate::Error;

const VERSION: i32 = 1;

/// Encodes a request to subscribe to display group events.
pub(crate) fn encode_subscribe_to_group_events(request_id: i32, group_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::SubscribeToGroupEvents);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&group_id);
    Ok(message)
}

/// Encodes a request to unsubscribe from display group events.
pub(crate) fn encode_unsubscribe_from_group_events(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::UnsubscribeFromGroupEvents);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    Ok(message)
}

/// Encodes a request to update the contract displayed in a display group.
///
/// # Arguments
/// * `request_id` - The request ID (should match the subscription request ID)
/// * `contract_info` - Contract to display, format: "contractID@exchange" (e.g., "265598@SMART"),
///   "none" for empty selection, or "combo" for combination contracts
pub(crate) fn encode_update_display_group(request_id: i32, contract_info: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::UpdateDisplayGroup);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&contract_info);
    Ok(message)
}

// === Protobuf Encoders ===

#[allow(dead_code)]
pub(crate) fn encode_query_display_groups_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::QueryDisplayGroupsRequest { req_id: Some(request_id) };
    Ok(encode_protobuf_message(
        OutgoingMessages::QueryDisplayGroups as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_subscribe_to_group_events_proto(request_id: i32, group_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::SubscribeToGroupEventsRequest {
        req_id: Some(request_id),
        group_id: Some(group_id),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::SubscribeToGroupEvents as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_update_display_group_proto(request_id: i32, contract_info: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::UpdateDisplayGroupRequest {
        req_id: Some(request_id),
        contract_info: Some(contract_info.to_string()),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::UpdateDisplayGroup as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_unsubscribe_from_group_events_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::UnsubscribeFromGroupEventsRequest { req_id: Some(request_id) };
    Ok(encode_protobuf_message(
        OutgoingMessages::UnsubscribeFromGroupEvents as i32,
        &request.encode_to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToField;

    #[test]
    fn test_encode_subscribe_to_group_events() {
        let request_id = 9000;
        let group_id = 1;

        let message = encode_subscribe_to_group_events(request_id, group_id).expect("encoding failed");

        assert_eq!(message[0], OutgoingMessages::SubscribeToGroupEvents.to_field());
        assert_eq!(message[1], "1"); // version
        assert_eq!(message[2], request_id.to_field());
        assert_eq!(message[3], group_id.to_field());
    }

    #[test]
    fn test_encode_unsubscribe_from_group_events() {
        let request_id = 9000;

        let message = encode_unsubscribe_from_group_events(request_id).expect("encoding failed");

        assert_eq!(message[0], OutgoingMessages::UnsubscribeFromGroupEvents.to_field());
        assert_eq!(message[1], "1"); // version
        assert_eq!(message[2], request_id.to_field());
    }

    #[test]
    fn test_encode_update_display_group() {
        let request_id = 9000;
        let contract_info = "265598@SMART";

        let message = encode_update_display_group(request_id, contract_info).expect("encoding failed");

        assert_eq!(message[0], OutgoingMessages::UpdateDisplayGroup.to_field());
        assert_eq!(message[1], "1"); // version
        assert_eq!(message[2], request_id.to_field());
        assert_eq!(message[3], contract_info);
    }

    #[test]
    fn test_encode_update_display_group_none() {
        let request_id = 9000;
        let contract_info = "none";

        let message = encode_update_display_group(request_id, contract_info).expect("encoding failed");

        assert_eq!(message[0], OutgoingMessages::UpdateDisplayGroup.to_field());
        assert_eq!(message[3], "none");
    }

    #[cfg(test)]
    mod proto_tests {
        use super::super::*;

        fn assert_msg_id(bytes: &[u8], expected: OutgoingMessages) {
            let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            assert_eq!(msg_id, expected as i32 + 200);
        }

        #[test]
        fn test_encode_query_display_groups_proto() {
            let bytes = encode_query_display_groups_proto(9000).unwrap();
            assert_msg_id(&bytes, OutgoingMessages::QueryDisplayGroups);
        }

        #[test]
        fn test_encode_subscribe_to_group_events_proto() {
            let bytes = encode_subscribe_to_group_events_proto(9000, 1).unwrap();
            assert_msg_id(&bytes, OutgoingMessages::SubscribeToGroupEvents);
        }

        #[test]
        fn test_encode_update_display_group_proto() {
            let bytes = encode_update_display_group_proto(9000, "265598@SMART").unwrap();
            assert_msg_id(&bytes, OutgoingMessages::UpdateDisplayGroup);
        }

        #[test]
        fn test_encode_unsubscribe_from_group_events_proto() {
            let bytes = encode_unsubscribe_from_group_events_proto(9000).unwrap();
            assert_msg_id(&bytes, OutgoingMessages::UnsubscribeFromGroupEvents);
        }
    }
}
