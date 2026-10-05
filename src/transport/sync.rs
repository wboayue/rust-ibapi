//! This module implements a message bus for handling communications with TWS.
//! It provides functionality for routing requests from the Client to TWS,
//! and responses from TWS back to the Client.

use std::collections::{hash_map, HashMap, HashSet};
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info, trace, warn};

use crate::accounts::types::AccountId;
use crate::client::id_generator::ClientIdManager;
use crate::client::ids::{OrderId, RequestId, WireId};
use crate::connection::sync::Connection;

use super::common::{log_orphan, report_unroutable_frame, validate_frame_length, Lease, LeaseRef};
use super::raw_capture::RawFrameTap;
use super::routing::{
    classify_error, determine_routing, order_routing_strategy, order_update_notice, DecodedError, ErrorDisposition, OrderRoutingStrategy,
    RoutingDecision,
};
use super::{
    Admit, BoundState, BufferBound, InternalSubscription, MessageBus, Response, RoutedItem, SharedCounts, SharedTicket, Signal, SubscriptionBuilder,
};
use crate::messages::{shared_channel_configuration, transport_reconnect_notice, IncomingMessages, Notice, OutgoingMessages, ResponseMessage};
use crate::subscriptions::notice_stream::sync_impl::NoticeStream;
use crate::Error;

// pub(crate) const MIN_SERVER_VERSION: i32 = 100;
// pub(crate) const MAX_SERVER_VERSION: i32 = server_versions::WSH_EVENT_DATA_FILTERS_DATE;
const TWS_READ_TIMEOUT: Duration = Duration::from_secs(1);

/// How long a frame may stall after its first byte before the read gives up
/// and the dispatcher reconnects. A healthy gateway never pauses this long
/// inside a frame.
const MID_FRAME_STALL: Duration = Duration::from_secs(30);

/// [`MID_FRAME_STALL`] in read timeouts: [`FrameRead`] counts timeouts rather
/// than reading a clock.
const MID_FRAME_TIMEOUT_LIMIT: u32 = (MID_FRAME_STALL.as_millis() / TWS_READ_TIMEOUT.as_millis()) as u32;
const _: () = assert!(MID_FRAME_TIMEOUT_LIMIT > 0, "MID_FRAME_STALL must exceed TWS_READ_TIMEOUT");

/// How long connect waits for each startup frame (handshake ack, account
/// info) before giving up. A loaded gateway can take longer than one read
/// timeout to answer.
const STARTUP_STALL: Duration = Duration::from_secs(30);

/// [`STARTUP_STALL`] in read timeouts, counted like [`MID_FRAME_TIMEOUT_LIMIT`].
pub(crate) const STARTUP_TIMEOUT_LIMIT: u32 = (STARTUP_STALL.as_millis() / TWS_READ_TIMEOUT.as_millis()) as u32;
const _: () = assert!(STARTUP_TIMEOUT_LIMIT > 0, "STARTUP_STALL must exceed TWS_READ_TIMEOUT");

/// Queue depth at which (and at every further multiple of which) a growing
/// sync channel logs a warning. Sync channels are unbounded — they never drop,
/// so the failure mode of a stalled consumer is silent memory growth. The
/// watermark makes it loud without changing the lossless semantics; see
/// #779 and #896.
const BACKLOG_WATERMARK: usize = 10_000;

/// `true` exactly when `depth` sits on a watermark multiple — one warning per
/// 10k messages of backlog, not one per message. The dispatcher is the sole
/// producer and reads the depth after each single-message send, so an upward
/// crossing cannot skip past the multiple.
fn backlog_watermark_crossed(depth: usize) -> bool {
    depth > 0 && depth.is_multiple_of(BACKLOG_WATERMARK)
}

/// Warn when a queue's depth crosses a watermark; `label` names the queue.
fn warn_if_backlogged(label: std::fmt::Arguments<'_>, depth: usize) {
    if backlog_watermark_crossed(depth) {
        warn!("{label} at {depth} messages and growing — consumer is stalling");
    }
}

// One live subscription to a shared (id-less) request. Registered under
// every response type of its request mapping; `sender` feeds its own queue.
#[derive(Debug)]
struct SharedSubscriber {
    request: OutgoingMessages,
    responses: &'static [IncomingMessages],
    sender: Sender<RoutedItem>,
    lease: LeaseRef,
}

impl SharedSubscriber {
    fn receives(&self, message_type: IncomingMessages) -> bool {
        self.responses.contains(&message_type)
    }
}

// For requests without an identifier, responses are routed by message type
// to every live subscription of a request mapped to that type. Each
// subscription owns its own queue (like the async broadcast `resubscribe`),
// so every subscriber sees every frame, nothing dispatched before it
// subscribed reaches it, and nothing is queued for a subscriber that no
// longer exists.
#[derive(Debug)]
struct SharedChannels {
    // Every response type of `CHANNEL_MAPPINGS`; a frame of one of these
    // types is a shared response whether or not anybody is subscribed.
    response_types: HashSet<IncomingMessages>,
    // Live subscriptions, added in `send_shared_request` and removed by the
    // cleanup thread on cancel or drop or, lazily, when a send finds the
    // queue gone.
    subscribers: Mutex<Vec<SharedSubscriber>>,
    // Live subscriptions per request type; see `SharedCounts`.
    counts: Mutex<SharedCounts>,
}

impl SharedChannels {
    pub fn new() -> Self {
        Self {
            response_types: shared_channel_configuration::CHANNEL_MAPPINGS
                .iter()
                .flat_map(|mapping| mapping.responses.iter().copied())
                .collect(),
            subscribers: Mutex::new(Vec::new()),
            counts: Mutex::new(SharedCounts::default()),
        }
    }

