//! Asynchronous builder implementations

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use crate::client::r#async::Client;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::subscriptions::Subscription;
use crate::transport::AsyncInternalSubscription;

/// Builder for creating requests with IDs
pub(crate) struct RequestBuilder<'a> {
    client: &'a Client,
    request_id: i32,
}

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
        T: crate::subscriptions::AsyncDataStream<T> + Send + 'static,
    {
        SubscriptionBuilder::<T>::new(self.client)
            .send_with_request_id::<T>(self.request_id, message)
            .await
    }

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_request(self.request_id, message).await
    }
}

/// Builder for creating shared channel requests (without request IDs)
pub(crate) struct SharedRequestBuilder<'a> {
    client: &'a Client,
    message_type: OutgoingMessages,
}

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
        T: crate::subscriptions::AsyncDataStream<T> + Send + 'static,
    {
        SubscriptionBuilder::<T>::new(self.client)
            .send_shared::<T>(self.message_type, message)
            .await
    }

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_shared_request(self.message_type, message).await
    }
}

/// Builder for creating order requests
pub(crate) struct OrderRequestBuilder<'a> {
    client: &'a Client,
    order_id: i32,
}

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
pub(crate) struct MessageBuilder<'a> {
    client: &'a Client,
}

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
pub(crate) struct SubscriptionBuilder<'a, T> {
    client: &'a Client,
    _phantom: PhantomData<T>,
}

impl<'a, T> SubscriptionBuilder<'a, T>
where
    T: Send + 'static,
{
    /// Creates a new subscription builder
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub async fn send_with_request_id<D>(self, request_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: crate::subscriptions::AsyncDataStream<T> + 'static,
    {
        // Send the request
        self.client.message_bus.send_request(message).await?;

        // Subscribe to the response channel
        let subscription = self.client.message_bus.subscribe(request_id).await;

        // Create subscription with decoder
        Ok(Subscription::new_from_internal::<D>(subscription, Arc::new(self.client.clone())))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub async fn send_shared<D>(self, message_type: OutgoingMessages, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: crate::subscriptions::AsyncDataStream<T> + 'static,
    {
        // Send the request
        self.client.message_bus.send_request(message).await?;

        // Subscribe to the shared channel
        let subscription = self.client.message_bus.subscribe_shared(message_type).await;

        Ok(Subscription::new_from_internal::<D>(subscription, Arc::new(self.client.clone())))
    }

    /// Sends an order request and builds the subscription
    pub async fn send_order<D>(self, order_id: i32, message: RequestMessage) -> Result<Subscription<T>, Error>
    where
        D: crate::subscriptions::AsyncDataStream<T> + 'static,
    {
        // Send the request
        self.client.message_bus.send_request(message).await?;

        // Subscribe to the order channel
        let subscription = self.client.message_bus.subscribe_order(order_id).await;

        Ok(Subscription::new_from_internal::<D>(subscription, Arc::new(self.client.clone())))
    }
}

/// Extension trait to add builder methods to Client
#[async_trait]
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
        T: Send + 'static;
}

impl SubscriptionBuilderExt for Client {
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: Send + 'static,
    {
        SubscriptionBuilder::new(self)
    }
}
