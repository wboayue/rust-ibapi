//! Subscription builder for unified subscription creation
//!
//! This module provides a builder pattern for creating subscriptions in a consistent way
//! across both sync and async implementations.

use std::marker::PhantomData;

use crate::client::{Client, DataStream, ResponseContext, Subscription};
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::transport::InternalSubscription;

/// Builder for creating subscriptions with consistent patterns
pub struct SubscriptionBuilder<'a, T> {
    client: &'a Client,
    context: ResponseContext,
    _phantom: PhantomData<T>,
}

impl<'a, T> SubscriptionBuilder<'a, T>
where
    T: DataStream<T> + 'static,
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
    #[cfg(feature = "sync")]
    pub fn build(self, subscription: InternalSubscription) -> Subscription<'a, T> {
        Subscription::new(self.client, subscription, self.context)
    }

    /// Sends a request with a specific request ID and builds the subscription
    #[cfg(feature = "sync")]
    pub fn send_with_request_id(self, request_id: i32, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_request(request_id, message)?;
        Ok(self.build(subscription))
    }

    /// Sends a shared request (no ID) and builds the subscription
    #[cfg(feature = "sync")]
    pub fn send_shared(self, message_type: OutgoingMessages, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_shared_request(message_type, message)?;
        Ok(self.build(subscription))
    }

    /// Sends an order request and builds the subscription
    #[cfg(feature = "sync")]
    pub fn send_order(self, order_id: i32, message: RequestMessage) -> Result<Subscription<'a, T>, Error> {
        let subscription = self.client.send_order(order_id, message)?;
        Ok(self.build(subscription))
    }
}

/// Extension trait to add subscription builder to Client
pub trait SubscriptionBuilderExt {
    /// Creates a new subscription builder
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: DataStream<T> + 'static;
}

impl SubscriptionBuilderExt for Client {
    fn subscription<T>(&self) -> SubscriptionBuilder<T>
    where
        T: DataStream<T> + 'static,
    {
        SubscriptionBuilder::new(self)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_subscription_builder_context() {
        // This is a compile-time test to ensure the builder pattern works
        // Actual runtime tests would require a mock client

        // Example of how the builder would be used:
        /*
        let client = Client::connect("127.0.0.1:4002", 100)?;

        // Simple subscription with request ID
        let sub = client
            .subscription::<MarketData>()
            .send_with_request_id(request_id, message)?;

        // Subscription with context
        let sub = client
            .subscription::<MarketDepth>()
            .with_smart_depth(true)
            .send_with_request_id(request_id, message)?;

        // Shared subscription
        let sub = client
            .subscription::<Position>()
            .send_shared(OutgoingMessages::ReqPositions, message)?;
        */
    }
}