    fn subscribers(&self) -> MutexGuard<'_, Vec<SharedSubscriber>> {
        self.subscribers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    // Registers `sender` for every response type of `request`. Panics if
    // `request` has no mapping.
    fn add(&self, request: OutgoingMessages, sender: Sender<RoutedItem>, lease: LeaseRef) {
        let responses = shared_channel_configuration::response_types(request)
            .unwrap_or_else(|| panic!("unsupported request message {request:?}. check mapping in messages::shared_channel_configuration"));
        self.subscribers().push(SharedSubscriber {
            request,
            responses,
            sender,
            lease,
        });
    }

    // Removes the subscription holding `lease`, if still registered.
    fn remove(&self, lease: &LeaseRef) {
        let mut subscribers = self.subscribers();
        let before = subscribers.len();
        subscribers.retain(|subscriber| !subscriber.lease.is(lease));
        debug!("cleanup shared subscription: removed={}", before != subscribers.len());
    }

    fn is_shared_response(&self, message_type: IncomingMessages) -> bool {
        self.response_types.contains(&message_type)
    }

    // Runs `write` for a new subscription of `message_type` and counts it on
    // success. The lock spans the write so the count and the wire agree.
    // `account` is the account-updates account, checked before the write.
    fn subscribe(
        &self,
        message_type: OutgoingMessages,
        account: Option<&AccountId>,
        write: impl FnOnce() -> Result<(), Error>,
    ) -> Result<SharedTicket, Error> {
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        counts.check_account_updates(account)?;
        write()?;
        Ok(counts.subscribe(message_type, account))
    }

    // Uncounts `ticket`'s subscription; runs `write` (the cancel) only when
    // `SharedCounts::unsubscribe` says so.
    fn unsubscribe(&self, ticket: SharedTicket, write: impl FnOnce() -> Result<(), Error>) -> Result<(), Error> {
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        if counts.unsubscribe(ticket) {
            write()
        } else {
            Ok(())
        }
    }

    // Every live shared subscription has just been failed: start a new
    // generation so their later drops cannot touch the next session's counts.
    fn reset_counts(&self) {
        self.counts.lock().unwrap_or_else(PoisonError::into_inner).reset();
    }

    // Sends `item()` to every subscriber selected by `filter`; returns how
    // many were selected. A send fails only when the subscriber's queue is
    // gone (its handle was dropped and the cleanup signal has not been
    // processed yet); it is removed here.
    fn send_to<F, I>(&self, filter: F, item: I) -> usize
    where
        F: Fn(&SharedSubscriber) -> bool,
        I: Fn() -> RoutedItem,
    {
        let mut selected = 0;
        self.subscribers().retain(|subscriber| {
            if !filter(subscriber) {
                return true;
            }
            selected += 1;
            match subscriber.sender.send(item()) {
                Ok(()) => {
                    warn_if_backlogged(format_args!("shared channel for {:?}", subscriber.request), subscriber.sender.len());
                    true
                }
                Err(_) => {
                    debug!("shared subscription {:?} dropped: removed", subscriber.request);
                    false
                }
            }
        });
        selected
    }

    // Deliver `message` to every live subscription registered for its type.
    // With none, the frame is dropped: a shared response nobody asked for
    // (e.g. `OpenOrder` from another client, or after the requester's drop).
    fn send_message(&self, message_type: IncomingMessages, message: &ResponseMessage) {
        if self.send_to(|subscriber| subscriber.receives(message_type), || message.clone().into()) == 0 {
            debug!("no shared subscription for {message_type:?}: frame dropped");
        }
    }

    // Deliver `message_fn()` once to every live subscription, then remove
    // every registration under the same lock, so one made meanwhile can't be
    // dropped unnotified. A failed handle's queue must not keep filling until
    // it is dropped.
    fn fail_all<F>(&self, message_fn: F)
    where
        F: Fn() -> RoutedItem,
    {
        let mut subscribers = self.subscribers();
        for subscriber in subscribers.iter() {
            let _ = subscriber.sender.send(message_fn());
        }
        subscribers.clear();
    }

    // Fail in-flight one-shot requests fast by delivering an error to the
    // one-shot subscriptions only. Used for request-less errors, which carry
    // no id to correlate. Streaming subscriptions are excluded so an
    // unrelated error can't terminate a live stream.
    fn fail_one_shot_channels<F>(&self, error_fn: F)
    where
        F: Fn() -> RoutedItem,
    {
        let one_shot = shared_channel_configuration::exclusive_one_shot_response_types();
        self.send_to(|subscriber| subscriber.responses.iter().any(|r| one_shot.contains(r)), error_fn);
    }
}

/// Fan-out for unrouted notices. Each subscriber gets its own unbounded
/// crossbeam channel, watermarked like the subscription queues; `broadcast`
/// lazily prunes subscribers whose receivers have been dropped
/// (`Sender::send` returns `Err` once the receiver is gone).
#[derive(Debug)]
pub(crate) struct NoticeBroadcaster {
    /// `None` once closed.
    senders: Mutex<Option<Vec<Sender<Notice>>>>,
}

impl super::common::NoticeSink for NoticeBroadcaster {
    fn deliver(&self, notice: Notice) {
        self.broadcast(notice);
    }
}

impl NoticeBroadcaster {
    pub(crate) fn new() -> Self {
        Self {
            senders: Mutex::new(Some(Vec::new())),
        }
    }

    /// After `close`, the returned receiver is already at end-of-stream,
    /// like the ones `close` ended.
    pub(crate) fn subscribe(&self) -> Receiver<Notice> {
        let (sender, receiver) = channel::unbounded();
        if let Some(senders) = self.senders.lock().unwrap().as_mut() {
            senders.push(sender);
        }
        receiver
    }

    pub(crate) fn broadcast(&self, notice: Notice) {
        if let Some(senders) = self.senders.lock().unwrap().as_mut() {
            senders.retain(|s| {
                let sent = s.send(notice.clone()).is_ok();
                if sent {
                    warn_if_backlogged(format_args!("notice stream"), s.len());
                }
                sent
            });
        }
    }

    /// Drop all senders so existing receivers see channel-closed, and end
    /// every later subscription on arrival.
    pub(crate) fn close(&self) {
        *self.senders.lock().unwrap() = None;
    }
}

