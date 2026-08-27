//! Common utilities for subscription processing

use serde::{Deserialize, Serialize};
use time_tz::Tz;

use crate::errors::Error;
use crate::messages::{IncomingMessages, Notice, OutgoingMessages, ResponseMessage};

/// An item yielded by a [`Subscription`](crate::subscriptions::Subscription).
///
/// Subscriptions yield `Result<SubscriptionItem<T>, Error>` items. `Data(T)` is
/// the decoded payload; `Notice` is a non-fatal IB notice (warning codes
/// 2100..=2169) bound to this subscription — the stream stays open. Use the
/// `filter_data` adapter on the `Subscription` (sync: via `SubscriptionItemIterExt`;
/// async: via `SubscriptionItemStreamExt`) when you only care about data and
/// want notices logged automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubscriptionItem<T> {
    /// A successfully decoded payload from the subscription stream.
    Data(T),
    /// A non-fatal IB notice bound to this subscription: warnings (codes
    /// 2100..=2169), warning-form order messages (code 399), and — on the
    /// order-update stream — order-bound errors. Receiving a notice does not
    /// terminate the stream.
    Notice(Notice),
}

impl<T> SubscriptionItem<T> {
    /// Returns the inner data value, dropping notices. Pure conversion — no side effects.
    pub fn into_data(self) -> Option<T> {
        match self {
            SubscriptionItem::Data(t) => Some(t),
            SubscriptionItem::Notice(_) => None,
        }
    }
}

/// Maps `Ok(Notice)` to `None` (logged at `warn!`); passes `Data` and `Err`
/// through unchanged.
pub(crate) fn filter_notice<T>(item: Result<SubscriptionItem<T>, Error>) -> Option<Result<T, Error>> {
    match item {
        Ok(SubscriptionItem::Data(t)) => Some(Ok(t)),
        Ok(SubscriptionItem::Notice(n)) => {
            log::warn!("ib notice on subscription: {n}");
            None
        }
        Err(e) => Some(Err(e)),
    }
}

/// Pre-classified channel item delivered from the dispatcher to subscriptions.
/// `Response` carries raw bytes the decoder must still interpret; `Notice` and
/// `Error` are pre-classified by the dispatcher so decoders never re-classify
/// warnings vs. hard errors.
#[derive(Debug, Clone)]
pub(crate) enum RoutedItem {
    Response(ResponseMessage),
    Notice(Notice),
    Error(Error),
}

impl From<ResponseMessage> for RoutedItem {
    fn from(message: ResponseMessage) -> Self {
        RoutedItem::Response(message)
    }
}

impl From<Error> for RoutedItem {
    fn from(error: Error) -> Self {
        RoutedItem::Error(error)
    }
}

impl RoutedItem {
    /// Translate to `Result<ResponseMessage, Error>`. Returns `None` for
    /// `Notice` so callers can skip and recv the next item.
    pub(crate) fn into_legacy(self) -> Option<Result<ResponseMessage, Error>> {
        match self {
            RoutedItem::Response(message) => Some(Ok(message)),
            RoutedItem::Error(error) => Some(Err(error)),
            RoutedItem::Notice(_) => None,
        }
    }
}

/// `true` when `message` is not this subscription's to decode.
///
/// Shared and order-id channels fan several message types out to subscriptions
/// that each declare a subset, so a frame belonging to a sibling is routine and
/// simply skipped.
///
/// Lives here because both drivers must answer it identically — the sync/async
/// pair is exactly where a duplicated predicate drifts.
pub(crate) fn is_undeclared(ids: &[IncomingMessages], message: &ResponseMessage) -> bool {
    !ids.contains(&message.message_type())
}

/// Context for decoding responses, providing all necessary state for decoders.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecoderContext {
    /// Server version for protocol compatibility
    pub server_version: i32,
    /// Timezone for parsing timestamps (from TWS connection)
    pub time_zone: Option<&'static Tz>,
    /// Type of the original request that initiated this subscription
    pub request_type: Option<OutgoingMessages>,
    /// Whether this is a smart depth subscription
    pub is_smart_depth: bool,
}

impl DecoderContext {
    /// Create a new context with server version and optional timezone
    pub fn new(server_version: i32, time_zone: Option<&'static Tz>) -> Self {
        Self {
            server_version,
            time_zone,
            request_type: None,
            is_smart_depth: false,
        }
    }

    /// Set the request type
    #[allow(dead_code)]
    pub fn with_request_type(mut self, request_type: OutgoingMessages) -> Self {
        self.request_type = Some(request_type);
        self
    }

    /// Set the smart depth flag
    pub fn with_smart_depth(mut self, is_smart_depth: bool) -> Self {
        self.is_smart_depth = is_smart_depth;
        self
    }
}

/// Common trait for decoding streaming data responses
///
/// This trait is shared between sync and async implementations to avoid code duplication.
/// Decoders receive a `DecoderContext` containing server version, timezone, and other
/// context needed to properly decode messages.
pub(crate) trait StreamDecoder<T> {
    /// Message types this stream can handle. **The complete set** — a type
    /// absent from this list never reaches [`Self::decode`], so adding a
    /// `decode` arm means adding the type here too or the arm is dead code.
    ///
    /// No default, deliberately: an omitted declaration would skip everything,
    /// so it must be a compile error. `response_message_ids_tests.rs` enforces
    /// that this list and the `decode` arms agree in both directions; see
    /// `docs/rules/wire/proto-only-decoding.md`.
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages];

    /// Decode a response message into the stream's data type.
    ///
    /// Takes the message by shared reference: decoding reads `raw_bytes` and
    /// hands it to `prost`, which consumes nothing. The `&mut` this carried
    /// until #740 was left over from the text era, when decoders advanced a
    /// field cursor with `next_int` / `next_string`. Those still exist, but the
    /// handshake is their only caller now.
    fn decode(context: &DecoderContext, message: &ResponseMessage) -> Result<T, Error>;

    /// Generate a cancellation message for this stream
    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Err(Error::NotImplemented)
    }

    /// Returns true if this decoded value represents the end of a snapshot subscription
    #[allow(unused)]
    fn is_snapshot_end(&self) -> bool {
        false
    }
}

/// Debug-build guard: panics when a `request_id`-keyed subscription is built for
/// a decoder declaring a response type the dispatcher cannot route to it.
///
/// This is the enforcement the registration rule otherwise lacks. A decoder
/// claiming a message type missing from `text_request_id_field` gets a
/// subscription that silently never yields it, and no `MessageBusStub` test
/// notices, because those inject responses below the dispatcher — before PR #730
/// only a live gateway showed the gap (PR #647). They do run this constructor,
/// which is why the check sits here rather than in a test of its own.
///
/// Compiled out of release builds; the invariant is over static tables, so it
/// cannot depend on anything a caller passes in.
pub(crate) fn debug_assert_request_id_routable<T, D: StreamDecoder<T>>(request_id: Option<i32>) {
    if !cfg!(debug_assertions) || request_id.is_none() {
        return;
    }

    if let Some(kind) = crate::transport::routing::first_unroutable_by_request_id(D::RESPONSE_MESSAGE_IDS) {
        panic!(
            "{} declares {kind:?} in RESPONSE_MESSAGE_IDS, but {kind:?} has no `text_request_id_field` entry \
             in src/messages.rs — a request_id-keyed subscription can never receive it. \
             See docs/rules/wire/proto-aware-accessors.md",
            std::any::type_name::<D>()
        );
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "response_message_ids_tests.rs"]
mod response_message_ids_tests;
