//! Common request/response helper functions to reduce boilerplate across modules

// Sync implementations
#[cfg(feature = "sync")]
mod sync_helpers {
    use crate::client::{Client, ClientRequestBuilders, SharesChannel, StreamDecoder, Subscription, SubscriptionBuilderExt};
    use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
    use crate::protocol::{check_version, ProtocolFeature};
    use crate::Error;

    /// Helper for requests that need a request ID and return a subscription
    pub fn request_with_id<T>(
        client: &Client,
        feature: ProtocolFeature,
        encoder: impl FnOnce(i32) -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        check_version(client.server_version(), feature)?;
        let builder = client.request();
        let request = encoder(builder.request_id())?;
        builder.send(request)
    }

    /// Helper for shared requests (no request ID) that return a subscription
    pub fn shared_subscription<T>(
        client: &Client,
        feature: ProtocolFeature,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
        Subscription<T>: SharesChannel,
    {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        client.subscription::<T>().send_shared(message_type, request)
    }

    /// Helper for shared requests without version check
    pub fn shared_request<T>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T>,
    {
        let request = encoder()?;
        client.shared_request(message_type).send(request)
    }

    /// Helper for one-shot requests that process a single response
    pub fn one_shot_request<R>(
        client: &Client,
        feature: ProtocolFeature,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
        processor: impl FnOnce(&mut ResponseMessage) -> Result<R, Error>,
        default: impl FnOnce() -> R,
    ) -> Result<R, Error> {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        let subscription = client.shared_request(message_type).send_raw(request)?;

        if let Some(Ok(mut message)) = subscription.next() {
            processor(&mut message)
        } else {
            Ok(default())
        }
    }

    /// Helper for one-shot requests with retry logic
    pub fn one_shot_with_retry<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<RequestMessage, Error>,
        processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| {
            let request = encoder()?;
            let subscription = client.shared_request(message_type).send_raw(request)?;

            match subscription.next() {
                Some(Ok(mut message)) => processor(&mut message),
                Some(Err(e)) => Err(e),
                None => on_none(),
            }
        })
    }

    /// Helper for one-shot requests with request ID and retry logic
    pub fn one_shot_request_with_retry<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<RequestMessage, Error>,
        processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| {
            let request_id = client.next_request_id();
            let request = encoder(request_id)?;
            let subscription = client.send_request(request_id, request)?;

            match subscription.next() {
                Some(Ok(mut message)) => processor(&mut message),
                Some(Err(e)) => Err(e),
                None => on_none(),
            }
        })
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_helpers {
    use crate::client::{Client, ClientRequestBuilders, SubscriptionBuilderExt};
    use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
    use crate::protocol::{check_version, ProtocolFeature};
    use crate::subscriptions::{StreamDecoder, Subscription};
    use crate::Error;
    #[allow(unused_imports)] // Used in one_shot_request
    use futures::StreamExt;

    /// Async helper for requests that need a request ID and return a subscription
    pub async fn request_with_id<T>(
        client: &Client,
        feature: ProtocolFeature,
        encoder: impl FnOnce(i32) -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        check_version(client.server_version(), feature)?;
        let builder = client.request();
        let request = encoder(builder.request_id())?;
        builder.send::<T>(request).await
    }

    /// Async helper for shared requests (no request ID) that return a subscription
    pub async fn shared_subscription<T>(
        client: &Client,
        feature: ProtocolFeature,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        client.subscription::<T>().send_shared::<T>(message_type, request).await
    }

    /// Async helper for shared requests without version check
    pub async fn shared_request<T>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
    ) -> Result<Subscription<T>, Error>
    where
        T: StreamDecoder<T> + Send + 'static,
    {
        let request = encoder()?;
        client.shared_request(message_type).send::<T>(request).await
    }

    /// Async helper for one-shot requests that process a single response
    pub async fn one_shot_request<R>(
        client: &Client,
        feature: ProtocolFeature,
        message_type: OutgoingMessages,
        encoder: impl FnOnce() -> Result<RequestMessage, Error>,
        processor: impl FnOnce(&mut ResponseMessage) -> Result<R, Error>,
        default: impl FnOnce() -> R,
    ) -> Result<R, Error> {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        let mut subscription = client.shared_request(message_type).send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => processor(&mut message),
            Some(Err(e)) => Err(e),
            None => Ok(default()),
        }
    }

    /// Async helper for one-shot requests with retry logic
    pub async fn one_shot_with_retry<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<RequestMessage, Error>,
        processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| async {
            let request = encoder()?;
            let mut subscription = client.shared_request(message_type).send_raw(request).await?;

            match subscription.next().await {
                Some(Ok(mut message)) => processor(&mut message),
                Some(Err(e)) => Err(e),
                None => on_none(),
            }
        })
        .await
    }

    /// Async helper for one-shot requests with request ID and retry logic
    pub async fn one_shot_request_with_retry<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<RequestMessage, Error>,
        processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| async {
            let request_id = client.next_request_id();
            let request = encoder(request_id)?;
            let mut subscription = client.send_request(request_id, request).await?;

            match subscription.next().await {
                Some(Ok(mut message)) => processor(&mut message),
                Some(Err(e)) => Err(e),
                None => on_none(),
            }
        })
        .await
    }
}

// Re-export based on feature flags
#[cfg(feature = "sync")]
pub use sync_helpers::*;

#[cfg(feature = "async")]
pub use async_helpers::*;
