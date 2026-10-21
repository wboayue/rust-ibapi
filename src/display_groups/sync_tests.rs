use super::*;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::common::test_utils::helpers::{assert_request, proto_response, request_message_count};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::display_groups::{display_group_list, display_group_updated, query_display_groups_request};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;

fn display_group_update_response(contract_info: &str) -> crate::messages::ResponseMessage {
    let bytes = display_group_updated().contract_info(contract_info).encode_proto();
    proto_response(IncomingMessages::DisplayGroupUpdated, bytes)
}

fn stubbed_subscription(responses: Vec<crate::messages::ResponseMessage>) -> (Arc<MessageBusStub>, DisplayGroupSubscription) {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(responses));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
    let subscription = client.subscribe_to_group_events(1).expect("failed to subscribe");
    (message_bus, subscription)
}

fn assert_first_data_eq(item: Option<Result<SubscriptionItem<DisplayGroupUpdate>, Error>>, expected_contract_info: &str) {
    let Some(Ok(SubscriptionItem::Data(update))) = item else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, expected_contract_info);
}

#[test]
fn test_display_groups() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::DisplayGroupList,
        display_group_list().request_id(TEST_REQ_ID_FIRST).groups("3|1|2").encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let groups = client.display_groups().expect("display_groups failed");
    assert_eq!(groups, vec![3, 1, 2]);
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &query_display_groups_request().request_id(TEST_REQ_ID_FIRST));
}

#[test]
fn test_update_display_group() {
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::messages::OutgoingMessages;

    let (message_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    subscription.update("265598@SMART").expect("update failed");

    let requests = message_bus.request_messages.read().unwrap();
    assert_eq!(requests.len(), 2);
    assert_proto_msg_id(&requests[0], OutgoingMessages::SubscribeToGroupEvents);
    assert_proto_msg_id(&requests[1], OutgoingMessages::UpdateDisplayGroup);
}

#[test]
fn test_subscription_derefs_to_inner_for_next() {
    let (_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    assert_first_data_eq(subscription.next(), "265598@SMART");
}

#[test]
fn test_borrowed_into_iter_yields_subscription_items() {
    let (_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    assert_first_data_eq((&subscription).into_iter().next(), "265598@SMART");
}

#[test]
fn test_owned_into_iter_consumes_subscription() {
    let (_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    assert_first_data_eq(subscription.into_iter().next(), "265598@SMART");
}
