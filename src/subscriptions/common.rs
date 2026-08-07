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
    /// A non-fatal IB notice (warning codes 2100..=2169) bound to this subscription.
    /// Receiving a notice does not terminate the stream.
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

/// Checks if an error indicates the end of a stream
#[allow(dead_code)]
pub(crate) fn is_stream_end(error: &Error) -> bool {
    matches!(error, Error::EndOfStream)
}

/// Checks if an error should be stored for later retrieval
#[allow(dead_code)]
pub(crate) fn should_store_error(error: &Error) -> bool {
    !is_stream_end(error)
}

/// Common error types that can occur during subscription processing
#[derive(Debug)]
pub(crate) enum ProcessingResult<T> {
    /// Successfully processed a value
    Success(T),
    /// Encountered an error that should be stored
    Error(Error),
    /// Stream has ended normally
    EndOfStream,
}

/// Process a decoding result into a common processing result.
///
/// Every error now terminates the subscription except [`Error::EndOfStream`],
/// whose name and disposition agree everywhere it is used — the transport
/// raises it too, and it always means the stream is over.
///
/// **No error variant means "skip" any more.** Whether a message belongs to this
/// subscription is answered before `decode` runs, from
/// [`StreamDecoder::RESPONSE_MESSAGE_IDS`]. That indirection was the defect
/// behind #508 and #731: `Error::UnexpectedResponse` is returned to users as a
/// real error by ~20 one-shot call sites, and *also* meant "silently drop this"
/// here, so any decoder that reused the variant inherited the skip disposition
/// without asking for it.
pub(crate) fn process_decode_result<T>(result: Result<T, Error>) -> ProcessingResult<T> {
    match result {
        Ok(val) => ProcessingResult::Success(val),
        Err(Error::EndOfStream) => ProcessingResult::EndOfStream,
        Err(err) => ProcessingResult::Error(err),
    }
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
    /// absent from this list never reaches [`Self::decode`].
    ///
    /// Load-bearing twice over, which is why it has no default: the subscription
    /// drivers skip anything not listed here (shared channels carry several
    /// types), and [`debug_assert_request_id_routable`] checks every entry
    /// against the routing allow-list when a `request_id`-keyed subscription is
    /// built.
    ///
    /// Adding a `decode` arm therefore means adding the type here too. Forget
    /// it and the arm is dead — but every domain's stub tests feed the types
    /// they care about through a real subscription, so the omission fails a
    /// test rather than reaching a user.
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages];

    /// Decode a response message into the stream's data type
    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<T, Error>;

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
