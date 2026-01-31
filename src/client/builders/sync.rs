//! Synchronous builder implementations

use std::marker::PhantomData;
use std::sync::Arc;

use crate::client::sync::Client;
use crate::client::StreamDecoder;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::subscriptions::sync::Subscription;
use crate::subscriptions::DecoderContext;
use crate::transport::InternalSubscription;

/// Builder for creating requests with IDs
#[allow(dead_code)]
pub(crate) struct RequestBuilder<'a> {
    client: &'a Client,
    request_id: i32,
}

#[allow(dead_code)]
impl<'a> RequestBuilder<'a> {
    /// Create a new request builder with an auto-generated request ID
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            request_id: client.next_request_id(),
        }
    }

    /// Create a new request builder with a specific request ID
    pub fn with_id(client: &'a Client, request_id: i32) -> Self {
        Self { client, request_id }
    }

    /// Get the request ID
    pub fn request_id(&self) -> i32 {
        self.request_id
    }

    /// Check server version requirement
    pub fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the request and create a subscription
    pub fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client).send_with_request_id(self.request_id, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: DecoderContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client)
            .with_context(context)
            .send_with_request_id(self.request_id, message)
    }

    /// Send the request without creating a subscription
    pub fn send_raw(self, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.client.send_request(self.request_id, message)
    }
}

/// Builder for creating shared channel requests (without request IDs)
#[allow(dead_code)]
pub(crate) struct SharedRequestBuilder<'a> {
    client: &'a Client,
    message_type: OutgoingMessages,
}

#[allow(dead_code)]
impl<'a> SharedRequestBuilder<'a> {
    /// Create a new shared request builder
    pub fn new(client: &'a Client, message_type: OutgoingMessages) -> Self {
        Self { client, message_type }
    }

    /// Check server version requirement
    pub fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the request and create a subscription
    pub fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client).send_shared(self.message_type, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: DecoderContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client)
            .with_context(context)
            .send_shared(self.message_type, message)
    }

    /// Send the request without creating a subscription
    pub fn send_raw(self, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.client.send_shared_request(self.message_type, message)
    }
}

/// Builder for creating order requests
#[allow(dead_code)]
pub(crate) struct OrderRequestBuilder<'a> {
    client: &'a Client,
    order_id: i32,
}

#[allow(dead_code)]
impl<'a> OrderRequestBuilder<'a> {
    /// Create a new order request builder with an auto-generated order ID
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            order_id: client.next_order_id(),
        }
    }

    /// Create a new order request builder with a specific order ID
    pub fn with_id(client: &'a Client, order_id: i32) -> Self {
        Self { client, order_id }
    }

    /// Get the order ID
    pub fn order_id(&self) -> i32 {
        self.order_id
    }

    /// Check server version requirement
    pub fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the order request
    pub fn send(self, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.client.send_order(self.order_id, message)
    }
}

/// Builder for simple message sends (no response expected)
#[allow(dead_code)]
pub(crate) struct MessageBuilder<'a> {
    client: &'a Client,
}

#[allow(dead_code)]
impl<'a> MessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Check server version requirement
    pub fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the message
    pub fn send(self, message: RequestMessage) -> Result<(), Error> {
        self.client.send_message(message)
    }
}

/// Builder for creating subscriptions with consistent patterns
#[allow(dead_code)]
pub(crate) struct SubscriptionBuilder<'a, T> {
    client: &'a Client,
    context: DecoderContext,
    _phantom: PhantomData<T>,
}

#[allow(dead_code)]
impl<'a, T> SubscriptionBuilder<'a, T>
where
    T: StreamDecoder<T>,
{
    /// Creates a new subscription builder
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            context: client.decoder_context(),
            _phantom: PhantomData,
        }
    }

    /// Sets the response context for special handling
    pub fn with_context(mut self, context: DecoderContext) -> Self {
        self.context = context;
        self
    }

    /// Sets smart depth flag in the context
    pub fn with_smart_depth(mut self, is_smart_depth: bool) -> Self {
        self.context.is_smart_depth = is_smart_depth;
        self
    }

    /// Builds a subscription from an internal subscription (already sent)
    pub fn build(self, subscription: InternalSubscription) -> Subscription<T> {
        Subscription::new(Arc::clone(&self.client.message_bus), subscription, self.context)
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub fn send_with_request_id(self, request_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error> {
        let subscription = self.client.send_request(request_id, message)?;
        Ok(self.build(subscription))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub fn send_shared(self, message_type: OutgoingMessages, message: RequestMessage) -> Result<Subscription<T>, Error> {
        let subscription = self.client.send_shared_request(message_type, message)?;
        Ok(self.build(subscription))
    }

    /// Sends an order request and builds the subscription
    pub fn send_order(self, order_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error> {
        let subscription = self.client.send_order(order_id, message)?;
        Ok(self.build(subscription))
    }
}

/// Extension trait to add builder methods to Client
#[allow(dead_code)]
pub(crate) trait ClientRequestBuilders {
    /// Create a request builder with an auto-generated request ID
    fn request(&self) -> RequestBuilder<'_>;

    /// Create a request builder with a specific request ID
    fn request_with_id(&self, request_id: i32) -> RequestBuilder<'_>;

    /// Create a shared request builder
    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder<'_>;

    /// Create an order request builder
    fn order_request(&self) -> OrderRequestBuilder<'_>;

    /// Create an order request builder with a specific order ID
    fn order_request_with_id(&self, order_id: i32) -> OrderRequestBuilder<'_>;

    /// Create a simple message builder
    fn message(&self) -> MessageBuilder<'_>;
}

#[allow(dead_code)]
impl ClientRequestBuilders for Client {
    fn request(&self) -> RequestBuilder<'_> {
        RequestBuilder::new(self)
    }

    fn request_with_id(&self, request_id: i32) -> RequestBuilder<'_> {
        RequestBuilder::with_id(self, request_id)
    }

    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder<'_> {
        SharedRequestBuilder::new(self, message_type)
    }

    fn order_request(&self) -> OrderRequestBuilder<'_> {
        OrderRequestBuilder::new(self)
    }

    fn order_request_with_id(&self, order_id: i32) -> OrderRequestBuilder<'_> {
        OrderRequestBuilder::with_id(self, order_id)
    }

    fn message(&self) -> MessageBuilder<'_> {
        MessageBuilder::new(self)
    }
}

