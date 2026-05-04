//! Common utilities shared between sync and async transport implementations

use std::time::Duration;

use log::{error, warn};

use super::routing::is_warning_error;

/// Log an Error/Warning message with the same field set across sync and async transports.
/// Warnings (codes in `WARNING_CODE_RANGE`) log at warn level; everything else at error level.
pub(crate) fn log_error_fields(request_id: i32, error_code: i32, error_message: &str, advanced_order_reject_json: &str, error_time: i64) {
    if is_warning_error(error_code) {
        warn!(
            "request_id: {request_id}, warning_code: {error_code}, warning_message: {error_message}, advanced_order_reject_json: {advanced_order_reject_json}, error_time: {error_time}"
        );
    } else {
        error!(
            "request_id: {request_id}, error_code: {error_code}, error_message: {error_message}, advanced_order_reject_json: {advanced_order_reject_json}, error_time: {error_time}"
        );
    }
}

/// Maximum number of reconnection attempts
pub(crate) const MAX_RECONNECT_ATTEMPTS: i32 = 20;

/// Fibonacci backoff for reconnection attempts
pub(crate) struct FibonacciBackoff {
    previous: u64,
    current: u64,
    max: u64,
}

impl FibonacciBackoff {
    pub(crate) fn new(max: u64) -> Self {
        FibonacciBackoff {
            previous: 0,
            current: 1,
            max,
        }
    }

    pub(crate) fn next_delay(&mut self) -> Duration {
        let next = self.previous + self.current;
        self.previous = self.current;
        self.current = next;

        if next > self.max {
            Duration::from_secs(self.max)
        } else {
            Duration::from_secs(next)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_backoff() {
        let mut backoff = FibonacciBackoff::new(10);

        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(3));
        assert_eq!(backoff.next_delay(), Duration::from_secs(5));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(10)); // capped at max
        assert_eq!(backoff.next_delay(), Duration::from_secs(10)); // stays at max
    }
}
