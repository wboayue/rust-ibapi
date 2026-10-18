//! Shared state of one `wait_for_fill` (#951), for the sync and async clients.

use std::time::Duration;

use crate::orders::{OrderOutcome, OrderStatus};
use crate::Error;

/// How long `wait_for_fill` reads its open-orders snapshot for the end marker
/// before returning. TWS sends it milliseconds after the orders; read through
/// it, a later open-orders request cannot end on this one's marker.
pub(crate) const SNAPSHOT_END_WAIT: Duration = Duration::from_secs(1);

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

    /// The outcome for an error on the status stream: an order rejection
    /// ([`Error::Notice`]) ends the wait; anything else fails it.
    pub(crate) fn error(error: Error) -> Result<OrderOutcome, Error> {
        match error {
            Error::Notice(notice) => Ok(OrderOutcome::Rejected(notice)),
            error => Err(error),
        }
    }

    /// The outcome when the time runs out.
    pub(crate) fn timed_out(&mut self) -> OrderOutcome {
        OrderOutcome::TimedOut(self.last.take())
    }
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod tests;
