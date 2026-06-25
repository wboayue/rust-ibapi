//! Error types for the IBAPI library.
//!
//! This module defines all error types that can occur during API operations,
//! including I/O errors, parsing errors, and TWS-specific protocol errors.

use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;

use crate::market_data::historical::HistoricalParseError;
use crate::messages::{Notice, ResponseMessage};
use crate::orders::builder::ValidationError;

/// The main error type for IBAPI operations.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new error variants
/// in future versions without breaking compatibility.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    // External error types
    /// I/O error from network operations.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Failed to parse an integer from string.
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    /// Invalid UTF-8 sequence in response data.
    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),

    /// Failed to parse time/date string.
    #[error(transparent)]
    ParseTime(#[from] time::error::Parse),

    /// Mutex was poisoned by a panic in another thread.
    #[error("{0}")]
    Poison(String),

    // IBAPI-specific errors
    /// Feature or method not yet implemented.
    #[error("not implemented")]
    NotImplemented,

    /// Failed to parse a protocol message.
    /// Contains: (field_index, field_value, error_description)
    #[error("parse error: {0} - {1} - {2}")]
    Parse(usize, String, String),

    /// Server version requirement not met.
    /// Contains: (required_version, actual_version, feature_name)
    #[error("server version {0} required, got {1}: {2}")]
    ServerVersion(i32, i32, String),

    /// Generic error with custom message.
    #[error("error occurred: {0}")]
    Simple(String),

    /// Invalid argument provided to API method.
    #[error("InvalidArgument: {0}")]
    InvalidArgument(String),

    /// Failed to establish connection to TWS/Gateway.
    #[error("ConnectionFailed")]
    ConnectionFailed,

    /// TWS/Gateway accepted the TCP connection but closed before completing
    /// the handshake — typically a host allow-list mismatch on the gateway.
    /// Payload carries the underlying diagnostic.
    #[error("connection rejected: {0}")]
    ConnectionRejected(String),

    /// IB Gateway sent a timezone name that could not be mapped to an IANA zone.
    #[error("unrecognized IB Gateway timezone {0:?}; register a mapping with `ibapi::register_timezone_alias({0:?}, \"<IANA-name>\")` before connecting, or set `IBAPI_TIMEZONE_ALIASES={0}=<IANA-name>` in the environment. To request it as a built-in, file an issue at https://github.com/wboayue/rust-ibapi/issues")]
    UnsupportedTimeZone(String),

    /// Connection was reset by TWS/Gateway.
    #[error("ConnectionReset")]
    ConnectionReset,

    /// Operation was cancelled by user or system.
    #[error("Cancelled")]
    Cancelled,

    /// Client is shutting down.
    #[error("Shutdown")]
    Shutdown,

    /// Reached end of data stream.
    #[error("EndOfStream")]
    EndOfStream,

    /// Received unexpected message type. The string carries the `Debug` repr
    /// of the offending wire envelope for diagnostic logging; the structured
    /// payload is no longer exposed (rust-ibapi 3.x retired
    /// `ResponseMessage` from the public surface).
    #[error("UnexpectedResponse: {0}")]
    UnexpectedResponse(String),

    /// Stream ended unexpectedly.
    #[error("UnexpectedEndOfStream")]
    UnexpectedEndOfStream,

    /// An IB notice frame (TWS error/warning/system message) received in
    /// response to a request. Carries the full typed [`Notice`] — code,
    /// message, optional timestamp, and advanced-order-reject JSON.
    ///
    /// Use [`Notice::category`] / [`Notice::is_order_rejection`] /
    /// [`Notice::is_warning`] to classify without string-parsing. Distinct
    /// from [`Error::ConnectionRejected`] (handshake-time refusal) and the
    /// transport variants ([`Error::Io`], [`Error::ConnectionReset`]).
    #[error("{0}")]
    Notice(Notice),

    /// Attempted to create a duplicate subscription.
    #[error("AlreadySubscribed")]
    AlreadySubscribed,

    /// Wraps errors parsing historical data parameters.
    #[error("HistoricalParseError: {0}")]
    HistoricalParseError(HistoricalParseError),

    /// Failed to decode a protobuf message.
    #[error("protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),
}

impl From<ResponseMessage> for Error {
    fn from(err: ResponseMessage) -> Error {
        Error::Notice(Notice::from(&err))
    }
}

impl From<&ResponseMessage> for Error {
    fn from(err: &ResponseMessage) -> Error {
        Error::Notice(Notice::from(err))
    }
}

impl From<crate::transport::routing::DecodedError> for Error {
    /// Project a dispatcher-decoded error payload to [`Error::Notice`].
    /// Mirrors the [`From<ResponseMessage>`] projection but skips the
    /// wire-message re-parse since the dispatcher already extracted the
    /// fields, and moves the message string instead of cloning.
    fn from(payload: crate::transport::routing::DecodedError) -> Error {
        Error::Notice(Notice::from(payload))
    }
}

impl Error {
    /// Build an [`Error::UnexpectedResponse`] from an internal `ResponseMessage`.
    /// Captures the `Debug` repr in the variant's `String` payload — the
    /// structured envelope is no longer exposed publicly. Crate-private; the
    /// variant's pattern `Error::UnexpectedResponse(_)` remains matchable by
    /// downstream code.
    pub(crate) fn unexpected_response(message: &ResponseMessage) -> Error {
        Error::UnexpectedResponse(format!("{message:?}"))
    }

    /// Build an [`Error::Parse`] when the failing input came from a text-protocol
    /// wire field whose index is not load-bearing (e.g. inside a helper that has
    /// lost the index, or in a proto codepath). Encapsulates the placeholder `0`
    /// so the variant tuple stays the same shape across call sites while
    /// readers don't have to remember the convention.
    pub(crate) fn parse_field(value: impl Into<String>, reason: impl Into<String>) -> Error {
        Error::Parse(0, value.into(), reason.into())
    }

