use std::sync::Arc;

use super::*;
use crate::client::ids::RequestId;
use crate::market_data::realtime::Bar;
use crate::messages::OutgoingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;

fn create_test_client() -> Client {
    Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::PROTOBUF)
}

#[tokio::test]
async fn test_request_builder_new() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    assert!(builder.request_id >= RequestId::nth(0));
}

#[tokio::test]
async fn test_request_builder_with_id() {
    let client = create_test_client();
    let request_id = RequestId::nth(42);
    let builder = RequestBuilder::with_id(&client, request_id);
    assert_eq!(builder.request_id(), request_id.raw());
}

#[tokio::test]
async fn test_shared_request_builder_new() {
    let client = create_test_client();
    let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
    assert_eq!(builder.message_type, OutgoingMessages::RequestMarketData);
}

#[tokio::test]
async fn test_subscription_builder_new() {
    let client = create_test_client();
    let context = client.decoder_context();
    let message_bus = client.message_bus.clone();
    let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new_with_components(context, message_bus);
    // Builder created successfully
    let _ = builder;
}

#[tokio::test]
async fn test_client_request_builders_trait() {
    let client = create_test_client();

    // Test request()
    let request_builder = client.request();
    assert!(request_builder.request_id >= RequestId::nth(0));

    // Test request_with_id()
    let request_builder = client.request_with_id(RequestId::nth(99));
    assert_eq!(request_builder.request_id(), RequestId::nth(99).raw());

    // Test shared_request()
    let shared_builder = client.shared_request(OutgoingMessages::RequestMarketData);
    assert_eq!(shared_builder.message_type, OutgoingMessages::RequestMarketData);
}

#[tokio::test]
async fn test_subscription_builder_ext_trait() {
    let client = create_test_client();
    let builder: SubscriptionBuilder<Bar> = client.subscription();
    // Builder created successfully through trait
    let _ = builder;
}

#[tokio::test]
async fn test_builder_patterns_table_driven() {
    struct TestCase {
        name: &'static str,
        request_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        expected_id_min: i32,
    }

    let test_cases = vec![
        TestCase {
            name: "auto_request_id",
            request_id: None,
            message_type: None,
            expected_id_min: RequestId::nth(0).raw(),
        },
        TestCase {
            name: "specific_request_id",
            request_id: Some(100),
            message_type: None,
            expected_id_min: 100,
        },
        TestCase {
            name: "shared_request_type",
            request_id: None,
            message_type: Some(OutgoingMessages::RequestAccountData),
            expected_id_min: 0,
        },
    ];

    for tc in test_cases {
        let client = create_test_client();

        if let Some(request_id) = tc.request_id {
            let builder = client.request_with_id(RequestId::nth(request_id));
            assert_eq!(builder.request_id(), RequestId::nth(request_id).raw(), "test case '{}' failed", tc.name);
        } else if let Some(message_type) = tc.message_type {
            let builder = client.shared_request(message_type);
            assert_eq!(builder.message_type, message_type, "test case '{}' failed", tc.name);
        } else {
            let builder = client.request();
            assert!(builder.request_id() >= tc.expected_id_min, "test case '{}' failed", tc.name);
        }
    }
}

#[tokio::test]
async fn test_request_builder_send_raw() {
    let client = create_test_client();

    let builder = client.request_with_id(RequestId::nth(123));
    let message = crate::messages::encode_protobuf_message(OutgoingMessages::RequestCurrentTime as i32, &[]);

    let result = builder.send_raw(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shared_request_builder_send_raw() {
    let client = create_test_client();

    let builder = client.shared_request(OutgoingMessages::RequestManagedAccounts);
    let message = crate::messages::encode_protobuf_message(OutgoingMessages::RequestManagedAccounts as i32, &[]);

    let result = builder.send_raw(message).await;
    assert!(result.is_ok());
}
