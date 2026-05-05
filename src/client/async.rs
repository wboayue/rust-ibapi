//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

use log::debug;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::connection::common::{ConnectionOptions, StartupMessageCallback};
use crate::connection::{r#async::AsyncConnection, ConnectionMetadata};
use crate::messages::OutgoingMessages;
use crate::transport::{
    r#async::{AsyncInternalSubscription, AsyncTcpMessageBus},
    AsyncMessageBus,
};
use crate::Error;

use super::id_generator::ClientIdManager;
use crate::contracts::Contract;
use crate::orders::OrderBuilder;

/// Asynchronous TWS API Client
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    pub(crate) message_bus: Arc<dyn AsyncMessageBus>,

    client_id: i32,                   // ID of client.
    id_manager: Arc<ClientIdManager>, // Manages request and order ID generation
}

impl Drop for Client {
    fn drop(&mut self) {
        debug!("dropping async client");
        // Request shutdown of the message bus synchronously
        self.message_bus.request_shutdown_sync();
    }
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
        Self::connect_with_callback(address, client_id, None).await
    }

    /// Establishes async connection to TWS or Gateway with a callback for startup messages
    ///
    /// This is similar to [`connect`](Self::connect), but allows you to provide a callback
    /// that will be invoked for any unsolicited messages received during the connection
    /// handshake (e.g., OpenOrder, OrderStatus).
    ///
    /// Note: The callback is only invoked during the initial connection, not during
    /// automatic reconnections.
    ///
    /// # Arguments
    /// * `address`          - address of server. e.g. 127.0.0.1:4002
    /// * `client_id`        - id of client. e.g. 100
    /// * `startup_callback` - optional callback for unsolicited messages during connection
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Client, StartupMessage, StartupMessageCallback};
    /// use std::sync::{Arc, Mutex};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let order_ids = Arc::new(Mutex::new(Vec::new()));
    ///     let order_ids_clone = order_ids.clone();
    ///
    ///     let callback: StartupMessageCallback = Box::new(move |msg| match msg {
    ///         StartupMessage::OpenOrder(order_data) => {
    ///             order_ids_clone.lock().unwrap().push(order_data.order_id);
    ///         }
    ///         StartupMessage::OrderStatus(_)
    ///         | StartupMessage::OpenOrderEnd
    ///         | StartupMessage::AccountUpdate(_)
    ///         | StartupMessage::Other(_) => {}
    ///     });
    ///
    ///     let client = Client::connect_with_callback("127.0.0.1:4002", 100, Some(callback))
    ///         .await
    ///         .expect("connection failed");
    ///
    ///     println!("Received {} startup open-orders", order_ids.lock().unwrap().len());
    /// }
    /// ```
    pub async fn connect_with_callback(address: &str, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Client, Error> {
        Self::connect_with_options(address, client_id, startup_callback.into()).await
    }

    /// Establishes async connection to TWS or Gateway with custom options
    ///
    /// This is similar to [`connect`](Self::connect), but allows you to configure
    /// connection options like `TCP_NODELAY` and startup callbacks via
    /// [`ConnectionOptions`].
    ///
    /// # Arguments
    /// * `address`   - address of server. e.g. 127.0.0.1:4002
    /// * `client_id` - id of client. e.g. 100
    /// * `options`   - connection options
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Client, ConnectionOptions};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let options = ConnectionOptions::default()
    ///         .tcp_no_delay(true);
    ///
    ///     let client = Client::connect_with_options("127.0.0.1:4002", 100, options)
    ///         .await
    ///         .expect("connection failed");
    /// }
    /// ```
    pub async fn connect_with_options(address: &str, client_id: i32, options: ConnectionOptions) -> Result<Client, Error> {
        let connection = AsyncConnection::connect_with_options(address, client_id, options).await?;
        let connection_metadata = connection.connection_metadata().await;

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
            id_manager: Arc::new(ClientIdManager::new(connection_metadata.next_order_id)),
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

    /// Returns the server's time zone
    pub fn time_zone(&self) -> Option<&'static Tz> {
        self.time_zone
    }

    /// Returns a decoder context for this client
    pub(crate) fn decoder_context(&self) -> crate::subscriptions::DecoderContext {
        crate::subscriptions::DecoderContext::new(self.server_version, self.time_zone)
    }

    /// Returns true if the client is currently connected to TWS/IB Gateway.
    ///
    /// This method checks if the underlying connection to TWS or IB Gateway is active.
    /// Returns false if the connection has been lost, shut down, or reset.
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
    ///     if client.is_connected() {
    ///         println!("Client is connected to TWS/Gateway");
    ///     } else {
    ///         println!("Client is not connected");
    ///     }
    /// }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.message_bus.is_connected()
    }

    /// Cleanly shuts down the message bus.
    ///
    /// All outstanding [`Subscription`](crate::subscriptions::Subscription)s see their channels
    /// close and their `next()` calls return `None`. The background dispatch task is awaited
    /// to completion before this returns.
    ///
    /// **Call this before dropping the final `Arc<Client>` if any spawned
    /// tasks hold that `Arc`.** Otherwise the tokio runtime will hang on
    /// shutdown — `Drop` cannot perform the full async shutdown because it
    /// is not async.
    ///
    /// Safe to call multiple times.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     // ... use client, spawn tasks holding Arc<Client> ...
    ///     client.disconnect().await;
    /// }
    /// ```
    pub async fn disconnect(&self) {
        self.message_bus.ensure_shutdown().await;
    }

    /// Subscribe to globally routed IB notices (notices with no `request_id` —
    /// connectivity codes 1100/1101/1102, farm-status 2104/2105/2106/2107/2108,
    /// and any other unrouted error/warning).
    ///
    /// Each call returns a fresh, independent [`NoticeStream`]; late subscribers
    /// do not see prior notices. The stream ends when the client disconnects.
    ///
    /// Per-subscription notices (codes carrying a real `request_id`) are not
    /// delivered here — they reach their owning subscription as
    /// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem::Notice).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut stream = client.notice_stream().expect("notice subscription failed");
    ///     while let Some(notice) = stream.next().await {
    ///         if notice.is_system_message() {
    ///             println!("connectivity: {notice}");
    ///         } else if notice.is_warning() {
    ///             println!("warning: {notice}");
    ///         } else {
    ///             eprintln!("error: {notice}");
    ///         }
    ///     }
    /// }
    /// ```
    pub fn notice_stream(&self) -> Result<crate::subscriptions::notice_stream::async_impl::NoticeStream, Error> {
        Ok(self.message_bus.notice_subscribe())
    }

    /// Returns the ID assigned to the [Client].
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id()
    }

    /// Returns the next request ID
    pub fn next_request_id(&self) -> i32 {
        self.id_manager.next_request_id()
    }

    /// Sets the current value of order ID.
    pub(crate) fn set_next_order_id(&self, order_id: i32) {
        self.id_manager.set_order_id(order_id);
    }

    /// Start building an order for the given contract
    ///
    /// This is the primary API for creating orders, providing a fluent interface
    /// that guides you through the order creation process.
    ///
    /// # Example
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     
    ///     let order_id = client.order(&contract)
    ///         .buy(100)
    ///         .limit(50.0)
    ///         .submit().await.expect("order submission failed");
    /// }
    /// ```
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<'a, Self> {
        OrderBuilder::new(self, contract)
    }

    /// Creates a market data subscription builder with a fluent interface.
    pub fn market_data<'a>(&'a self, contract: &'a Contract) -> crate::market_data::builder::MarketDataBuilder<'a, Self> {
        crate::market_data::builder::MarketDataBuilder::new(self, contract)
    }

    /// Check server version requirement
    pub fn check_server_version(&self, required_version: i32, feature: &str) -> Result<(), Error> {
        if self.server_version < required_version {
            return Err(Error::Simple(format!(
                "Server version {} is too old. {} requires version {}",
                self.server_version, feature, required_version
            )));
        }
        Ok(())
    }

    pub(crate) async fn send_request(&self, request_id: i32, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.send_request(request_id, message).await
    }

    pub(crate) async fn send_shared_request(&self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.send_shared_request(message_type, message).await
    }

    pub(crate) async fn send_order(&self, order_id: i32, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.send_order_request(order_id, message).await
    }

    /// Create order update subscription
    pub(crate) async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.create_order_update_subscription().await
    }

    pub(crate) async fn send_message(&self, message: Vec<u8>) -> Result<(), Error> {
        self.message_bus.send_message(message).await
    }

    // Domain methods (accounts, contracts, orders, market_data, news, scanner,
    // display_groups, wsh) are now defined directly in each domain's async.rs.

    /// Creates a stubbed client for testing
    #[cfg(test)]
    pub fn stubbed(message_bus: Arc<dyn AsyncMessageBus>, server_version: i32) -> Self {
        use crate::connection::ConnectionMetadata;

        let connection_metadata = ConnectionMetadata {
            client_id: 100,
            next_order_id: 9000,
            server_version,
            managed_accounts: String::new(),
            connection_time: None,
            time_zone: None,
        };

        Client::new(connection_metadata, message_bus).expect("Failed to create stubbed client")
    }

    /// Get a reference to the message bus for testing
    #[cfg(test)]
    pub fn message_bus(&self) -> &Arc<dyn AsyncMessageBus> {
        &self.message_bus
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