/// Extension trait to add subscription builder to Client
pub(crate) trait SubscriptionBuilderExt {
    /// Creates a new subscription builder
    fn subscription<T>(&self) -> SubscriptionBuilder<'_, T>
    where
        T: StreamDecoder<T>;
}

impl SubscriptionBuilderExt for Client {
    fn subscription<T>(&self) -> SubscriptionBuilder<'_, T>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::common::mocks::MockGateway;
    use crate::client::common::tests::setup_connect;
    use crate::market_data::realtime::Bar;
    use crate::messages::OutgoingMessages;
    use crate::subscriptions::DecoderContext;

    fn create_test_client() -> (Client, MockGateway) {
        let gateway = setup_connect();
        let address = gateway.address();
        let client = Client::connect(&address, 100).expect("Client connection should succeed");
        (client, gateway)
    }

    #[test]
    fn test_request_builder_new() {
        let (client, _gateway) = create_test_client();
        let builder = RequestBuilder::new(&client);
        assert!(builder.request_id > 0);
    }

    #[test]
    fn test_request_builder_with_id() {
        let (client, _gateway) = create_test_client();
        let request_id = 42;
        let builder = RequestBuilder::with_id(&client, request_id);
        assert_eq!(builder.request_id(), request_id);
    }

    #[test]
    fn test_request_builder_check_version_success() {
        let (client, _gateway) = create_test_client();
        let builder = RequestBuilder::new(&client);
        let result = builder.check_version(100, "test_feature");
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_builder_check_version_failure() {
        let (client, _gateway) = create_test_client();
        let builder = RequestBuilder::new(&client);
        let result = builder.check_version(999999, "future_feature");
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_request_builder_new() {
        let (client, _gateway) = create_test_client();
        let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
        assert_eq!(builder.message_type, OutgoingMessages::RequestMarketData);
    }

    #[test]
    fn test_shared_request_builder_check_version() {
        let (client, _gateway) = create_test_client();
        let builder = SharedRequestBuilder::new(&client, OutgoingMessages::RequestMarketData);
        let result = builder.check_version(100, "test_feature");
        assert!(result.is_ok());
    }

    #[test]
    fn test_order_request_builder_new() {
        let (client, _gateway) = create_test_client();
        let builder = OrderRequestBuilder::new(&client);
        assert!(builder.order_id > 0);
    }

    #[test]
    fn test_order_request_builder_with_id() {
        let (client, _gateway) = create_test_client();
        let order_id = 12345;
        let builder = OrderRequestBuilder::with_id(&client, order_id);
        assert_eq!(builder.order_id(), order_id);
    }

    #[test]
    fn test_message_builder_new() {
        let (client, _gateway) = create_test_client();
        let builder = MessageBuilder::new(&client);
        // MessageBuilder doesn't have public fields to test, just ensure it creates
        let _ = builder;
    }

    #[test]
    fn test_message_builder_check_version() {
        let (client, _gateway) = create_test_client();
        let builder = MessageBuilder::new(&client);
        let result = builder.check_version(100, "test_feature");
        assert!(result.is_ok());
    }

    #[test]
    fn test_subscription_builder_new() {
        let (client, _gateway) = create_test_client();
        let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client);
        // Builder created successfully
        let _ = builder;
    }

    #[test]
    fn test_subscription_builder_with_context() {
        let (client, _gateway) = create_test_client();
        let context = client
            .decoder_context()
            .with_smart_depth(true)
            .with_request_type(OutgoingMessages::RequestMarketData);
        let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client).with_context(context.clone());
        assert_eq!(builder.context, context);
    }

    #[test]
    fn test_subscription_builder_with_smart_depth() {
        let (client, _gateway) = create_test_client();
        let builder: SubscriptionBuilder<Bar> = SubscriptionBuilder::new(&client).with_smart_depth(true);
        assert!(builder.context.is_smart_depth);
    }

    #[test]
    fn test_client_request_builders_trait() {
        let (client, _gateway) = create_test_client();

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
        let (client, _gateway) = create_test_client();
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
            let (client, _gateway) = create_test_client();

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
            let (client, _gateway) = create_test_client();
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
}
