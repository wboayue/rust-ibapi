//! Common utilities shared between sync and async transport implementations

use std::time::Duration;

use log::{error, warn};

use super::routing::Severity;
use crate::messages::Notice;

/// Log an unrouted notice (no subscription owner) at the appropriate severity.
/// Single source of truth for the unrouted log-line format — both sync and
/// async transports' `log_unrouted` methods delegate here. PR 5 adds the
/// global-notice broadcast call in the per-transport wrapper, not here.
pub(crate) fn log_unrouted_notice(severity: Severity, notice: &Notice) {
    match severity {
        Severity::Warning => warn!("warning: {notice}"),
        Severity::HardError => error!("error: {notice}"),
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
