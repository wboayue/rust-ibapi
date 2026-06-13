//! Common utilities shared between sync and async transport implementations

use std::time::Duration;

use log::{error, info, warn};

use crate::messages::Notice;
use crate::subscriptions::common::RoutedItem;

/// Codes within `WARNING_CODE_RANGE` that report healthy data-farm
/// connectivity (e.g. "Market data farm connection is OK") rather than a
/// problem. IB's message-codes reference classifies these as System
/// Notifications, not warnings, so they're logged at info instead of warn.
const BENIGN_CONNECTIVITY_CODES: [i32; 3] = [
    2104, // Market data farm connection is OK
    2106, // HMDS data farm connection is OK
    2158, // Sec-def data farm connection is OK
];

fn is_benign_connectivity_notice(code: i32) -> bool {
    BENIGN_CONNECTIVITY_CODES.contains(&code)
}

/// Log an unrouted notice (no subscription owner) at the appropriate severity.
pub(crate) fn log_unrouted_notice(notice: &Notice) {
    if is_benign_connectivity_notice(notice.code) {
        info!("{notice}");
    } else if notice.is_warning() {
        warn!("warning: {notice}");
    } else {
        error!("error: {notice}");
    }
}

/// Log a routed notice/error that arrived bound to an id with no matching
/// request or order channel. The dispatcher only constructs `Notice` and
/// `Error` variants for this path; `Response` is unreachable here.
pub(crate) fn log_orphan(request_id: i32, item: &RoutedItem) {
    match item {
        RoutedItem::Notice(n) => info!("no recipient for notice (id={request_id}): {n}"),
        RoutedItem::Error(e) => info!("no recipient for error (id={request_id}): {e}"),
        RoutedItem::Response(_) => {}
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
#[path = "common_tests.rs"]
mod tests;
