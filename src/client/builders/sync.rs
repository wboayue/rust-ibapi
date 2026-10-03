//! Synchronous builder implementations

use crate::client::ids::RequestId;
use crate::transport::BufferBound;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::client::sync::Client;
use crate::client::StreamDecoder;
use crate::errors::Error;
use crate::messages::OutgoingMessages;
use crate::subscriptions::sync::Subscription;
use crate::subscriptions::DecoderContext;
use crate::transport::InternalSubscription;

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
    pub fn send<T>(self, message: Vec<u8>) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client).send_with_request_id(self.request_id, message)
    }

    /// [`send`](Self::send) with a cap on unread items.
    pub fn send_bounded<T>(self, message: Vec<u8>, bound: BufferBound) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client).send_with_request_id_bounded(self.request_id, message, bound)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: Vec<u8>, context: DecoderContext) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        SubscriptionBuilder::new(self.client)
            .with_context(context)
            .send_with_request_id(self.request_id, message)
    }

    /// Send the request without creating a subscription
    pub fn send_raw(self, message: Vec<u8>) -> Result<InternalSubscription, Error> {
        self.client.send_request(self.request_id, message)
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
    pub fn send_raw(self, message: Vec<u8>) -> Result<InternalSubscription, Error> {
        self.client.send_shared_request(self.message_type, message)
    }
}

/// Builder for creating subscriptions with consistent patterns
pub(crate) struct SubscriptionBuilder<'a, T> {
    client: &'a Client,
    context: DecoderContext,
    _phantom: PhantomData<T>,
}

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

    /// Builds a subscription from an internal subscription (already sent)
    pub fn build(self, subscription: InternalSubscription) -> Subscription<T> {
        Subscription::new(Arc::clone(&self.client.message_bus), subscription, self.context)
    }

    /// Sends a request with a specific request ID and builds the subscription
    pub fn send_with_request_id(self, request_id: RequestId, message: Vec<u8>) -> Result<Subscription<T>, Error> {
        let subscription = self.client.send_request(request_id, message)?;
        Ok(self.build(subscription))
    }

    /// [`send_with_request_id`](Self::send_with_request_id) with a cap on
    /// unread items (`MessageBus::send_request_bounded`).
    pub fn send_with_request_id_bounded(self, request_id: RequestId, message: Vec<u8>, bound: BufferBound) -> Result<Subscription<T>, Error> {
        log::debug!("send_message({request_id:?}), buffer limit {}", bound.limit);
        let subscription = self.client.message_bus.send_request_bounded(request_id, &message, bound)?;
        Ok(self.build(subscription))
    }

    /// Sends a shared request (no ID) and builds the subscription
    pub fn send_shared(self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<Subscription<T>, Error> {
        let subscription = self.client.send_shared_request(message_type, message)?;
        Ok(self.build(subscription))
    }
}

/// Extension trait to add builder methods to Client
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
#[path = "sync_tests.rs"]
mod tests;