/// Lock the order-update slot, recovering from poisoning: the slot holds no
/// invariant a panic could break.
fn lock_slot(slot: &Mutex<Option<Entry<RoutedItem>>>) -> MutexGuard<'_, Option<Entry<RoutedItem>>> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug)]
pub struct TcpMessageBus<S: Stream> {
    connection: Connection<S>,
    handles: Mutex<Vec<JoinHandle<()>>>,
    requests: SenderHash<RequestId, RoutedItem>,
    orders: SenderHash<OrderId, RoutedItem>,
    executions: SenderHash<String, RoutedItem>,
    shared_channels: SharedChannels,
    signals_send: Sender<Signal>,
    signals_recv: Receiver<Signal>,
    shutdown_send: Sender<()>,
    shutdown_recv: Receiver<()>,
    /// Shared with the connection so a reconnect in progress sees the request.
    shutdown: Arc<ShutdownSignal>,
    /// The client's order-ID generator, raised from the NextValidId frame the
    /// reconnect handshake re-receives. Installed once via
    /// [`Self::set_order_ids`] before the dispatcher thread starts; absent in
    /// bus-only test fixtures, which never reconnect a client.
    order_ids: OnceLock<Arc<ClientIdManager>>,
    order_update_stream: Mutex<Option<Entry<RoutedItem>>>,
    /// Session state, and what `wait_connected` blocks on.
    connection_state: ConnectionSignal,
}

impl<S: Stream> TcpMessageBus<S> {
    pub fn new(connection: Connection<S>) -> Result<TcpMessageBus<S>, Error> {
        let (signals_send, signals_recv) = channel::unbounded();
        let (shutdown_send, shutdown_recv) = channel::bounded(1);
        let shutdown = connection.shutdown_signal();

        Ok(TcpMessageBus {
            connection,
            handles: Mutex::new(Vec::default()),
            requests: SenderHash::new(),
            orders: SenderHash::new(),
            executions: SenderHash::new(),
            shared_channels: SharedChannels::new(),
            signals_send,
            signals_recv,
            shutdown_send,
            shutdown_recv,
            shutdown,
            order_ids: OnceLock::new(),
            order_update_stream: Mutex::new(None),
            connection_state: ConnectionSignal::default(),
        })
    }

    /// Installs the client's order-ID generator so a successful reconnect
    /// re-seeds it from the handshake's NextValidId. Called exactly once,
    /// before the dispatcher thread starts.
    pub(crate) fn set_order_ids(&self, order_ids: Arc<ClientIdManager>) {
        self.order_ids.set(order_ids).expect("order-id generator installed twice");
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown.is_requested()
    }

    fn request_shutdown(&self) {
        debug!("shutdown requested");

        self.requests.fail_all(|| Error::Shutdown.into());
        self.orders.fail_all(|| Error::Shutdown.into());
        self.shared_channels.fail_all(|| Error::Shutdown.into());
        // Aliases of the routes just failed.
        self.executions.clear();
        self.connection.notice_broadcaster.close();

        // Both latch: the connection signal releases every `wait_connected`
        // with `Error::Shutdown`, and the shutdown signal ends any backoff
        // wait `Connection::reconnect` is in.
        self.connection_state.shutdown();
        self.shutdown.request();

        // After the flag: `create_order_update_subscription` checks it under
        // the same lock, so no stream can register once this slot is emptied.
        // The subscription holds a sender clone, so only a sent item ends it.
        if let Some(entry) = lock_slot(&self.order_update_stream).take() {
            let _ = entry.sender.send(Error::Shutdown.into());
        }

        // bounded(1) + try_send: if a shutdown is already pending,
        // Err(Full) is the desired no-op (idempotent across duplicate calls).
        let _ = self.shutdown_send.try_send(());

        // Break the dispatcher's blocked read so it exits without waiting
        // for the 1s socket-read timeout. Errors are non-fatal — a closed
        // or already-shutdown socket still terminates the read.
        if let Err(e) = self.connection.shutdown_read() {
            debug!("shutdown_read returned: {e:?}");
        }
    }

    /// The send gate: every bus-originated write checks here first, and is
    /// refused with `Error::ConnectionReset` unless the session is live and
    /// not shutting down.
    ///
    /// Without it, a request sent while the connection is down would be lost
    /// on the dead socket - or, worse, interleave with the handshake the
    /// reconnect is writing on the new one - and the channel it registered
    /// would never be reset again, so its caller would block forever. Callers
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
    /// handshake writes through `Connection`, not here.
    fn write_message(&self, message: &[u8]) -> Result<(), Error> {
        self.ensure_connected()?;
        self.connection.write_message(message)
    }

    fn reset(&self) {
        debug!("reset message bus");

        self.requests.fail_all(|| Error::ConnectionReset.into());
        self.orders.fail_all(|| Error::ConnectionReset.into());
        self.shared_channels.fail_all(|| Error::ConnectionReset.into());
        self.shared_channels.reset_counts();
        // Aliases of the routes just failed.
        self.executions.clear();
    }

    // The three cleanup handlers below remove a registration only when it
    // holds the cancelled or dropped subscription's lease: a signal can be
    // processed arbitrarily late, and unconditional removal would take out a
    // newer registration under the same key (place then cancel on one order
    // id, or an order update stream recreated after a reconnect reset).
    // Identity alone, not liveness: a sync subscription has one holder, and
    // its lease is released before its signal is sent, at cancel or drop.
    //
    // `clean_request` and `clean_order` also drop the subscription's
    // execution-id aliases, matched by lease rather than key, so a stale
    // signal still releases its own aliases and never a newer registration's.
    // Not gated on `removed`: a stale signal still owns aliases to release.

    fn clean_request(&self, request_id: RequestId, lease: &LeaseRef) {
        let removed = self.requests.remove_if_same(request_id, lease);
        let aliases = self.executions.remove_all_same(lease);
        debug!(
            "cleanup request_id {request_id}: removed={removed}, aliases={aliases}, requests.len()={}",
            self.requests.len()
        );
    }

