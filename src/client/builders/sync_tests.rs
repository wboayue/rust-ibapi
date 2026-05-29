use std::sync::Arc;

use super::*;
use crate::market_data::realtime::Bar;
use crate::messages::OutgoingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::DecoderContext;

fn create_test_client() -> Client {
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::PROTOBUF_SCAN_DATA);
    client.set_next_order_id(9000);
    client
}

#[test]
fn test_request_builder_new() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    assert!(builder.request_id > 0);
}

#[test]
fn test_request_builder_with_id() {
    let client = create_test_client();
    let request_id = 42;
    let builder = RequestBuilder::with_id(&client, request_id);
    assert_eq!(builder.request_id(), request_id);
}

#[test]
fn test_request_builder_check_version_success() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    let result = builder.check_version(100, "test_feature");
    assert!(result.is_ok());
}

#[test]
fn test_request_builder_check_version_failure() {
    let client = create_test_client();
    let builder = RequestBuilder::new(&client);
    let result = builder.check_version(999999, "future_feature");
    assert!(result.is_err());
}

#[test]
fn test_shared_request_builder_new() {
    let client = create_test_client();
    let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
    assert_eq!(builder.message_type, OutgoingMessages::RequestMarketData);
}

#[test]
fn test_shared_request_builder_check_version() {
    let client = create_test_client();
    let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
    let result = builder.check_version(100, "test_feature");
    assert!(result.is_ok());
}

#[test]
fn test_order_request_builder_new() {
    let client = create_test_client();
    let builder = OrderRequestBuilder::new(&client);
    assert!(builder.order_id > 0);
}

#[test]
fn test_order_request_builder_with_id() {
    let client = create_test_client();
    let order_id = 12345;
    let builder = OrderRequestBuilder::with_id(&client, order_id);
    assert_eq!(builder.order_id(), order_id);
}

#[test]
fn test_message_builder_new() {
    let client = create_test_client();
    let builder = MessageBuilder::new(&client);
    // MessageBuilder doesn't have public fields to test, just ensure it creates
    let _ = builder;
}

#[test]
fn test_message_builder_check_version() {
    let client = create_test_client();
    let builder = MessageBuilder::new(&client);
    let result = builder.check_version(100, "test_feature");
    assert!(result.is_ok());
}

#[test]
fn test_subscription_builder_new() {
    let client = create_test_client();
    let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client);
    // Builder created successfully
    let _ = builder;
}

#[test]
fn test_subscription_builder_with_context() {
    let client = create_test_client();
    let context = client
        .decoder_context()
        .with_smart_depth(true)
        .with_request_type(OutgoingMessages::RequestMarketData);
    let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client).with_context(context.clone());
    assert_eq!(builder.context, context);
}

#[test]
fn test_subscription_builder_with_smart_depth() {
    let client = create_test_client();
    let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client).with_smart_depth(true);
    assert!(builder.context.is_smart_depth);
}

#[test]
fn test_client_request_builders_trait() {
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

#[test]
fn test_subscription_builder_ext_trait() {
    let client = create_test_client();
    let builder: SubscriptionBuilder<Bar> = client.subscription();
    // Builder created successfully through trait
    let _ = builder;
}

#[test]
fn test_builder_patterns_table_driven() {
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

#[test]
fn test_response_context_modifications() {
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
        let mut builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client);

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
