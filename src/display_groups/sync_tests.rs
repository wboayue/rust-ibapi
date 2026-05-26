use super::*;
use crate::common::test_utils::helpers::{proto_response, TEST_REQ_ID_FIRST};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::display_groups::display_group_updated;
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;

fn display_group_update_response(contract_info: &str) -> crate::messages::ResponseMessage {
    let bytes = display_group_updated()
        .request_id(TEST_REQ_ID_FIRST)
        .contract_info(contract_info)
        .encode_proto();
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
fn test_update_display_group() {
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::messages::OutgoingMessages;

    let (message_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    subscription.update("265598@SMART").expect("update failed");

    let requests = message_bus.request_messages.read().unwrap();
    // First request is subscribe, second is update
    assert_eq!(requests.len(), 2);

    assert_proto_msg_id(&requests[0], OutgoingMessages::SubscribeToGroupEvents);
    assert_proto_msg_id(&requests[1], OutgoingMessages::UpdateDisplayGroup);
}

#[test]
fn test_subscription_derefs_to_inner_for_next() {
    let (_bus, subscription) = stubbed_subscription(vec![display_group_update_response("265598@SMART")]);
    // `.next()` is Subscription<T>::next reached via Deref::deref.
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