    fn clean_order(&self, order_id: OrderId, lease: &LeaseRef) {
        let removed = self.orders.remove_if_same(order_id, lease);
        let aliases = self.executions.remove_all_same(lease);
        debug!(
            "cleanup order_id {order_id}: removed={removed}, aliases={aliases}, orders.len()={}",
            self.orders.len()
        );
    }

    fn clear_order_update_stream(&self, lease: &LeaseRef) {
        let mut stream = lock_slot(&self.order_update_stream);
        let removed = stream.as_ref().is_some_and(|registered| registered.lease.is(lease));
        if removed {
            *stream = None;
        }
        debug!("cleanup order_update_stream: removed={removed}");
    }

    fn read_message(&self) -> Response {
        self.connection.read_message()
    }
    pub(crate) fn dispatch(&self) -> Result<(), Error> {
        match self.read_message() {
            Ok(message) => {
                if message.is_shutdown() {
                    self.request_shutdown();
                    Err(Error::Shutdown)
                } else {
                    self.dispatch_message(message);
                    Ok(())
                }
            }
            Err(ref err) if err.is_read_timeout() => {
                if self.is_shutting_down() {
                    debug!("dispatcher thread exiting");
                    return Err(Error::Shutdown);
                }
                Ok(())
            }
            Err(ref err) if err.is_connection_lost() => {
                if self.is_shutting_down() {
                    debug!("dispatcher thread exiting");
                    return Err(Error::Shutdown);
                }
                error!("error reading next message (will attempt reconnect): {err:?}");
                self.connection_state.set_disconnected();

                // Fail every registered channel before reconnecting, not
                // after: nothing they wait for can arrive on the dead
                // session, and the reconnect can run for minutes.
                self.reset();

                match self.connection.reconnect() {
                    Ok(()) => {}
                    // Shutdown was requested while reconnecting: not a failure,
                    // and the flag is already set, so just exit the dispatcher.
                    Err(Error::Shutdown) => {
                        debug!("shutdown requested during reconnect; dispatcher thread exiting");
                        return Err(Error::Shutdown);
                    }
                    Err(reconnect_err) => {
                        error!("failed to reconnect to TWS/Gateway: {reconnect_err:?}");
                        self.request_shutdown();
                        return Err(Error::ConnectionFailed);
                    }
                }

                // Shutdown may have been requested while the connect was in
                // flight; the reconnect succeeded anyway, so check before
                // reporting the session as live.
                if self.is_shutting_down() {
                    debug!("shutdown requested during reconnect; dispatcher thread exiting");
                    return Err(Error::Shutdown);
                }

                // The handshake re-received NextValidId; raise the client's
                // generator from the server's floor so allocation never
                // resumes below it. Only the initial connection seeded the
                // generator before this. Do it before reporting the session
                // as live so a caller gating on `is_connected()` cannot
                // allocate below the new floor.
                if let Some(order_ids) = self.order_ids.get() {
                    order_ids.raise_order_id_from_server(self.connection.connection_metadata().next_order_id);
                }

                info!("successfully reconnected to TWS/Gateway");
                self.connection_state.set_connected();

                // TWS never frames the socket reconnect itself and does not replay
                // restoration notices on the new connection, so the notice fan-out —
                // the carrier 1100/1101/1102 arrive on — learns the socket generation
                // changed from this synthesized notice alone; see
                // `TRANSPORT_RECONNECT_CODE`. Published after the session is live
                // so a consumer that resubscribes on it lands on the new session,
                // into maps the reset above can no longer wipe.
                self.connection.notice_broadcaster.broadcast(transport_reconnect_notice());
                Ok(())
            }
            Err(err) => {
                error!("error reading next message (shutting down): {err:?}");
                self.request_shutdown();
                Err(err)
            }
        }
    }

