use super::*;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn test_decode_display_group_updated_proto() {
    let bytes = display_group_updated().contract_info("265598@SMART").encode_proto();

    let result = decode_display_group_updated_proto(&bytes).unwrap();
    assert_eq!(result.contract_info, "265598@SMART");
}

#[test]
fn test_decode_display_group_updated_proto_empty_contract_info() {
    // Wire may omit contract_info; decoder must yield an empty string, not error.
    let bytes = crate::proto::DisplayGroupUpdated {
        req_id: None,
        contract_info: None,
    }
    .encode_to_vec();

    let result = decode_display_group_updated_proto(&bytes).unwrap();
    assert_eq!(result.contract_info, "");
}

#[test]
fn test_decode_display_group_updated_rejects_text_framing() {
    let message = ResponseMessage::from("68\01\09000\0265598@SMART\0");
    let err = decode_display_group_updated(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}
