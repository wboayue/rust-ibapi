//! Client state set by the connection handshake, shared with the transport so
//! a reconnect's handshake refreshes it.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use super::id_generator::ClientIdManager;
use crate::connection::ConnectionMetadata;

/// The server version negotiated with the current session. Starts at the
/// first connect's; a successful reconnect stores the new handshake's. Keeps
/// the last good value while a reconnect is in progress.
#[derive(Debug)]
pub(crate) struct ServerVersion(AtomicI32);

impl ServerVersion {
    pub(crate) fn new(version: i32) -> Self {
        Self(AtomicI32::new(version))
    }

    pub(crate) fn get(&self) -> i32 {
        self.0.load(Ordering::Acquire)
    }

    fn set(&self, version: i32) {
        self.0.store(version, Ordering::Release);
    }
}

/// Client state the transport raises from what TWS sends: the order-id floor
/// and the server version, refreshed by a reconnect's handshake, and the
/// order-id floor again when TWS lists another client's order. The client
/// hands it to the transport once, before the transport starts reading.
#[derive(Debug)]
pub(crate) struct HandshakeState {
    pub(crate) order_ids: Arc<ClientIdManager>,
    pub(crate) server_version: Arc<ServerVersion>,
}

impl HandshakeState {
    /// Applies a reconnect's handshake. The transport calls it before
    /// reporting the session as live, so a caller gating on `is_connected()`
    /// can't allocate below the new order-id floor or check a request against
    /// the old server version.
    pub(crate) fn refresh(&self, metadata: &ConnectionMetadata) {
        // Only the first connect seeded the generator; raise it from the
        // server's floor so allocation never resumes below it.
        self.order_ids.raise_order_id_from_server(metadata.next_order_id);
        self.server_version.set(metadata.server_version);
    }

    /// Raises the order-id floor past `order_id`, an order TWS listed for
    /// another client. Listing all clients' open orders makes TWS raise this
    /// client's next valid order id above the highest order id listed, and
    /// refuse lower ids with 103 "Duplicate order id", without sending the
    /// new id (`docs/wire-facts/orders.md`).
    pub(crate) fn raise_past_listed_order(&self, order_id: i32) {
        self.order_ids.raise_order_id_from_server(order_id.saturating_add(1));
    }
}

#[cfg(test)]
#[path = "handshake_state_tests.rs"]
mod tests;
