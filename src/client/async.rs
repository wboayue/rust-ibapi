//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;
use time_tz::Tz;

use crate::connection::{r#async::AsyncConnection, ConnectionMetadata};
use crate::transport::{r#async::AsyncTcpMessageBus, AsyncMessageBus};
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
    ///
    /// Connects to server using the given connection string
    ///
    /// # Arguments
    /// * `address`   - address of server. e.g. 127.0.0.1:4002
    /// * `client_id` - id of client. e.g. 100
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     println!("server_version: {}", client.server_version());
    ///     println!("connection_time: {:?}", client.connection_time());
    ///     println!("next_order_id: {}", client.next_order_id());
    /// }
    /// ```
    pub async fn connect(address: &str, client_id: i32) -> Result<Client, Error> {
        let connection = AsyncConnection::connect(address, client_id).await?;
        let connection_metadata = connection.connection_metadata();

        let message_bus = Arc::new(AsyncTcpMessageBus::new(connection)?);

        // Start background task to read messages from TWS
        message_bus
            .clone()
            .process_messages(connection_metadata.server_version, Duration::from_secs(1))?;

        Client::new(connection_metadata, message_bus)
    }

    fn new(connection_metadata: ConnectionMetadata, message_bus: Arc<dyn AsyncMessageBus>) -> Result<Client, Error> {
        let client = Client {
            server_version: connection_metadata.server_version,
            connection_time: connection_metadata.connection_time,
            time_zone: connection_metadata.time_zone,
            message_bus,
            client_id: connection_metadata.client_id,
            id_manager: ClientIdManager::new(connection_metadata.next_order_id),
        };

        Ok(client)
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
