//! Asynchronous builder implementations

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use crate::client::r#async::Client;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::subscriptions::{ResponseContext, StreamDecoder, Subscription};
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};

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
    pub async fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the request and create a subscription
    pub async fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let server_version = self.client.server_version();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(server_version, message_bus)
            .send_with_request_id::<T>(self.request_id, message)
            .await
    }

    /// Send the request and create a subscription with context
    pub async fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let server_version = self.client.server_version();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(server_version, message_bus)
            .with_context(context)
            .send_with_request_id::<T>(self.request_id, message)
            .await
    }

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_request(self.request_id, message).await
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
    pub async fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the request and create a subscription
    pub async fn send<T>(self, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let server_version = self.client.server_version();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(server_version, message_bus)
            .send_shared::<T>(self.message_type, message)
            .await
    }

    /// Send the request and create a subscription with context
    pub async fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let server_version = self.client.server_version();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(server_version, message_bus)
            .with_context(context)
            .send_shared::<T>(self.message_type, message)
            .await
    }

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_shared_request(self.message_type, message).await
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
    pub async fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the order request
    pub async fn send(self, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_order(self.order_id, message).await
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
    pub async fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the message
    pub async fn send(self, message: RequestMessage) -> Result<(), Error> {
        self.client.send_message(message).await
    }
}

/// Builder for creating subscriptions with consistent patterns
#[allow(dead_code)]
pub(crate) struct SubscriptionBuilder<T> {
    server_version: i32,
    message_bus: Arc<dyn AsyncMessageBus>,
    context: ResponseContext,
    _phantom: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> SubscriptionBuilder<T>
where
    T: Send + 'static,
{
    /// Creates a new subscription builder from components
    pub fn new_with_components(server_version: i32, message_bus: Arc<dyn AsyncMessageBus>) -> Self {
        Self {
            server_version,
            message_bus,
            context: ResponseContext::default(),
            _phantom: PhantomData,
        }
    }

    /// Sets the response context
    pub fn with_context(mut self, context: ResponseContext) -> Self {
        self.context = context;
        self
    }

    /// Sets smart depth flag in the context
    pub fn with_smart_depth(mut self, is_smart_depth: bool) -> Self {
        self.context.is_smart_depth = is_smart_depth;
        self
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub async fn send_with_request_id<D>(self, request_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: StreamDecoder<T> + 'static,
    {
        // Use atomic subscribe + send
        let subscription = self.message_bus.send_request(request_id, message).await?;

        // Create subscription with decoder
        Ok(Subscription::new_from_internal::<D>(
            subscription,
            self.server_version,
            self.message_bus.clone(),
            Some(request_id),
            None,
            None,
            self.context,
        ))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub async fn send_shared<D>(self, message_type: OutgoingMessages, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: StreamDecoder<T> + 'static,
    {
        // Use atomic subscribe + send
        let subscription = self.message_bus.send_shared_request(message_type, message).await?;

        Ok(Subscription::new_from_internal::<D>(
            subscription,
            self.server_version,
            self.message_bus.clone(),
            None,
            None,
            Some(message_type),
            self.context,
        ))
    }

    /// Sends an order request and builds the subscription
    pub async fn send_order<D>(self, order_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: StreamDecoder<T> + 'static,
    {
        // Use atomic subscribe + send
        let subscription = self.message_bus.send_order_request(order_id, message).await?;

        Ok(Subscription::new_from_internal::<D>(
            subscription,
            self.server_version,
            self.message_bus.clone(),
            None,
            Some(order_id),
            None,
            self.context,
        ))
    }
}

/// Extension trait to add builder methods to Client
#[async_trait]
#[allow(dead_code)]
pub trait ClientRequestBuilders {
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
pub trait SubscriptionBuilderExt {
    /// Creates a new subscription builder
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: Send + 'static;
}

impl SubscriptionBuilderExt for Client {
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: Send + 'static,
    {
        let server_version = self.server_version();
        let message_bus = self.message_bus.clone();
        SubscriptionBuilder::new_with_components(server_version, message_bus)
    }
}
