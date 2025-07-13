//! Synchronous builder implementations

use std::marker::PhantomData;

use crate::client::sync::Client;
use crate::client::{ResponseContext, StreamDecoder};
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::subscriptions::Subscription;
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
    pub fn send<T>(self, message: RequestMessage) -> Result<Subscription<'a, T>, Error>
    where
        T: StreamDecoder<T> + 'static,
    {
        SubscriptionBuilder::new(self.client).send_with_request_id(self.request_id, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<'a, T>, Error>
    where
        T: StreamDecoder<T> + 'static,
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
    pub fn send<T>(self, message: RequestMessage) -> Result<Subscription<'a, T>, Error>
    where
        T: StreamDecoder<T> + 'static,
    {
        SubscriptionBuilder::new(self.client).send_shared(self.message_type, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<'a, T>, Error>
    where
        T: StreamDecoder<T> + 'static,
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
    context: ResponseContext,
    _phantom: PhantomData<T>,
}

#[allow(dead_code)]
impl<'a, T> SubscriptionBuilder<'a, T>
where
    T: StreamDecoder<T> + 'static,
{
    /// Creates a new subscription builder
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            context: ResponseContext::default(),
            _phantom: PhantomData,
        }
    }

    /// Sets the response context for special handling
    pub fn with_context(mut self, context: ResponseContext) -> Self {
        self.context = context;
        self
    }

    /// Sets smart depth flag in the context
    pub fn with_smart_depth(mut self, is_smart_depth: bool) -> Self {
        self.context.is_smart_depth = is_smart_depth;
        self
    }

    /// Builds a subscription from an internal subscription (already sent)
    pub fn build(self, subscription: InternalSubscription) -> Subscription<'a, T> {
        Subscription::new(self.client, subscription, Some(self.context))
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub fn send_with_request_id(self, request_id: i32, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_request(request_id, message)?;
        Ok(self.build(subscription))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub fn send_shared(self, message_type: OutgoingMessages, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_shared_request(message_type, message)?;
        Ok(self.build(subscription))
    }

    /// Sends an order request and builds the subscription
    pub fn send_order(self, order_id: i32, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_order(order_id, message)?;
        Ok(self.build(subscription))
    }
}

/// Extension trait to add builder methods to Client
#[allow(dead_code)]
pub trait ClientRequestBuilders {
    /// Create a request builder with an auto-generated request ID
    fn request(&self) -> RequestBuilder;

    /// Create a request builder with a specific request ID
    fn request_with_id(&self, request_id: i32) -> RequestBuilder;

    /// Create a shared request builder
    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder;

    /// Create an order request builder
    fn order_request(&self) -> OrderRequestBuilder;

    /// Create an order request builder with a specific order ID
    fn order_request_with_id(&self, order_id: i32) -> OrderRequestBuilder;

    /// Create a simple message builder
    fn message(&self) -> MessageBuilder;
}

#[allow(dead_code)]
impl ClientRequestBuilders for Client {
    fn request(&self) -> RequestBuilder {
        RequestBuilder::new(self)
    }

    fn request_with_id(&self, request_id: i32) -> RequestBuilder {
        RequestBuilder::with_id(self, request_id)
    }

    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder {
        SharedRequestBuilder::new(self, message_type)
    }

    fn order_request(&self) -> OrderRequestBuilder {
        OrderRequestBuilder::new(self)
    }

    fn order_request_with_id(&self, order_id: i32) -> OrderRequestBuilder {
        OrderRequestBuilder::with_id(self, order_id)
    }

    fn message(&self) -> MessageBuilder {
        MessageBuilder::new(self)
    }
}

/// Extension trait to add subscription builder to Client
pub trait SubscriptionBuilderExt {
    /// Creates a new subscription builder
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: StreamDecoder<T> + 'static;
}

impl SubscriptionBuilderExt for Client {
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: StreamDecoder<T> + 'static,
    {
        SubscriptionBuilder::new(self)
    }
}