    // Dispatcher thread reads messages from TWS and dispatches them to
    // appropriate channel.
    fn start_dispatcher_thread(self: &Arc<Self>) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);
        thread::spawn(move || {
            loop {
                match message_bus.dispatch() {
                    Ok(_) => {}
                    Err(Error::Shutdown | Error::ConnectionFailed) => break,
                    Err(e) => {
                        error!("Dispatcher encountered an error: {e:?}");
                        break;
                    }
                }
            }
            debug!("Dispatcher thread finished.");
        })
    }

    fn dispatch_message(&self, message: ResponseMessage) {
        match determine_routing(&message) {
            RoutingDecision::Error(payload) => self.route_error_message(payload),
            RoutingDecision::ByOrderId(_) => {
                // Order-related messages
                self.process_orders(message);
            }
            RoutingDecision::ByRequestId(id) => {
                self.process_response_with_id(WireId::classify(id), message, false);
            }
            _ => {
                // All other messages
                self.process_response(message, false);
            }
        }
    }

    /// Route an error frame by severity and request id. Mirrors the async
    /// transport's `route_error_message`.
    fn route_error_message(&self, payload: DecodedError) {
        let sent_to_update_stream = order_update_notice(&payload).is_some_and(|notice| self.send_order_update_item(RoutedItem::Notice(notice)));
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
                self.deliver(id, item, sent_to_update_stream);
            }
        }
    }

    fn process_response(&self, message: ResponseMessage, routed: bool) {
        let id = message.request_id().and_then(WireId::classify);
        self.process_response_with_id(id, message, routed);
    }

    /// The id's range picks the table: a request and an order can never be
    /// confused, whatever is registered under the number.
    fn process_response_with_id(&self, id: Option<WireId>, message: ResponseMessage, routed: bool) {
        match id {
            Some(WireId::Request(request_id)) if self.requests.contains(&request_id) => {
                let _ = self.requests.deliver(&request_id, message.into());
                return;
            }
            Some(WireId::Order(order_id)) if self.orders.contains(&order_id) => {
                let _ = self.orders.deliver(&order_id, message.into());
                return;
            }
            _ => {}
        }
        if self.shared_channels.is_shared_response(message.message_type()) {
            self.shared_channels.send_message(message.message_type(), &message);
        } else if !routed {
            report_unroutable_frame(&message, &*self.connection.notice_broadcaster);
        }
    }

    /// Deliver a pre-classified Notice or Error to the request or order
    /// subscription its id names.
    fn deliver(&self, id: WireId, item: RoutedItem, sent_to_update_stream: bool) {
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

    fn process_orders(&self, message: ResponseMessage) {
        let strategy = order_routing_strategy(message.message_type());
        let message_order_id = message.order_id().map(OrderId::from);
        let message_request_id = message.request_id().and_then(RequestId::from_raw);

        match strategy {
            OrderRoutingStrategy::OrderUpdateOnly => {
                self.send_order_update(&message);
            }
            OrderRoutingStrategy::ExecutionData => {
                let sent_to_update_stream = self.send_order_update(&message);
                let execution_id = message.execution_id();
                if let Err(item) = self.deliver_to_order_or_request(message_order_id, message_request_id, message.into(), execution_id.as_ref()) {
                    if !sent_to_update_stream {
                        warn!("could not route message {item:?}");
                    }
                }
            }
            OrderRoutingStrategy::ExecutionDataEnd => {
                if let Err(item) = self.deliver_to_order_or_request(message_order_id, message_request_id, message.into(), None) {
                    warn!("could not route message {item:?}");
                }
            }
            OrderRoutingStrategy::OrderOrShared => {
                let sent_to_update_stream = self.send_order_update(&message);

                if let Some(order_id) = message_order_id {
                    if self.orders.contains(&order_id) {
                        let _ = self.orders.deliver(&order_id, message.into());
                    } else {
                        self.shared_channels.send_message(message.message_type(), &message);
                    }
                    return;
                }
                if !sent_to_update_stream {
                    warn!("could not route message {message:?}");
                }
            }
            OrderRoutingStrategy::ByExecutionId => {
                let sent_to_update_stream = self.send_order_update(&message);

                let unrouted = match message.execution_id() {
                    Some(execution_id) => self.executions.deliver(&execution_id, message.into()),
                    None => Err(message.into()),
                };
                if let Err(item) = unrouted {
                    if !sent_to_update_stream {
                        warn!("could not route commission report {item:?}");
                    }
                }
            }
            OrderRoutingStrategy::SharedOnly => {
                self.shared_channels.send_message(message.message_type(), &message);
            }
            OrderRoutingStrategy::ByOrderId => {
                warn!("unhandled order message type: {message:?}");
            }
        }
    }

    /// Register `request_id`'s channel, optionally with an unread-item cap,
    /// then write the request.
    fn open_request(&self, request_id: RequestId, message: &[u8], bound: Option<BufferBound>) -> Result<InternalSubscription, Error> {
        self.ensure_connected()?;

        let (sender, receiver) = channel::unbounded();
        let lease = Lease::new();
        let lease_ref = lease.downgrade();

        match bound {
            Some(bound) => self.requests.insert_bounded(request_id, sender.clone(), lease_ref.clone(), bound),
            None => self.requests.insert(request_id, sender.clone(), lease_ref.clone()),
        };

        // The gate can close between `ensure_connected` and the write, so take
        // the registration back out on failure rather than leave a channel no
        // reset will clear. `remove_if_same` so a newer registration under the
        // same id survives.
        if let Err(e) = self.write_message(message) {
            self.requests.remove_if_same(request_id, &lease_ref);
            return Err(e);
        }

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender)
            .signaler(self.signals_send.clone())
            .lease(lease)
            .request_id(request_id)
            .build();

        Ok(subscription)
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

    // Sends an order update message to the order update stream if it exists.
    // Returns true if the message was sent to the order update stream.
    fn send_order_update(&self, message: &ResponseMessage) -> bool {
        self.send_order_update_item(message.clone().into())
    }

    fn send_order_update_item(&self, item: RoutedItem) -> bool {
        let order_update_stream = lock_slot(&self.order_update_stream);
        let Some(entry) = order_update_stream.as_ref() else {
            return false;
        };
        if let Err(e) = entry.sender.send(item) {
            warn!("error sending to order update stream: {e}");
            return false;
        }
        warn_if_backlogged(format_args!("order update stream"), entry.sender.len());
        true
    }

    // The cleanup thread receives signals as subscribers are cancelled or
    // dropped and releases the sender channels. Exits promptly on shutdown via
    // select! over the signal channel and the shutdown-notify channel — no
    // polling.
    fn start_cleanup_thread(self: &Arc<Self>) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);

        thread::spawn(move || {
            let signal_recv = message_bus.signals_recv.clone();
            let shutdown_recv = message_bus.shutdown_recv.clone();

            loop {
                crossbeam::select! {
                    recv(signal_recv) -> signal => match signal {
                        Ok(Signal::Request(request_id, lease)) => message_bus.clean_request(request_id, &lease),
                        Ok(Signal::Order(order_id, lease)) => message_bus.clean_order(order_id, &lease),
                        Ok(Signal::OrderUpdateStream(lease)) => message_bus.clear_order_update_stream(&lease),
                        Ok(Signal::Shared(lease)) => message_bus.shared_channels.remove(&lease),
                        Err(_) => {
                            debug!("cleanup signal channel closed");
                            return;
                        }
                    },
                    recv(shutdown_recv) -> _ => {
                        debug!("cleanup thread exiting");
                        return;
                    }
                }
            }
        })
    }

    pub(crate) fn process_messages(self: &Arc<Self>, _server_version: i32) -> Result<(), Error> {
        let handle = self.start_dispatcher_thread();
        self.add_join_handle(handle);

        let handle = self.start_cleanup_thread();
        self.add_join_handle(handle);

        Ok(())
    }

    fn add_join_handle(&self, handle: JoinHandle<()>) {
        let mut handles = self.handles.lock().unwrap();
        handles.push(handle);
    }

    pub fn join(&self) {
        let mut handles = self.handles.lock().unwrap();

        for handle in handles.drain(..) {
            if let Err(e) = handle.join() {
                warn!("could not join thread: {e:?}");
            }
        }
    }

    // Registers a subscription of `message_type` and writes its request;
    // `account` for account updates (see `SharedCounts`).
    fn send_shared(&self, message_type: OutgoingMessages, account: Option<&AccountId>, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.ensure_connected()?;

        // A queue of its own, registered before the write so no response can
        // arrive ahead of it. A failed write or a refused account takes the
        // registration with it.
        let (sender, receiver) = channel::unbounded();
        let lease = Lease::new();
        let lease_ref = lease.downgrade();
        self.shared_channels.add(message_type, sender.clone(), lease_ref.clone());
        let ticket = match self.shared_channels.subscribe(message_type, account, || self.write_message(message)) {
            Ok(ticket) => ticket,
            Err(e) => {
                self.shared_channels.remove(&lease_ref);
                return Err(e);
            }
        };

        // The lease is the drop signal's identity: `Signal::Shared` removes
        // exactly this registration.
        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender)
            .signaler(self.signals_send.clone())
            .lease(lease)
            .shared(ticket)
            .build();

        Ok(subscription)
    }
}

