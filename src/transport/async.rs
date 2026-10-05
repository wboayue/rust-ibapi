//! Asynchronous transport implementation

mod connection_signal;
mod io;
mod registry;
mod shutdown;
pub(crate) use connection_signal::ConnectionSignal;
/// The frame reader itself, for tests that drive it over an in-memory cursor
/// rather than a socket. Production callers reach it through `AsyncIo`.
#[cfg(test)]
pub(crate) use io::read_framed_message;
/// The stream traits, for test fixtures that stand in for a socket.
#[cfg(test)]
pub(crate) use io::{AsyncIo, AsyncReconnect};
pub(crate) use io::{AsyncStream, AsyncTcpSocket};
use registry::{Route, SenderHash, SharedChannels};
pub(crate) use shutdown::ShutdownSignal;

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use futures::Stream;
use log::{debug, error, info, warn};
use tokio::runtime::Handle;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task;
use tokio::time::Duration;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

use crate::accounts::types::AccountId;
use crate::client::id_generator::ClientIdManager;
use crate::client::ids::{OrderId, RequestId, WireId};
use crate::connection::r#async::AsyncConnection;
use crate::messages::{transport_reconnect_notice, IncomingMessages, Notice, OutgoingMessages, ResponseMessage};
use crate::Error;

use super::common::{log_orphan, report_unroutable_frame, Lease, LeaseRef};
use super::routing::{
    classify_error, determine_routing, order_routing_strategy, order_update_notice, DecodedError, ErrorDisposition, OrderRoutingStrategy,
    RoutingDecision,
};
use super::{BufferBound, RoutedItem, SharedTicket};

/// Default capacity for broadcast channels. Market-data channels take the
/// per-client override from `ClientBuilder::channel_capacity`; the notice
/// fan-out channels always use this default. When a consumer falls behind by
/// more than the capacity, the channel evicts the oldest frames and a data
/// subscription receives a `SUBSCRIPTION_LAG_CODE` notice naming the count.
pub(crate) const BROADCAST_CHANNEL_CAPACITY: usize = 1024;

/// Capacity floor for order-class request routes: one per order
/// (`place_order`, `cancel_order`, `exercise_options`) and `executions`.
/// Their traffic is per order or per request and ends, so this floor only
/// keeps a small `ClientBuilder::channel_capacity` from shrinking them; see #896.
pub(crate) const ORDER_CAPACITY: usize = 1024;

/// Capacity floor for the order-class streams that aggregate every order's
/// traffic for the session: the order update stream and the shared
/// open/completed-order channels. Completeness is their contract, so a lag
/// there should mean a stalled consumer, never a busy day. Tokio
/// preallocates every slot (~120 bytes each), which is why request routes
/// get the smaller [`ORDER_CAPACITY`] instead; see #896.
pub(crate) const ORDER_STREAM_CAPACITY: usize = 8192;

/// What a channel carries, which decides its capacity and how loudly a lag
/// on it is reported (#896). Market data is bounded and lossy by design;
/// order channels are sized so a lag means a stalled consumer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ChannelClass {
    /// Market data and every other non-order stream; sized by
    /// `ClientBuilder::channel_capacity`.
    #[default]
    MarketData,
    /// A request route for one order or one `executions` request.
    Order,
    /// An aggregate of every order's traffic.
    OrderStream,
}

impl ChannelClass {
    /// The class of a shared channel's request type.
    fn of_shared(request: OutgoingMessages) -> Self {
        match request {
            OutgoingMessages::RequestOpenOrders
            | OutgoingMessages::RequestAllOpenOrders
            | OutgoingMessages::RequestAutoOpenOrders
            | OutgoingMessages::RequestCompletedOrders => Self::OrderStream,
            _ => Self::MarketData,
        }
    }

    /// Capacity given the client's `channel_capacity`, which can raise an
    /// order-class floor but never lower it.
    fn capacity(self, channel_capacity: usize) -> usize {
        match self {
            Self::MarketData => channel_capacity,
            Self::Order => channel_capacity.max(ORDER_CAPACITY),
            Self::OrderStream => channel_capacity.max(ORDER_STREAM_CAPACITY),
        }
    }
}

/// Fan-out for unrouted notices, the async counterpart of the sync
/// `NoticeBroadcaster`. It holds the only `broadcast::Sender`, so `close`
/// ends every `NoticeStream`, including one pre-bound by the builder.
#[derive(Debug)]
pub(crate) struct NoticeBroadcaster {
    /// `None` once closed.
    sender: std::sync::Mutex<Option<broadcast::Sender<Notice>>>,
}

impl super::common::NoticeSink for NoticeBroadcaster {
    fn deliver(&self, notice: Notice) {
        self.broadcast(notice);
    }
}

impl NoticeBroadcaster {
    pub(crate) fn new(sender: broadcast::Sender<Notice>) -> Self {
        Self {
            sender: std::sync::Mutex::new(Some(sender)),
        }
    }

    /// After `close`, the returned receiver is already at end-of-stream,
    /// like the ones `close` ended.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Notice> {
        match self.sender.lock().unwrap().as_ref() {
            Some(sender) => sender.subscribe(),
            None => broadcast::channel(1).1,
        }
    }

    pub(crate) fn broadcast(&self, notice: Notice) {
        if let Some(sender) = self.sender.lock().unwrap().as_ref() {
            let _ = sender.send(notice);
        }
    }

    /// Drop the sender so existing receivers see channel-closed, and end
    /// every later subscription on arrival.
    pub(crate) fn close(&self) {
        *self.sender.lock().unwrap() = None;
    }
}

