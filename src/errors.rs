//! Error types for the IBAPI library.
//!
//! This module defines all error types that can occur during API operations,
//! including I/O errors, parsing errors, and TWS-specific protocol errors.

use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;

use crate::market_data::historical::HistoricalParseError;
use crate::messages::{ResponseMessage, CODE_INDEX, MESSAGE_INDEX};

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

    /// Received unexpected message type.
    #[error("UnexpectedResponse: {0:?}")]
    UnexpectedResponse(ResponseMessage),

    /// Stream ended unexpectedly.
    #[error("UnexpectedEndOfStream")]
    UnexpectedEndOfStream,

    /// Error message from TWS/Gateway.
    /// Contains: (error_code, error_message)
    #[error("[{0}] {1}")]
    Message(i32, String),

    /// Attempted to create a duplicate subscription.
    #[error("AlreadySubscribed")]
    AlreadySubscribed,

    #[error("HistoricalParseError: {0}")]
    HistoricalParseError(HistoricalParseError),
}

impl From<ResponseMessage> for Error {
    fn from(err: ResponseMessage) -> Error {
        let code = err.peek_int(CODE_INDEX).unwrap();
        let message = err.peek_string(MESSAGE_INDEX);
        Error::Message(code, message)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Error {
        Error::Poison(format!("Mutex poison error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;
    use std::io;
    use std::sync::{Mutex, PoisonError};
    use time::macros::format_description;
    use time::Time;

    #[test]
    fn test_error_debug() {
        let error = Error::Simple("test error".to_string());
        assert_eq!(format!("{error:?}"), "Simple(\"test error\")");
    }

    #[test]
    fn test_error_display() {
        let cases = vec![
            (Error::Io(io::Error::new(io::ErrorKind::NotFound, "file not found")), "file not found"),
            (Error::ParseInt("123x".parse::<i32>().unwrap_err()), "invalid digit found in string"),
            (
                Error::FromUtf8(String::from_utf8(vec![0, 159, 146, 150]).unwrap_err()),
                "invalid utf-8 sequence of 1 bytes from index 1",
            ),
            (
                Error::ParseTime(Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err()),
                "the 'month' component could not be parsed",
            ),
            (Error::Poison("test poison".to_string()), "test poison"),
            (Error::NotImplemented, "not implemented"),
            (
                Error::Parse(1, "value".to_string(), "message".to_string()),
                "parse error: 1 - value - message",
            ),
            (
                Error::ServerVersion(2, 1, "old version".to_string()),
                "server version 2 required, got 1: old version",
            ),
            (Error::ConnectionFailed, "ConnectionFailed"),
            (Error::Cancelled, "Cancelled"),
            (Error::Simple("simple error".to_string()), "error occurred: simple error"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_error_is_error() {
        let error = Error::Simple("test error".to_string());
        // With thiserror, source() returns the underlying error if using #[from]
        // For Simple errors, there's no underlying source
        assert!(error.source().is_none());
    }

    #[test]
    fn test_from_io_error() {
        let io_error = io::Error::other("io error");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_error = "abc".parse::<i32>().unwrap_err();
        let error: Error = parse_error.into();
        assert!(matches!(error, Error::ParseInt(_)));
    }

    #[test]
    fn test_from_utf8_error() {
        let utf8_error = String::from_utf8(vec![0, 159, 146, 150]).unwrap_err();
        let error: Error = utf8_error.into();
        assert!(matches!(error, Error::FromUtf8(_)));
    }

    #[test]
    fn test_from_parse_time_error() {
        let time_error = Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err();
        let error: Error = time_error.into();
        assert!(matches!(error, Error::ParseTime(_)));
    }

    #[test]
    fn test_from_poison_error() {
        let mutex = Mutex::new(());
        let poison_error = PoisonError::new(mutex);
        let error: Error = poison_error.into();
        assert!(matches!(error, Error::Poison(_)));
    }

    #[test]
    fn test_non_exhaustive() {
        fn assert_non_exhaustive<T: StdError>() {}
        assert_non_exhaustive::<Error>();
    }
}