impl<S: Stream> MessageBus for TcpMessageBus<S> {
    fn send_request(&self, request_id: RequestId, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.open_request(request_id, message, None)
    }

    fn send_request_bounded(&self, request_id: RequestId, message: &[u8], bound: BufferBound) -> Result<InternalSubscription, Error> {
        self.open_request(request_id, message, Some(bound))
    }

    fn send_order_request(&self, order_id: OrderId, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.ensure_connected()?;

        let (sender, receiver) = channel::unbounded();
        let lease = Lease::new();
        let lease_ref = lease.downgrade();

        self.orders.insert(order_id, sender.clone(), lease_ref.clone());
        debug!("Registered order subscription for order_id={}", order_id);

        // See `send_request`: a failed write takes its registration with it.
        if let Err(e) = self.write_message(message) {
            self.orders.remove_if_same(order_id, &lease_ref);
            return Err(e);
        }

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender)
            .signaler(self.signals_send.clone())
            .lease(lease)
            .order_id(order_id)
            .build();

        Ok(subscription)
    }

    fn send_message(&self, message: &[u8]) -> Result<(), Error> {
        self.write_message(message)?;
        Ok(())
    }

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error> {
        let mut order_update_stream = lock_slot(&self.order_update_stream);

        // Not `ensure_connected`: nothing is written, and the stream may be
        // created while a reconnect is in progress.
        if self.is_shutting_down() {
            return Err(Error::Shutdown);
        }

        // A registration with a dead lease is a cancelled or dropped stream
        // whose cleanup signal has not been processed yet; replace it rather
        // than refusing, so cancel- or drop-then-recreate never races the
        // cleanup thread. Its stale signal then finds another lease and leaves
        // the replacement alone.
        if order_update_stream.as_ref().is_some_and(|registered| registered.lease.is_live()) {
            return Err(Error::AlreadySubscribed);
        }

        let (sender, receiver) = channel::unbounded();
        let lease = Lease::new();

        *order_update_stream = Some(Entry::new(sender.clone(), lease.downgrade()));

        // The lease gives the subscription's drop signal its identity — see
        // `clear_order_update_stream`.
        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender)
            .signaler(self.signals_send.clone())
            .lease(lease)
            .build();

        Ok(subscription)
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.send_shared(message_type, None, message)
    }

    fn send_account_updates_request(&self, account: &AccountId, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.send_shared(OutgoingMessages::RequestAccountData, Some(account), message)
    }

    fn cancel_shared_subscription(&self, ticket: SharedTicket, message: Option<&[u8]>) -> Result<(), Error> {
        self.shared_channels
            .unsubscribe(ticket, || message.map_or(Ok(()), |message| self.write_message(message)))
    }

    fn notice_subscribe(&self) -> NoticeStream {
        NoticeStream::new(self.connection.notice_broadcaster.subscribe())
    }

    fn ensure_shutdown(&self) {
        self.request_shutdown();
        self.join();
    }

    fn wait_connected(&self) -> Result<(), Error> {
        self.connection_state.wait_connected()
    }

    fn is_connected(&self) -> bool {
        self.connection_state.is_connected() && !self.is_shutting_down()
    }
}

#[derive(Debug)]
struct Entry<V> {
    sender: Sender<V>,
    /// The subscription's lease: which subscription this is, and whether it lives.
    lease: LeaseRef,
    /// The unread-item cap of a route opened with `send_request_bounded`.
    bound: Option<BoundState>,
}

impl<V> Entry<V> {
    fn new(sender: Sender<V>, lease: LeaseRef) -> Self {
        Self { sender, lease, bound: None }
    }

    fn closed(&self) -> bool {
        self.bound.as_ref().is_some_and(BoundState::closed)
    }

    /// An unbounded registration sharing this one's channel and lease.
    fn alias(&self) -> Self {
        Self::new(self.sender.clone(), self.lease.clone())
    }
}

impl Entry<RoutedItem> {
    /// Send `item`, subject to a bounded route's cap ([`BoundState::admit`]).
    /// Never blocks.
    fn deliver(&self, id: &impl std::fmt::Debug, item: RoutedItem) {
        let item = match &self.bound {
            Some(bound) => match bound.admit(&item, self.sender.len()) {
                Admit::Deliver => item,
                Admit::Overflow => bound.overflow_error(),
                Admit::Discard => {
                    trace!("discarding item for closed route {id:?}");
                    return;
                }
            },
            None => item,
        };
        if let Err(err) = self.sender.send(item) {
            warn!("error sending: {id:?}, {err}")
        } else {
            warn_if_backlogged(format_args!("subscription queue for {id:?}"), self.sender.len());
        }
    }
}

