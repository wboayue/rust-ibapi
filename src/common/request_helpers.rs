//! Common request/response helper functions to reduce boilerplate across modules

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

/// Fold a one-shot subscription's single response into a result.
///
/// `Some(Err)` propagates the routed error — e.g. a request-less hard error
/// fanned out to one-shot shared channels — instead of masking it as a
/// default value (#694). `on_none` decides what a closed stream means for
/// the caller (a default value, or `Error::UnexpectedEndOfStream`).
///
/// `processor` therefore never sees an `IncomingMessages::Error` frame. The
/// dispatcher classifies those into `RoutedItem::Error`/`Notice`, and
/// `RoutedItem::into_legacy` turns them into the `Some(Err)` arm above — so a
/// `decode_*_message` dispatcher that matches on `IncomingMessages::Error` is
/// writing an arm that cannot fire. See
/// `docs/rules/wire/proto-only-decoding.md`.
pub(crate) fn fold_one_shot<R>(
    response: Option<Result<ResponseMessage, Error>>,
    processor: impl FnOnce(&ResponseMessage) -> Result<R, Error>,
    on_none: impl FnOnce() -> Result<R, Error>,
) -> Result<R, Error> {
    fold_one_shot_mut(response, |message| processor(message), on_none)
}

/// [`fold_one_shot`] for decoders that need `&mut ResponseMessage`.
///
/// The option-computation decoders take the message mutably (they advance a
/// text cursor through `DecoderContext`), which the shared `&`-taking
/// [`fold_one_shot`] cannot express. Same disposition of `Some(Err)` / `None`,
/// so the two stay one decision rather than two.
pub(crate) fn fold_one_shot_mut<R>(
    response: Option<Result<ResponseMessage, Error>>,
    processor: impl FnOnce(&mut ResponseMessage) -> Result<R, Error>,
    on_none: impl FnOnce() -> Result<R, Error>,
) -> Result<R, Error> {
    match response {
        Some(Ok(mut message)) => processor(&mut message),
        Some(Err(e)) => Err(e),
        None => on_none(),
    }
}

/// Pair the inbound message type a one-shot request expects with the decoder
/// for that message's protobuf payload.
///
/// Narrowing used to be opt-in: each domain hand-wrote a `decode_*_message`
/// wrapper doing `expect_type(..)?` before its real decoder, and the sites that
/// had no wrapper decoded whatever frame arrived on the shared channel.
/// Threading the expected type through the processor puts both halves at the
/// call site, where the request that asked for the frame is.
///
/// **The type system does not enforce this**, and saying otherwise would be the
/// more dangerous mistake. `expected` and `decode` are unrelated — `R` is
/// inferred from the decoder — so a mispaired call compiles and feeds one
/// message's bytes to another's prost type; and the one-shot helpers still take
/// a bare `impl Fn(&ResponseMessage)`, so a decoder that skips the narrow also
/// compiles. Both are caught by `test_expect_proto_sites_match_the_roster`
/// (`src/common/one_shot_pairing_tests.rs`), which is the gate, not the signature.
///
/// Not for [`StreamDecoder`](crate::client::StreamDecoder) implementations —
/// but not because a narrow there is redundant. `decode` still owes its
/// `_ => Err(unexpected_response(..))` backstop, and a single-type decoder has
/// no match to hang one on, so `message.expect_type(..)?` *is* its backstop:
/// drop it and `test_response_message_ids_match_decode_arms` reads the decoder
/// as claiming an arm for every message type. What `expect_proto` would add
/// there is the payload decode, which `decode` already owns.
pub(crate) fn expect_proto<R>(
    expected: IncomingMessages,
    decode: impl Fn(&[u8]) -> Result<R, Error>,
) -> impl Fn(&ResponseMessage) -> Result<R, Error> {
    move |message| decode(message.expect_type(expected)?.require_proto()?)
}

#[cfg(test)]
#[path = "request_helpers_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "one_shot_pairing_tests.rs"]
mod one_shot_pairing_tests;

// Sync implementations
#[cfg(feature = "sync")]
mod sync_helpers {
    use crate::client::blocking::{ClientRequestBuilders, SharesChannel, Subscription, SubscriptionBuilderExt};
    use crate::client::sync::Client;
    use crate::client::StreamDecoder;
    use crate::messages::{OutgoingMessages, ResponseMessage};
    use crate::protocol::{check_version, ProtocolFeature};
    use crate::Error;

    /// Helper for requests that need a request ID and return a subscription
    pub fn request_with_id<T>(
        client: &Client,
        feature: ProtocolFeature,
        encoder: impl FnOnce(i32) -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
        processor: impl FnOnce(&ResponseMessage) -> Result<R, Error>,
        on_none: impl FnOnce() -> Result<R, Error>,
    ) -> Result<R, Error> {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        let subscription = client.shared_request(message_type).send_raw(request)?;

        super::fold_one_shot(subscription.next(), processor, on_none)
    }

    /// Helper for one-shot requests with retry logic
    pub fn one_shot_with_retry<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::blocking::retry_on_connection_reset(|| {
            let request = encoder()?;
            let subscription = client.shared_request(message_type).send_raw(request)?;

            super::fold_one_shot(subscription.next(), &processor, &on_none)
        })
    }

    /// Helper for one-shot requests with request ID and retry logic
    pub fn one_shot_request_with_retry<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::blocking::retry_on_connection_reset(|| {
            let request_id = client.next_request_id();
            let request = encoder(request_id)?;
            let subscription = client.send_request(request_id, request)?;

            super::fold_one_shot(subscription.next(), &processor, &on_none)
        })
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_helpers {
    use crate::client::{Client, ClientRequestBuilders, SubscriptionBuilderExt};
    use crate::messages::{OutgoingMessages, ResponseMessage};
    use crate::protocol::{check_version, ProtocolFeature};
    use crate::subscriptions::{StreamDecoder, Subscription};
    use crate::Error;
    #[allow(unused_imports)] // Used in one_shot_request
    use futures::StreamExt;

    /// Async helper for requests that need a request ID and return a subscription
    pub async fn request_with_id<T>(
        client: &Client,
        feature: ProtocolFeature,
        encoder: impl FnOnce(i32) -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
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
        encoder: impl FnOnce() -> Result<Vec<u8>, Error>,
        processor: impl FnOnce(&ResponseMessage) -> Result<R, Error>,
        on_none: impl FnOnce() -> Result<R, Error>,
    ) -> Result<R, Error> {
        check_version(client.server_version(), feature)?;
        let request = encoder()?;
        let mut subscription = client.shared_request(message_type).send_raw(request).await?;

        super::fold_one_shot(subscription.next().await, processor, on_none)
    }

    /// Async helper for one-shot requests with retry logic
    pub async fn one_shot_with_retry<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| async {
            let request = encoder()?;
            let mut subscription = client.shared_request(message_type).send_raw(request).await?;

            super::fold_one_shot(subscription.next().await, &processor, &on_none)
        })
        .await
    }

    /// Async helper for one-shot requests with request ID and retry logic
    pub async fn one_shot_request_with_retry<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
        on_none: impl Fn() -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(|| async {
            let request_id = client.next_request_id();
            let request = encoder(request_id)?;
            let mut subscription = client.send_request(request_id, request).await?;

            super::fold_one_shot(subscription.next().await, &processor, &on_none)
        })
        .await
    }
}

// Re-export based on feature flags
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync_helpers::*;
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync_helpers::*;

#[cfg(feature = "async")]
pub use async_helpers::*;
