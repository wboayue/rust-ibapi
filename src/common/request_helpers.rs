//! Common request/response helper functions to reduce boilerplate across modules

use std::time::Duration;

use crate::common::error_helpers;
use crate::messages::{IncomingMessages, Notice, ResponseMessage};
use crate::proto::payload::ProtoPayload;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

/// How long a one-shot on a shared channel waits for its frame before failing
/// with [`Error::Timeout`]. A shared response carries no request id, so a
/// frame TWS never sends (or sent before the subscription existed) would
/// otherwise block the caller for good.
pub(crate) const ONE_SHOT_TIMEOUT: Duration = Duration::from_secs(30);

/// Fold a one-shot subscription's single response into a result.
///
/// Private on purpose. The only callers are the retrying helpers below, and
/// that is what makes "every one-shot retries" true — reachable from a domain
/// module, it is a ready-made way to hand-roll a one-shot without retry, which
/// is the bug #741 removed.
///
/// `Some(Err)` propagates the routed error — e.g. a request-less hard error
/// fanned out to one-shot shared channels — instead of masking it as a
/// default value (#694). A closed stream is `Error::UnexpectedEndOfStream`;
/// the ten sites that want an empty collection instead say so with
/// [`empty_on_end_of_stream`].
///
/// `processor` therefore never sees an `IncomingMessages::Error` frame. The
/// dispatcher classifies those into `RoutedItem::Error`/`Notice`, and
/// `RoutedItem::into_legacy` turns them into the `Some(Err)` arm above — so a
/// `decode_*_message` dispatcher that matches on `IncomingMessages::Error` is
/// writing an arm that cannot fire. See
/// `docs/rules/wire/proto-only-decoding.md`.
fn fold_one_shot<R>(
    response: Option<Result<ResponseMessage, Error>>,
    processor: impl FnOnce(&ResponseMessage) -> Result<R, Error>,
) -> Result<R, Error> {
    match response {
        Some(Ok(message)) => processor(&message),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

/// Read a closed stream as an empty collection rather than an error.
///
/// For requests where "TWS sent nothing" is a legitimate empty answer —
/// no histogram entries, no matching news providers — rather than a
/// truncated one. Chained onto the call so the choice reads where the
/// meaning is, not as a closure argument every other site copies:
///
/// ```ignore
/// one_shot_by_request_id(self, encoder, processor).or_else(empty_on_end_of_stream)
/// ```
///
/// This *was* the `on_none` parameter, passed identically by 44 of 54 sites
/// and as one of two spellings of the same empty vector (`Vec::new()`,
/// `Vec::default()`) by the other 10 — the tell that nobody was choosing.
pub(crate) fn empty_on_end_of_stream<R: Default>(error: Error) -> Result<R, Error> {
    match error {
        Error::UnexpectedEndOfStream => Ok(R::default()),
        other => Err(other),
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
/// The expected message type is not an argument: it is
/// [`ProtoPayload::MESSAGE_ID`] on the payload the decoder accepts, so a call
/// site cannot name one without the other and cannot name them inconsistently.
/// That replaced a hand-listed roster and a source-scraping test, which is what
/// checking the pairing costs when it is spelled at the call site — twice, since
/// sync and async each carry one.
///
/// What the type system still does not enforce is that a one-shot goes through
/// this at all: the helpers take a bare `impl Fn(&ResponseMessage)`, so a
/// processor that skips the narrow compiles. The narrow is unskippable only in
/// the sense that there is no longer a plausible-looking way to write it wrong.
///
/// Not for [`StreamDecoder`](crate::client::StreamDecoder) implementations —
/// but not because a narrow there is redundant. `decode` still owes its
/// `_ => Err(unexpected_response(..))` backstop, and a single-type decoder has
/// no match to hang one on, so `message.expect_type(..)?` *is* its backstop:
/// drop it and `response_message_ids_match_decode_arms` reads the decoder
/// as claiming an arm for every message type. What `expect_proto` would add
/// there is the payload decode, which `decode` already owns.
pub(crate) fn expect_proto<P, R>(decode: impl Fn(P) -> Result<R, Error>) -> impl Fn(&ResponseMessage) -> Result<R, Error>
where
    P: ProtoPayload,
{
    move |message| decode(message.expect_type(P::MESSAGE_ID)?.proto()?)
}

/// A one-shot request answered by a single frame on its own request id,
/// which TWS can cancel. Read with `one_shot_cancelling`, so a request
/// dropped before its answer (a dropped future, a timeout, a local error) is
/// cancelled at TWS instead of running on against pacing (#1107).
pub(crate) trait CancellableOneShot: 'static {
    /// The answer's payload; its message type is the only frame read.
    type Proto: ProtoPayload;
    type Output: Send + 'static;

    fn decode(payload: Self::Proto) -> Result<Self::Output, Error>;

    fn cancel(request_id: i32) -> Result<Vec<u8>, Error>;
}

/// The answer to a [`CancellableOneShot`], as a subscription reads it.
///
/// The answer ends the request (live: exactly one frame, then nothing), so it
/// is a snapshot end: a subscription that got it sends no cancel. Nor does one
/// a TWS error ended, since a one-shot has nothing left to cancel.
///
/// Not in the `response_message_ids_tests` roster: `RESPONSE_MESSAGE_IDS` and
/// the one `decode` arm both come from `K::Proto`, so they cannot disagree.
pub(crate) struct OneShot<K: CancellableOneShot>(K::Output);

impl<K: CancellableOneShot> StreamDecoder<OneShot<K>> for OneShot<K> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[K::Proto::MESSAGE_ID];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<Self, Error> {
        let payload = message.expect_type(K::Proto::MESSAGE_ID)?.proto()?;
        K::decode(payload).map(Self)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        K::cancel(error_helpers::require_request_id_for(request_id, "cancel one-shot request")?)
    }

    fn is_snapshot_end(&self) -> bool {
        true
    }

    fn rejection_ends_request(_notice: &Notice) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "request_helpers_tests.rs"]
mod tests;

// Sync implementations
#[cfg(feature = "sync")]
mod sync_impl {
    use std::time::{Duration, Instant};

    use super::{CancellableOneShot, OneShot};
    use crate::client::blocking::ClientRequestBuilders;
    use crate::client::sync::Client;
    use crate::messages::{OutgoingMessages, ResponseMessage};
    use crate::subscriptions::common::filter_notice;
    use crate::subscriptions::sync::Subscription;
    use crate::subscriptions::StreamDecoder;
    use crate::Error;

    /// One-shot request answered on the shared channel for its message type,
    /// within [`ONE_SHOT_TIMEOUT`](super::ONE_SHOT_TIMEOUT).
    pub fn one_shot_shared<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_within(client, message_type, super::ONE_SHOT_TIMEOUT, encoder, processor)
    }

    /// [`one_shot_shared`] with the wait for the frame bounded by `timeout`.
    pub fn one_shot_shared_within<R>(
        client: &Client,
        message_type: OutgoingMessages,
        timeout: Duration,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_matching_within(client, message_type, timeout, encoder, |message| processor(message).map(Some))
    }

    /// [`one_shot_shared`] for a response another caller's request of the same
    /// type can also produce: `processor` answers `Ok(None)` for a frame that is
    /// not this request's, and the next frame is read, until
    /// [`ONE_SHOT_TIMEOUT`](super::ONE_SHOT_TIMEOUT) passes.
    pub fn one_shot_shared_matching<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<Option<R>, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_matching_within(client, message_type, super::ONE_SHOT_TIMEOUT, encoder, processor)
    }

    /// [`one_shot_shared_matching`] with the wait for the frame bounded by `timeout`.
    pub fn one_shot_shared_matching_within<R>(
        client: &Client,
        message_type: OutgoingMessages,
        timeout: Duration,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<Option<R>, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::blocking::retry_on_connection_reset(client, || {
            let request = encoder()?;
            let subscription = client.shared_request(message_type).send_raw(request)?;
            let deadline = Instant::now() + timeout;

            loop {
                // Checked before each read: a queued frame is returned even
                // past the deadline, so a run of foreign frames would overrun it.
                if Instant::now() >= deadline {
                    return Err(Error::Timeout);
                }
                if let Some(result) = super::fold_one_shot(subscription.next_before(deadline)?, &processor)? {
                    return Ok(result);
                }
            }
        })
    }

    /// One-shot request answered on its own request-id channel.
    pub fn one_shot_by_request_id<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::blocking::retry_on_connection_reset(client, || {
            let request_id = client.mint_request_id();
            let request = encoder(request_id.raw())?;
            let subscription = client.send_request(request_id, request)?;

            super::fold_one_shot(subscription.next(), &processor)
        })
    }

    /// [`one_shot_by_request_id`] for a request TWS can cancel: dropped before
    /// its answer, the request is cancelled at TWS.
    pub fn one_shot_cancelling<K: CancellableOneShot>(client: &Client, encoder: impl Fn(i32) -> Result<Vec<u8>, Error>) -> Result<K::Output, Error> {
        crate::common::retry::blocking::retry_on_connection_reset(client, || {
            let builder = client.request();
            let request = encoder(builder.request_id())?;
            let subscription = builder.send::<OneShot<K>>(request)?;

            next_frame(&subscription).map(|answer| answer.0)
        })
    }

    /// The next frame of a request-id subscription, skipping notices. A close
    /// before it is `Error::UnexpectedEndOfStream`.
    pub fn next_frame<T: StreamDecoder<T>>(subscription: &Subscription<T>) -> Result<T, Error> {
        loop {
            let item = subscription.next().ok_or(Error::UnexpectedEndOfStream)?;
            if let Some(frame) = filter_notice(item) {
                return frame;
            }
        }
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_impl {
    use std::time::Duration;

    use futures::StreamExt;
    use tokio::time::Instant;

    use super::{CancellableOneShot, OneShot};
    use crate::client::{Client, ClientRequestBuilders};
    use crate::messages::{OutgoingMessages, ResponseMessage};
    use crate::subscriptions::common::{filter_notice, lag_as_error};
    use crate::subscriptions::r#async::Subscription;
    use crate::subscriptions::StreamDecoder;
    use crate::Error;

    /// One-shot request answered on the shared channel for its message type,
    /// within [`ONE_SHOT_TIMEOUT`](super::ONE_SHOT_TIMEOUT).
    pub async fn one_shot_shared<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_within(client, message_type, super::ONE_SHOT_TIMEOUT, encoder, processor).await
    }

    /// [`one_shot_shared`] with the wait for the frame bounded by `timeout`.
    pub async fn one_shot_shared_within<R>(
        client: &Client,
        message_type: OutgoingMessages,
        timeout: Duration,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_matching_within(client, message_type, timeout, encoder, |message| processor(message).map(Some)).await
    }

    /// [`one_shot_shared`] for a response another caller's request of the same
    /// type can also produce: `processor` answers `Ok(None)` for a frame that is
    /// not this request's, and the next frame is read, until
    /// [`ONE_SHOT_TIMEOUT`](super::ONE_SHOT_TIMEOUT) passes.
    pub async fn one_shot_shared_matching<R>(
        client: &Client,
        message_type: OutgoingMessages,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<Option<R>, Error>,
    ) -> Result<R, Error> {
        one_shot_shared_matching_within(client, message_type, super::ONE_SHOT_TIMEOUT, encoder, processor).await
    }

    /// [`one_shot_shared_matching`] with the wait for the frame bounded by `timeout`.
    pub async fn one_shot_shared_matching_within<R>(
        client: &Client,
        message_type: OutgoingMessages,
        timeout: Duration,
        encoder: impl Fn() -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<Option<R>, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(client, || async {
            let request = encoder()?;
            let mut subscription = client.shared_request(message_type).send_raw(request).await?;
            let deadline = Instant::now() + timeout;

            loop {
                // Checked before each read: `timeout_at` polls the read first,
                // so a run of queued foreign frames would overrun the deadline.
                if Instant::now() >= deadline {
                    return Err(Error::Timeout);
                }
                if let Some(result) = super::fold_one_shot(subscription.next_before(deadline).await?, &processor)? {
                    return Ok(result);
                }
            }
        })
        .await
    }

    /// One-shot request answered on its own request-id channel.
    pub async fn one_shot_by_request_id<R>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<Vec<u8>, Error>,
        processor: impl Fn(&ResponseMessage) -> Result<R, Error>,
    ) -> Result<R, Error> {
        crate::common::retry::retry_on_connection_reset(client, || async {
            let request_id = client.mint_request_id();
            let request = encoder(request_id.raw())?;
            let mut subscription = client.send_request(request_id, request).await?;

            super::fold_one_shot(subscription.next().await, &processor)
        })
        .await
    }

    /// [`one_shot_by_request_id`] for a request TWS can cancel: dropped before
    /// its answer, the request is cancelled at TWS.
    pub async fn one_shot_cancelling<K: CancellableOneShot>(
        client: &Client,
        encoder: impl Fn(i32) -> Result<Vec<u8>, Error>,
    ) -> Result<K::Output, Error> {
        crate::common::retry::retry_on_connection_reset(client, || async {
            let builder = client.request();
            let request = encoder(builder.request_id())?;
            let mut subscription = builder.send::<OneShot<K>>(request).await?;

            next_frame(&mut subscription).await.map(|answer| answer.0)
        })
        .await
    }

    /// The next frame of a request-id subscription, skipping notices. A close
    /// before it is `Error::UnexpectedEndOfStream`; a broadcast lag fails the
    /// read, since the frame it evicted can't be read.
    pub async fn next_frame<T: StreamDecoder<T> + Send + 'static>(subscription: &mut Subscription<T>) -> Result<T, Error> {
        loop {
            let item = subscription.next().await.ok_or(Error::UnexpectedEndOfStream)?;
            if let Some(frame) = filter_notice(lag_as_error(item)) {
                return frame;
            }
        }
    }
}

// Re-export based on feature flags
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync_impl::*;
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync_impl::*;

#[cfg(feature = "async")]
pub use async_impl::*;
