//! Client implementation for connecting to and communicating with TWS and IB Gateway.
//!
//! The Client provides the main interface for establishing connections, sending requests,
//! and receiving responses from the Interactive Brokers API. It manages message routing,
//! subscriptions, and maintains the connection state.

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use log::debug;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::connection::common::{ConnectionOptions, StartupMessageCallback};
use crate::connection::{sync::Connection, ConnectionMetadata};
use crate::contracts::Contract;
use crate::errors::Error;
use crate::market_data::builder::MarketDataBuilder;
use crate::messages::OutgoingMessages;
use crate::orders::OrderBuilder;
use crate::transport::{InternalSubscription, MessageBus, TcpMessageBus};

use super::id_generator::ClientIdManager;

// Client

/// TWS API Client. Manages the connection to TWS or Gateway.
/// Tracks some global information such as server version and server time.
/// Supports generation of order ids.
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    pub(crate) message_bus: Arc<dyn MessageBus>,

    client_id: i32,              // ID of client.
    id_manager: ClientIdManager, // Manages request and order ID generation
}

impl Client {
    /// Establishes connection to TWS or Gateway
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// println!("server_version: {}", client.server_version());
    /// println!("connection_time: {:?}", client.connection_time());
    /// println!("next_order_id: {}", client.next_order_id());
    /// ```
    pub fn connect(address: &str, client_id: i32) -> Result<Client, Error> {
        Self::connect_with_callback(address, client_id, None)
    }

    /// Establishes connection to TWS or Gateway with a callback for startup messages
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::{StartupMessage, StartupMessageCallback};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let order_ids = Arc::new(Mutex::new(Vec::new()));
    /// let order_ids_clone = order_ids.clone();
    ///
    /// let callback: StartupMessageCallback = Box::new(move |msg| match msg {
    ///     StartupMessage::OpenOrder(order_data) => {
    ///         order_ids_clone.lock().unwrap().push(order_data.order_id);
    ///     }
    ///     StartupMessage::OrderStatus(_)
    ///     | StartupMessage::OpenOrderEnd
    ///     | StartupMessage::AccountUpdate(_)
    ///     | StartupMessage::Other(_) => {}
    /// });
    ///
    /// let client = Client::connect_with_callback("127.0.0.1:4002", 100, Some(callback))
    ///     .expect("connection failed");
    ///
    /// println!("Received {} startup open-orders", order_ids.lock().unwrap().len());
    /// ```
    pub fn connect_with_callback(address: &str, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Client, Error> {
        Self::connect_with_options(address, client_id, startup_callback.into())
    }

    /// Establishes connection to TWS or Gateway with custom options
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::ConnectionOptions;
    ///
    /// let options = ConnectionOptions::default()
    ///     .tcp_no_delay(true);
    ///
    /// let client = Client::connect_with_options("127.0.0.1:4002", 100, options)
    ///     .expect("connection failed");
    /// ```
    pub fn connect_with_options(address: &str, client_id: i32, options: ConnectionOptions) -> Result<Client, Error> {
        let connection = Connection::connect_with_options(address, client_id, options)?;
        let connection_metadata = connection.connection_metadata();

        let message_bus = Arc::new(TcpMessageBus::new(connection)?);

        // Starts thread to read messages from TWS
        message_bus.process_messages(connection_metadata.server_version, Duration::from_secs(1))?;

        Client::new(connection_metadata, message_bus)
    }

    fn new(connection_metadata: ConnectionMetadata, message_bus: Arc<dyn MessageBus>) -> Result<Client, Error> {
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

    /// Returns the ID assigned to the [Client].
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the next request ID.
    pub fn next_request_id(&self) -> i32 {
        self.id_manager.next_request_id()
    }

    /// Returns and increments the order ID.
    ///
    /// The client maintains a sequence of order IDs. This function returns the next order ID in the sequence.
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id()
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(100)
    ///     .limit(50.0)
    ///     .submit().expect("order submission failed");
    /// ```
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<'a, Self> {
        OrderBuilder::new(self, contract)
    }

    /// Returns the version of the TWS API server to which the client is connected.
    /// This version is determined during the initial connection handshake.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let server_version = client.server_version();
    /// println!("Connected to TWS server version: {server_version:?}");
    /// ```
    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    /// The time of the server when the client connected
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// if client.is_connected() {
    ///     println!("Client is connected to TWS/Gateway");
    /// } else {
    ///     println!("Client is not connected");
    /// }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.message_bus.is_connected()
    }

