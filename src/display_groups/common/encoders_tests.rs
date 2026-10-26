use super::*;
use crate::testdata::builders::display_groups::{
    query_display_groups_request, subscribe_to_group_events_request, unsubscribe_from_group_events_request, update_display_group_request,
};
use crate::testdata::builders::RequestEncoder;

#[test]
fn encode_query_display_groups_works() {
    let bytes = encode_query_display_groups(9000).unwrap();
    assert_eq!(bytes, query_display_groups_request().request_id(9000).encode_request());
}

#[test]
fn encode_subscribe_to_group_events_works() {
    let bytes = encode_subscribe_to_group_events(9000, 4).unwrap();
    assert_eq!(bytes, subscribe_to_group_events_request().request_id(9000).group_id(4).encode_request());
}

#[test]
fn encode_update_display_group_works() {
    let bytes = encode_update_display_group(9000, "8314@SMART").unwrap();
    assert_eq!(
        bytes,
        update_display_group_request()
            .request_id(9000)
            .contract_info("8314@SMART")
            .encode_request()
    );
}

#[test]
fn encode_unsubscribe_from_group_events_works() {
    let bytes = encode_unsubscribe_from_group_events(9000).unwrap();
    assert_eq!(bytes, unsubscribe_from_group_events_request().request_id(9000).encode_request());
}
