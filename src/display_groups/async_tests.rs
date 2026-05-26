use super::*;
use crate::common::test_utils::helpers::{assert_proto_msg_id, proto_response};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;
use futures::StreamExt;
use std::sync::Arc;

fn display_group_update_response(contract_info: &str) -> crate::messages::ResponseMessage {
    let bytes = display_group_updated().contract_info(contract_info).encode_proto();
    proto_response(IncomingMessages::DisplayGroupUpdated, bytes)
}

#[tokio::test]
async fn test_subscribe_to_group_events() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response(
        "265598@SMART",
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let mut subscription = client.subscribe_to_group_events(1).await.expect("failed to subscribe");

    {
        let requests = message_bus.request_messages.read().unwrap();
        assert_eq!(requests.len(), 1);
        assert_proto_msg_id(&requests[0], OutgoingMessages::SubscribeToGroupEvents);
    }

    let Some(Ok(SubscriptionItem::Data(update))) = subscription.next().await else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, "265598@SMART");
}

#[tokio::test]
async fn test_subscribe_to_group_events_empty_group() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response("")]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let mut subscription = client.subscribe_to_group_events(2).await.expect("failed to subscribe");

    let Some(Ok(SubscriptionItem::Data(update))) = subscription.next().await else {
        panic!("expected Data");
    };
    assert_eq!(update.contract_info, "");
}

#[tokio::test]
async fn test_update_display_group() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![display_group_update_response(
        "265598@SMART",
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client.subscribe_to_group_events(1).await.expect("failed to subscribe");
    subscription.update("265598@SMART").await.expect("update failed");

    let requests = message_bus.request_messages.read().unwrap();
    assert_eq!(requests.len(), 2);
    assert_proto_msg_id(&requests[0], OutgoingMessages::SubscribeToGroupEvents);
    assert_proto_msg_id(&requests[1], OutgoingMessages::UpdateDisplayGroup);
}

#[tokio::test]
async fn test_subscribe_to_group_events_skips_wrong_message_type() {
    // Regression for rule 15: wrong-type frames must skip-classify, not terminate.
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
