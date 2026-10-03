//! Asynchronous builder implementations

use crate::client::ids::RequestId;
use crate::transport::BufferBound;
use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use crate::client::r#async::Client;
use crate::errors::Error;
use crate::messages::OutgoingMessages;
use crate::subscriptions::{DecoderContext, StreamDecoder, Subscription};
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};

/// Builder for creating requests with IDs
pub(crate) struct RequestBuilder<'a> {
    client: &'a Client,
    request_id: RequestId,
}

impl<'a> RequestBuilder<'a> {
    /// Create a new request builder with an auto-generated request ID
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            request_id: client.mint_request_id(),
        }
    }

    /// Create a new request builder with a specific request ID
    pub fn with_id(client: &'a Client, request_id: RequestId) -> Self {
        Self { client, request_id }
    }

    /// Get the request ID
    pub fn request_id(&self) -> i32 {
        self.request_id.raw()
    }

    /// Send the request and create a subscription
    pub async fn send<T>(self, message: Vec<u8>) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let context = self.client.decoder_context();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(context, message_bus)
            .send_with_request_id(self.request_id, message)
            .await
    }

    /// [`send`](Self::send) with a cap on unread items.
    pub async fn send_bounded<T>(self, message: Vec<u8>, bound: BufferBound) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let context = self.client.decoder_context();
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(context, message_bus)
            .send_with_request_id_bounded(self.request_id, message, bound)
            .await
    }

    /// Send the request and create a subscription with context
    pub async fn send_with_context<T>(self, message: Vec<u8>, context: DecoderContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let message_bus = self.client.message_bus.clone();
        SubscriptionBuilder::<T>::new_with_components(context, message_bus)
            .send_with_request_id(self.request_id, message)
            .await
    }

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
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

    /// Send the request without creating a subscription
    pub async fn send_raw(self, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.client.send_shared_request(self.message_type, message).await
    }
}

/// Builder for creating subscriptions with consistent patterns
pub(crate) struct SubscriptionBuilder<T> {
    message_bus: Arc<dyn AsyncMessageBus>,
    context: DecoderContext,
    _phantom: PhantomData<T>,
}

impl<T> SubscriptionBuilder<T>
where
    T: Send + 'static,
{
    /// Creates a new subscription builder from components
    pub fn new_with_components(context: DecoderContext, message_bus: Arc<dyn AsyncMessageBus>) -> Self {
        Self {
            message_bus,
            context,
            _phantom: PhantomData,
        }
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub async fn send_with_request_id(self, request_id: RequestId, message: Vec<u8>) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        let subscription = self.message_bus.send_request(request_id, message).await?;

        Ok(Subscription::new_from_internal(
            subscription,
            self.message_bus.clone(),
            Some(request_id.raw()),
            None,
            self.context,
        ))
    }

    /// [`send_with_request_id`](Self::send_with_request_id) with a cap on
    /// unread items (`AsyncMessageBus::send_request_bounded`).
    pub async fn send_with_request_id_bounded(self, request_id: RequestId, message: Vec<u8>, bound: BufferBound) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        let subscription = self.message_bus.send_request_bounded(request_id, message, bound).await?;

        Ok(Subscription::new_from_internal(
            subscription,
            self.message_bus.clone(),
            Some(request_id.raw()),
            None,
            self.context,
        ))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub async fn send_shared(self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        let subscription = self.message_bus.send_shared_request(message_type, message).await?;

        Ok(Subscription::new_from_internal(
            subscription,
            self.message_bus.clone(),
            None,
            None,
            self.context,
        ))
    }
}

/// Extension trait to add builder methods to Client
#[async_trait]
pub(crate) trait ClientRequestBuilders {
    /// Create a request builder with an auto-generated request ID
    fn request(&self) -> RequestBuilder<'_>;

    /// Create a request builder with a specific request ID
    fn request_with_id(&self, request_id: RequestId) -> RequestBuilder<'_>;

    /// Create a shared request builder
    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder<'_>;
}

impl ClientRequestBuilders for Client {
    fn request(&self) -> RequestBuilder<'_> {
        RequestBuilder::new(self)
    }

    fn request_with_id(&self, request_id: RequestId) -> RequestBuilder<'_> {
        RequestBuilder::with_id(self, request_id)
    }

    fn shared_request(&self, message_type: OutgoingMessages) -> SharedRequestBuilder<'_> {
        SharedRequestBuilder::new(self, message_type)
    }
}

/// Extension trait to add subscription builder to Client
pub(crate) trait SubscriptionBuilderExt {
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
        let context = self.decoder_context();
        let message_bus = self.message_bus.clone();
        SubscriptionBuilder::new_with_components(context, message_bus)
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
