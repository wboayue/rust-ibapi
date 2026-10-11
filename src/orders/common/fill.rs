//! Shared state of one `wait_for_fill` (#951), for the sync and async clients.

use crate::orders::{OrderOutcome, OrderStatus};

/// The last status seen, and whether a status ends the wait.
#[derive(Debug, Default)]
pub(crate) struct FillTracker {
    last: Option<OrderStatus>,
}

impl FillTracker {
    /// Take one status; `Some` ends the wait ([`OrderStatus::outcome`]).
    pub(crate) fn observe(&mut self, status: OrderStatus) -> Option<OrderOutcome> {
        let outcome = status.outcome();
        if outcome.is_none() {
            self.last = Some(status);
        }
        outcome
    }

    /// The outcome when the time runs out.
    pub(crate) fn timed_out(self) -> OrderOutcome {
        OrderOutcome::TimedOut(self.last)
    }
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod tests;
