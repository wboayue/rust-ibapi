//! Shared state of one `wait_for_fill` (#951), for the sync and async clients.

use std::time::{Duration, Instant};

use crate::client::ids::OrderId;
use crate::orders::{OrderData, OrderOutcome, OrderStatus, OrderStatusKind, Orders};
use crate::Error;

/// The least time `wait_for_fill` gives each lookup reply (open orders, then
/// completed orders) to reach its end marker, even past the deadline. TWS
/// sends it milliseconds after the orders; read through it, a later request
/// of the same kind cannot end on this one's marker.
pub(crate) const LOOKUP_MIN_WAIT: Duration = Duration::from_secs(1);

/// How long to read a lookup reply for, waiting until `deadline` but at least
/// [`LOOKUP_MIN_WAIT`].
pub(crate) fn lookup_wait(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now()).max(LOOKUP_MIN_WAIT)
}

/// Whether an open-orders reply lists `order_id`, so the order is working.
pub(crate) fn lists_order(rows: &[Orders], order_id: OrderId) -> bool {
    rows.iter().any(|row| match row {
        Orders::OrderData(data) => data.order.order_id == order_id.value(),
        Orders::OrderStatus(status) => status.order_id == order_id.value(),
    })
}

/// The status of `order_id`, placed by `client_id`, in a completed-orders
/// reply; the last match wins. Built from the order, which has no fill
/// prices: they are `None`. A completed order's `total_quantity` is what was
/// left unfilled (live: `0` once filled, `400` for 1000 cancelled after 600
/// filled), so it is the remaining quantity as is (#1116).
pub(crate) fn completed_status(rows: &[Orders], client_id: i32, order_id: OrderId) -> Option<OrderStatus> {
    rows.iter().rev().find_map(|row| match row {
        Orders::OrderData(data) if data.order.client_id == client_id && data.order.order_id == order_id.value() => Some(status_of_completed(data)),
        _ => None,
    })
}

fn status_of_completed(data: &OrderData) -> OrderStatus {
    let order = &data.order;
    OrderStatus {
        order_id: order.order_id,
        status: data.order_state.status.clone(),
        filled: order.filled_quantity,
        remaining: order.total_quantity,
        average_fill_price: None,
        perm_id: order.perm_id,
        parent_id: order.parent_id,
        last_fill_price: None,
        client_id: order.client_id,
        why_held: String::new(),
        market_cap_price: None,
    }
}

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

    /// Take the status built from a completed-order record ([`completed_status`]).
    /// Its status is final, so the status alone decides: unlike a live status,
    /// nothing remaining after a fill doesn't mean filled here, since a
    /// `totalQuantity` TWS left out decodes as `0`.
    pub(crate) fn observe_completed(&mut self, status: OrderStatus) -> Option<OrderOutcome> {
        if status.status == OrderStatusKind::Filled {
            Some(OrderOutcome::Filled(status))
        } else if status.status.is_terminal() {
            Some(OrderOutcome::Ended(status))
        } else {
            self.last = Some(status);
            None
        }
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
