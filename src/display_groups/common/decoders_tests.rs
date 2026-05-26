use super::*;
use crate::common::test_utils::helpers::proto_response;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn test_decode_display_group_updated_proto() {
    let bytes = display_group_updated().request_id(1).contract_info("265598@SMART").encode_proto();

    let result = decode_display_group_updated_proto(&bytes).unwrap();
    assert_eq!(result.contract_info, "265598@SMART");
}

#[test]
fn test_decode_display_group_updated() {
    let bytes = display_group_updated().contract_info("265598@SMART").encode_proto();
    let message = proto_response(IncomingMessages::DisplayGroupUpdated, bytes);

    let result = decode_display_group_updated(&message).expect("decoding failed");
    assert_eq!(result.contract_info, "265598@SMART");
}

#[test]
fn test_decode_display_group_updated_empty_contract_info() {
    let bytes = display_group_updated().contract_info("").encode_proto();
    let message = proto_response(IncomingMessages::DisplayGroupUpdated, bytes);

    let result = decode_display_group_updated(&message).expect("decoding failed");
    assert_eq!(result.contract_info, "");
}

#[test]
fn test_decode_display_group_updated_wrong_message_type() {
    let bytes = display_group_updated().contract_info("265598@SMART").encode_proto();
    // Mis-frame it as DisplayGroupList (67) instead of DisplayGroupUpdated (68).
    let message = proto_response(IncomingMessages::DisplayGroupList, bytes);

    let result = decode_display_group_updated(&message);
    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "got {result:?}");
}

#[test]
fn test_decode_display_group_updated_rejects_text_framing() {
    let message = ResponseMessage::from("68\01\09000\0265598@SMART\0");
    let err = decode_display_group_updated(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}