#[derive(Debug)]
struct SenderHash<K, V> {
    senders: RwLock<HashMap<K, Entry<V>>>,
}

impl<K: std::hash::Hash + Eq + std::fmt::Debug, V: std::fmt::Debug> SenderHash<K, V> {
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(HashMap::new()),
        }
    }

    // A panic while holding the lock leaves the map consistent (every
    // operation is a single map call), so recover rather than cascade the
    // panic into every later route and teardown.
    fn read(&self) -> RwLockReadGuard<'_, HashMap<K, Entry<V>>> {
        self.senders.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<K, Entry<V>>> {
        self.senders.write().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(test)]
    pub fn copy_sender(&self, id: K) -> Option<Sender<V>> {
        self.with_entry(&id, |entry| entry.sender.clone())
    }

    #[cfg(test)]
    pub fn lease(&self, id: K) -> Option<LeaseRef> {
        self.with_entry(&id, |entry| entry.lease.clone())
    }

    /// Run `f` on `id`'s entry while holding the read lock, so no removal can
    /// land between the lookup and `f`.
    #[cfg(test)]
    fn with_entry<R>(&self, id: &K, f: impl FnOnce(&Entry<V>) -> R) -> Option<R> {
        let senders = self.read();
        senders.get(id).map(f)
    }

    pub fn insert(&self, id: K, sender: Sender<V>, lease: LeaseRef) {
        self.insert_entry(id, Entry::new(sender, lease));
    }

    /// Like [`insert`](Self::insert), with an unread-item cap: see [`BoundState::admit`].
    pub fn insert_bounded(&self, id: K, sender: Sender<V>, lease: LeaseRef, bound: BufferBound) {
        let entry = Entry {
            bound: Some(BoundState::new(bound)),
            ..Entry::new(sender, lease)
        };
        self.insert_entry(id, entry);
    }

    fn insert_entry(&self, id: K, entry: Entry<V>) {
        self.write().insert(id, entry);
    }

    /// Remove the entry for `id` only if it holds `lease`. Returns whether an
    /// entry was removed. Used by signal cleanup so a stale signal cannot
    /// remove a newer registration under the same key.
    pub fn remove_if_same(&self, id: K, lease: &LeaseRef) -> bool {
        match self.write().entry(id) {
            hash_map::Entry::Occupied(registered) if registered.get().lease.is(lease) => {
                registered.remove();
                true
            }
            _ => false,
        }
    }

    /// Remove every entry holding `lease`. Returns how many were removed.
    pub fn remove_all_same(&self, lease: &LeaseRef) -> usize {
        let mut senders = self.write();
        let before = senders.len();
        senders.retain(|_, registered| !registered.lease.is(lease));
        before - senders.len()
    }

    pub fn contains(&self, id: &K) -> bool {
        let senders = self.read();
        senders.contains_key(id)
    }

    pub fn len(&self) -> usize {
        let senders = self.read();
        senders.len()
    }

    pub fn clear(&self) {
        let mut senders = self.write();
        senders.clear();
    }
}

impl<K: std::hash::Hash + Eq + std::fmt::Debug> SenderHash<K, RoutedItem> {
    /// Deliver `item` to `id`'s route; hands it back when nothing is
    /// registered under `id`.
    pub fn deliver(&self, id: &K, item: RoutedItem) -> Result<(), RoutedItem> {
        self.deliver_then(id, item, |_| {})
    }

    /// Deliver `item` to `id`'s route, then alias that route under `alias`
    /// in `aliases` while still holding `id`'s read lock, so a concurrent
    /// cleanup either removes the registration first (no alias is stored) or
    /// prunes the alias after it. Lock order: `self` (read), then `aliases`
    /// (write); nothing takes them the other way round. Hands the item back
    /// when nothing is registered under `id`.
    pub fn deliver_aliased<A: std::hash::Hash + Eq + Clone + std::fmt::Debug>(
        &self,
        id: &K,
        item: RoutedItem,
        alias: Option<&A>,
        aliases: &SenderHash<A, RoutedItem>,
    ) -> Result<(), RoutedItem> {
        self.deliver_then(id, item, |entry| {
            if let Some(alias) = alias {
                aliases.insert_entry(alias.clone(), entry.alias());
            }
        })
    }

    /// Deliver `item` to `id`'s route and run `then` on it, both under the
    /// read lock.
    fn deliver_then(&self, id: &K, item: RoutedItem, then: impl FnOnce(&Entry<RoutedItem>)) -> Result<(), RoutedItem> {
        let senders = self.read();
        let Some(entry) = senders.get(id) else {
            return Err(item);
        };
        entry.deliver(id, item);
        then(entry);
        Ok(())
    }

    /// Send `message_fn()` to every route, skipping closed bounded ones (their
    /// stream already ended), then clear them all under the same write lock,
    /// so a route registered meanwhile can't be cleared unnotified.
    pub fn fail_all<F>(&self, message_fn: F)
    where
        F: Fn() -> RoutedItem,
    {
        let mut senders = self.write();
        for entry in senders.values().filter(|entry| !entry.closed()) {
            if let Err(e) = entry.sender.send(message_fn()) {
                warn!("error sending notification: {e}");
            }
        }
        senders.clear();
    }
}

#[derive(Debug)]
pub(crate) struct TcpSocket {
    reader: Mutex<TcpStream>,
    writer: Mutex<TcpStream>,
    /// Extra clone of the active stream used solely to break the dispatcher's
    /// blocking read on shutdown. Refreshed on every `reconnect`.
    shutdown_handle: Mutex<TcpStream>,
    connection_url: String,
    tcp_no_delay: bool,
    /// Byte-level capture of the inbound stream. Disabled unless
    /// `IBAPI_RAW_CAPTURE_DIR` is set; see [`RawFrameTap`].
    tap: RawFrameTap,
}
impl TcpSocket {
    pub fn connect(address: &str, tcp_no_delay: bool) -> Result<Self, Error> {
        let stream = TcpStream::connect(address)?;
        Self::new(stream, address, tcp_no_delay)
    }

