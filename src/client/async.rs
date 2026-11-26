//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

use log::debug;
use time::OffsetDateTime;
use time_tz::Tz;
use tokio::sync::broadcast;

use crate::accounts::common::split_account_list;
use crate::client::builders::client_builder::async_impl::ClientBuilder;
use crate::client::builders::client_builder::ClientConfig;
use crate::connection::{r#async::AsyncConnection, ConnectionMetadata};
use crate::messages::{Notice, OutgoingMessages};
use crate::transport::{
    r#async::{AsyncInternalSubscription, AsyncTcpMessageBus},
    AsyncMessageBus,
};
use crate::Error;

use super::handshake_state::{HandshakeState, ServerVersion};
use super::id_generator::ClientIdManager;
use super::ids::{OrderId, RequestId};

/// Asynchronous TWS API Client
pub struct Client {
    /// IB server version, shared with the transport so a reconnect updates it.
    server_version: Arc<ServerVersion>,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    handshake_managed_accounts: Vec<String>,
    pub(crate) message_bus: Arc<dyn AsyncMessageBus>,

    client_id: i32,                   // ID of client.
    id_manager: Arc<ClientIdManager>, // Manages request and order ID generation
    /// `ClientBuilder::one_shot_timeout`; `None` leaves request-id one-shots unbounded.
    one_shot_timeout: Option<Duration>,
}

impl Drop for Client {
    fn drop(&mut self) {
        debug!("dropping async client");
        // Request shutdown of the message bus synchronously
        self.message_bus.request_shutdown_sync();
    }
}

impl Client {
    /// Establishes a connection to TWS or Gateway with no extra configuration.
    ///
    /// One-liner shortcut equivalent to
    /// `Client::builder().address(address).client_id(client_id).connect().await`.
    /// For `tcp_no_delay`, startup callbacks, or a pre-bound `NoticeStream`,
    /// use [`Client::builder`] instead.
    ///
    /// # Arguments
    /// * `address`   - address of server. e.g. 127.0.0.1:4002
    /// * `client_id` - id of client. e.g. 100
    ///
    /// # Errors
    ///
    /// [`Error::ClientIdInUse`] when another connection holds `client_id`;
    /// connect with a different id. Not retried here.
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
        Self::builder().address(address).client_id(client_id).connect().await
    }

    /// Begin a fluent connection builder.
    ///
    /// See [`ClientBuilder`] for the configurators (`address`, `client_id`,
    /// `tcp_no_delay`, `startup_callback`) and the two terminals
    /// (`connect`, `connect_with_notice_stream`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run() -> Result<(), ibapi::Error> {
    /// use ibapi::Client;
    ///
    /// let client = Client::builder()
    ///     .address("127.0.0.1:4002")
    ///     .client_id(100)
    ///     .tcp_no_delay(true)
    ///     .connect()
    ///     .await?;
    /// drop(client);
    /// # Ok(()) }
    /// ```
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Internal entry point shared by `Client::connect` and `ClientBuilder`.
    /// Builds the `AsyncConnection`, wraps it in an `AsyncTcpMessageBus`, and
    /// kicks off the dispatcher task.
    pub(crate) async fn connect_with_config(
        config: ClientConfig,
        notice_sender: broadcast::Sender<Notice>,
        channel_capacity: usize,
    ) -> Result<Client, Error> {
        let connection = AsyncConnection::connect(config.connection, notice_sender).await?;
        let connection_metadata = connection.connection_metadata().await;

        let message_bus = Arc::new(AsyncTcpMessageBus::with_channel_capacity(connection, channel_capacity));

        let mut client = Client::new(connection_metadata, message_bus.clone())?;
        client.one_shot_timeout = config.one_shot_timeout;

        // Share the state a reconnect's handshake refreshes (order-id floor,
        // server version) with the bus; must be installed before the
        // processing task starts.
        message_bus.set_handshake_state(client.handshake_state());
        if let Some(limiter) = config.rate_limiter {
            message_bus.set_rate_limiter(limiter);
        }

        // Start background task to read messages from TWS
        message_bus.clone().process_messages()?;

        Ok(client)
    }

    fn new(connection_metadata: ConnectionMetadata, message_bus: Arc<dyn AsyncMessageBus>) -> Result<Client, Error> {
        let client = Client {
            server_version: Arc::new(ServerVersion::new(connection_metadata.server_version)),
            connection_time: connection_metadata.connection_time,
            time_zone: connection_metadata.time_zone,
            handshake_managed_accounts: split_account_list(&connection_metadata.managed_accounts),
            message_bus,
            client_id: connection_metadata.client_id,
            id_manager: Arc::new(ClientIdManager::new(connection_metadata.next_order_id)?),
            one_shot_timeout: None,
        };

        Ok(client)
    }

    /// The state the transport refreshes on reconnect, sharing this client's.
    fn handshake_state(&self) -> HandshakeState {
        HandshakeState {
            order_ids: self.id_manager.clone(),
            server_version: self.server_version.clone(),
        }
    }

    /// Returns the version of the TWS API server to which the client is connected,
    /// as negotiated in the connection handshake. Updated when the client
    /// reconnects, since the gateway may come back on a different build.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let server_version = client.server_version();
    ///     println!("Connected to TWS server version: {server_version:?}");
    /// }
    /// ```
    pub fn server_version(&self) -> i32 {
        self.server_version.get()
    }

    /// Returns the gateway's clock reading when the client connected, or
    /// `None` if the zone is unrecognized (see [`time_zone`](Self::time_zone))
    /// or the reading is malformed.
    ///
    /// From the initial connection; a reconnect doesn't update it.
    pub fn connection_time(&self) -> Option<OffsetDateTime> {
        self.connection_time
    }

    /// Returns the time zone of the gateway host, from the connection handshake.
    ///
    /// Informational: the crate does not convert any data with it. `None` if
    /// the gateway sent a name that no alias or IANA zone matches (logged as a
    /// warning at connect). Map such a name with
    /// [`register_timezone_alias`](crate::register_timezone_alias). See
    /// [Gateway Time Zone](https://github.com/wboayue/rust-ibapi/blob/main/docs/troubleshooting.md#gateway-time-zone).
    ///
    /// From the initial connection; a reconnect doesn't update it.
    pub fn time_zone(&self) -> Option<&'static Tz> {
        self.time_zone
    }

    /// Returns the accounts TWS sent when the client connected, with no round
    /// trip. Empty if TWS sent none.
    ///
    /// From the initial connection; a reconnect doesn't update it. For the
    /// current list, call [`managed_accounts`](Self::managed_accounts).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     for account in client.handshake_managed_accounts() {
    ///         println!("account: {account}");
    ///     }
    /// }
    /// ```
    pub fn handshake_managed_accounts(&self) -> &[String] {
        &self.handshake_managed_accounts
    }

    /// Returns a decoder context for this client
    pub(crate) fn decoder_context(&self) -> crate::subscriptions::DecoderContext {
        crate::subscriptions::DecoderContext::new(self.server_version())
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

    /// Resolves once the client is connected to TWS/IB Gateway again, so a
    /// stream loop can resubscribe after [`Error::is_connection_lost`].
    ///
    /// Resolves at once while connected. While the client is reconnecting it
    /// waits for the reconnect to finish, which with
    /// `ClientBuilder::reconnect_forever` may be never; wrap it in `tokio::time::timeout` to bound the wait.
    ///
    /// # Errors
    ///
    /// - [`Error::ConnectionFailed`] if the reconnect gave up after its last attempt.
    /// - [`Error::Shutdown`] if the client was shut down.
    ///
    /// Both are terminal: every later request fails with the same error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     loop {
    ///         let subscription = match client.realtime_bars(&contract).subscribe().await {
    ///             Ok(subscription) => subscription,
    ///             Err(e) if e.is_connection_lost() => {
    ///                 client.wait_connected().await.expect("reconnect gave up");
    ///                 continue;
    ///             }
    ///             Err(e) => panic!("request failed: {e}"),
    ///         };
    ///         // ... read until a connection-lost error, wait_connected, loop
    /// #       drop(subscription);
    /// #       break;
    ///     }
    /// }
    /// ```
    pub async fn wait_connected(&self) -> Result<(), Error> {
        self.message_bus.wait_connected(None).await
    }

    /// Streams items from the subscription `open` returns, resubscribing after
    /// each transport reconnect: the packaged form of the
    /// [`Error::is_connection_lost`] → [`wait_connected`](Self::wait_connected) → resubscribe loop.
    ///
    /// Yields the same `Result<SubscriptionItem<T>, Error>` items as the subscription. When the
    /// connection drops, it drops the dead subscription, waits for the reconnect, calls `open`
    /// again, and yields a [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem::Notice)
    /// with code [`TRANSPORT_RECONNECT_CODE`](crate::TRANSPORT_RECONNECT_CODE) before
    /// the new subscription's first item. Data from the outage is missing. A first `open` that
    /// fails with a lost connection is retried too, without a notice.
    ///
    /// An internet outage leaves the socket up, and TWS resumes most streams on their own. It
    /// ends a historical bar request with `keep_up_to_date` with error
    /// [`SUBSCRIPTION_DISCONNECTED_CODE`](crate::SUBSCRIPTION_DISCONNECTED_CODE) (10182) when
    /// its HMDS farm breaks. The wrapper yields that 10182 as a `SubscriptionItem::Notice`, the
    /// gap marker, waits until no HMDS farm reports broken (or TWS reports connectivity restored
    /// with 1101 or 1102, or 15 minutes pass), then calls `open` again. A 10182 that arrives
    /// with no HMDS farm reported broken waits 30 seconds instead, so a TWS that kept answering
    /// with 10182 would get one request every 30 seconds. A 10182 on the reopened request starts
    /// another wait, and a reconnect during the wait reopens as above.
    ///
    /// TWS has confirmed the farms back when every HMDS farm that reported broken has reported
    /// OK since, or a 1101 or 1102 arrived. The wait can also end unconfirmed: at the 15-minute
    /// limit, after the 30-second wait, or when the broken farms went inactive (2107) instead of
    /// reporting OK. TWS may hold a request sent while its farm is down and never answer it. So
    /// after an unconfirmed reopen, if TWS later confirms the farms back before the reopened
    /// request delivers a bar, the wrapper writes that request's cancel and calls `open` once
    /// more, without a notice. This happens at most once per 10182. The outage runs exercised
    /// the 15-minute limit and the second reopen; a farm going inactive and a 10182 with no farm
    /// broken did not occur, and those paths rest on tests only. The wait can last minutes with
    /// no bars. A consumer that completes a forming bar on a timer must treat the 10182 notice
    /// as the signal to stop until the reopened stream's first bars arrive; a bar completed
    /// during the wait rests on bars from before the outage.
    ///
    /// TWS sends 1101 ("connectivity restored, data lost") when its own link to IB comes back
    /// and it no longer serves the requests open before the loss. On a 1101 while the stream is
    /// open, the wrapper yields the 1101 as a `SubscriptionItem::Notice`, the gap marker, writes
    /// the old request's cancel, then calls `open` again. The cancel always goes out before the
    /// new request, because TWS may still count the old request against a limit such as market
    /// depth lines. An error TWS sends for the old request cannot end the new stream. Each 1101
    /// reopens once; 1101s that arrive before the wrapper next runs reopen once together. A 1101
    /// that ends a wait after 10182 reopens once, without a second notice. 1102 ("data
    /// maintained") reopens nothing, except the second reopen after an unconfirmed wait
    /// described above. A caller that re-requests on 1101 itself must stop, or it
    /// opens a second request. No 1101 arrived in the outage runs: this reopen follows IB's
    /// documentation, not a capture.
    ///
    /// It ends after yielding any other error, the error `wait_connected` returns when the
    /// reconnect gave up, or when the subscription itself ends (a snapshot that finished).
    ///
    /// [`filter_data`](crate::subscriptions::SubscriptionItemStreamExt::filter_data) drops notices, gap included; match the
    /// items to see it.
    ///
    /// `T` must be [`Resubscribable`](crate::subscriptions::Resubscribable): order streams are not,
    /// so a resubscribe can't place or cancel again. After a resubscribe the caller sees:
    ///
    /// | Stream | After the gap notice |
    /// |---|---|
    /// | market data, realtime bars, tick-by-tick, news bulletins | new data only |
    /// | market depth | the book rebuilt from scratch: reset yours on the notice |
    /// | historical bars, `subscribe_with_updates` | the initial bars again (duplicates) |
    /// | historical bars, `subscribe` | the whole request again, `Bars` then `End`, if cut before `End`; the stream ends after `End` |
    /// | historical bars, `subscribe_with_updates`, after a 10182 notice | the initial bars again once the HMDS farm is back |
    /// | account, position, PnL, account summary | the initial snapshot again |
    /// | scanner | a fresh scan |
    /// | any of the above, after a 1101 notice | the same as after a reconnect |
    ///
    /// The stream isn't `Unpin`: pin it (`std::pin::pin!` or `Box::pin`) before calling `next`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::{SUBSCRIPTION_DISCONNECTED_CODE, TRANSPORT_RECONNECT_CODE};
    /// use ibapi::prelude::*;
    ///
    /// # async fn run(client: &ibapi::Client) {
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let mut items = std::pin::pin!(client.resubscribing(|| client.realtime_bars(&contract).subscribe()));
    /// while let Some(item) = items.next().await {
    ///     match item {
    ///         Ok(SubscriptionItem::Data(bar)) => println!("{bar:?}"),
    ///         Ok(SubscriptionItem::Notice(n)) if matches!(n.code, TRANSPORT_RECONNECT_CODE | SUBSCRIPTION_DISCONNECTED_CODE | 1101) => {
    ///             println!("gap: {n}")
    ///         }
    ///         Ok(SubscriptionItem::Notice(n)) => println!("notice: {n}"),
    ///         Err(e) => panic!("stream ended: {e}"),
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// Order streams don't compile:
    ///
    /// ```compile_fail,E0277
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::Order;
    ///
    /// # async fn run(client: &ibapi::Client) {
    /// let (contract, order) = (Contract::stock("AAPL").build(), Order::default());
    /// let _ = client.resubscribing(|| client.place_order(1, &contract, &order));
    /// # }
    /// ```
    pub fn resubscribing<'a, T, F, Fut>(
        &'a self,
        open: F,
    ) -> impl futures::Stream<Item = Result<crate::subscriptions::SubscriptionItem<T>, Error>> + 'a
    where
        T: crate::subscriptions::Resubscribable + Send + 'static,
        F: FnMut() -> Fut + 'a,
        Fut: std::future::Future<Output = Result<crate::subscriptions::r#async::Subscription<T>, Error>> + 'a,
    {
        crate::subscriptions::resubscribing(self, open)
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
    /// Each call returns a fresh, independent [`NoticeStream`](crate::subscriptions::NoticeStream);
    /// late subscribers do not see prior notices. The stream ends when the client shuts
    /// down; one created after shutdown is already ended.
    ///
    /// Per-subscription notices (codes carrying a real `request_id`) are not
    /// delivered here — they reach their owning subscription as
    /// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem::Notice)
    /// (via [`futures::StreamExt::next`] on the [`Subscription`](crate::subscriptions::Subscription) stream).
    ///
    /// # Note on handshake-time notices
    ///
    /// Notices emitted during the connection handshake — the typical
    /// 2104/2106/2158 farm-status burst that arrives before `connect` returns —
    /// will not be observed by a `NoticeStream` created afterwards. Use
    /// [`ClientBuilder::connect_with_notice_stream`](crate::ClientBuilder::connect_with_notice_stream)
    /// to capture those (the pre-bound stream covers handshake AND post-connect
    /// notices, and survives auto-reconnects).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut stream = client.notice_stream();
    ///     while let Some(notice) = stream.next().await {
    ///         if notice.is_system_message() {
    ///             println!("connectivity: {notice}");
    ///         } else if notice.is_informational() {
    ///             println!("info: {notice}");
    ///         } else {
    ///             eprintln!("error: {notice}");
    ///         }
    ///     }
    /// }
    /// ```
    pub fn notice_stream(&self) -> crate::subscriptions::notice_stream::async_impl::NoticeStream {
        self.message_bus.notice_subscribe()
    }

    /// Returns the ID assigned to the [Client].
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id().value()
    }

    /// How long a one-shot request waits for TWS's answer, if the caller bounded it.
    pub(crate) fn one_shot_timeout(&self) -> Option<Duration> {
        self.one_shot_timeout
    }

    /// Allocates a request ID for a request this crate sends.
    pub(crate) fn mint_request_id(&self) -> RequestId {
        self.id_manager.next_request_id()
    }

    /// The client's request- and order-ID generators: the one way in for
    /// minting, reserving and reading ids.
    pub(crate) fn id_manager(&self) -> &ClientIdManager {
        &self.id_manager
    }

    pub(crate) async fn send_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.send_request(request_id, message).await
    }

    pub(crate) async fn send_shared_request(&self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.send_shared_request(message_type, message).await
    }

    pub(crate) async fn send_order(&self, order_id: OrderId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
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
    pub(crate) fn stubbed(message_bus: Arc<dyn AsyncMessageBus>, server_version: i32) -> Self {
        use crate::connection::ConnectionMetadata;

        let connection_metadata = ConnectionMetadata {
            client_id: 100,
            next_order_id: crate::common::test_utils::helpers::TEST_ORDER_ID_SEED,
            server_version,
            managed_accounts: String::new(),
            connection_time: None,
            time_zone: None,
        };

        Client::new(connection_metadata, message_bus).expect("Failed to create stubbed client")
    }

    /// [`stubbed`](Self::stubbed) with `ClientBuilder::one_shot_timeout` set.
    #[cfg(test)]
    pub(crate) fn with_one_shot_timeout(mut self, timeout: Duration) -> Self {
        self.one_shot_timeout = Some(timeout);
        self
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
