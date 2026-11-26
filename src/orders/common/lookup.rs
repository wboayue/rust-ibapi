//! Finding one order among this client's open orders and the completed
//! orders, for `find_order`, `find_order_by_perm_id` and `wait_for_fill`.

use std::time::{Duration, Instant};

use crate::builder::ValidationError;
use crate::client::ids::OrderId;
use crate::orders::{OrderData, OrderLookup, Orders};
use crate::Error;

/// How often a lookup reads completed orders again while neither reply lists
/// the order, for the first [`LOOKUP_RECHECK_WINDOW`] of the lookup. A
/// finished order leaves open orders before it reaches completed orders:
/// live, a rejected order was in neither for 19-204 ms after its `Inactive`
/// status. At this interval the record is read about 100 ms after it appears
/// at most, and a lookup sends at most ten requests a second, well under
/// TWS's limit of 50 messages a second.
pub(crate) const LOOKUP_RECHECK_INTERVAL: Duration = Duration::from_millis(100);

/// How long a lookup re-reads completed orders at [`LOOKUP_RECHECK_INTERVAL`],
/// five times the longest gap seen live. An order still listed nowhere after
/// it is most likely unknown, and each completed-orders reply carries every
/// client's records (230 on the paper account probed), so a long lookup slows
/// to [`LOOKUP_RECHECK_INTERVAL_SLOW`].
pub(crate) const LOOKUP_RECHECK_WINDOW: Duration = Duration::from_secs(1);

/// How often a lookup reads completed orders again after
/// [`LOOKUP_RECHECK_WINDOW`].
pub(crate) const LOOKUP_RECHECK_INTERVAL_SLOW: Duration = Duration::from_secs(1);

/// The order a lookup is for.
#[derive(Clone, Copy, Debug)]
pub(crate) enum OrderKey {
    /// An API order id and the client that placed it: ids are unique only per client.
    Id { client_id: i32, order_id: OrderId },
    /// TWS's permanent id, unique across clients.
    PermId(i64),
}

impl OrderKey {
    /// The key for `perm_id`, checked where it enters the crate. TWS assigns
    /// positive perm IDs. A zero key would match records that carry no perm ID.
    pub(crate) fn perm_id(perm_id: i64) -> Result<Self, Error> {
        if perm_id <= 0 {
            return Err(Error::Validation(ValidationError::InvalidField {
                field: "perm_id",
                value: perm_id.to_string(),
                reason: "permanent order ID must be positive".into(),
            }));
        }
        Ok(OrderKey::PermId(perm_id))
    }

    /// [`OrderLookup::Working`] when an open-orders reply lists the order with
    /// a status that is not terminal. A finished order can still be listed for
    /// a few milliseconds with its final status (live: `Inactive`), which is
    /// not working. An id matches only a row of its client: an `all_open_orders`
    /// reply overlapping this one reaches the lookup too, and ids repeat across
    /// clients.
    pub(crate) fn working(&self, rows: Vec<Orders>) -> Option<OrderLookup> {
        let (mut order, mut status, mut last) = (None, None, None);
        for row in rows {
            match row {
                Orders::OrderData(data) if self.matches_open(data.order.client_id, data.order.order_id, data.order.perm_id) => {
                    last = Some(data.order_state.status.clone());
                    order = Some(data);
                }
                Orders::OrderStatus(row) if self.matches_open(row.client_id, row.order_id, row.perm_id) => {
                    last = Some(row.status.clone());
                    status = Some(row);
                }
                _ => {}
            }
        }
        last.filter(|kind| !kind.is_terminal()).map(|_| OrderLookup::Working { order, status })
    }

    /// [`OrderLookup::Finished`] when a completed-orders reply lists the
    /// order; the last match wins. An API id must be nonzero and belong to its client.
    pub(crate) fn finished(&self, rows: Vec<Orders>) -> Option<OrderLookup> {
        rows.into_iter().rev().find_map(|row| match row {
            Orders::OrderData(order) if self.matches_completed(&order) => {
                let status = order.completed_order_status();
                Some(OrderLookup::Finished { order, status })
            }
            _ => None,
        })
    }

    fn matches_open(&self, row_client_id: i32, order_id: i32, perm_id: i64) -> bool {
        match *self {
            OrderKey::Id { client_id, order_id: id } => row_client_id == client_id && order_id == id.value(),
            OrderKey::PermId(id) => perm_id == id,
        }
    }

    fn matches_completed(&self, data: &OrderData) -> bool {
        match *self {
            OrderKey::Id { client_id, order_id } => {
                order_id.value() != 0 && data.order.client_id == client_id && data.order.order_id == order_id.value()
            }
            OrderKey::PermId(id) => data.order.perm_id == id,
        }
    }
}

/// How long to wait at `now` before reading completed orders again for a
/// lookup that began at `start`, or `None` once `deadline` has passed. The
/// interval is [`LOOKUP_RECHECK_INTERVAL`] during [`LOOKUP_RECHECK_WINDOW`],
/// then [`LOOKUP_RECHECK_INTERVAL_SLOW`]. The last wait ends at the deadline,
/// so a bound shorter than the interval still gets a second read. No deadline
/// (a bound too large to represent) re-checks for good.
pub(crate) fn recheck_delay(deadline: Option<Instant>, start: Instant, now: Instant) -> Option<Duration> {
    let interval = if now.saturating_duration_since(start) < LOOKUP_RECHECK_WINDOW {
        LOOKUP_RECHECK_INTERVAL
    } else {
        LOOKUP_RECHECK_INTERVAL_SLOW
    };
    match deadline {
        Some(deadline) if now >= deadline => None,
        Some(deadline) => Some(interval.min(deadline - now)),
        None => Some(interval),
    }
}

#[cfg(test)]
#[path = "lookup_tests.rs"]
mod tests;
