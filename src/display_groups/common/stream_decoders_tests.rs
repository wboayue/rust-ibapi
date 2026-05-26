use super::*;
use crate::common::test_utils::helpers::proto_response;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;

fn test_context() -> DecoderContext {
    DecoderContext::new(176, None)
}

fn updated_proto_message(contract_info: &str) -> ResponseMessage {
    let bytes = display_group_updated().contract_info(contract_info).encode_proto();
    proto_response(IncomingMessages::DisplayGroupUpdated, bytes)
}

#[test]
fn test_decode_display_group_update() {
    let mut message = updated_proto_message("265598@SMART");

    let result = DisplayGroupUpdate::decode(&test_context(), &mut message).expect("decoding failed");

    assert_eq!(result.contract_info, "265598@SMART");
}

#[test]
fn test_decode_display_group_update_empty() {
    let mut message = updated_proto_message("");

    let result = DisplayGroupUpdate::decode(&test_context(), &mut message).expect("decoding failed");

    assert_eq!(result.contract_info, "");
}

#[test]
fn test_decode_wrong_message_type() {
    let bytes = display_group_updated().contract_info("data").encode_proto();
    let mut message = proto_response(IncomingMessages::DisplayGroupList, bytes);

    let result = DisplayGroupUpdate::decode(&test_context(), &mut message);

    assert!(result.is_err());
}

#[test]
fn test_decode_error_message_surfaces_tws_error() {
    // Error on the request_id channel surfaces as Error::Notice, not silently
    // skipped via UnexpectedResponse (#434).
    use crate::common::test_utils::helpers::{assert_tws_error_message, proto_error_response};
    let mut message = proto_error_response(9000, 10089, "Requested market data is not subscribed");
    let err = DisplayGroupUpdate::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}

#[test]
fn test_cancel_message() {
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::messages::OutgoingMessages;

    let message = DisplayGroupUpdate::cancel_message(176, Some(9000), None).expect("cancel message failed");
    assert_proto_msg_id(&message, OutgoingMessages::UnsubscribeFromGroupEvents);
}

#[test]
fn test_cancel_message_no_request_id() {
    let result = DisplayGroupUpdate::cancel_message(176, None, None);

    assert!(result.is_err());
}
