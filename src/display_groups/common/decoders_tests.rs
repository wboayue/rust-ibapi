use super::*;
use crate::common::test_utils::helpers::assert_rejects_text_framing;
use crate::messages::IncomingMessages;
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
    assert_rejects_text_framing(
        IncomingMessages::DisplayGroupUpdated,
        "68\01\09000\0265598@SMART\0",
        decode_display_group_updated,
    );
}
