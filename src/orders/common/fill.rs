//! Shared state of one `wait_for_fill` (#951), for the sync and async clients.

use super::decoders::decode_order_status;
use crate::messages::ResponseMessage;
use crate::orders::{OrderOutcome, OrderStatus, OrderStatusKind};
use crate::transport::RoutedItem;
use crate::Error;

/// The last status seen, and whether a status ends the wait.
#[derive(Debug, Default)]
pub(crate) struct FillTracker {
    last: Option<OrderStatus>,
}

impl FillTracker {
    /// Take one item from the waiter's channel. `Some` ends the wait; an
    /// error (connection reset, shutdown) ends it as `Err`.
    pub(crate) fn on_item(&mut self, item: RoutedItem) -> Result<Option<OrderOutcome>, Error> {
        match item {
            RoutedItem::Response(message) => self.observe(&message),
            RoutedItem::Notice(_) => Ok(None),
            RoutedItem::Error(error) => Err(error),
        }
    }

    /// Take one `OrderStatus` frame. Partial fills keep the wait going.
    fn observe(&mut self, message: &ResponseMessage) -> Result<Option<OrderOutcome>, Error> {
        let status = decode_order_status(message)?;
        if is_filled(&status) {
            return Ok(Some(OrderOutcome::Filled(status)));
        }
        if status.status.is_terminal() {
            return Ok(Some(OrderOutcome::Ended(status)));
        }
        self.last = Some(status);
        Ok(None)
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
