//! Consolidated error handling utilities for the client
//!
//! This module provides common error handling functions used throughout
//! the client implementation.

// TODO: Remove this when async reconnection/retry logic is implemented (see transport/async.rs:175)
// Currently only is_connection_error and is_timeout_error are used by sync transport
#![allow(dead_code)]

use std::io::ErrorKind;

use crate::errors::Error;

/// Maximum number of retries for transient errors
pub(crate) const MAX_RETRIES: u32 = 3;

/// Checks if the error is a connection-related IO error that should trigger reconnection
pub(crate) fn is_connection_error(error: &Error) -> bool {
    match error {
        Error::Io(io_err) => matches!(
            io_err.kind(),
            ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::UnexpectedEof
        ),
        Error::ConnectionReset | Error::ConnectionFailed => true,
        _ => false,
    }
}

/// Checks if the error is a timeout that can be safely ignored
pub(crate) fn is_timeout_error(error: &Error) -> bool {
    match error {
        Error::Io(io_err) => matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
        _ => false,
    }
}

/// Checks if an error should trigger a retry
pub(crate) fn should_retry_request(error: &Error, retry_count: u32) -> bool {
    retry_count < MAX_RETRIES && (is_connection_error(error) || is_transient_error(error))
}

/// Checks if an error is transient and may succeed on retry
pub(crate) fn is_transient_error(error: &Error) -> bool {
    match error {
        Error::UnexpectedResponse(_) => true,
        Error::Io(io_err) => matches!(io_err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock | ErrorKind::TimedOut),
        _ => false,
    }
}

/// Checks if an error is fatal and should not be retried
pub(crate) fn is_fatal_error(error: &Error) -> bool {
    matches!(
        error,
        Error::Shutdown | Error::InvalidArgument(_) | Error::NotImplemented | Error::ServerVersion(_, _, _) | Error::AlreadySubscribed
    )
}

/// Converts an error to a user-friendly message
pub(crate) fn error_message(error: &Error) -> String {
    match error {
        Error::ConnectionFailed => "Connection to TWS/Gateway failed".to_string(),
        Error::ConnectionReset => "Connection was reset by TWS/Gateway".to_string(),
        Error::Shutdown => "Client is shutting down".to_string(),
        Error::Cancelled => "Operation was cancelled".to_string(),
        Error::EndOfStream => "No more data available".to_string(),
        Error::ServerVersion(required, actual, feature) => {
            format!("Server version {required} required for {feature}, but connected to version {actual}")
        }
        Error::Message(code, msg) => format!("TWS Error [{code}]: {msg}"),
        _ => error.to_string(),
    }
}

/// Error categories for logging and metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorCategory {
    Connection,
    Parsing,
    Validation,
    ServerError,
    Timeout,
    Cancelled,
    Fatal,
    Transient,
}

/// Categorizes an error for logging and metrics purposes
pub(crate) fn categorize_error(error: &Error) -> ErrorCategory {
    match error {
        Error::ConnectionFailed | Error::ConnectionReset => ErrorCategory::Connection,
        Error::Io(io_err) if is_connection_io_error(io_err) => ErrorCategory::Connection,
        Error::Io(io_err) if is_timeout_io_error(io_err) => ErrorCategory::Timeout,
        Error::Parse(_, _, _) | Error::ParseInt(_) | Error::FromUtf8(_) | Error::ParseTime(_) => ErrorCategory::Parsing,
        Error::InvalidArgument(_) | Error::ServerVersion(_, _, _) => ErrorCategory::Validation,
        Error::Message(_, _) => ErrorCategory::ServerError,
        Error::Cancelled => ErrorCategory::Cancelled,
        Error::Shutdown | Error::NotImplemented | Error::AlreadySubscribed => ErrorCategory::Fatal,
        Error::UnexpectedResponse(_) | Error::UnexpectedEndOfStream => ErrorCategory::Transient,
        _ => ErrorCategory::Transient,
    }
}

fn is_connection_io_error(io_err: &std::io::Error) -> bool {
    matches!(
        io_err.kind(),
        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe | ErrorKind::ConnectionRefused
    )
}

fn is_timeout_io_error(io_err: &std::io::Error) -> bool {
    matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_is_connection_error() {
        let io_err = Error::Io(std::sync::Arc::new(io::Error::new(ErrorKind::ConnectionReset, "reset")));
        assert!(is_connection_error(&io_err));

        assert!(is_connection_error(&Error::ConnectionReset));
        assert!(is_connection_error(&Error::ConnectionFailed));
        assert!(!is_connection_error(&Error::Cancelled));
    }

    #[test]
    fn test_is_timeout_error() {
        let timeout_err = Error::Io(std::sync::Arc::new(io::Error::new(ErrorKind::WouldBlock, "would block")));
        assert!(is_timeout_error(&timeout_err));

        let non_timeout = Error::Io(std::sync::Arc::new(io::Error::new(ErrorKind::Other, "other")));
        assert!(!is_timeout_error(&non_timeout));
    }

    #[test]
    fn test_should_retry_request() {
        let conn_err = Error::ConnectionReset;
        assert!(should_retry_request(&conn_err, 0));
        assert!(should_retry_request(&conn_err, 2));
        assert!(!should_retry_request(&conn_err, MAX_RETRIES));

        let fatal_err = Error::Shutdown;
        assert!(!should_retry_request(&fatal_err, 0));
    }

    #[test]
    fn test_is_fatal_error() {
        assert!(is_fatal_error(&Error::Shutdown));
        assert!(is_fatal_error(&Error::InvalidArgument("test".to_string())));
        assert!(is_fatal_error(&Error::NotImplemented));
        assert!(!is_fatal_error(&Error::ConnectionReset));
    }

    #[test]
    fn test_error_categorization() {
        assert_eq!(categorize_error(&Error::ConnectionFailed), ErrorCategory::Connection);
        assert_eq!(
            categorize_error(&Error::ParseInt("123x".parse::<i32>().unwrap_err())),
            ErrorCategory::Parsing
        );
        assert_eq!(categorize_error(&Error::Cancelled), ErrorCategory::Cancelled);
        assert_eq!(categorize_error(&Error::Message(200, "test".to_string())), ErrorCategory::ServerError);
    }
}
