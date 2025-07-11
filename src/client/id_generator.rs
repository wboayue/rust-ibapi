//! ID generation for requests and orders
//!
//! This module provides thread-safe ID generation for request IDs and order IDs.
//! Request IDs are used to track API requests, while order IDs are used for order placement.

use std::sync::atomic::{AtomicI32, Ordering};

/// Starting value for request IDs
const INITIAL_REQUEST_ID: i32 = 9000;

/// Thread-safe ID generator using atomic operations
#[derive(Debug)]
pub struct IdGenerator {
    next_id: AtomicI32,
}

impl IdGenerator {
    /// Creates a new ID generator with the specified starting value
    pub fn new(start: i32) -> Self {
        Self {
            next_id: AtomicI32::new(start),
        }
    }

    /// Creates a new ID generator for request IDs (starts at 9000)
    pub fn new_request_id_generator() -> Self {
        Self::new(INITIAL_REQUEST_ID)
    }

    /// Creates a new ID generator for order IDs with the server-provided starting value
    pub fn new_order_id_generator(start: i32) -> Self {
        Self::new(start)
    }

    /// Gets the next ID, incrementing the internal counter
    pub fn next(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Gets the current ID without incrementing
    #[allow(dead_code)]
    pub fn current(&self) -> i32 {
        self.next_id.load(Ordering::Relaxed)
    }

    /// Sets the next ID value (useful for order ID updates from server)
    pub fn set(&self, value: i32) {
        self.next_id.store(value, Ordering::Relaxed);
    }

    /// Resets the generator to a new starting value
    #[allow(dead_code)]
    pub fn reset(&self, start: i32) {
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
pub struct ClientIdManager {
    request_ids: IdGenerator,
    order_ids: IdGenerator,
}

impl ClientIdManager {
    /// Creates a new ID manager with the initial order ID from the server
    pub fn new(initial_order_id: i32) -> Self {
        Self {
            request_ids: IdGenerator::new_request_id_generator(),
            order_ids: IdGenerator::new_order_id_generator(initial_order_id),
        }
    }

    /// Gets the next request ID
    pub fn next_request_id(&self) -> i32 {
        self.request_ids.next()
    }

    /// Gets the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.order_ids.next()
    }

    /// Updates the order ID (e.g., from server's next valid ID response)
    pub fn set_order_id(&self, order_id: i32) {
        self.order_ids.set(order_id);
    }

    /// Gets the current order ID without incrementing
    #[allow(dead_code)]
    pub fn current_order_id(&self) -> i32 {
        self.order_ids.current()
    }

    /// Gets the current request ID without incrementing
    #[allow(dead_code)]
    pub fn current_request_id(&self) -> i32 {
        self.request_ids.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_id_generator_basic() {
        let gen = IdGenerator::new(100);
        assert_eq!(gen.current(), 100);
        assert_eq!(gen.next(), 100);
        assert_eq!(gen.next(), 101);
        assert_eq!(gen.next(), 102);
        assert_eq!(gen.current(), 103);
    }

    #[test]
    fn test_id_generator_set() {
        let gen = IdGenerator::new(100);
        assert_eq!(gen.next(), 100);
        gen.set(200);
        assert_eq!(gen.next(), 200);
        assert_eq!(gen.next(), 201);
    }

    #[test]
    fn test_id_generator_thread_safe() {
        let gen = Arc::new(IdGenerator::new(0));
        let mut handles = vec![];

        // Spawn 10 threads, each getting 100 IDs
        for _ in 0..10 {
            let gen_clone = Arc::clone(&gen);
            let handle = thread::spawn(move || {
                let mut ids = vec![];
                for _ in 0..100 {
                    ids.push(gen_clone.next());
                }
                ids
            });
            handles.push(handle);
        }

        // Collect all IDs
        let mut all_ids = vec![];
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        // Check that we have 1000 unique IDs from 0 to 999
        all_ids.sort();
        assert_eq!(all_ids.len(), 1000);
        for (i, id) in all_ids.iter().enumerate() {
            assert_eq!(*id, i as i32);
        }
    }

    #[test]
    fn test_request_id_generator() {
        let gen = IdGenerator::new_request_id_generator();
        assert_eq!(gen.current(), INITIAL_REQUEST_ID);
        assert_eq!(gen.next(), INITIAL_REQUEST_ID);
        assert_eq!(gen.next(), INITIAL_REQUEST_ID + 1);
    }

    #[test]
    fn test_client_id_manager() {
        let manager = ClientIdManager::new(50);

        // Test request IDs
        assert_eq!(manager.current_request_id(), INITIAL_REQUEST_ID);
        assert_eq!(manager.next_request_id(), INITIAL_REQUEST_ID);
        assert_eq!(manager.next_request_id(), INITIAL_REQUEST_ID + 1);

        // Test order IDs
        assert_eq!(manager.current_order_id(), 50);
        assert_eq!(manager.next_order_id(), 50);
        assert_eq!(manager.next_order_id(), 51);

        // Test order ID update
        manager.set_order_id(100);
        assert_eq!(manager.next_order_id(), 100);
        assert_eq!(manager.next_order_id(), 101);
    }
}
