//! Finding one order among this client's open orders and the completed
//! orders, for `find_order`, `find_order_by_perm_id` and `wait_for_fill`.

use std::time::{Duration, Instant};

use crate::builder::ValidationError;
use crate::client::ids::OrderId;
use crate::orders::{Order, OrderData, OrderLookup, Orders};
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

/// Identifies an order by a scoped API ID or a permanent ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderKey {
    /// An API order id and the client that placed it: ids are unique only per client.
    Id {
        /// API client that owns the order ID.
        client_id: i32,
        /// Order ID in that client's namespace.
        order_id: OrderId,
    },
    /// TWS's positive permanent ID, unique across clients. Zero and negative values never match.
    PermId(i64),
}

impl OrderKey {
    /// Creates a key for a positive TWS permanent ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `perm_id` is zero or negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::OrderKey;
    ///
    /// assert_eq!(OrderKey::perm_id(913)?, OrderKey::PermId(913));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn perm_id(perm_id: i64) -> Result<Self, Error> {
        if perm_id <= 0 {
            return Err(Error::Validation(ValidationError::InvalidField {
                field: "perm_id",
                value: perm_id.to_string(),
                reason: "permanent order ID must be positive".into(),
            }));
        }
        Ok(OrderKey::PermId(perm_id))
    }

    /// Matches a received order's identity without using its account or reference.
    ///
    /// For completed rows, use [`OrderEvidence::lookup`]. TWS can leave the API order ID
    /// and client ID unset when it completes an order.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Order, OrderKey};
    ///
    /// let order = Order { client_id: 7, order_id: 13, ..Order::default() };
    /// assert!(OrderKey::Id { client_id: 7, order_id: 13.into() }.matches_order(&order));
    /// assert!(!OrderKey::Id { client_id: 8, order_id: 13.into() }.matches_order(&order));
    /// ```
    pub fn matches_order(self, order: &Order) -> bool {
        self.matches_open(order.client_id, order.order_id, order.perm_id)
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
                Orders::OrderData(data) if self.matches_order(&data.order) => {
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
            OrderKey::PermId(id) => id > 0 && perm_id == id,
        }
    }

    fn matches_completed(&self, data: &OrderData) -> bool {
        !matches!(*self, OrderKey::Id { order_id, .. } if order_id.value() == 0) && self.matches_order(&data.order)
    }
}

/// Completeness of a collected open- or completed-order reply.
#[derive(Clone, Copy, Debug)]
pub enum OrderRows<'a> {
    /// Order details received through the reply's end marker, in receipt order.
    Complete(&'a [OrderData]),
    /// No complete reply, including after a timeout, disconnect, lag, or failed request.
    Unavailable,
}

/// Borrowed open- and completed-order evidence for finding one or more orders.
#[derive(Clone, Copy, Debug)]
pub struct OrderEvidence<'a> {
    /// Rows from an open-orders reply.
    pub open: OrderRows<'a>,
    /// Rows from a completed-orders reply.
    pub completed: OrderRows<'a>,
}

/// The result of looking up an order in the available replies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrderEvidenceMatch<'a> {
    /// The last matching open-order row has a nonterminal status.
    Working(&'a OrderData),
    /// The last matching completed-order row has a terminal status.
    Finished(&'a OrderData),
    /// Both complete replies omit the requested order's details.
    Absent,
    /// A required reply is unavailable, or a matching row cannot establish a working or completed order.
    Incomplete,
    /// The scoped API ID has been associated with different positive permanent IDs in the replies.
    Ambiguous,
}

impl<'a> OrderEvidence<'a> {
    /// Looks up one order without consuming the rows or sending a request.
    ///
    /// Pass an `account` to require an exact account code on matching rows. API IDs are
    /// scoped by client. An API ID of zero cannot match a completed row. A permanent ID
    /// can match a completed row with missing API routing IDs.
    ///
    /// A working row in a complete open reply can establish a match without a completed
    /// reply. If both replies are available, a conflicting positive permanent ID makes
    /// the lookup ambiguous. Otherwise the working row takes precedence. Terminal open
    /// rows require a completed row before this lookup reports `Finished`.
    ///
    /// The lookup uses order-detail rows. It does not read separate status or execution rows.
    /// An order can leave open orders before appearing in completed orders. For fill or delivery
    /// decisions reconcile additional evidence.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{OrderData, OrderEvidence, OrderEvidenceMatch, OrderKey, OrderRows};
    ///
    /// let mut data = OrderData::default();
    /// data.order.client_id = 7;
    /// data.order.order_id = 13;
    /// let open = [data];
    /// let evidence = OrderEvidence { open: OrderRows::Complete(&open), completed: OrderRows::Unavailable };
    /// let key = OrderKey::Id { client_id: 7, order_id: 13.into() };
    /// assert_eq!(evidence.lookup(key, None), OrderEvidenceMatch::Working(&open[0]));
    /// ```
    pub fn lookup(self, key: OrderKey, account: Option<&str>) -> OrderEvidenceMatch<'a> {
        use OrderEvidenceMatch::{Absent, Ambiguous, Finished, Incomplete, Working};

        let OrderRows::Complete(open) = self.open else { return Incomplete };
        let mut perm = None;
        let Ok(open) = last_match(open.iter().filter(|data| key.matches_order(&data.order)), account, &mut perm) else {
            return Ambiguous;
        };
        let completed = match self.completed {
            OrderRows::Complete(rows) => {
                let Ok(completed) = last_match(rows.iter().filter(|data| key.matches_completed(data)), account, &mut perm) else {
                    return Ambiguous;
                };
                completed
            }
            OrderRows::Unavailable => None,
        };
        match (open, completed) {
            (Some(row), _) if !row.order_state.status.is_terminal() => Working(row),
            (_, Some(row)) if row.order_state.status.is_terminal() => Finished(row),
            (None, None) if matches!(self.completed, OrderRows::Complete(_)) => Absent,
            _ => Incomplete,
        }
    }
}

fn last_match<'a>(rows: impl Iterator<Item = &'a OrderData>, account: Option<&str>, perm: &mut Option<i64>) -> Result<Option<&'a OrderData>, ()> {
    let mut latest = None;
    for data in rows.filter(|row| account.is_none_or(|account| row.order.account == account)) {
        let id = data.order.perm_id;
        if id > 0 {
            if perm.is_some_and(|known| known != id) {
                return Err(());
            }
            *perm = Some(id);
        }
        latest = Some(data);
    }
    Ok(latest)
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
