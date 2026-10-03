//! ID generation for requests and orders
//!
//! This module provides thread-safe ID generation for request IDs and order IDs.
//! Request IDs are used to track API requests, while order IDs are used for order placement.

use std::sync::atomic::{AtomicI32, Ordering};

use super::ids::{OrderId, RequestId, REQUEST_ID_CEILING, REQUEST_ID_FLOOR};
use crate::Error;

/// Thread-safe ID generator using atomic operations
#[derive(Debug)]
pub(crate) struct IdGenerator {
    next_id: AtomicI32,
}

impl IdGenerator {
    /// Creates a new ID generator with the specified starting value
    pub(crate) fn new(start: i32) -> Self {
        Self {
            next_id: AtomicI32::new(start),
        }
    }

    /// Creates a new ID generator for request IDs (starts at [`REQUEST_ID_FLOOR`])
    pub(crate) fn new_request_id_generator() -> Self {
        Self::new(REQUEST_ID_FLOOR)
    }

    /// Creates a new ID generator for order IDs with the server-provided starting value
    pub(crate) fn new_order_id_generator(start: i32) -> Self {
        Self::new(start)
    }

    /// Gets the next ID, incrementing the internal counter
    pub(crate) fn next(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Like [`next`](Self::next), but `None` once the next ID would exceed
    /// `max`; the counter then stays put.
    pub(crate) fn next_up_to(&self, max: i32) -> Option<i32> {
        self.next_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| (id <= max).then(|| id + 1))
            .ok()
    }

    /// Gets the current ID without incrementing
    #[allow(dead_code)]
    pub(crate) fn current(&self) -> i32 {
        self.next_id.load(Ordering::Relaxed)
    }

    /// Raises the next ID value to at least `value`; never lowers it.
    ///
    /// Server responses are lower bounds, not overwrites: IDs already allocated
    /// locally — including IDs whose request has not reached the server yet —
    /// stay reserved, so a stale or racing server value can never make `next`
    /// reissue an ID.
    pub(crate) fn raise(&self, value: i32) {
        self.next_id.fetch_max(value, Ordering::Relaxed);
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Manages both request and order ID generation for a client
#[derive(Debug)]
pub(crate) struct ClientIdManager {
    request_ids: IdGenerator,
    order_ids: IdGenerator,
}

impl ClientIdManager {
    /// Creates a new ID manager with the initial order ID from the server.
    /// A seed in the request range is refused: every order the session
    /// placed would collide with request IDs.
    pub(crate) fn new(initial_order_id: i32) -> Result<Self, Error> {
        if OrderId::from(initial_order_id).checked().is_err() {
            return Err(Error::ConnectionRejected(format!(
                "server's next valid order id {initial_order_id} is at or above {REQUEST_ID_FLOOR}, which is reserved for request ids"
            )));
        }
        Ok(Self {
            request_ids: IdGenerator::new_request_id_generator(),
            order_ids: IdGenerator::new_order_id_generator(initial_order_id),
        })
    }

    /// Gets the next request ID.
    ///
    /// # Panics
    ///
    /// Past [`REQUEST_ID_CEILING`]: some 647M requests in one process is a
    /// bug, and reusing an ID would misroute its responses.
    pub(crate) fn next_request_id(&self) -> RequestId {
        self.request_ids
            .next_up_to(REQUEST_ID_CEILING)
            .and_then(RequestId::from_raw)
            .unwrap_or_else(|| {
                log::error!("request ids exhausted at {REQUEST_ID_CEILING}");
                panic!("request ids exhausted at {REQUEST_ID_CEILING}")
            })
    }

    /// Gets the next order ID
    pub(crate) fn next_order_id(&self) -> OrderId {
        OrderId::from(self.order_ids.next())
    }

    /// Raises the order ID to at least the given value (e.g., from the server's
    /// next valid ID response); never lowers it below locally allocated IDs.
    pub(crate) fn raise_order_id(&self, order_id: OrderId) {
        self.order_ids.raise(order_id.value());
    }

    /// Raises the order ID from a reconnect handshake's next valid ID. A value
    /// in the request range is still applied — the server will not accept
    /// lower ids — but logged, since every order placed from it will be
    /// rejected ([`OrderId::checked`]).
    pub(crate) fn raise_order_id_from_server(&self, next_valid_id: i32) {
        let order_id = OrderId::from(next_valid_id);
        if order_id.checked().is_err() {
            log::error!("server's next valid order id {next_valid_id} is at or above {REQUEST_ID_FLOOR}, which is reserved for request ids; orders will be rejected");
        }
        self.raise_order_id(order_id);
    }

    /// Gets the current order ID without incrementing
    #[allow(dead_code)]
    pub(crate) fn current_order_id(&self) -> i32 {
        self.order_ids.current()
    }

    /// Gets the current request ID without incrementing
    #[allow(dead_code)]
    pub(crate) fn current_request_id(&self) -> i32 {
        self.request_ids.current()
    }
}

#[cfg(test)]
#[path = "id_generator_tests.rs"]
mod tests;
