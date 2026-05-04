//! This module implements a message bus for handling communications with TWS.
//! It provides functionality for routing requests from the Client to TWS,
//! and responses from TWS back to the Client.

use std::collections::HashMap;
use std::io::{prelude::*, Cursor};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info, warn};

use crate::connection::sync::Connection;

use super::common::log_error_payload;
use super::routing::{determine_routing, is_warning_error, order_routing_strategy, OrderRoutingStrategy, RoutingDecision, UNSPECIFIED_REQUEST_ID};
use super::{InternalSubscription, MessageBus, Response, Signal, SubscriptionBuilder};
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::Error;

// pub(crate) const MIN_SERVER_VERSION: i32 = 100;
// pub(crate) const MAX_SERVER_VERSION: i32 = server_versions::WSH_EVENT_DATA_FILTERS_DATE;
const TWS_READ_TIMEOUT: Duration = Duration::from_secs(1);

// For requests without an identifier, shared channels are created
// to route request/response pairs based on message type.
#[derive(Debug)]
struct SharedChannels {
    // Maps an inbound reply to channel used to send responses.
    senders: HashMap<IncomingMessages, Vec<Arc<Sender<Response>>>>,
    // Maps an outbound request to channel used to receive responses.
    receivers: HashMap<OutgoingMessages, Arc<Receiver<Response>>>,
}

impl SharedChannels {
    // Creates new instance and registers request/reply pairs.
    pub fn new() -> Self {
        let mut instance = Self {
            senders: HashMap::new(),
            receivers: HashMap::new(),
        };

        // Register request/response pairs.
        for mapping in shared_channel_configuration::CHANNEL_MAPPINGS {
            instance.register(mapping.request, mapping.responses);
        }

        instance
    }

    // Maps an outgoing message to incoming message(s)
    fn register(&mut self, outbound: OutgoingMessages, inbounds: &[IncomingMessages]) {
        let (sender, receiver) = channel::unbounded::<Response>();

        self.receivers.insert(outbound, Arc::new(receiver));

        let sender = &Arc::new(sender);

        for inbound in inbounds {
            if !self.senders.contains_key(inbound) {
                self.senders.insert(*inbound, Vec::new());
            }
            self.senders.get_mut(inbound).unwrap().push(Arc::clone(sender));
        }
    }

    // Get receiver for specified message type. Panics if receiver not found.
    fn get_receiver(&self, message_type: OutgoingMessages) -> Arc<Receiver<Response>> {
        let receiver = self
            .receivers
            .get(&message_type)
            .unwrap_or_else(|| panic!("unsupported request message {message_type:?}. check mapping in messages::shared_channel_configuration"));

        Arc::clone(receiver)
    }

    fn contains_sender(&self, message_type: IncomingMessages) -> bool {
        self.senders.contains_key(&message_type)
    }

    // Notify all listeners of a given message type with message.
    fn send_message(&self, message_type: IncomingMessages, message: &ResponseMessage) {
        if let Some(senders) = self.senders.get(&message_type) {
            for sender in senders.iter() {
                if let Err(e) = sender.send(Ok(message.clone())) {
                    warn!("error sending message: {e}");
                }
            }
        }
    }

