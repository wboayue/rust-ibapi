//! Common utilities for subscription processing

use crate::errors::Error;
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};

/// Maximum number of retry attempts when encountering unexpected responses.
/// This prevents infinite loops when TWS sends unexpected message types.
pub(crate) const MAX_DECODE_RETRIES: usize = 10;

/// Result of checking whether a retry should be attempted
#[derive(Debug, PartialEq)]
pub(crate) enum RetryDecision {
    /// Continue retrying
    Continue,
    /// Stop retrying, max attempts exceeded
    Stop,
}

/// Checks if a retry should be attempted and logs appropriately.
/// Returns `RetryDecision::Continue` if retry count is below max, `RetryDecision::Stop` otherwise.
pub(crate) fn check_retry(retry_count: usize) -> RetryDecision {
    if retry_count < MAX_DECODE_RETRIES {
        log::warn!("retrying after unexpected response (attempt {}/{})", retry_count + 1, MAX_DECODE_RETRIES);
        RetryDecision::Continue
    } else {
        log::error!("max retries ({}) exceeded, stopping subscription", MAX_DECODE_RETRIES);
        RetryDecision::Stop
    }
}

/// Checks if an error indicates the subscription should retry processing
#[allow(dead_code)]
pub(crate) fn should_retry_error(error: &Error) -> bool {
    matches!(error, Error::UnexpectedResponse(_))
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
    /// Encountered an error that should be retried
    Retry,
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
        Err(Error::UnexpectedResponse(_)) => ProcessingResult::Retry,
        Err(err) => ProcessingResult::Error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ResponseMessage;

    #[test]
    fn test_should_retry_error() {
        let test_msg = ResponseMessage::from_simple("test");
        assert!(should_retry_error(&Error::UnexpectedResponse(test_msg)));
        assert!(!should_retry_error(&Error::EndOfStream));
        assert!(!should_retry_error(&Error::ConnectionFailed));
    }

    #[test]
    fn test_is_stream_end() {
        let test_msg = ResponseMessage::from_simple("test");
        assert!(is_stream_end(&Error::EndOfStream));
        assert!(!is_stream_end(&Error::UnexpectedResponse(test_msg)));
        assert!(!is_stream_end(&Error::ConnectionFailed));
    }

    #[test]
    fn test_should_store_error() {
        let test_msg = ResponseMessage::from_simple("test");
        assert!(!should_store_error(&Error::EndOfStream));
        assert!(should_store_error(&Error::UnexpectedResponse(test_msg)));
        assert!(should_store_error(&Error::ConnectionFailed));
    }

    #[test]
    fn test_check_retry() {
        // Should continue when under max retries
        assert_eq!(check_retry(0), RetryDecision::Continue);
        assert_eq!(check_retry(5), RetryDecision::Continue);
        assert_eq!(check_retry(MAX_DECODE_RETRIES - 1), RetryDecision::Continue);

        // Should stop when at or over max retries
        assert_eq!(check_retry(MAX_DECODE_RETRIES), RetryDecision::Stop);
        assert_eq!(check_retry(MAX_DECODE_RETRIES + 1), RetryDecision::Stop);
    }

    #[test]
    fn test_process_decode_result() {
        // Test success case
        match process_decode_result::<i32>(Ok(42)) {
            ProcessingResult::Success(val) => assert_eq!(val, 42),
            _ => panic!("Expected Success"),
        }

        // Test EndOfStream
        match process_decode_result::<i32>(Err(Error::EndOfStream)) {
            ProcessingResult::EndOfStream => {}
            _ => panic!("Expected EndOfStream"),
        }

        // Test retry case
        let test_msg = ResponseMessage::from_simple("test");
        match process_decode_result::<i32>(Err(Error::UnexpectedResponse(test_msg))) {
            ProcessingResult::Retry => {}
            _ => panic!("Expected Retry"),
        }

        // Test error case
        match process_decode_result::<i32>(Err(Error::ConnectionFailed)) {
            ProcessingResult::Error(Error::ConnectionFailed) => {}
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_response_context_default() {
        let context = ResponseContext::default();
        assert!(!context.is_smart_depth);
        assert!(context.request_type.is_none());
    }

    #[test]
    fn test_response_context_clone() {
        let context = ResponseContext {
            is_smart_depth: true,
            request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
        };

        let cloned = context.clone();
        assert_eq!(context, cloned);
        assert_eq!(cloned.is_smart_depth, true);
        assert_eq!(cloned.request_type, Some(crate::messages::OutgoingMessages::RequestMarketData));
    }

    #[test]
    fn test_response_context_equality() {
        struct TestCase {
            name: &'static str,
            context1: ResponseContext,
            context2: ResponseContext,
            expected: bool,
        }

        let test_cases = vec![
            TestCase {
                name: "default_contexts_equal",
                context1: ResponseContext::default(),
                context2: ResponseContext::default(),
                expected: true,
            },
            TestCase {
                name: "same_values_equal",
                context1: ResponseContext {
                    is_smart_depth: true,
                    request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
                },
                context2: ResponseContext {
                    is_smart_depth: true,
                    request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
                },
                expected: true,
            },
            TestCase {
                name: "different_smart_depth",
                context1: ResponseContext {
                    is_smart_depth: true,
                    request_type: None,
                },
                context2: ResponseContext {
                    is_smart_depth: false,
                    request_type: None,
                },
                expected: false,
            },
            TestCase {
                name: "different_request_type",
                context1: ResponseContext {
                    is_smart_depth: false,
                    request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
                },
                context2: ResponseContext {
                    is_smart_depth: false,
                    request_type: Some(crate::messages::OutgoingMessages::CancelMarketData),
                },
                expected: false,
            },
            TestCase {
                name: "one_none_one_some",
                context1: ResponseContext {
                    is_smart_depth: false,
                    request_type: None,
                },
                context2: ResponseContext {
                    is_smart_depth: false,
                    request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
                },
                expected: false,
            },
        ];

        for tc in test_cases {
            assert_eq!(tc.context1 == tc.context2, tc.expected, "test case '{}' failed", tc.name);
        }
    }

    #[test]
    fn test_response_context_debug_format() {
        let context = ResponseContext {
            is_smart_depth: true,
            request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
        };

        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("ResponseContext"));
        assert!(debug_str.contains("is_smart_depth"));
        assert!(debug_str.contains("true"));
        assert!(debug_str.contains("request_type"));
        assert!(debug_str.contains("Some"));
    }
}

/// Context information for response handling
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseContext {
    /// Type of the original request that initiated this subscription
    pub request_type: Option<OutgoingMessages>,
    /// Whether this is a smart depth subscription
    pub is_smart_depth: bool,
}

/// Common trait for decoding streaming data responses
///
/// This trait is shared between sync and async implementations to avoid code duplication.
/// The key change from the original design is that `decode` takes `server_version` directly
/// instead of the entire `Client`, making it possible to share implementations.
pub(crate) trait StreamDecoder<T> {
    /// Message types this stream can handle
    #[allow(dead_code)]
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[];

    /// Decode a response message into the stream's data type
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<T, Error>;

    /// Generate a cancellation message for this stream
    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        Err(Error::NotImplemented)
    }

    /// Returns true if this decoded value represents the end of a snapshot subscription
    #[allow(unused)]
    fn is_snapshot_end(&self) -> bool {
        false
    }
}
