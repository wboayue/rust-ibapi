//! Helper functions to reduce boilerplate in request/response patterns

#[cfg(all(feature = "sync", not(feature = "async")))]
use crate::client::{Client, ClientRequestBuilders, SharesChannel, StreamDecoder, Subscription, SubscriptionBuilderExt};
#[cfg(all(feature = "sync", not(feature = "async")))]
use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
#[cfg(all(feature = "sync", not(feature = "async")))]
use crate::protocol::{check_version, ProtocolFeature};
#[cfg(all(feature = "sync", not(feature = "async")))]
use crate::Error;

/// Helper for requests that need a request ID and return a subscription
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(in crate::accounts) fn request_with_id<'a, T>(
    client: &'a Client,
    feature: ProtocolFeature,
    encoder: impl FnOnce(i32) -> Result<RequestMessage, Error>,
) -> Result<Subscription<'a, T>, Error>
where
    T: StreamDecoder<T>,
{
    check_version(client.server_version(), feature)?;
    let builder = client.request();
    let request = encoder(builder.request_id())?;
    builder.send(request)
}

/// Helper for shared requests (no request ID) that return a subscription
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(in crate::accounts) fn shared_subscription<'a, T>(
    client: &'a Client,
    feature: ProtocolFeature,
    message_type: OutgoingMessages,
    encoder: impl FnOnce() -> Result<RequestMessage, Error>,
) -> Result<Subscription<'a, T>, Error>
where
    T: StreamDecoder<T>,
    Subscription<'a, T>: SharesChannel,
{
    check_version(client.server_version(), feature)?;
    let request = encoder()?;
    client.subscription::<T>().send_shared(message_type, request)
}

/// Helper for shared requests without version check
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(in crate::accounts) fn shared_request<'a, T>(
    client: &'a Client,
    message_type: OutgoingMessages,
    encoder: impl FnOnce() -> Result<RequestMessage, Error>,
) -> Result<Subscription<'a, T>, Error>
where
    T: StreamDecoder<T>,
{
    let request = encoder()?;
    client.shared_request(message_type).send(request)
}

/// Helper for one-shot requests that process a single response
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(in crate::accounts) fn one_shot_request<R>(
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
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(in crate::accounts) fn one_shot_with_retry<R>(
    client: &Client,
    message_type: OutgoingMessages,
    encoder: impl Fn() -> Result<RequestMessage, Error>,
    processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
    on_none: impl Fn() -> Result<R, Error>,
) -> Result<R, Error> {
    super::retry::retry_on_connection_reset(|| {
        let request = encoder()?;
        let subscription = client.shared_request(message_type).send_raw(request)?;

        match subscription.next() {
            Some(Ok(mut message)) => processor(&mut message),
            Some(Err(e)) => Err(e),
            None => on_none(),
        }
    })
}

// Async versions
#[cfg(feature = "async")]
pub(in crate::accounts) mod async_helpers {
    use crate::client::{Client, ClientRequestBuilders, SubscriptionBuilderExt};
    use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
    use crate::protocol::{check_version, ProtocolFeature};
    use crate::subscriptions::{StreamDecoder, Subscription};
    use crate::Error;
    #[allow(unused_imports)] // Used in one_shot_request
    use futures::StreamExt;

    /// Async helper for requests that need a request ID and return a subscription
    pub(in crate::accounts) async fn request_with_id<T>(
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
    pub(in crate::accounts) async fn shared_subscription<T>(
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
    pub(in crate::accounts) async fn shared_request<T>(
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
    pub(in crate::accounts) async fn one_shot_request<R>(
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

        if let Some(mut message) = subscription.next().await {
            processor(&mut message)
        } else {
            Ok(default())
        }
    }

    /// Async helper for one-shot requests with retry logic
    pub(in crate::accounts) async fn one_shot_with_retry<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<RequestMessage, Error>,
        processor: impl Fn(&mut ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::accounts::common::retry::retry_on_connection_reset_async(|| async {
            let request = encoder()?;
            let mut subscription = client.shared_request(message_type).send_raw(request).await?;

            match subscription.next().await {
                Some(mut message) => processor(&mut message),
                None => on_none(),
            }
        })
        .await
    }
}