    // Notify all senders with a given message
    fn notify_all<F>(&self, message_fn: F)
    where
        F: Fn() -> Response,
    {
        for senders in self.senders.values() {
            for sender in senders {
                if let Err(e) = sender.send(message_fn()) {
                    warn!("error sending notification: {e}");
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TcpMessageBus<S: Stream> {
    connection: Connection<S>,
    handles: Mutex<Vec<JoinHandle<()>>>,
    requests: SenderHash<i32, Response>,
    orders: SenderHash<i32, Response>,
    executions: SenderHash<String, Response>,
    shared_channels: SharedChannels,
    signals_send: Sender<Signal>,
    signals_recv: Receiver<Signal>,
    shutdown_requested: AtomicBool,
    order_update_stream: Mutex<Option<Sender<Response>>>,
    connected: AtomicBool,
}

impl<S: Stream> TcpMessageBus<S> {
    pub fn new(connection: Connection<S>) -> Result<TcpMessageBus<S>, Error> {
        let (signals_send, signals_recv) = channel::unbounded();

        Ok(TcpMessageBus {
            connection,
            handles: Mutex::new(Vec::default()),
            requests: SenderHash::new(),
            orders: SenderHash::new(),
            executions: SenderHash::new(),
            shared_channels: SharedChannels::new(),
            signals_send,
            signals_recv,
            shutdown_requested: AtomicBool::new(false),
            order_update_stream: Mutex::new(None),
            connected: AtomicBool::new(true),
        })
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    fn request_shutdown(&self) {
        debug!("shutdown requested");

        self.requests.notify_all(|| Err(Error::Shutdown));
        self.orders.notify_all(|| Err(Error::Shutdown));
        self.shared_channels.notify_all(|| Err(Error::Shutdown));

        self.requests.clear();
        self.orders.clear();
        self.executions.clear();

        self.connected.store(false, Ordering::Relaxed);
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    fn reset(&self) {
        debug!("reset message bus");

        self.requests.notify_all(|| Err(Error::ConnectionReset));
        self.orders.notify_all(|| Err(Error::ConnectionReset));
        self.shared_channels.notify_all(|| Err(Error::ConnectionReset));

        self.requests.clear();
        self.orders.clear();
        self.executions.clear();

        self.connected.store(false, Ordering::Relaxed);
    }

    fn clean_request(&self, request_id: i32) {
        self.requests.remove(&request_id);
        debug!("released request_id {}, requests.len()={}", request_id, self.requests.len());
    }

    fn clean_order(&self, order_id: i32) {
        self.orders.remove(&order_id);
        debug!("released order_id {}, orders.len()={}", order_id, self.orders.len());
    }

    fn clear_order_update_stream(&self) {
        let mut stream = if let Ok(stream) = self.order_update_stream.lock() {
            stream
        } else {
            warn!("failed to lock order_update_stream");
            return;
        };

        *stream = None;
        debug!("released order_update_stream");
    }

    fn read_message(&self) -> Response {
        self.connection.read_message()
    }
    pub(crate) fn dispatch(&self) -> Result<(), Error> {
        use crate::client::error_handler::{is_connection_error, is_timeout_error};

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
            Err(ref err) if is_timeout_error(err) => {
                if self.is_shutting_down() {
                    debug!("dispatcher thread exiting");
                    return Err(Error::Shutdown);
                }
                Ok(())
            }
            Err(ref err) if is_connection_error(err) => {
                error!("error reading next message (will attempt reconnect): {err:?}");
                self.connected.store(false, Ordering::Relaxed);

                if let Err(reconnect_err) = self.connection.reconnect() {
                    error!("failed to reconnect to TWS/Gateway: {reconnect_err:?}");
                    self.request_shutdown();
                    return Err(Error::ConnectionFailed);
                }

                info!("successfully reconnected to TWS/Gateway");
                self.connected.store(true, Ordering::Relaxed);
                self.reset();
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
        // Use common routing logic
        match determine_routing(&message) {
            RoutingDecision::Error(payload) => {
                let routed = self.send_order_update(&message);

                if payload.request_id == UNSPECIFIED_REQUEST_ID || is_warning_error(payload.error_code) {
                    log_error_payload(&payload);
                } else {
                    self.process_response_with_id(payload.request_id, message, routed);
                }
            }
            RoutingDecision::ByOrderId(_) => {
                // Order-related messages
                self.process_orders(message);
            }
            RoutingDecision::ByRequestId(id) => {
                self.process_response_with_id(id, message, false);
            }
            _ => {
                // All other messages
                self.process_response(message, false);
            }
        }
    }

    fn process_response(&self, message: ResponseMessage, routed: bool) {
        let request_id = message.request_id().unwrap_or(-1);
        self.process_response_with_id(request_id, message, routed);
    }

    fn process_response_with_id(&self, request_id: i32, message: ResponseMessage, routed: bool) {
        if self.requests.contains(&request_id) {
            self.requests.send(&request_id, Ok(message)).unwrap();
        } else if self.orders.contains(&request_id) {
            self.orders.send(&request_id, Ok(message)).unwrap();
        } else if self.shared_channels.contains_sender(message.message_type()) {
            self.shared_channels.send_message(message.message_type(), &message);
        } else if !routed {
            info!("no recipient found for: {message:?}")
        }
    }

    fn process_orders(&self, message: ResponseMessage) {
        let strategy = order_routing_strategy(message.message_type());

        match strategy {
            OrderRoutingStrategy::ExecutionData => {
                let sent_to_update_stream = self.send_order_update(&message);

                // Try order_id channel first, then request_id, storing execution_id mapping
                if let Some(order_id) = message.order_id() {
                    if self.orders.contains(&order_id) {
                        self.store_execution_mapping_orders(&message, order_id);
                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                        return;
                    }
                }
                if let Some(request_id) = message.request_id() {
                    if self.requests.contains(&request_id) {
                        self.store_execution_mapping_requests(&message, request_id);
                        if let Err(e) = self.requests.send(&request_id, Ok(message)) {
                            warn!("error routing message for request_id({request_id}): {e}");
                        }
                        return;
                    }
                }
                if !sent_to_update_stream {
                    warn!("could not route message {message:?}");
                }
            }
            OrderRoutingStrategy::ExecutionDataEnd => {
                if let Some(order_id) = message.order_id() {
                    if self.orders.contains(&order_id) {
                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                        return;
                    }
                }
                if let Some(request_id) = message.request_id() {
                    if self.requests.contains(&request_id) {
                        if let Err(e) = self.requests.send(&request_id, Ok(message)) {
                            warn!("error routing message for request_id({request_id}): {e}");
                        }
                        return;
                    }
                }
                warn!("could not route message {message:?}");
            }
            OrderRoutingStrategy::OrderOrShared => {
                let sent_to_update_stream = self.send_order_update(&message);

                if let Some(order_id) = message.order_id() {
                    if self.orders.contains(&order_id) {
                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                        return;
                    }
                    if self.shared_channels.contains_sender(IncomingMessages::OpenOrder) {
                        self.shared_channels.send_message(message.message_type(), &message);
                        return;
                    }
                }
                if !sent_to_update_stream {
                    warn!("could not route message {message:?}");
                }
            }
            OrderRoutingStrategy::ByExecutionId => {
                let sent_to_update_stream = self.send_order_update(&message);

                if let Some(execution_id) = message.execution_id() {
                    if let Err(e) = self.executions.send(&execution_id, Ok(message)) {
                        warn!("error sending commission report for execution {execution_id}: {e}");
                    }
                } else if !sent_to_update_stream {
                    warn!("could not route commission report {message:?}");
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

    fn store_execution_mapping_orders(&self, message: &ResponseMessage, order_id: i32) {
        if let Some(sender) = self.orders.copy_sender(order_id) {
            if let Some(execution_id) = message.execution_id() {
                self.executions.insert(execution_id, sender);
            }
        }
    }

    fn store_execution_mapping_requests(&self, message: &ResponseMessage, request_id: i32) {
        if let Some(sender) = self.requests.copy_sender(request_id) {
            if let Some(execution_id) = message.execution_id() {
                self.executions.insert(execution_id, sender);
            }
        }
    }

    // Sends an order update message to the order update stream if it exists.
    // Returns true if the message was sent to the order update stream.
    fn send_order_update(&self, message: &ResponseMessage) -> bool {
        if let Ok(order_update_stream) = self.order_update_stream.lock() {
            if let Some(sender) = order_update_stream.as_ref() {
                if let Err(e) = sender.send(Ok(message.clone())) {
                    warn!("error sending to order update stream: {e}");
                    return false;
                }
                return true;
            }
        }
        false
    }

    // The cleanup thread receives signals as subscribers are dropped and
    // releases the sender channels
    fn start_cleanup_thread(self: &Arc<Self>, timeout: std::time::Duration) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);

        thread::spawn(move || {
            let signal_recv = message_bus.signals_recv.clone();

            loop {
                if let Ok(signal) = signal_recv.recv_timeout(timeout) {
                    match signal {
                        Signal::Request(request_id) => {
                            message_bus.clean_request(request_id);
                        }
                        Signal::Order(order_id) => {
                            message_bus.clean_order(order_id);
                        }
                        Signal::OrderUpdateStream => {
                            message_bus.clear_order_update_stream();
                        }
                    }
                }

                if message_bus.is_shutting_down() {
                    debug!("cleanup thread exiting");
                    return;
                }
            }
        })
    }

    pub(crate) fn process_messages(self: &Arc<Self>, _server_version: i32, timeout: std::time::Duration) -> Result<(), Error> {
        let handle = self.start_dispatcher_thread();
        self.add_join_handle(handle);

        let handle = self.start_cleanup_thread(timeout);
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
}

impl<S: Stream> MessageBus for TcpMessageBus<S> {
    fn send_request(&self, request_id: i32, message: &[u8]) -> Result<InternalSubscription, Error> {
        let (sender, receiver) = channel::unbounded();
        let sender_copy = sender.clone();

        self.requests.insert(request_id, sender);

        self.connection.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender_copy)
            .signaler(self.signals_send.clone())
            .request_id(request_id)
            .build();

        Ok(subscription)
    }

    fn cancel_subscription(&self, request_id: i32, message: &[u8]) -> Result<(), Error> {
        self.connection.write_message(message)?;

        if let Err(e) = self.requests.send(&request_id, Err(Error::Cancelled)) {
            info!("error sending cancel notification: {e}");
        }

        self.requests.remove(&request_id);

        Ok(())
    }

    fn send_order_request(&self, order_id: i32, message: &[u8]) -> Result<InternalSubscription, Error> {
        let (sender, receiver) = channel::unbounded();
        let sender_copy = sender.clone();

        self.orders.insert(order_id, sender);
        debug!("Registered order subscription for order_id={}", order_id);

        self.connection.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender_copy)
            .signaler(self.signals_send.clone())
            .order_id(order_id)
            .build();

        Ok(subscription)
    }

    fn send_message(&self, message: &[u8]) -> Result<(), Error> {
        self.connection.write_message(message)?;
        Ok(())
    }

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error> {
        let mut order_update_stream = self.order_update_stream.lock().unwrap();

        if order_update_stream.is_some() {
            return Err(Error::AlreadySubscribed);
        }

        let (sender, receiver) = channel::unbounded();

        *order_update_stream = Some(sender);

        let subscription = SubscriptionBuilder::new().receiver(receiver).signaler(self.signals_send.clone()).build();

        Ok(subscription)
    }

    fn cancel_order_subscription(&self, request_id: i32, message: &[u8]) -> Result<(), Error> {
        self.connection.write_message(message)?;

        if let Err(e) = self.orders.send(&request_id, Err(Error::Cancelled)) {
            info!("error sending cancel notification: {e}");
        }

        self.orders.remove(&request_id);

        Ok(())
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &[u8]) -> Result<InternalSubscription, Error> {
        self.connection.write_message(message)?;

        let shared_receiver = self.shared_channels.get_receiver(message_type);

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(shared_receiver)
            .message_type(message_type)
            .build();

        Ok(subscription)
    }

    fn cancel_shared_subscription(&self, _message_type: OutgoingMessages, message: &[u8]) -> Result<(), Error> {
        self.connection.write_message(message)?;
        // TODO send cancel
        Ok(())
    }

    fn ensure_shutdown(&self) {
        self.request_shutdown();
        self.join();
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed) && !self.is_shutting_down()
    }
}

#[derive(Debug)]
struct SenderHash<K, V> {
    senders: RwLock<HashMap<K, Sender<V>>>,
}

impl<K: std::hash::Hash + Eq + std::fmt::Debug, V: std::fmt::Debug> SenderHash<K, V> {
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(HashMap::new()),
        }
    }

    pub fn send(&self, id: &K, message: V) -> Result<(), Error> {
        let senders = self.senders.read().unwrap();
        debug!("senders: {senders:?}");
        if let Some(sender) = senders.get(id) {
            if let Err(err) = sender.send(message) {
                warn!("error sending: {id:?}, {err}")
            }
        } else {
            warn!("no recipient found for: {id:?}, {message:?}")
        }
        Ok(())
    }

    pub fn copy_sender(&self, id: K) -> Option<Sender<V>> {
        let senders = self.senders.read().unwrap();
        senders.get(&id).cloned()
    }

    pub fn insert(&self, id: K, message: Sender<V>) -> Option<Sender<V>> {
        let mut senders = self.senders.write().unwrap();
        senders.insert(id, message)
    }

    pub fn remove(&self, id: &K) -> Option<Sender<V>> {
        let mut senders = self.senders.write().unwrap();
        senders.remove(id)
    }

    pub fn contains(&self, id: &K) -> bool {
        let senders = self.senders.read().unwrap();
        senders.contains_key(id)
    }

    pub fn len(&self) -> usize {
        let senders = self.senders.read().unwrap();
        senders.len()
    }

    pub fn clear(&self) {
        let mut senders = self.senders.write().unwrap();
        senders.clear();
    }

    pub fn notify_all<F>(&self, message_fn: F)
    where
        F: Fn() -> V,
    {
        let senders = self.senders.read().unwrap();
        for sender in senders.values() {
            if let Err(e) = sender.send(message_fn()) {
                warn!("error sending notification: {e}");
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct TcpSocket {
    reader: Mutex<TcpStream>,
    writer: Mutex<TcpStream>,
    connection_url: String,
    tcp_no_delay: bool,
}
impl TcpSocket {
    pub fn connect(address: &str, tcp_no_delay: bool) -> Result<Self, Error> {
        let stream = TcpStream::connect(address)?;
        Self::new(stream, address, tcp_no_delay)
    }

    pub fn new(stream: TcpStream, connection_url: &str, tcp_no_delay: bool) -> Result<Self, Error> {
        let writer = stream.try_clone()?;

        stream.set_read_timeout(Some(TWS_READ_TIMEOUT))?;
        stream.set_nodelay(tcp_no_delay)?;

        Ok(Self {
            reader: Mutex::new(stream),
            writer: Mutex::new(writer),
            connection_url: connection_url.to_string(),
            tcp_no_delay,
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
                *writer = stream;

                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
    fn sleep(&self, duration: std::time::Duration) {
        thread::sleep(duration)
    }
}

pub(crate) trait Reconnect {
    fn reconnect(&self) -> Result<(), Error>;
    fn sleep(&self, duration: std::time::Duration);
}

pub(crate) trait Stream: Io + Reconnect + Sync + Send + 'static + std::fmt::Debug {}
impl Stream for TcpSocket {}

fn read_header(reader: &mut impl Read) -> Result<usize, Error> {
    let buffer = &mut [0_u8; 4];
    reader.read_exact(buffer)?;
    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;
    Ok(count as usize)
}

pub(crate) fn read_message(reader: &mut impl Read) -> Result<Vec<u8>, Error> {
    let message_size = read_header(reader)?;
    let mut data = vec![0_u8; message_size];
    reader.read_exact(&mut data)?;
    Ok(data)
}

impl Io for TcpSocket {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.lock()?;
        read_message(&mut *reader)
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

#[cfg(test)]
mod memory;
#[cfg(test)]
pub(crate) use memory::MemoryStream;

#[cfg(test)]
pub(crate) mod test_listener;

#[cfg(test)]
mod tests;
