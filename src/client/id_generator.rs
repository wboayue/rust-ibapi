//! ID generation for requests and orders
//!
//! This module provides thread-safe ID generation for request IDs and order IDs.
//! Request IDs are used to track API requests, while order IDs are used for order placement.

use std::sync::atomic::{AtomicI32, Ordering};

/// Starting value for request IDs
const INITIAL_REQUEST_ID: i32 = 9000;

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

    /// Creates a new ID generator for request IDs (starts at 9000)
    pub(crate) fn new_request_id_generator() -> Self {
        Self::new(INITIAL_REQUEST_ID)
    }

    /// Creates a new ID generator for order IDs with the server-provided starting value
    pub(crate) fn new_order_id_generator(start: i32) -> Self {
        Self::new(start)
    }

    /// Gets the next ID, incrementing the internal counter
    pub(crate) fn next(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Gets the current ID without incrementing
    #[allow(dead_code)]
    pub(crate) fn current(&self) -> i32 {
        self.next_id.load(Ordering::Relaxed)
    }

    /// Sets the next ID value (useful for order ID updates from server)
    pub(crate) fn set(&self, value: i32) {
        self.next_id.store(value, Ordering::Relaxed);
    }

    /// Resets the generator to a new starting value
    #[allow(dead_code)]
    pub(crate) fn reset(&self, start: i32) {
        self.set(start);
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
    /// Creates a new ID manager with the initial order ID from the server
    pub(crate) fn new(initial_order_id: i32) -> Self {
        Self {
            request_ids: IdGenerator::new_request_id_generator(),
            order_ids: IdGenerator::new_order_id_generator(initial_order_id),
        }
    }

    /// Gets the next request ID
    pub(crate) fn next_request_id(&self) -> i32 {
        self.request_ids.next()
    }

    /// Gets the next order ID
    pub(crate) fn next_order_id(&self) -> i32 {
        self.order_ids.next()
    }

    /// Updates the order ID (e.g., from server's next valid ID response)
    pub(crate) fn set_order_id(&self, order_id: i32) {
        self.order_ids.set(order_id);
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