    /// Same as [`Error::parse_field`], but named for proto-decoded inputs where
    /// the first arg is a logical field/identifier rather than a wire-field
    /// string value. Variant shape is identical; the name disambiguates intent
    /// at the call site.
    pub(crate) fn parse_proto(field: impl Into<String>, reason: impl Into<String>) -> Error {
        Error::Parse(0, field.into(), reason.into())
    }

    /// Build an [`Error::Parse`] for cursor EOF: the message ran out of fields
    /// while the caller was trying to read field index `i`. `label` names the
    /// expected type ("int", "string", "datetime", ...) so the resulting
    /// `parse error: i -  - expected <label> and found end of message`
    /// pinpoints both location and intent.
    pub(crate) fn eof_at(i: usize, label: &str) -> Error {
        Error::Parse(i, String::new(), format!("expected {label} and found end of message"))
    }

    /// Returns `true` if this error means the TWS/Gateway connection is gone and
    /// the client should reconnect, rather than retry the in-flight request.
    ///
    /// Matches the transport-reset variants ([`Error::ConnectionReset`],
    /// [`Error::ConnectionFailed`]) and connection-kind [`Error::Io`] errors
    /// (broken pipe, unexpected EOF, connection reset/abort). Intentional teardown
    /// ([`Error::Shutdown`]) and handshake refusal ([`Error::ConnectionRejected`])
    /// are **not** connection-loss — reconnecting cannot recover them.
    ///
    /// # Examples
    ///
    /// In a subscription read loop, branch on this predicate to decide whether to
    /// re-establish the connection or surface a request-level failure:
    ///
    /// ```
    /// use ibapi::Error;
    ///
    /// fn on_stream_error(err: Error) -> Result<(), Error> {
    ///     if err.is_connection_lost() {
    ///         // tear down and resubscribe, then keep going
    ///         Ok(())
    ///     } else {
    ///         // a request-level failure — surface it to the caller
    ///         Err(err)
    ///     }
    /// }
    ///
    /// assert!(on_stream_error(Error::ConnectionReset).is_ok());
    /// assert!(on_stream_error(Error::Shutdown).is_err());
    /// ```
    pub fn is_connection_lost(&self) -> bool {
        use std::io::ErrorKind;
        match self {
            Error::Io(io_err) => matches!(
                io_err.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe
            ),
            Error::ConnectionReset | Error::ConnectionFailed => true,
            _ => false,
        }
    }
}

// Manual Clone because `std::io::Error` and `time::error::Parse` don't derive it.
// `ParseTime` is lossy: it collapses to `Error::Simple` and a cloned value
// no longer matches `Error::ParseTime(_)`.
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Io(e) => Error::Io(std::io::Error::new(e.kind(), e.to_string())),
            Error::ParseInt(e) => Error::ParseInt(e.clone()),
            Error::FromUtf8(e) => Error::FromUtf8(e.clone()),
            Error::ParseTime(_) => Error::Simple(self.to_string()),
            Error::Poison(s) => Error::Poison(s.clone()),
            Error::NotImplemented => Error::NotImplemented,
            Error::Parse(i, v, m) => Error::Parse(*i, v.clone(), m.clone()),
            Error::ServerVersion(a, b, s) => Error::ServerVersion(*a, *b, s.clone()),
            Error::Simple(s) => Error::Simple(s.clone()),
            Error::InvalidArgument(s) => Error::InvalidArgument(s.clone()),
            Error::ConnectionFailed => Error::ConnectionFailed,
            Error::ConnectionRejected(s) => Error::ConnectionRejected(s.clone()),
            Error::UnsupportedTimeZone(s) => Error::UnsupportedTimeZone(s.clone()),
            Error::ConnectionReset => Error::ConnectionReset,
            Error::Cancelled => Error::Cancelled,
            Error::Shutdown => Error::Shutdown,
            Error::EndOfStream => Error::EndOfStream,
            Error::UnexpectedResponse(m) => Error::UnexpectedResponse(m.clone()),
            Error::UnexpectedEndOfStream => Error::UnexpectedEndOfStream,
            Error::Notice(n) => Error::Notice(n.clone()),
            Error::AlreadySubscribed => Error::AlreadySubscribed,
            Error::HistoricalParseError(e) => Error::HistoricalParseError(e.clone()),
            Error::ProtobufDecode(e) => Error::ProtobufDecode(e.clone()),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Error {
        Error::Poison(format!("Mutex poison error: {err}"))
    }
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::InvalidQuantity(q) => Error::InvalidArgument(format!("Invalid quantity: {}", q)),
            ValidationError::InvalidPrice(p) => Error::InvalidArgument(format!("Invalid price: {}", p)),
            ValidationError::MissingRequiredField(field) => Error::InvalidArgument(format!("Missing required field: {}", field)),
            ValidationError::InvalidCombination(msg) => Error::InvalidArgument(format!("Invalid combination: {}", msg)),
            ValidationError::InvalidStopPrice { stop, current } => {
                Error::InvalidArgument(format!("Invalid stop price {} for current price {}", stop, current))
            }
            ValidationError::InvalidLimitPrice { limit, current } => {
                Error::InvalidArgument(format!("Invalid limit price {} for current price {}", limit, current))
            }
            ValidationError::InvalidBracketOrder(msg) => Error::InvalidArgument(format!("Invalid bracket order: {}", msg)),
            ValidationError::InvalidPercentage { field, value, min, max } => {
                Error::InvalidArgument(format!("Invalid {}: {} (must be between {} and {})", field, value, min, max))
            }
        }
    }
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod tests;
