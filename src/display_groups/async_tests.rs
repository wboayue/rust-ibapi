use super::*;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::common::test_utils::helpers::{assert_request, proto_response, request_message_count};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::display_groups::{
    display_group_list, display_group_updated, query_display_groups_request, subscribe_to_group_events_request, update_display_group_request,
};
use crate::testdata::builders::ResponseProtoEncoder;
use futures::StreamExt;
use std::sync::Arc;

fn display_group_update_response(contract_info: &str) -> crate::messages::ResponseMessage {
    let bytes = display_group_updated().contract_info(contract_info).encode_proto();
    proto_response(IncomingMessages::DisplayGroupUpdated, bytes)
}

#[tokio::test]
async fn display_groups() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::DisplayGroupList,
        display_group_list().request_id(TEST_REQ_ID_FIRST).groups("3|1|2").encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let groups = client.display_groups().await.expect("display_groups failed");
    assert_eq!(groups, vec![3, 1, 2]);
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &query_display_groups_request().request_id(TEST_REQ_ID_FIRST));
}

#[tokio::test]
async fn subscribe_to_group_events() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response(
        "265598@SMART",
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let mut subscription = client.subscribe_to_group_events(1).await.expect("failed to subscribe");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &subscribe_to_group_events_request().group_id(1));

    let Some(Ok(SubscriptionItem::Data(update))) = subscription.next().await else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, "265598@SMART");
}

#[tokio::test]
async fn subscribe_to_group_events_empty_group() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response("")]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let mut subscription = client.subscribe_to_group_events(2).await.expect("failed to subscribe");

    let Some(Ok(SubscriptionItem::Data(update))) = subscription.next().await else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, "");
}

#[tokio::test]
async fn update_display_group() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response(
        "265598@SMART",
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client.subscribe_to_group_events(1).await.expect("failed to subscribe");
    subscription.update("265598@SMART").await.expect("update failed");

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 0, &subscribe_to_group_events_request().group_id(1));
    assert_request(&message_bus, 1, &update_display_group_request().contract_info("265598@SMART"));
}

#[tokio::test]
async fn subscribe_to_group_events_skips_wrong_message_type() {
    // Regression for docs/rules/wire/proto-only-decoding.md: wrong-type frames
    // must skip-classify, not terminate.
    let wrong = proto_response(
        IncomingMessages::DisplayGroupList,
        display_group_updated().contract_info("wrong message").encode_proto(),
    );
    let correct = display_group_update_response("correct message");
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![wrong, correct]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let mut subscription = client.subscribe_to_group_events(1).await.expect("failed to subscribe");

    let Some(Ok(SubscriptionItem::Data(update))) = subscription.next().await else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, "correct message");
}