    pub fn new(stream: TcpStream, connection_url: &str, tcp_no_delay: bool) -> Result<Self, Error> {
        let writer = stream.try_clone()?;
        let shutdown_handle = stream.try_clone()?;

        stream.set_read_timeout(Some(TWS_READ_TIMEOUT))?;
        stream.set_nodelay(tcp_no_delay)?;

        Ok(Self {
            reader: Mutex::new(stream),
            writer: Mutex::new(writer),
            shutdown_handle: Mutex::new(shutdown_handle),
            connection_url: connection_url.to_string(),
            tcp_no_delay,
            tap: RawFrameTap::from_env(),
        })
    }
}

impl Reconnect for TcpSocket {
    fn reconnect(&self) -> Result<(), Error> {
        match TcpStream::connect(&self.connection_url) {
            Ok(stream) => {
                stream.set_read_timeout(Some(TWS_READ_TIMEOUT))?;
                stream.set_nodelay(self.tcp_no_delay)?;

                let mut reader = self.reader.lock()?;
                *reader = stream.try_clone()?;

                let mut writer = self.writer.lock()?;
                *writer = stream.try_clone()?;

                let mut shutdown_handle = self.shutdown_handle.lock()?;
                *shutdown_handle = stream;

                // One capture file per TCP stream: splicing two of them would
                // read back as a desync at the seam that never happened.
                self.tap.start_new_segment();

                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
    fn sleep(&self, duration: std::time::Duration, shutdown: &ShutdownSignal) {
        shutdown.wait_timeout(duration)
    }
    fn shutdown_read(&self) -> Result<(), Error> {
        let handle = self.shutdown_handle.lock()?;
        // Shutdown::Read is enough to break the blocked read; future writes
        // (none expected during shutdown) remain functional.
        handle.shutdown(std::net::Shutdown::Read)?;
        Ok(())
    }
}

pub(crate) trait Reconnect {
    fn reconnect(&self) -> Result<(), Error>;
    /// Wait out the reconnect backoff, returning early once `shutdown` is
    /// requested. In-memory test streams return immediately.
    fn sleep(&self, duration: std::time::Duration, shutdown: &ShutdownSignal);
    /// Interrupt any in-flight blocking read so the dispatcher exits promptly
    /// on shutdown. For `TcpSocket` this shuts down the read half; for the
    /// in-memory test fixtures it closes their inbound queue.
    fn shutdown_read(&self) -> Result<(), Error>;
}

pub(crate) trait Stream: Io + Reconnect + Sync + Send + 'static + std::fmt::Debug {}
impl Stream for TcpSocket {}

/// Reads one frame's bytes, keeping partial progress across read timeouts.
///
/// `read_exact` drops the bytes it consumed when a read times out, and the
/// dispatcher treats a timeout as idle, so a frame straddling the socket's
/// read timeout used to desync the stream for good (#892). Here a timeout
/// before the frame's first byte is returned as-is (idle: the dispatcher
/// polls its shutdown flag, the handshake fails fast); after it, timeouts are
/// waited out, up to [`MID_FRAME_TIMEOUT_LIMIT`] in a row for the frame.
struct FrameRead<'a, R> {
    reader: &'a mut R,
    /// Any byte of this frame consumed.
    started: bool,
    /// Consecutive timeouts since the last progress, across prefix and body.
    timeouts: u32,
}

impl<'a, R: Read> FrameRead<'a, R> {
    fn new(reader: &'a mut R) -> Self {
        Self {
            reader,
            started: false,
            timeouts: 0,
        }
    }

    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.reader.read(&mut buf[filled..]) {
                // `read_exact`'s message, which the handshake surfaces in `ConnectionRejected`.
                Ok(0) => return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "failed to fill whole buffer").into()),
                Ok(n) => {
                    filled += n;
                    self.started = true;
                    self.timeouts = 0;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                    if !self.started {
                        return Err(e.into());
                    }
                    self.timeouts += 1;
                    if self.timeouts > MID_FRAME_TIMEOUT_LIMIT {
                        return Err(Error::InvalidFrame(format!("frame stalled mid-read for {} read timeouts", self.timeouts)));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

/// Read the 4-byte big-endian length prefix, taps it, then validates it.
///
/// The tap runs *before* [`validate_frame_length`] on purpose — a prefix that
/// fails validation is exactly the byte sequence a framing desync leaves
/// behind, so it has to reach the capture even though it never reaches a
/// caller. See [`RawFrameTap`].
fn read_header(frame: &mut FrameRead<'_, impl Read>, tap: &RawFrameTap) -> Result<usize, Error> {
    let mut buffer = [0_u8; 4];
    frame.fill(&mut buffer)?;
    tap.record_length_prefix(&buffer);
    validate_frame_length(u32::from_be_bytes(buffer) as usize)
}

pub(crate) fn read_message(reader: &mut impl Read, tap: &RawFrameTap) -> Result<Vec<u8>, Error> {
    let mut frame = FrameRead::new(reader);
    let message_size = read_header(&mut frame, tap)?;
    let mut data = vec![0_u8; message_size];
    frame.fill(&mut data)?;
    tap.record_body(&data);
    Ok(data)
}

impl Io for TcpSocket {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.lock()?;
        read_message(&mut *reader, &self.tap)
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut writer = self.writer.lock()?;
        writer.write_all(buf)?;
        Ok(())
    }
}

pub(crate) trait Io {
    fn read_message(&self) -> Result<Vec<u8>, Error>;
    fn write_all(&self, buf: &[u8]) -> Result<(), Error>;
}

mod connection_signal;
mod shutdown;
pub(crate) use connection_signal::ConnectionSignal;
pub(crate) use shutdown::ShutdownSignal;

#[cfg(test)]
mod memory;
#[cfg(test)]
pub(crate) use memory::MemoryStream;

#[cfg(test)]
pub(crate) mod test_listener;

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
