use super::*;
use std::io;

#[test]
fn test_is_connection_error() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "io_connection_reset",
            error: Error::Io(io::Error::new(ErrorKind::ConnectionReset, "reset")),
            expected: true,
        },
        TestCase {
            name: "io_connection_aborted",
            error: Error::Io(io::Error::new(ErrorKind::ConnectionAborted, "aborted")),
            expected: true,
        },
        TestCase {
            name: "io_unexpected_eof",
            error: Error::Io(io::Error::new(ErrorKind::UnexpectedEof, "eof")),
            expected: true,
        },
        TestCase {
            name: "io_broken_pipe",
            error: Error::Io(io::Error::new(ErrorKind::BrokenPipe, "broken pipe")),
            expected: true,
        },
        TestCase {
            name: "connection_reset",
            error: Error::ConnectionReset,
            expected: true,
        },
        TestCase {
            name: "connection_failed",
            error: Error::ConnectionFailed,
            expected: true,
        },
        TestCase {
            name: "cancelled_not_connection",
            error: Error::Cancelled,
            expected: false,
        },
        TestCase {
            name: "io_other_not_connection",
            error: Error::Io(io::Error::other("other")),
            expected: false,
        },
        TestCase {
            name: "parse_error_not_connection",
            error: Error::Parse(0, "field".to_string(), "error".to_string()),
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(is_connection_error(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_is_timeout_error() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "io_would_block",
            error: Error::Io(io::Error::new(ErrorKind::WouldBlock, "would block")),
            expected: true,
        },
        TestCase {
            name: "io_timed_out",
            error: Error::Io(io::Error::new(ErrorKind::TimedOut, "timeout")),
            expected: true,
        },
        TestCase {
            name: "io_other_not_timeout",
            error: Error::Io(io::Error::other("other")),
            expected: false,
        },
        TestCase {
            name: "non_io_error",
            error: Error::Cancelled,
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(is_timeout_error(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_is_transient_error() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "unexpected_response",
            error: Error::UnexpectedResponse(crate::messages::ResponseMessage {
                i: 0,
                fields: vec!["45".to_string()], // TickGeneric message type
                server_version: 0,
                is_protobuf: false,
                raw_bytes: None,
            }),
            expected: true,
        },
        TestCase {
            name: "io_interrupted",
            error: Error::Io(io::Error::new(ErrorKind::Interrupted, "interrupted")),
            expected: true,
        },
        TestCase {
            name: "io_would_block",
            error: Error::Io(io::Error::new(ErrorKind::WouldBlock, "would block")),
            expected: true,
        },
        TestCase {
            name: "io_timed_out",
            error: Error::Io(io::Error::new(ErrorKind::TimedOut, "timeout")),
            expected: true,
        },
        TestCase {
            name: "connection_error_not_transient",
            error: Error::ConnectionReset,
            expected: false,
        },
        TestCase {
            name: "fatal_error_not_transient",
            error: Error::Shutdown,
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(is_transient_error(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_should_retry_request() {
    struct TestCase {
        name: &'static str,
        error: Error,
        retry_count: u32,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "connection_error_first_retry",
            error: Error::ConnectionReset,
            retry_count: 0,
            expected: true,
        },
        TestCase {
            name: "connection_error_second_retry",
            error: Error::ConnectionReset,
            retry_count: 1,
            expected: true,
        },
        TestCase {
            name: "connection_error_max_retries",
            error: Error::ConnectionReset,
            retry_count: MAX_RETRIES,
            expected: false,
        },
        TestCase {
            name: "transient_error_first_retry",
            error: Error::UnexpectedResponse(crate::messages::ResponseMessage {
                i: 0,
                fields: vec!["45".to_string()], // TickGeneric message type
                server_version: 0,
                is_protobuf: false,
                raw_bytes: None,
            }),
            retry_count: 0,
            expected: true,
        },
        TestCase {
            name: "fatal_error_no_retry",
            error: Error::Shutdown,
            retry_count: 0,
            expected: false,
        },
        TestCase {
            name: "non_retryable_error",
            error: Error::InvalidArgument("test".to_string()),
            retry_count: 0,
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(
            should_retry_request(&tc.error, tc.retry_count),
            tc.expected,
            "test case '{}' failed",
            tc.name
        );
    }
}

#[test]
fn test_is_fatal_error() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "shutdown",
            error: Error::Shutdown,
            expected: true,
        },
        TestCase {
            name: "invalid_argument",
            error: Error::InvalidArgument("test".to_string()),
            expected: true,
        },
        TestCase {
            name: "not_implemented",
            error: Error::NotImplemented,
            expected: true,
        },
        TestCase {
            name: "server_version",
            error: Error::ServerVersion(100, 90, "feature".to_string()),
            expected: true,
        },
        TestCase {
            name: "already_subscribed",
            error: Error::AlreadySubscribed,
            expected: true,
        },
        TestCase {
            name: "connection_reset_not_fatal",
            error: Error::ConnectionReset,
            expected: false,
        },
        TestCase {
            name: "cancelled_not_fatal",
            error: Error::Cancelled,
            expected: false,
        },
        TestCase {
            name: "io_error_not_fatal",
            error: Error::Io(io::Error::other("io error")),
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(is_fatal_error(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_error_message() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: &'static str,
    }

    let test_cases = vec![
        TestCase {
            name: "connection_failed",
            error: Error::ConnectionFailed,
            expected: "Connection to TWS/Gateway failed",
        },
        TestCase {
            name: "connection_reset",
            error: Error::ConnectionReset,
            expected: "Connection was reset by TWS/Gateway",
        },
        TestCase {
            name: "shutdown",
            error: Error::Shutdown,
            expected: "Client is shutting down",
        },
        TestCase {
            name: "cancelled",
            error: Error::Cancelled,
            expected: "Operation was cancelled",
        },
        TestCase {
            name: "end_of_stream",
            error: Error::EndOfStream,
            expected: "No more data available",
        },
        TestCase {
            name: "server_version",
            error: Error::ServerVersion(100, 90, "feature".to_string()),
            expected: "Server version 100 required for feature, but connected to version 90",
        },
        TestCase {
            name: "tws_message",
            error: Error::Message(200, "test error".to_string()),
            expected: "TWS Error [200]: test error",
        },
    ];

    for tc in test_cases {
        assert_eq!(error_message(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }

    // Test fallback to default to_string()
    let parse_err = Error::ParseInt("123x".parse::<i32>().unwrap_err());
    let msg = error_message(&parse_err);
    assert!(msg.contains("invalid digit"), "parse error should use default to_string()");
}

#[test]
fn test_error_categorization() {
    struct TestCase {
        name: &'static str,
        error: Error,
        expected: ErrorCategory,
    }

    let test_cases = vec![
        // Connection category
        TestCase {
            name: "connection_failed",
            error: Error::ConnectionFailed,
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "connection_reset",
            error: Error::ConnectionReset,
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "io_connection_reset",
            error: Error::Io(io::Error::new(ErrorKind::ConnectionReset, "reset")),
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "io_connection_aborted",
            error: Error::Io(io::Error::new(ErrorKind::ConnectionAborted, "aborted")),
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "io_unexpected_eof",
            error: Error::Io(io::Error::new(ErrorKind::UnexpectedEof, "eof")),
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "io_broken_pipe",
            error: Error::Io(io::Error::new(ErrorKind::BrokenPipe, "pipe")),
            expected: ErrorCategory::Connection,
        },
        TestCase {
            name: "io_connection_refused",
            error: Error::Io(io::Error::new(ErrorKind::ConnectionRefused, "refused")),
            expected: ErrorCategory::Connection,
        },
        // Timeout category
        TestCase {
            name: "io_would_block",
            error: Error::Io(io::Error::new(ErrorKind::WouldBlock, "would block")),
            expected: ErrorCategory::Timeout,
        },
        TestCase {
            name: "io_timed_out",
            error: Error::Io(io::Error::new(ErrorKind::TimedOut, "timeout")),
            expected: ErrorCategory::Timeout,
        },
        // Parsing category
        TestCase {
            name: "parse_error",
            error: Error::Parse(0, "field".to_string(), "error".to_string()),
            expected: ErrorCategory::Parsing,
        },
        TestCase {
            name: "parse_int",
            error: Error::ParseInt("123x".parse::<i32>().unwrap_err()),
            expected: ErrorCategory::Parsing,
        },
        TestCase {
            name: "from_utf8",
            error: Error::FromUtf8(String::from_utf8(vec![0xFF, 0xFE]).unwrap_err()),
            expected: ErrorCategory::Parsing,
        },
        // Validation category
        TestCase {
            name: "invalid_argument",
            error: Error::InvalidArgument("test".to_string()),
            expected: ErrorCategory::Validation,
        },
        TestCase {
            name: "server_version",
            error: Error::ServerVersion(100, 90, "feature".to_string()),
            expected: ErrorCategory::Validation,
        },
        // Server error category
        TestCase {
            name: "tws_message",
            error: Error::Message(200, "test".to_string()),
            expected: ErrorCategory::ServerError,
        },
        // Cancelled category
        TestCase {
            name: "cancelled",
            error: Error::Cancelled,
            expected: ErrorCategory::Cancelled,
        },
        // Fatal category
        TestCase {
            name: "shutdown",
            error: Error::Shutdown,
            expected: ErrorCategory::Fatal,
        },
        TestCase {
            name: "not_implemented",
            error: Error::NotImplemented,
            expected: ErrorCategory::Fatal,
        },
        TestCase {
            name: "already_subscribed",
            error: Error::AlreadySubscribed,
            expected: ErrorCategory::Fatal,
        },
        // Transient category
        TestCase {
            name: "unexpected_response",
            error: Error::UnexpectedResponse(crate::messages::ResponseMessage {
                i: 0,
                fields: vec!["45".to_string()], // TickGeneric message type
                server_version: 0,
                is_protobuf: false,
                raw_bytes: None,
            }),
            expected: ErrorCategory::Transient,
        },
        TestCase {
            name: "unexpected_end_of_stream",
            error: Error::UnexpectedEndOfStream,
            expected: ErrorCategory::Transient,
        },
        TestCase {
            name: "simple_error_transient",
            error: Error::Simple("test".to_string()),
            expected: ErrorCategory::Transient,
        },
        TestCase {
            name: "end_of_stream_transient",
            error: Error::EndOfStream,
            expected: ErrorCategory::Transient,
        },
    ];

    for tc in test_cases {
        assert_eq!(categorize_error(&tc.error), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_is_connection_io_error() {
    struct TestCase {
        name: &'static str,
        error_kind: ErrorKind,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "connection_reset",
            error_kind: ErrorKind::ConnectionReset,
            expected: true,
        },
        TestCase {
            name: "connection_aborted",
            error_kind: ErrorKind::ConnectionAborted,
            expected: true,
        },
        TestCase {
            name: "unexpected_eof",
            error_kind: ErrorKind::UnexpectedEof,
            expected: true,
        },
        TestCase {
            name: "broken_pipe",
            error_kind: ErrorKind::BrokenPipe,
            expected: true,
        },
        TestCase {
            name: "connection_refused",
            error_kind: ErrorKind::ConnectionRefused,
            expected: true,
        },
        TestCase {
            name: "permission_denied_not_connection",
            error_kind: ErrorKind::PermissionDenied,
            expected: false,
        },
        TestCase {
            name: "not_found_not_connection",
            error_kind: ErrorKind::NotFound,
            expected: false,
        },
    ];

    for tc in test_cases {
        let io_err = io::Error::new(tc.error_kind, "test");
        assert_eq!(is_connection_io_error(&io_err), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_is_timeout_io_error() {
    struct TestCase {
        name: &'static str,
        error_kind: ErrorKind,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "would_block",
            error_kind: ErrorKind::WouldBlock,
            expected: true,
        },
        TestCase {
            name: "timed_out",
            error_kind: ErrorKind::TimedOut,
            expected: true,
        },
        TestCase {
            name: "interrupted_not_timeout",
            error_kind: ErrorKind::Interrupted,
            expected: false,
        },
        TestCase {
            name: "other_not_timeout",
            error_kind: ErrorKind::Other,
            expected: false,
        },
    ];

    for tc in test_cases {
        let io_err = io::Error::new(tc.error_kind, "test");
        assert_eq!(is_timeout_io_error(&io_err), tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn test_max_retries_constant() {
    assert_eq!(MAX_RETRIES, 3, "MAX_RETRIES should be 3");
}

#[test]
fn test_error_category_equality() {
    assert_eq!(ErrorCategory::Connection, ErrorCategory::Connection);
    assert_ne!(ErrorCategory::Connection, ErrorCategory::Parsing);
    assert_ne!(ErrorCategory::Timeout, ErrorCategory::Fatal);
}