/// Cleanup signal for removing channels when subscriptions are dropped. Every
/// variant but `Shared` (whose channels persist) carries the subscription's
/// lease, so cleanup removes only its own registration, and only once no
/// clone holds the lease.
#[derive(Debug, Clone)]
pub enum CleanupSignal {
    Request(RequestId, LeaseRef),
    Order(OrderId, LeaseRef),
    Shared(SharedTicket),
    OrderUpdateStream(LeaseRef),
}

/// The registration a leased subscription releases: with the lease, it builds
/// the subscription's [`CleanupSignal`] when the signal is sent.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RouteKey {
    Request(RequestId),
    Order(OrderId),
    OrderUpdateStream,
}

impl RouteKey {
    fn signal(self, lease: LeaseRef) -> CleanupSignal {
        match self {
            RouteKey::Request(request_id) => CleanupSignal::Request(request_id, lease),
            RouteKey::Order(order_id) => CleanupSignal::Order(order_id, lease),
            RouteKey::OrderUpdateStream => CleanupSignal::OrderUpdateStream(lease),
        }
    }
}

/// Asynchronous message bus trait
#[async_trait]
pub trait AsyncMessageBus: Send + Sync {
    async fn send_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error>;

    /// [`send_request`](Self::send_request) with a cap on unread items: see
    /// [`BoundState::admit`](super::BoundState::admit). Past the cap the route
    /// queues `Error::BufferLimitExceeded` and discards later frames.
    async fn send_request_bounded(&self, request_id: RequestId, message: Vec<u8>, bound: BufferBound) -> Result<AsyncInternalSubscription, Error>;

    /// [`send_request`](Self::send_request) for `executions`: an order-class
    /// route, which `ClientBuilder::channel_capacity` can raise but not shrink.
    async fn send_executions_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error>;

