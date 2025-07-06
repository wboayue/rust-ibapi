//! Request builder pattern for simplifying client method implementations
//!
//! This module provides a builder pattern to reduce boilerplate in client methods
//! that follow a common request/response pattern.

// TODO: Remove this when more client methods are refactored to use the builder pattern
#![allow(dead_code)]

// TODO: Implement async version
#![cfg(feature = "sync")]

use crate::client::subscription_builder::SubscriptionBuilder;
use crate::client::Client;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::subscriptions::{ResponseContext, Subscription};
use crate::transport::InternalSubscription;

/// Builder for creating requests with IDs
pub struct RequestBuilder<'a> {
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
    pub fn check_version(self, required_version: i32, feature: &str) -> Result<Self, Error> {
        self.client.check_server_version(required_version, feature)?;
        Ok(self)
    }

    /// Send the request and create a subscription
    pub fn send<T>(self, message: RequestMessage) -> Result<Subscription<'a, T>, Error>
    where
        T: crate::subscriptions::DataStream<T> + 'static,
    {
        SubscriptionBuilder::new(self.client).send_with_request_id(self.request_id, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<'a, T>, Error>
    where
        T: crate::subscriptions::DataStream<T> + 'static,
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
pub struct SharedRequestBuilder<'a> {
    client: &'a Client,
    message_type: OutgoingMessages,
}

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
        T: crate::subscriptions::DataStream<T> + 'static,
    {
        SubscriptionBuilder::new(self.client).send_shared(self.message_type, message)
    }

    /// Send the request and create a subscription with context
    pub fn send_with_context<T>(self, message: RequestMessage, context: ResponseContext) -> Result<Subscription<'a, T>, Error>
    where
        T: crate::subscriptions::DataStream<T> + 'static,
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
pub struct OrderRequestBuilder<'a> {
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
pub struct MessageBuilder<'a> {
    client: &'a Client,
}

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

/// Extension trait to add builder methods to Client
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
