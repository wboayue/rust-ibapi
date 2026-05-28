use std::sync::Arc;

use super::*;
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
    assert!(builder.request_id > 0);
}

#[tokio::test]
async fn test_request_builder_with_id() {
    let client = create_test_client();
    let request_id = 42;
    let builder = RequestBuilder::with_id(&client, request_id);
    assert_eq!(builder.request_id(), request_id);
}

#[tokio::test]
async fn test_request_builder_check_version_success() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    let result = builder.check_version(100, "test_feature").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_request_builder_check_version_failure() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    let result = builder.check_version(999999, "future_feature").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_shared_request_builder_new() {
    let client = create_test_client();
    let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
    assert_eq!(builder.message_type, OutgoingMessages::RequestMarketData);
}

#[tokio::test]
async fn test_shared_request_builder_check_version() {
    let client = create_test_client();
    let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
    let result = builder.check_version(100, "test_feature").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_order_request_builder_new() {
    let client = create_test_client();
    let builder = OrderRequestBuilder::new(&client);
    assert!(builder.order_id > 0);
}

#[tokio::test]
async fn test_order_request_builder_with_id() {
    let client = create_test_client();
    let order_id = 12345;
    let builder = OrderRequestBuilder::with_id(&client, order_id);
    assert_eq!(builder.order_id(), order_id);
}

#[tokio::test]
async fn test_message_builder_new() {
    let client = create_test_client();
    let builder = MessageBuilder::new(&client);
    // MessageBuilder doesn't have public fields to test, just ensure it creates
    let _ = builder;
}

#[tokio::test]
async fn test_message_builder_check_version() {
    let client = create_test_client();
    let builder = MessageBuilder::new(&client);
    let result = builder.check_version(100, "test_feature").await;
    assert!(result.is_ok());
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
async fn test_subscription_builder_with_context() {
    let client = create_test_client();
    let context = client
        .decoder_context()
        .with_smart_depth(true)
        .with_request_type(OutgoingMessages::RequestMarketData);
    let message_bus = client.message_bus.clone();
    let builder: SubscriptionBuilder<Bar> =
        SubscriptionBuilder::new_with_components(client.decoder_context(), message_bus).with_context(context.clone());
    assert_eq!(builder.context, context);
}

#[tokio::test]
async fn test_subscription_builder_with_smart_depth() {
    let client = create_test_client();
    let context = client.decoder_context();
    let message_bus = client.message_bus.clone();
    let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new_with_components(context, message_bus).with_smart_depth(true);
    assert!(builder.context.is_smart_depth);
}

#[tokio::test]
async fn test_client_request_builders_trait() {
    let client = create_test_client();

    // Test request()
    let request_builder = client.request();
    assert!(request_builder.request_id > 0);

    // Test request_with_id()
    let request_builder = client.request_with_id(99);
    assert_eq!(request_builder.request_id(), 99);

    // Test shared_request()
    let shared_builder = client.shared_request(OutgoingMessages::RequestMarketData);
    assert_eq!(shared_builder.message_type, OutgoingMessages::RequestMarketData);

    // Test order_request()
    let order_builder = client.order_request();
    assert!(order_builder.order_id > 0);

    // Test order_request_with_id()
    let order_builder = client.order_request_with_id(999);
    assert_eq!(order_builder.order_id(), 999);

    // Test message()
    let _message_builder = client.message();
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
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        expected_id_min: i32,
    }

    let test_cases = vec![
        TestCase {
            name: "auto_request_id",
            request_id: None,
            order_id: None,
            message_type: None,
            expected_id_min: 1,
        },
        TestCase {
            name: "specific_request_id",
            request_id: Some(100),
            order_id: None,
            message_type: None,
            expected_id_min: 100,
        },
        TestCase {
            name: "specific_order_id",
            request_id: None,
            order_id: Some(500),
            message_type: None,
            expected_id_min: 500,
        },
        TestCase {
            name: "shared_request_type",
            request_id: None,
            order_id: None,
            message_type: Some(OutgoingMessages::RequestAccountData),
            expected_id_min: 0,
        },
    ];

    for tc in test_cases {
        let client = create_test_client();

        if let Some(request_id) = tc.request_id {
            let builder = client.request_with_id(request_id);
            assert_eq!(builder.request_id(), request_id, "test case '{}' failed", tc.name);
        } else if let Some(order_id) = tc.order_id {
            let builder = client.order_request_with_id(order_id);
            assert_eq!(builder.order_id(), order_id, "test case '{}' failed", tc.name);
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
async fn test_response_context_modifications() {
    struct TestCase {
        name: &'static str,
        initial_smart_depth: bool,
        initial_request_type: Option<OutgoingMessages>,
        set_smart_depth: Option<bool>,
        set_request_type: Option<OutgoingMessages>,
        expected_smart_depth: bool,
        expected_request_type: Option<OutgoingMessages>,
    }

    let test_cases = vec![
        TestCase {
            name: "default_context",
            initial_smart_depth: false,
            initial_request_type: None,
            set_smart_depth: None,
            set_request_type: None,
            expected_smart_depth: false,
            expected_request_type: None,
        },
        TestCase {
            name: "set_smart_depth_true",
            initial_smart_depth: false,
            initial_request_type: None,
            set_smart_depth: Some(true),
            set_request_type: None,
            expected_smart_depth: true,
            expected_request_type: None,
        },
        TestCase {
            name: "set_request_type",
            initial_smart_depth: false,
            initial_request_type: None,
            set_smart_depth: None,
            set_request_type: Some(OutgoingMessages::RequestMarketData),
            expected_smart_depth: false,
            expected_request_type: Some(OutgoingMessages::RequestMarketData),
        },
        TestCase {
            name: "set_both",
            initial_smart_depth: false,
            initial_request_type: None,
            set_smart_depth: Some(true),
            set_request_type: Some(OutgoingMessages::CancelMarketData),
            expected_smart_depth: true,
            expected_request_type: Some(OutgoingMessages::CancelMarketData),
        },
    ];

    for tc in test_cases {
        let client = create_test_client();
        let context = client.decoder_context();
        let message_bus = client.message_bus.clone();
        let mut builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new_with_components(context, message_bus);

        // Set initial context
        builder.context.is_smart_depth = tc.initial_smart_depth;
        builder.context.request_type = tc.initial_request_type;

        // Apply modifications
        if let Some(smart_depth) = tc.set_smart_depth {
            builder = builder.with_smart_depth(smart_depth);
        }

        if let Some(request_type) = tc.set_request_type {
            let context = DecoderContext::new(builder.context.server_version, builder.context.time_zone)
                .with_smart_depth(builder.context.is_smart_depth)
                .with_request_type(request_type);
            builder = builder.with_context(context);
        }

        // Verify expectations
        assert_eq!(
            builder.context.is_smart_depth, tc.expected_smart_depth,
            "test case '{}' failed: smart_depth mismatch",
            tc.name
        );
        assert_eq!(
            builder.context.request_type, tc.expected_request_type,
            "test case '{}' failed: request_type mismatch",
            tc.name
        );
    }
}

#[tokio::test]
async fn test_request_builder_send_raw() {
    let client = create_test_client();

    let builder = client.request_with_id(123);
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

#[tokio::test]
async fn test_order_request_builder_check_version() {
    let client = create_test_client();

    let builder = client.order_request_with_id(456);
    let result = builder.check_version(90, "test_feature").await;
    assert!(result.is_ok());

    let builder = client.order_request_with_id(457);
    let result = builder.check_version(999999, "future_feature").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_order_request_builder_send() {
    let client = create_test_client();

    let builder = client.order_request_with_id(789);
    let message = crate::messages::encode_protobuf_message(OutgoingMessages::PlaceOrder as i32, &[]);

    let result = builder.send(message).await;
    assert!(result.is_ok());
}