    async fn send_order_request(&self, order_id: OrderId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error>;

    async fn send_shared_request(&self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error>;

    /// `send_shared_request` for `RequestAccountData`, refused with
    /// `Error::AccountUpdatesInUse` while another account is live.
    async fn send_account_updates_request(&self, account: &AccountId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error>;

    async fn send_message(&self, message: Vec<u8>) -> Result<(), Error>;

    /// Ends the subscription `ticket` names. `message` is the cancel to
    /// write for the last one; `None` for a stream TWS never cancels, which
    /// still releases the count.
    async fn cancel_shared_subscription(&self, ticket: SharedTicket, message: Option<Vec<u8>>) -> Result<(), Error>;

    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error>;

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::async_impl::NoticeStream;

    async fn ensure_shutdown(&self);

    /// Shut the bus down: end every subscription with `Error::Shutdown` and
    /// stop the dispatcher. Needs no runtime, so `Drop` can call it.
    fn request_shutdown_sync(&self);

    /// Resolve once the session is connected again, returning
    /// [`Error::Shutdown`] if the session will never reconnect.
    async fn wait_connected(&self) -> Result<(), Error>;

    fn is_connected(&self) -> bool;

    /// The runtime the bus was built on. `Drop` impls spawn their async
    /// cleanup here, so it runs even when the drop is on a thread with no
    /// runtime of its own.
    fn runtime_handle(&self) -> &Handle;
}

/// Internal subscription for async implementation.
///
/// Holds a `BroadcastStream<RoutedItem>` for poll-based consumption plus a
/// `template_receiver` kept solely so `Clone` can `resubscribe()` to produce an
/// independent stream. We cannot store the `Sender` instead — that would keep
/// the channel alive past the external sender's drop, breaking the
/// "channel closes when senders drop" termination contract.
pub struct AsyncInternalSubscription {
    /// Held only for `Clone` via `resubscribe()`. Never polled directly.
    template_receiver: broadcast::Receiver<RoutedItem>,
    stream: BroadcastStream<RoutedItem>,
    cleanup_sender: Option<mpsc::UnboundedSender<CleanupSignal>>,
    /// The registration's liveness, shared by clones, and its key; released
    /// before the cleanup signal is sent. `None` for shared-channel
    /// subscriptions, whose channels persist.
    lease: Option<(Lease, RouteKey)>,
    /// The shared-channel ticket, for a shared-channel subscription.
    shared: Option<SharedTicket>,
    /// Items this receiver has read, shared with a bounded route
    /// (`registry::RouteBound`) so it can tell how many are unread. `Sender::len()`
    /// can't: it counts values not yet seen by every receiver, and
    /// `template_receiver` never reads. Only the original handle counts;
    /// clones start at the tail with `None`.
    reads: Option<Arc<AtomicUsize>>,
    /// The class of the channel read, which picks the lag notice: on an
    /// order channel a lag means lost order state and is logged as an error.
    class: ChannelClass,
}

impl Clone for AsyncInternalSubscription {
    fn clone(&self) -> Self {
        // For clones, both template and stream start at the current tail of
        // the broadcast channel — clones see future messages, not the
        // original's history.
        let new_template = self.template_receiver.resubscribe();
        let new_polling = self.template_receiver.resubscribe();
        Self {
            template_receiver: new_template,
            stream: BroadcastStream::new(new_polling),
            cleanup_sender: self.cleanup_sender.clone(),
            // Each clone sends its own cleanup signal on drop; stale ones
            // no-op while another clone still holds the lease.
            lease: self.lease.clone(),
            shared: self.shared,
            reads: None,
            class: self.class,
        }
    }
}

impl AsyncInternalSubscription {
    /// Construct an internal subscription wrapping a broadcast receiver.
    ///
    /// **Receiver positioning matters.** The receiver you pass is what feeds
    /// the stream — its position in the broadcast channel determines what
    /// the subscription sees. Pass the receiver paired with the messages
    /// you want consumed (typically the `rx` returned alongside the `tx`).
    /// **Do not** pass `rx.resubscribe()` unless you specifically want to
    /// skip messages already queued in `rx`; `resubscribe()` positions a
    /// fresh receiver at the channel's *current tail*, which silently
    /// hides already-queued items and causes "test ran to completion but
    /// the assertion never fired" failures.
    #[cfg(test)]
    pub(crate) fn new(receiver: broadcast::Receiver<RoutedItem>) -> Self {
        let template = receiver.resubscribe();
        Self {
            template_receiver: template,
            stream: BroadcastStream::new(receiver),
            cleanup_sender: None,
            lease: None,
            shared: None,
            reads: None,
            class: ChannelClass::default(),
        }
    }

    pub(crate) fn with_cleanup(receiver: broadcast::Receiver<RoutedItem>, cleanup_sender: mpsc::UnboundedSender<CleanupSignal>) -> Self {
        let template = receiver.resubscribe();
        Self {
            template_receiver: template,
            stream: BroadcastStream::new(receiver),
            cleanup_sender: Some(cleanup_sender),
            lease: None,
            shared: None,
            reads: None,
            class: ChannelClass::default(),
        }
    }

    /// Hold `lease` for the registration under `key` this subscription reads from.
    pub(crate) fn leased(mut self, lease: Lease, key: RouteKey) -> Self {
        self.lease = Some((lease, key));
        self
    }

    /// Mark this a subscription to the shared channel `ticket` names.
    fn shared(mut self, ticket: SharedTicket) -> Self {
        self.shared = Some(ticket);
        self
    }

    /// Mark the class of the channel this subscription reads.
    fn class(mut self, class: ChannelClass) -> Self {
        self.class = class;
        self
    }

    /// Count this handle's reads into `reads`, for a bounded route.
    fn counting_reads(mut self, reads: Arc<AtomicUsize>) -> Self {
        self.reads = Some(reads);
        self
    }

    /// Poll the underlying broadcast stream, converting a `Lagged` error into
    /// an in-band [`SUBSCRIPTION_LAG_CODE`](crate::messages::SUBSCRIPTION_LAG_CODE)
    /// notice (which also logs: `error!` on an order-class channel, else
    /// `warn!`). The single place lag is handled — every
    /// consumer polls through here, so none can reintroduce a silent swallow.
    pub(crate) fn poll_next_routed(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<RoutedItem>> {
        use std::task::Poll;
        match std::pin::Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => {
                if let Some(reads) = &self.reads {
                    reads.fetch_add(1, Ordering::Relaxed);
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(Some(Err(BroadcastStreamRecvError::Lagged(skipped)))) => {
                let notice = match self.class {
                    ChannelClass::MarketData => crate::messages::subscription_lag_notice(skipped),
                    ChannelClass::Order | ChannelClass::OrderStream => crate::messages::order_lag_notice(skipped),
                };
                Poll::Ready(Some(RoutedItem::Notice(notice)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    pub async fn next(&mut self) -> Option<Result<ResponseMessage, Error>> {
        loop {
            let item = std::future::poll_fn(|cx| self.poll_next_routed(cx)).await?;
            // A lag notice maps to `None` here (`into_legacy` drops notices);
            // its `warn!` already fired inside `poll_next_routed`.
            if let Some(legacy) = item.into_legacy() {
                return Some(legacy);
            }
        }
    }

    /// Receive the next typed envelope (Response / Notice / Error) without
    /// the legacy projection. Kept because `src/transport/async_tests.rs`
    /// stub fixtures drive the channel directly via this helper.
    #[cfg(test)]
    pub(crate) async fn next_routed(&mut self) -> Option<RoutedItem> {
        std::future::poll_fn(|cx| self.poll_next_routed(cx)).await
    }

    /// Non-blocking poll for "is anything immediately available?". Returns
    /// `None` if the stream is pending or closed. Used by test fixtures that
    /// assert no cross-talk between subscriptions.
    #[cfg(test)]
    pub(crate) fn try_next_routed(&mut self) -> Option<RoutedItem> {
        use futures::FutureExt;
        std::future::poll_fn(|cx| self.poll_next_routed(cx)).now_or_never()?
    }

    /// The shared-channel ticket when this is a shared-channel subscription;
    /// `None` otherwise.
    pub(crate) fn shared_ticket(&self) -> Option<SharedTicket> {
        self.shared
    }

    /// Send the cleanup signal, releasing this handle's lease first.
    ///
    /// The order matters: the cleanup task removes a registration only once
    /// its lease is dead, and a lease released after the send (by the field
    /// drops that follow `drop(&mut self)`) could still count as live when a
    /// concurrently processed signal checks it — leaking the registration.
    fn send_cleanup_signal(&mut self) {
        let signal = match self.lease.take() {
            Some((lease, key)) => {
                let lease_ref = lease.downgrade();
                drop(lease);
                Some(key.signal(lease_ref))
            }
            None => self.shared.map(CleanupSignal::Shared),
        };
        let (Some(sender), Some(signal)) = (self.cleanup_sender.take(), signal) else {
            return;
        };
        let _ = sender.send(signal);
    }
}

/// Send cleanup signal when subscription is dropped
impl Drop for AsyncInternalSubscription {
    fn drop(&mut self) {
        self.send_cleanup_signal();
    }
}

/// Lock the order-update slot, recovering from poisoning: the slot holds no
/// invariant a panic could break.
fn lock_slot(slot: &std::sync::Mutex<Option<Route>>) -> std::sync::MutexGuard<'_, Option<Route>> {
    slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Asynchronous TCP message bus implementation
pub struct AsyncTcpMessageBus<S: AsyncStream = AsyncTcpSocket> {
    connection: Arc<AsyncConnection<S>>,
    requests: Arc<SenderHash<RequestId>>,
    orders: Arc<SenderHash<OrderId>>,
    /// Execution-id aliases of request and order routes, for the commission
    /// reports that follow an execution. Pruned when the owning request or
    /// order subscription is cleaned up.
    executions: Arc<SenderHash<String>>,
    shared_channels: SharedChannels,
    /// Optional channel for order update stream. A std lock, like the
    /// registries', so shutdown can empty it from `Drop`.
    order_update_stream: Arc<std::sync::Mutex<Option<Route>>>,
    /// Capacity of the market-data channels this bus creates, and the
    /// order-class floors' override when larger (see [`ChannelClass`]).
    /// Default `BROADCAST_CHANNEL_CAPACITY`; see `ClientBuilder::channel_capacity`.
    channel_capacity: usize,
    /// Channel for cleanup signals
    cleanup_sender: mpsc::UnboundedSender<CleanupSignal>,
    /// Held by a test to stall the cleanup task before its next signal.
    #[cfg(test)]
    cleanup_gate: Arc<tokio::sync::Mutex<()>>,
    /// Runtime the bus was built on; see [`AsyncMessageBus::runtime_handle`].
    runtime: Handle,
    /// Handle to the message processing task
    process_task: Arc<RwLock<Option<task::JoinHandle<()>>>>,
    /// Latching shutdown flag, shared with the connection so a reconnect in
    /// progress sees the request.
    shutdown: Arc<ShutdownSignal>,
    /// The client's order-ID generator, raised from the NextValidId frame the
    /// reconnect handshake re-receives. Installed once via
    /// [`Self::set_order_ids`] before the processing task starts; absent in
    /// bus-only test fixtures, which never reconnect a client.
    order_ids: OnceLock<Arc<ClientIdManager>>,
    /// Session state, and what `wait_connected` awaits.
    connection_state: Arc<ConnectionSignal>,
}

impl<S: AsyncStream> Drop for AsyncTcpMessageBus<S> {
    fn drop(&mut self) {
        debug!("dropping async tcp message bus");
        // Latch both flags; the message loop and any reconnect in progress
        // observe them on their next check, and no `wait_connected` is left
        // waiting on a session that is going away.
        self.connection_state.shutdown();
        self.shutdown.request();
    }
}

impl<S: AsyncStream> AsyncTcpMessageBus<S> {
    /// Create a bus with the default channel capacity. Production goes
    /// through `with_channel_capacity` (the builder owns the default), so
    /// this exists for test fixtures that don't care about capacity.
    #[cfg(test)]
    pub fn new(connection: AsyncConnection<S>) -> Result<Self, Error> {
        Self::with_channel_capacity(connection, BROADCAST_CHANNEL_CAPACITY)
    }

    /// Create a new async TCP message bus whose market-data channels hold up
    /// to `channel_capacity` frames per subscription before evicting the
    /// oldest. Order-class channels hold at least their [`ChannelClass`] floor.
    pub fn with_channel_capacity(connection: AsyncConnection<S>, channel_capacity: usize) -> Result<Self, Error> {
        let (cleanup_sender, cleanup_receiver) = mpsc::unbounded_channel();

        let shutdown = connection.shutdown_signal();

        let message_bus = Self {
            connection: Arc::new(connection),
            requests: Arc::new(SenderHash::new()),
            orders: Arc::new(SenderHash::new()),
            executions: Arc::new(SenderHash::new()),
            shared_channels: SharedChannels::new(|request| ChannelClass::of_shared(request).capacity(channel_capacity)),
            order_update_stream: Arc::new(std::sync::Mutex::new(None)),
            channel_capacity,
            cleanup_sender,
            #[cfg(test)]
            cleanup_gate: Arc::default(),
            runtime: Handle::current(),
            process_task: Arc::new(RwLock::new(None)),
            shutdown,
            order_ids: OnceLock::new(),
            connection_state: Arc::new(ConnectionSignal::default()),
        };

        // Start cleanup task
        let requests = message_bus.requests.clone();
        let orders = message_bus.orders.clone();
        let executions = message_bus.executions.clone();
        let order_update_stream = message_bus.order_update_stream.clone();
        #[cfg(test)]
        let cleanup_gate = message_bus.cleanup_gate.clone();

        // A signal can be processed arbitrarily long after the drop that sent
        // it — including after a newer subscription registered under the same
        // key — so removal is gated on the registration holding the signal's
        // lease, and that lease being dead. See `SenderHash::release` and
        // `AsyncInternalSubscription::send_cleanup_signal`.
        task::spawn(async move {
            let mut receiver = cleanup_receiver;
            while let Some(signal) = receiver.recv().await {
                #[cfg(test)]
                drop(cleanup_gate.lock().await);
                match signal {
                    CleanupSignal::Request(request_id, lease) => {
                        requests.release(request_id, &lease, "request");
                        executions.prune_dead();
                    }
                    CleanupSignal::Order(order_id, lease) => {
                        orders.release(order_id, &lease, "order");
                        executions.prune_dead();
                    }
                    CleanupSignal::Shared(ticket) => {
                        // Shared channels are persistent and should not be removed
                        // They are created at initialization and reused across multiple requests
                        debug!("Subscription for shared channel {:?} ended (channel remains active)", ticket.message_type);
                    }
                    CleanupSignal::OrderUpdateStream(lease) => {
                        let mut stream = lock_slot(&order_update_stream);
                        let removed = stream.as_ref().is_some_and(|route| route.released(&lease));
                        if removed {
                            *stream = None;
                        }
                        debug!("cleanup order update stream: removed={removed}");
                    }
                }
            }
        });

        Ok(message_bus)
    }

    /// Installs the client's order-ID generator so a successful reconnect
    /// re-seeds it from the handshake's NextValidId. Called exactly once,
    /// before [`Self::process_messages`] starts the processing task.
    pub(crate) fn set_order_ids(&self, order_ids: Arc<ClientIdManager>) {
        self.order_ids.set(order_ids).expect("order-id generator installed twice");
    }

    /// Start processing messages from TWS
    pub fn process_messages(self: Arc<Self>, _server_version: i32, _reconnect_delay: Duration) -> Result<(), Error> {
        let message_bus = self.clone();
        let shutdown = self.shutdown.clone();

        let handle = task::spawn(async move {
            loop {
                // The flag latches, so a request made while the loop was off
                // in `reconnect()` is still seen here.
                if shutdown.is_requested() {
                    debug!("Shutdown requested, stopping message processing");
                    break;
                }

                // Use select with shutdown notification instead of a polling sleep.
                // This prevents cancelling read_and_route_message mid-read, which
                // would corrupt the TCP stream (read_exact is not cancellation-safe).
                tokio::select! {
                    _ = shutdown.wait() => {
                        debug!("Shutdown notification received, stopping message processing");
                        break;
                    }
                    result = message_bus.read_and_route_message() => {
                        match result {
                            Ok(_) => continue,
                            Err(ref err) if err.is_read_timeout() => {
                                if message_bus.shutdown.is_requested() {
                                    debug!("dispatcher task exiting");
                                    break;
                                }
                                continue;
                            }
                            Err(ref err) if err.is_connection_lost() => {
                                error!("Connection error detected, attempting to reconnect: {err:?}");
                                message_bus.connection_state.set_disconnected();

                                // Fail every registered channel before
                                // reconnecting, not after: nothing they wait
                                // for can arrive on the dead session, and the
                                // reconnect can run for minutes.
                                message_bus.reset_channels().await;

                                match message_bus.connection.reconnect().await {
                                    Ok(_) => {
                                        // Shutdown may have been requested while the
                                        // connect was in flight; the reconnect
                                        // succeeded anyway, so check before reporting
                                        // the session as live.
                                        if message_bus.shutdown.is_requested() {
                                            debug!("shutdown requested during reconnect; dispatcher task exiting");
                                            break;
                                        }

                                        // The handshake re-received NextValidId; raise
                                        // the client's generator from the server's floor so
                                        // allocation never resumes below it. Only the initial
                                        // connection seeded the generator before this. Do it
                                        // before reporting the session as live so a caller
                                        // gating on `is_connected()` cannot allocate below
                                        // the new floor.
                                        if let Some(order_ids) = message_bus.order_ids.get() {
                                            let metadata = message_bus.connection.connection_metadata().await;
                                            order_ids.raise_order_id_from_server(metadata.next_order_id);
                                        }

                                        info!("Successfully reconnected to TWS/Gateway");
                                        message_bus.connection_state.set_connected();

                                        // The notice stream is the central carrier of
                                        // connection-status information (1100/1101/1102
                                        // land there), but TWS never sends socket reconnect
                                        // notices itself and does not replay restoration
                                        // notices on the new connection — so publish the
                                        // reconnect there the same way `report_unroutable_frame`
                                        // publishes decode failures: as a synthesized notice.
                                        // Published after the session is live so a consumer
                                        // that resubscribes on it lands on the new session,
                                        // into maps the reset above can no longer wipe.
                                        message_bus.connection.notice_broadcaster.broadcast(transport_reconnect_notice());
                                    }
                                    // Shutdown was requested while reconnecting:
                                    // not a failure, and the flag is already
                                    // set, so just exit the task.
                                    Err(Error::Shutdown) => {
                                        debug!("shutdown requested during reconnect; dispatcher task exiting");
                                        break;
                                    }
                                    Err(e) => {
                                        error!("Failed to reconnect to TWS/Gateway: {e:?}");
                                        break;
                                    }
                                }
                                continue;
                            }
                            Err(Error::Shutdown) => {
                                error!("Received shutdown signal, stopping message processing.");
                                break;
                            }
                            Err(err) => {
                                error!("Error processing message (shutting down): {err:?}");
                                break;
                            }
                        }
                    }
                }
            }

            // Every exit ends the session, so every exit clears the channels:
            // a reconnect that gave up or a fatal read error reaches here
            // without a shutdown request, and a live subscription holds the
            // bus, so without this its sender would never drop. Idempotent
            // when a shutdown already ran it.
            message_bus.request_shutdown();
        });

        // Store the task handle
        let process_task = self.process_task.clone();
        tokio::spawn(async move {
            let mut task_guard = process_task.write().await;
            *task_guard = Some(handle);
        });

        Ok(())
    }

    /// Read a message and route it to the appropriate channel
    pub(crate) async fn read_and_route_message(&self) -> Result<(), Error> {
        let message = self.connection.read_message().await?;

        // Use common routing logic
        match determine_routing(&message) {
            RoutingDecision::ByRequestId(id) => self.route_to_request_channel(id, message).await,
            RoutingDecision::ByOrderId(order_id) => self.route_to_order_channel(order_id, message).await,
            RoutingDecision::ByMessageType(message_type) => self.route_to_shared_channel(message_type, message).await,
            RoutingDecision::SharedMessage(message_type) => self.route_to_shared_channel(message_type, message).await,
            RoutingDecision::Error(payload) => self.route_error_message(payload).await,
            RoutingDecision::Shutdown => {
                debug!("Received shutdown message, calling request_shutdown");
                self.request_shutdown();
                Err(Error::Shutdown)
            }
        }
    }

    /// The send gate: every bus-originated write checks here first, and is
    /// refused with `Error::ConnectionReset` unless the session is live and
    /// not shutting down.
    ///
    /// Without it, a request sent while the connection is down would be lost
    /// on the dead socket - or, worse, interleave with the handshake the
    /// reconnect is writing on the new one - and the channel it registered
    /// would never be reset again, so its caller would await forever. Callers
    /// wait the reconnect out through `wait_connected` instead.
    ///
    /// The check-then-write window is not closed: a caller that passes here
    /// just before the dispatcher flips the state can still write, and in
    /// principle race the socket swap inside `reconnect`. Closing that needs a
    /// session-level gate held across the handshake.
    fn ensure_connected(&self) -> Result<(), Error> {
        if self.is_connected() {
            Ok(())
        } else {
            Err(Error::ConnectionReset)
        }
    }

    /// The one funnel for every bus-originated write, so no send can reach a
    /// socket the session no longer owns. The dispatcher's own reconnect
    /// handshake writes through `AsyncConnection`, not here.
    async fn write_message(&self, message: &[u8]) -> Result<(), Error> {
        self.ensure_connected()?;
        self.connection.write_message(message).await
    }

    /// Fail all registered channels with `Error::ConnectionReset`, before a
    /// reconnect is attempted.
    async fn reset_channels(&self) {
        debug!("resetting message bus channels");

        self.requests.fail_all(|| Error::ConnectionReset.into());
        self.orders.fail_all(|| Error::ConnectionReset.into());
        // Aliases of the routes just failed.
        self.executions.clear();
        // Shared channels too, mirroring sync's `fail_all`: an in-flight
        // open_orders/positions subscription awaits an end marker only the
        // pre-reconnect request could produce, so it would hang forever.
        // Unfiltered on purpose — `fail_one_shot_channels`' one-shot filter
        // protects live streams from *unrelated* errors, but a reset
        // terminates every stream by definition. The channels persist across
        // sessions, so they are not closed.
        self.shared_channels.notify_all(|| Error::ConnectionReset.into());
        self.shared_channels.reset_counts().await;
    }

    /// End every subscription with `Error::Shutdown`, then close the channels.
    /// Needs no runtime, so `Client::drop` runs it (`request_shutdown_sync`).
    fn request_shutdown(&self) {
        debug!("shutdown requested");

        // Both latch: the connection signal releases every `wait_connected`
        // with `Error::Shutdown`, and the shutdown flag stops the dispatcher.
        self.connection_state.shutdown();
        self.shutdown.request();

        // Fail every subscription, then drop its sender so it ends; the
        // sync bus does the same. Notice streams just end.
        self.requests.fail_all(|| Error::Shutdown.into());
        self.orders.fail_all(|| Error::Shutdown.into());
        // Execution aliases hold sender clones; clear them or the channels stay open.
        self.executions.clear();
        self.shared_channels.close(|| Error::Shutdown.into());
        // After the flag: `create_order_update_subscription` checks it under
        // the same lock, so no stream can register once this slot is emptied.
        if let Some(route) = lock_slot(&self.order_update_stream).take() {
            let _ = route.sender.send(Error::Shutdown.into());
        }

        self.connection.notice_broadcaster.close();
    }

    /// Route error message using routing decision
    async fn route_error_message(&self, payload: DecodedError) -> Result<(), Error> {
        let sent_to_update_stream = match order_update_notice(&payload) {
            Some(notice) => self.send_order_update_item(RoutedItem::Notice(notice)),
            None => false,
        };
        match classify_error(payload) {
            ErrorDisposition::NoticeOnly(notice) => {
                notice.log();
                self.connection.notice_broadcaster.broadcast(notice);
            }
            ErrorDisposition::NoticeAndFailOneShots(notice, error) => {
                notice.log();
                self.connection.notice_broadcaster.broadcast(notice);
                self.shared_channels.fail_one_shot_channels(|| RoutedItem::Error(error.clone()));
            }
            ErrorDisposition::Route(id, item) => {
                self.deliver(id, item, sent_to_update_stream).await;
            }
        }
        Ok(())
    }

    /// Deliver a pre-classified Notice or Error to the request or order
    /// subscription its id names.
    async fn deliver(&self, id: WireId, item: RoutedItem, sent_to_update_stream: bool) {
        let unrouted = match id {
            WireId::Request(request_id) => self.requests.deliver(&request_id, item),
            WireId::Order(order_id) => self.orders.deliver(&order_id, item),
        };
        if let Err(item) = unrouted {
            if !sent_to_update_stream {
                log_orphan(id, &item);
            }
        }
    }

    /// Route a frame to the request its id names. Only a request-range id
    /// can name one ([`RequestId::from_raw`]); the types routed here are data
    /// messages, which no order subscription reads.
    async fn route_to_request_channel(&self, id: i32, message: ResponseMessage) -> Result<(), Error> {
        if let Some(request_id) = RequestId::from_raw(id) {
            let _ = self.requests.deliver(&request_id, message.into());
        }
        Ok(())
    }

    /// Route message to order-specific channel
    async fn route_to_order_channel(&self, order_id: i32, message: ResponseMessage) -> Result<(), Error> {
        let routed = self.send_order_update(&message);
        let strategy = order_routing_strategy(message.message_type());
        let message_order_id = message.order_id().map(OrderId::from);
        let message_request_id = message.request_id().and_then(RequestId::from_raw);

        match strategy {
            OrderRoutingStrategy::OrderUpdateOnly => {}
            OrderRoutingStrategy::ExecutionData => {
                let execution_id = message.execution_id();
                if let Err(item) = self.deliver_to_order_or_request(message_order_id, message_request_id, message.into(), execution_id.as_ref()) {
                    if !routed {
                        warn!("could not route ExecutionData message {item:?}");
                    }
                }
            }
            OrderRoutingStrategy::ExecutionDataEnd => {
                if let Err(item) = self.deliver_to_order_or_request(message_order_id, message_request_id, message.into(), None) {
                    warn!("could not route ExecutionDataEnd message {item:?}");
                }
            }
            OrderRoutingStrategy::OrderOrShared => {
                if let Some(order_id) = message_order_id {
                    // `contains` first, not `deliver`'s hand-back: the shared
                    // fallback needs the message, and a second lookup is
                    // cheaper than cloning every frame.
                    if self.orders.contains(&order_id) {
                        let _ = self.orders.deliver(&order_id, message.into());
                        return Ok(());
                    }
                    if self.shared_channels.send_message(message.message_type(), &message) {
                        return Ok(());
                    }
                }
                if !routed {
                    warn!("could not route message {:?}", message);
                }
            }
            OrderRoutingStrategy::ByExecutionId => {
                let unrouted = match message.execution_id() {
                    Some(execution_id) => self.executions.deliver(&execution_id, message.into()),
                    None => Err(message.into()),
                };
                if let Err(item) = unrouted {
                    if !routed {
                        warn!("could not route commission report {item:?}");
                    }
                }
            }
            OrderRoutingStrategy::SharedOnly => {
                if !self.shared_channels.send_message(message.message_type(), &message) && !routed {
                    warn!("could not route message {:?}", message);
                }
            }
            OrderRoutingStrategy::ByOrderId => {
                let unrouted = if order_id >= 0 {
                    self.orders.deliver(&OrderId::from(order_id), message.into())
                } else {
                    Err(message.into())
                };
                if let Err(item) = unrouted {
                    if !routed {
                        warn!("could not route message {item:?}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Deliver to `order_id`'s route, else `request_id`'s, aliasing the route
    /// under `execution_id` for the commission report that follows. Hands
    /// the item back when neither is registered.
    fn deliver_to_order_or_request(
        &self,
        order_id: Option<OrderId>,
        request_id: Option<RequestId>,
        item: RoutedItem,
        execution_id: Option<&String>,
    ) -> Result<(), RoutedItem> {
        match order_id {
            Some(id) => self.orders.deliver_aliased(&id, item, execution_id, &self.executions),
            None => Err(item),
        }
        .or_else(|item| match request_id {
            Some(id) => self.requests.deliver_aliased(&id, item, execution_id, &self.executions),
            None => Err(item),
        })
    }

    /// Register a channel of `class` under `id` in `routes`, optionally with
    /// an unread-item cap (which sets the capacity), then write the request.
    /// `key` names the registration for the cleanup signal that releases it.
    async fn open_route<K: Hash + Eq + Copy + Display + Debug + Send + Sync>(
        &self,
        routes: &SenderHash<K>,
        id: K,
        key: fn(K) -> RouteKey,
        message: Vec<u8>,
        class: ChannelClass,
        bound: Option<BufferBound>,
    ) -> Result<AsyncInternalSubscription, Error> {
        self.ensure_connected()?;

        let capacity = bound.map_or(class.capacity(self.channel_capacity), |bound| bound.limit + 1);
        let (sender, receiver) = broadcast::channel(capacity);
        let lease = Lease::new();
        let lease_ref = lease.downgrade();
        let reads = bound.map(|_| Arc::new(AtomicUsize::new(0)));
        let route = match (bound, &reads) {
            (Some(bound), Some(reads)) => Route::bounded(sender, lease_ref.clone(), bound, reads.clone()),
            _ => Route::unbounded(sender, lease_ref.clone()),
        };
        routes.insert(id, route);

        // Owned before the write: a caller that drops this future while the
        // write is pending (a timeout, a `select!`) drops the subscription with
        // it, and its cleanup signal releases the registration. Code after the
        // `await` never runs in that case. On a failed write below, the drop
        // sends a second, harmless signal: `release` spares a replacement.
        let subscription = AsyncInternalSubscription::with_cleanup(receiver, self.cleanup_sender.clone())
            .leased(lease, key(id))
            .class(class);
        let subscription = match reads {
            Some(reads) => subscription.counting_reads(reads),
            None => subscription,
        };

        // The gate can close between `ensure_connected` and the write, so take
        // the registration back out on failure rather than leave a channel no
        // reset will clear. `remove_if_same` so a newer registration under the
        // same id survives.
        if let Err(e) = self.write_message(&message).await {
            routes.remove_if_same(id, &lease_ref);
            return Err(e);
        }

        Ok(subscription)
    }

    /// Route message to shared channel
    async fn route_to_shared_channel(&self, message_type: IncomingMessages, message: ResponseMessage) -> Result<(), Error> {
        // Send order-related messages to order update stream
        match message_type {
            IncomingMessages::OpenOrder
            | IncomingMessages::OrderStatus
            | IncomingMessages::ExecutionData
            | IncomingMessages::CommissionsReport
            | IncomingMessages::CompletedOrder => {
                self.send_order_update(&message);
            }
            _ => {}
        }

        if !self.shared_channels.send_message(message_type, &message) {
            // Nothing claimed the frame. Silent until now, which is why a
            // desynchronized stream looked identical to an idle one.
            report_unroutable_frame(&message, &self.connection.notice_broadcaster);
        }

        Ok(())
    }

    /// Send message to order update stream if it exists
    fn send_order_update(&self, message: &ResponseMessage) -> bool {
        self.send_order_update_item(message.clone().into())
    }

    fn send_order_update_item(&self, item: RoutedItem) -> bool {
        let order_update_stream = lock_slot(&self.order_update_stream);
        if let Some(route) = order_update_stream.as_ref() {
            if let Err(e) = route.sender.send(item) {
                warn!("error sending to order update stream: {e}");
                return false;
            }
            return true;
        }
        false
    }

    // Registers a subscription of `message_type` and writes its request;
    // `account` for account updates (see `SharedCounts`).
    async fn send_shared(
        &self,
        message_type: OutgoingMessages,
        account: Option<&AccountId>,
        message: Vec<u8>,
    ) -> Result<AsyncInternalSubscription, Error> {
        self.ensure_connected()?;

        let (receiver, ticket) = self
            .shared_channels
            .subscribe(message_type, account, || self.write_message(&message))
            .await?;

        Ok(AsyncInternalSubscription::with_cleanup(receiver, self.cleanup_sender.clone())
            .shared(ticket)
            .class(ChannelClass::of_shared(message_type)))
    }
}

#[async_trait]
impl<S: AsyncStream> AsyncMessageBus for AsyncTcpMessageBus<S> {
    async fn send_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.open_route(&self.requests, request_id, RouteKey::Request, message, ChannelClass::MarketData, None)
            .await
    }

    async fn send_request_bounded(&self, request_id: RequestId, message: Vec<u8>, bound: BufferBound) -> Result<AsyncInternalSubscription, Error> {
        self.open_route(
            &self.requests,
            request_id,
            RouteKey::Request,
            message,
            ChannelClass::MarketData,
            Some(bound),
        )
        .await
    }

    async fn send_executions_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.open_route(&self.requests, request_id, RouteKey::Request, message, ChannelClass::Order, None)
            .await
    }

    async fn send_order_request(&self, order_id: OrderId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.open_route(&self.orders, order_id, RouteKey::Order, message, ChannelClass::Order, None)
            .await
    }

    async fn send_shared_request(&self, message_type: OutgoingMessages, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.send_shared(message_type, None, message).await
    }

    async fn send_account_updates_request(&self, account: &AccountId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.send_shared(OutgoingMessages::RequestAccountData, Some(account), message).await
    }

    async fn cancel_shared_subscription(&self, ticket: SharedTicket, message: Option<Vec<u8>>) -> Result<(), Error> {
        self.shared_channels
            .unsubscribe(ticket, || async move {
                match message {
                    Some(message) => self.write_message(&message).await,
                    None => Ok(()),
                }
            })
            .await
    }

    async fn send_message(&self, message: Vec<u8>) -> Result<(), Error> {
        self.write_message(&message).await
    }

    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        let mut order_update_stream = lock_slot(&self.order_update_stream);

        // `request_shutdown` sets the flag before emptying this slot under the
        // same lock, so no stream can register past shutdown.
        if self.shutdown.is_requested() {
            return Err(Error::Shutdown);
        }

        // A registration with a dead lease is a dropped stream whose cleanup
        // signal has not been processed yet (see `SenderHash::release`);
        // replace it rather than refusing, so drop-then-recreate never races
        // the cleanup task.
        if order_update_stream.as_ref().is_some_and(|route| route.lease.is_live()) {
            return Err(Error::AlreadySubscribed);
        }

        let (sender, receiver) = broadcast::channel(ChannelClass::OrderStream.capacity(self.channel_capacity));
        let lease = Lease::new();

        *order_update_stream = Some(Route::unbounded(sender, lease.downgrade()));

        Ok(AsyncInternalSubscription::with_cleanup(receiver, self.cleanup_sender.clone())
            .leased(lease, RouteKey::OrderUpdateStream)
            .class(ChannelClass::OrderStream))
    }

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::async_impl::NoticeStream {
        crate::subscriptions::notice_stream::async_impl::NoticeStream::new(self.connection.notice_broadcaster.subscribe())
    }

    async fn ensure_shutdown(&self) {
        debug!("ensure_shutdown called");

        self.request_shutdown();

        // Wait for the processing task to finish
        let task_handle = {
            let mut task_guard = self.process_task.write().await;
            task_guard.take()
        };

        if let Some(handle) = task_handle {
            debug!("Waiting for processing task to finish");
            if let Err(e) = handle.await {
                warn!("Error joining processing task: {e}");
            }
            debug!("Processing task finished");
        }
    }

    fn request_shutdown_sync(&self) {
        self.request_shutdown();
    }

    async fn wait_connected(&self) -> Result<(), Error> {
        self.connection_state.wait_connected().await
    }

    fn is_connected(&self) -> bool {
        self.connection_state.is_connected() && !self.shutdown.is_requested()
    }

    fn runtime_handle(&self) -> &Handle {
        &self.runtime
    }
}

#[cfg(test)]
mod memory;
#[cfg(test)]
pub(crate) use memory::MemoryStream;

#[cfg(test)]
pub(crate) mod test_listener;

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "async_submission_tests.rs"]
mod submission_tests;
