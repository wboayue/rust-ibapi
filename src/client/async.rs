//! Asynchronous client implementation

use std::sync::Arc;

use time::OffsetDateTime;
use time_tz::Tz;

use crate::transport::AsyncMessageBus;
use crate::Error;

use super::id_generator::ClientIdManager;

/// Asynchronous TWS API Client
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    pub(crate) message_bus: Arc<dyn AsyncMessageBus>,

    client_id: i32,              // ID of client.
    id_manager: ClientIdManager, // Manages request and order ID generation
}

impl Client {
    /// Establishes async connection to TWS or Gateway
    pub async fn connect(_address: &str, _client_id: i32) -> Result<Client, Error> {
        // TODO: Implement actual async connection
        Err(Error::Simple("Async client not yet implemented".to_string()))
    }

    /// Returns the server version
    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    /// Returns the connection time
    pub fn connection_time(&self) -> Option<OffsetDateTime> {
        self.connection_time
    }

    /// Returns the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id()
    }

    /// Returns the next request ID
    pub(crate) fn next_request_id(&self) -> i32 {
        self.id_manager.next_request_id()
    }
}