    /// Cleanly shuts down the message bus.
    ///
    /// All outstanding [`Subscription`](crate::subscriptions::Subscription)s see their channels
    /// close and their `next()` calls return `None`. Background worker threads are joined
    /// before this returns.
    ///
    /// Call this before dropping the final `Arc<Client>` if any spawned
    /// threads hold that `Arc` — otherwise `Drop` never runs and those
    /// threads block forever in `subscription.next()`.
    ///
    /// Safe to call multiple times.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// // ... use client, spawn threads holding Arc<Client> ...
    /// client.disconnect();
    /// ```
    pub fn disconnect(&self) {
        self.message_bus.ensure_shutdown();
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
    /// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem::Notice)
    /// (see [`Subscription::next`](crate::client::blocking::Subscription::next)).
    ///
    /// # Note on handshake-time notices
    ///
    /// Notices emitted during the connection handshake — the typical
    /// 2104/2106/2158 farm-status burst that arrives before `connect` returns —
    /// will not be observed by a `NoticeStream` created afterwards. Use
    /// [`ConnectionOptions::startup_notice_callback`](crate::ConnectionOptions::startup_notice_callback)
    /// to capture those.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let stream = client.notice_stream().expect("notice subscription failed");
    /// for notice in stream.iter() {
    ///     if notice.is_system_message() {
    ///         println!("connectivity: {notice}");
    ///     } else if notice.is_warning() {
    ///         println!("warning: {notice}");
    ///     } else {
    ///         eprintln!("error: {notice}");
    ///     }
    /// }
    /// ```
    pub fn notice_stream(&self) -> Result<crate::subscriptions::notice_stream::sync_impl::NoticeStream, Error> {
        Ok(self.message_bus.notice_subscribe())
    }

    /// Requests real time market data.
    ///
    /// Creates a market data subscription builder with a fluent interface.
    ///
    /// This is the preferred way to subscribe to market data, providing a more
    /// intuitive and discoverable API than the raw method.
    ///
    /// # Arguments
    /// * `contract` - The contract to receive market data for
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::TickTypes;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// // Subscribe to real-time streaming data with specific tick types
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233", "236"])  // RTVolume and Shortable
    ///     .subscribe()
    ///     .expect("subscription failed");
    ///
    /// for tick in subscription.iter_data() {
    ///     match tick? {
    ///         TickTypes::Price(price) => println!("Price: {price:?}"),
    ///         TickTypes::Size(size) => println!("Size: {size:?}"),
    ///         TickTypes::SnapshotEnd => subscription.cancel(),
    ///         _ => {}
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::TickTypes;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// // Request a one-time snapshot
    /// let subscription = client.market_data(&contract)
    ///     .snapshot()
    ///     .subscribe()
    ///     .expect("subscription failed");
    ///
    /// for tick in subscription.iter_data() {
    ///     if let TickTypes::SnapshotEnd = tick? {
    ///         println!("Snapshot complete");
    ///         break;
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn market_data<'a>(&'a self, contract: &'a Contract) -> MarketDataBuilder<'a, Self> {
        MarketDataBuilder::new(self, contract)
    }

    // == Internal Use ==

    #[cfg(test)]
    pub(crate) fn stubbed(message_bus: Arc<dyn MessageBus>, server_version: i32) -> Client {
        Client {
            server_version,
            connection_time: None,
            time_zone: None,
            message_bus,
            client_id: 100,
            id_manager: ClientIdManager::new(-1),
        }
    }

    pub(crate) fn send_request(&self, request_id: i32, message: Vec<u8>) -> Result<InternalSubscription, Error> {
        debug!("send_message({request_id:?})");
        self.message_bus.send_request(request_id, &message)
    }

    pub(crate) fn send_order(&self, order_id: i32, message: Vec<u8>) -> Result<InternalSubscription, Error> {
        debug!("send_order({order_id:?})");
        self.message_bus.send_order_request(order_id, &message)
    }

    pub(crate) fn send_message(&self, message: Vec<u8>) -> Result<(), Error> {
        debug!("send_message()");
        self.message_bus.send_message(&message)
    }

    /// Creates a subscription for order updates if one is not already active.
    pub(crate) fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error> {
        self.message_bus.create_order_update_subscription()
    }

    /// Sends request for the next valid order id.
    pub(crate) fn send_shared_request(&self, message_id: OutgoingMessages, message: Vec<u8>) -> Result<InternalSubscription, Error> {
        self.message_bus.send_shared_request(message_id, &message)
    }

    pub(crate) fn check_server_version(&self, version: i32, message: &str) -> Result<(), Error> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(Error::ServerVersion(version, self.server_version, message.into()))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        debug!("dropping basic client");
        self.message_bus.ensure_shutdown();
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("server_version", &self.server_version)
            .field("server_time", &self.connection_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

/// Subscriptions facilitate handling responses from TWS that may be delayed or delivered periodically.
///
/// They offer both blocking and non-blocking methods for retrieving data.
///
/// In the simplest case a subscription can be implicitly converted to blocking iterator
/// that cancels the subscription when it goes out of scope.
///
/// ```no_run
/// use ibapi::contracts::Contract;
/// use ibapi::market_data::realtime::{BarSize, WhatToShow};
/// use ibapi::market_data::TradingHours;
/// use ibapi::client::blocking::Client;
///
/// let connection_url = "127.0.0.1:4002";
/// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
///
/// // Request real-time bars data for AAPL with 5-second intervals
/// let contract = Contract::stock("AAPL").build();
/// let subscription = client
///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended)
///     .expect("realtime bars request failed!");
///
/// // Use the subscription as a blocking iterator
/// for bar in subscription {
///     // Process each bar here (e.g., print or use in calculations)
///     println!("Received bar: {bar:?}");
/// }
/// // The subscription goes out of scope and is automatically cancelled.
/// ```
///
/// Subscriptions can be explicitly canceled using the [cancel](Subscription::cancel) method.
///
// Re-export SharesChannel trait from subscriptions module
pub use crate::subscriptions::SharesChannel;

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
