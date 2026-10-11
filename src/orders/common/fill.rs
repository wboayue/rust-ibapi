//! Shared state of one `wait_for_fill` (#951), for the sync and async clients.

use crate::orders::{OrderOutcome, OrderStatus, OrderStatusKind};

/// The last status seen, and whether a status ends the wait.
#[derive(Debug, Default)]
pub(crate) struct FillTracker {
    last: Option<OrderStatus>,
}

impl FillTracker {
    /// Take one status; `Some` ends the wait. Partial fills keep it going.
    pub(crate) fn observe(&mut self, status: OrderStatus) -> Option<OrderOutcome> {
        if is_filled(&status) {
            return Some(OrderOutcome::Filled(status));
        }
        if status.status.is_terminal() {
            return Some(OrderOutcome::Ended(status));
        }
        self.last = Some(status);
        None
    }

    /// The outcome when the time runs out.
    pub(crate) fn timed_out(self) -> OrderOutcome {
        OrderOutcome::TimedOut(self.last)
    }
}

/// Market orders do not always report `Filled`; nothing left after a fill is
/// one. `filled > 0` keeps an early all-zero status from counting.
fn is_filled(status: &OrderStatus) -> bool {
    status.status == OrderStatusKind::Filled || (status.remaining == 0.0 && status.filled > 0.0)
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod tests;
