use super::*;
use crate::common::test_utils::helpers::assert_proto_msg_id;

#[test]
fn test_encode_query_display_groups() {
    let bytes = encode_query_display_groups(9000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::QueryDisplayGroups);
}

#[test]
fn test_encode_subscribe_to_group_events() {
    let bytes = encode_subscribe_to_group_events(9000, 1).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::SubscribeToGroupEvents);
}

#[test]
fn test_encode_update_display_group() {
    let bytes = encode_update_display_group(9000, "265598@SMART").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::UpdateDisplayGroup);
}

#[test]
fn test_encode_unsubscribe_from_group_events() {
    let bytes = encode_unsubscribe_from_group_events(9000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::UnsubscribeFromGroupEvents);
}
