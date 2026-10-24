use super::*;
use crate::common::test_utils::helpers::assert_rejects_text_framing;
use crate::messages::IncomingMessages;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn decode_display_group_updated_proto_works() {
    let p = display_group_updated().contract_info("265598@SMART").to_proto();

    let result = decode_display_group_updated_proto(p).unwrap();
    assert_eq!(result.contract_info, "265598@SMART");
}

#[test]
fn decode_display_group_updated_proto_empty_contract_info() {
    // Wire may omit contract_info; decoder must yield an empty string, not error.
    let p = crate::proto::DisplayGroupUpdated {
        req_id: None,
        contract_info: None,
    };

    let result = decode_display_group_updated_proto(p).unwrap();
    assert_eq!(result.contract_info, "");
}

#[test]
fn decode_display_group_list_proto_works() {
    // (wire groups, expected ids); None expected is a malformed list.
    let cases: &[(Option<&str>, Option<&[i32]>)] = &[
        (Some("3|1|2"), Some(&[3, 1, 2])),
        (Some("1"), Some(&[1])),
        (Some("1||2"), Some(&[1, 2])),
        (Some(""), Some(&[])),
        (None, Some(&[])),
        (Some("1|x"), None),
        (Some("1|"), Some(&[1])),
    ];
    for (groups, expected) in cases {
        let p = crate::proto::DisplayGroupList {
            req_id: Some(1),
            groups: groups.map(str::to_string),
        };
        let result = decode_display_group_list_proto(p);
        match expected {
            Some(ids) => assert_eq!(result.unwrap(), *ids, "groups {groups:?}"),
            None => assert!(matches!(result, Err(Error::Parse(..))), "groups {groups:?}: {result:?}"),
        }
    }
}

#[test]
fn decode_display_group_updated_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::DisplayGroupUpdated,
        "68\01\09000\0265598@SMART\0",
        decode_display_group_updated,
    );
}
