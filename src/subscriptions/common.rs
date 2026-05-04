//! Common utilities for subscription processing

use serde::{Deserialize, Serialize};
use time_tz::Tz;

use crate::errors::Error;
use crate::messages::{IncomingMessages, Notice, OutgoingMessages, ResponseMessage};

/// An item yielded by a [`Subscription`](crate::subscriptions::Subscription).
///
/// Subscriptions return `Option<Result<SubscriptionItem<T>, Error>>` from `next`,
/// `try_next`, and `next_timeout`. `Data(T)` is the decoded payload; `Notice` is a
/// non-fatal IB notice (warning codes 2100..=2169) bound to this subscription —
/// the stream stays open. Use [`Subscription::iter_data`](crate::subscriptions::Subscription::iter_data)
/// (or async [`Subscription::data_stream`](crate::subscriptions::Subscription::data_stream))
/// when you only care about data and want notices logged automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubscriptionItem<T> {
    /// A successfully decoded payload from the subscription stream.
    Data(T),
    /// A non-fatal IB notice (warning codes 2100..=2169) bound to this subscription.
    /// Receiving a notice does not terminate the stream.
    Notice(Notice),
}

/// Pre-classified channel item delivered from the dispatcher to subscriptions.
/// `Response` carries raw bytes the decoder must still interpret; `Notice` and
/// `Error` are pre-classified by the dispatcher so decoders never re-classify
/// warnings vs. hard errors.
#[derive(Debug, Clone)]
pub(crate) enum RoutedItem {
    Response(ResponseMessage),
    #[allow(dead_code)]
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
    /// Message not intended for this subscription — skip silently.
    /// Occurs on shared broadcast channels where messages from other
    /// subscriptions can arrive on the same channel.
    Skip,
    /// Encountered an error that should be stored
    Error(Error),
    /// Stream has ended normally
    EndOfStream,
}

/// Process a decoding result into a common processing result
pub(crate) fn process_decode_result<T>(result: Result<T, Error>) -> ProcessingResult<T> {
    match result {
        Ok(val) => ProcessingResult::Success(val),
        Err(Error::EndOfStream) => ProcessingResult::EndOfStream,
        Err(Error::UnexpectedResponse(_)) => ProcessingResult::Skip,
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
    /// Message types this stream can handle
    #[allow(dead_code)]
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[];

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

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
