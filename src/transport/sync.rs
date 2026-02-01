//! This module implements a message bus for handling communications with TWS.
//! It provides functionality for routing requests from the Client to TWS,
//! and responses from TWS back to the Client.

use std::collections::HashMap;
use std::io::{prelude::*, Cursor};
use std::net::TcpStream;
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info, warn};

use crate::connection::sync::Connection;

use super::routing::{determine_routing, is_warning_error, RoutingDecision, UNSPECIFIED_REQUEST_ID};
use super::{InternalSubscription, MessageBus, Response, Signal, SubscriptionBuilder};
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::{server_versions, Error};

// pub(crate) const MIN_SERVER_VERSION: i32 = 100;
// pub(crate) const MAX_SERVER_VERSION: i32 = server_versions::WSH_EVENT_DATA_FILTERS_DATE;
const TWS_READ_TIMEOUT: Duration = Duration::from_secs(1);

// Defines the range of warning codes (2100–2169) used by the TWS API.
const WARNING_CODES: RangeInclusive<i32> = 2100..=2169;

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
    pub(crate) fn dispatch(&self, server_version: i32) -> Result<(), Error> {
        use crate::client::error_handler::{is_connection_error, is_timeout_error};

        match self.read_message() {
            Ok(message) => {
                if message.is_shutdown() {
                    self.request_shutdown();
                    Err(Error::Shutdown)
                } else {
                    self.dispatch_message(server_version, message);
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
    fn start_dispatcher_thread(self: &Arc<Self>, server_version: i32) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);
        thread::spawn(move || {
            loop {
                match message_bus.dispatch(server_version) {
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

    fn dispatch_message(&self, server_version: i32, message: ResponseMessage) {
        // Use common routing logic
        match determine_routing(&message) {
            RoutingDecision::Error { request_id, error_code } => {
                let routed = self.send_order_update(&message);

                // Check if this is a warning or unspecified error
                if request_id == UNSPECIFIED_REQUEST_ID || is_warning_error(error_code) {
                    error_event(server_version, message).unwrap();
                } else {
                    self.process_response(message, routed);
                }
            }
            RoutingDecision::ByOrderId(_) => {
                // Order-related messages
                self.process_orders(message);
            }
            _ => {
                // All other messages
                self.process_response(message, false);
            }
        }
    }

    fn process_response(&self, message: ResponseMessage, routed: bool) {
        let request_id = message.request_id().unwrap_or(-1); // pass in request id?
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
        match message.message_type() {
            IncomingMessages::ExecutionData => {
                let sent_to_update_stream = self.send_order_update(&message);
                let order_id = message.order_id();
                let request_id = message.request_id();
                debug!(
                    "ExecutionData: order_id={:?}, request_id={:?}, orders.contains(order_id)={}, orders.len={}",
                    order_id,
                    request_id,
                    order_id.is_some_and(|id| self.orders.contains(&id)),
                    self.orders.len()
                );

                match (order_id, request_id) {
                    // First check matching orders channel
                    (Some(order_id), _) if self.orders.contains(&order_id) => {
                        // Store execution-to-order mapping for commission reports
                        if let Some(sender) = self.orders.copy_sender(order_id) {
                            if let Some(execution_id) = message.execution_id() {
                                self.executions.insert(execution_id, sender);
                            }
                        }

                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                    }
                    (_, Some(request_id)) if self.requests.contains(&request_id) => {
                        if let Some(sender) = self.requests.copy_sender(request_id) {
                            if let Some(execution_id) = message.execution_id() {
                                self.executions.insert(execution_id, sender);
                            }
                        }

                        if let Err(e) = self.requests.send(&request_id, Ok(message)) {
                            warn!("error routing message for request_id({request_id}): {e}");
                        }
                    }
                    _ => {
                        if !sent_to_update_stream {
                            warn!("could not route message {message:?}");
                        }
                    }
                }
            }
            IncomingMessages::ExecutionDataEnd => {
                match (message.order_id(), message.request_id()) {
                    // First check matching orders channel
                    (Some(order_id), _) if self.orders.contains(&order_id) => {
                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                    }
                    (_, Some(request_id)) if self.requests.contains(&request_id) => {
                        if let Err(e) = self.requests.send(&request_id, Ok(message)) {
                            warn!("error routing message for request_id({request_id}): {e}");
                        }
                    }
                    _ => {
                        warn!("could not route message {message:?}");
                    }
                }
            }
            IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => {
                let sent_to_update_stream = self.send_order_update(&message);

                if let Some(order_id) = message.order_id() {
                    if self.orders.contains(&order_id) {
                        if let Err(e) = self.orders.send(&order_id, Ok(message)) {
                            warn!("error routing message for order_id({order_id}): {e}");
                        }
                    } else if self.shared_channels.contains_sender(IncomingMessages::OpenOrder) {
                        self.shared_channels.send_message(message.message_type(), &message);
                    } else if !sent_to_update_stream {
                        warn!("could not route message {message:?}");
                    }
                } else if !sent_to_update_stream {
                    warn!("could not route message {message:?}");
                }
            }
            IncomingMessages::CompletedOrder | IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => {
                self.shared_channels.send_message(message.message_type(), &message);
            }
            IncomingMessages::CommissionsReport => {
                let sent_to_update_stream = self.send_order_update(&message);
                let exec_id = message.execution_id();
                debug!(
                    "CommissionReport: exec_id={:?}, executions.contains={}",
                    exec_id,
                    exec_id.as_ref().is_some_and(|id| self.executions.contains(id))
                );

                if let Some(execution_id) = message.execution_id() {
                    if let Err(e) = self.executions.send(&execution_id, Ok(message)) {
                        warn!("error sending commission report for execution {execution_id}: {e}");
                    }
                } else if !sent_to_update_stream {
                    warn!("could not route commission report {message:?}");
                }
            }
            _ => {
                warn!("unhandled order message type: {message:?}");
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

    pub(crate) fn process_messages(self: &Arc<Self>, server_version: i32, timeout: std::time::Duration) -> Result<(), Error> {
        let handle = self.start_dispatcher_thread(server_version);
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
    fn send_request(&self, request_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
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

    fn cancel_subscription(&self, request_id: i32, message: &RequestMessage) -> Result<(), Error> {
        self.connection.write_message(message)?;

        if let Err(e) = self.requests.send(&request_id, Err(Error::Cancelled)) {
            info!("error sending cancel notification: {e}");
        }

        self.requests.remove(&request_id);

        Ok(())
    }

    fn send_order_request(&self, order_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
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

    fn send_message(&self, message: &RequestMessage) -> Result<(), Error> {
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

    fn cancel_order_subscription(&self, request_id: i32, message: &RequestMessage) -> Result<(), Error> {
        self.connection.write_message(message)?;

        if let Err(e) = self.orders.send(&request_id, Err(Error::Cancelled)) {
            info!("error sending cancel notification: {e}");
        }

        self.orders.remove(&request_id);

        Ok(())
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        self.connection.write_message(message)?;

        let shared_receiver = self.shared_channels.get_receiver(message_type);

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(shared_receiver)
            .message_type(message_type)
            .build();

        Ok(subscription)
    }

    fn cancel_shared_subscription(&self, _message_type: OutgoingMessages, message: &RequestMessage) -> Result<(), Error> {
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

fn error_event(server_version: i32, mut packet: ResponseMessage) -> Result<(), Error> {
    packet.skip(); // message_id

    let version = packet.next_int()?;

    if version < 2 {
        let message = packet.next_string()?;
        error!("version 2 error: {message}");
        Ok(())
    } else {
        let request_id = packet.next_int()?;
        let error_code = packet.next_int()?;
        let error_message = packet.next_string()?;

        let mut advanced_order_reject_json: String = "".to_string();
        if server_version >= server_versions::ADVANCED_ORDER_REJECT {
            advanced_order_reject_json = packet.next_string()?;
        }
        // Log warnings and errors differently
        let is_warning = WARNING_CODES.contains(&error_code);
        if is_warning {
            warn!(
                "request_id: {request_id}, warning_code: {error_code}, warning_message: {error_message}, advanced_order_reject_json: {advanced_order_reject_json}"
            );
        } else {
            error!(
                "request_id: {request_id}, error_code: {error_code}, error_message: {error_message}, advanced_order_reject_json: {advanced_order_reject_json}"
            );
        }
        Ok(())
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
}
impl TcpSocket {
    pub fn new(stream: TcpStream, connection_url: &str) -> Result<Self, Error> {
        let writer = stream.try_clone()?;

        stream.set_read_timeout(Some(TWS_READ_TIMEOUT))?;

        Ok(Self {
            reader: Mutex::new(stream),
            writer: Mutex::new(writer),
            connection_url: connection_url.to_string(),
        })
    }
}

impl Reconnect for TcpSocket {
    fn reconnect(&self) -> Result<(), Error> {
        match TcpStream::connect(&self.connection_url) {
            Ok(stream) => {
                stream.set_read_timeout(Some(TWS_READ_TIMEOUT))?;

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
mod tests {
    use super::*;
    use crate::connection::sync::Connection;
    use crate::tests::assert_send_and_sync;
    use crate::transport::common::MAX_RECONNECT_ATTEMPTS;

    // Additional imports for connection tests
    use crate::client::sync::Client;
    use crate::contracts::Contract;
    use crate::messages::{encode_length, OutgoingMessages, RequestMessage};
    use crate::orders::common::encoders::encode_place_order;
    use crate::orders::{order_builder, Action};
    use log::{debug, trace};
    use std::collections::VecDeque;
    use std::io::ErrorKind;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    // Test helper function for encoding contract data requests
    fn encode_request_contract_data(_server_version: i32, request_id: i32, contract: &Contract) -> Result<RequestMessage, Error> {
        const VERSION: i32 = 8;

        let mut packet = RequestMessage::default();
        packet.push_field(&OutgoingMessages::RequestContractData);
        packet.push_field(&VERSION);
        packet.push_field(&request_id);
        packet.push_field(&contract.contract_id);
        packet.push_field(&contract.symbol);
        packet.push_field(&contract.security_type);
        packet.push_field(&contract.last_trade_date_or_contract_month);
        packet.push_field(&contract.strike);
        packet.push_field(&contract.right);
        packet.push_field(&contract.multiplier);
        packet.push_field(&contract.exchange);
        packet.push_field(&contract.primary_exchange);
        packet.push_field(&contract.currency);
        packet.push_field(&contract.local_symbol);
        packet.push_field(&contract.trading_class);
        packet.push_field(&contract.include_expired);

        // Server version 173 includes security_id fields (>= 45)
        packet.push_field(&contract.security_id_type);
        packet.push_field(&contract.security_id);

        // Server version 200 includes issuer_id (>= 176)
        packet.push_field(&contract.issuer_id);

        Ok(packet)
    }

    #[test]
    fn test_thread_safe() {
        assert_send_and_sync::<Connection<TcpSocket>>();
        assert_send_and_sync::<TcpMessageBus<TcpSocket>>();
    }

    #[test]
    fn test_error_event_warning_handling() {
        // Test that warning error codes (2100-2169) are handled correctly
        let server_version = 100;

        // Create a warning message (error code 2104 is a common warning)
        // Format: "4|2|123|2104|Market data farm connection is OK:usfarm.nj"
        let warning_message = ResponseMessage::from_simple("4|2|123|2104|Market data farm connection is OK:usfarm.nj");

        // This should not panic and should handle as a warning
        let result = error_event(server_version, warning_message);
        assert!(result.is_ok());

        // Test actual error (non-warning code)
        // Format: "4|2|456|200|No security definition has been found"
        let error_message = ResponseMessage::from_simple("4|2|456|200|No security definition has been found");

        // This should also not panic and should handle as an error
        let result = error_event(server_version, error_message);
        assert!(result.is_ok());
    }

    // Connection test helpers

    fn mock_socket_error(kind: ErrorKind) -> Error {
        let message = format!("Simulated {} error", kind);
        debug!("mock -> {message}");
        let io_error = std::io::Error::new(kind, message);
        Error::Io(io_error)
    }

    #[derive(Debug)]
    struct MockSocket {
        // Read only
        exchanges: Vec<Exchange>,
        expected_retries: usize,
        reconnect_call_count: AtomicUsize,

        // Accessed from reader thread
        // Mutated by reader thread
        keep_alive: AtomicBool,

        // Accessed from reader thread
        // Mutated by writer threads
        write_call_count: AtomicUsize,
        responses_len: AtomicUsize,

        // Accessed from read thread
        // Mutated by reader thread & writer threads
        read_call_count: AtomicUsize,
    }

    impl MockSocket {
        pub fn new(exchanges: Vec<Exchange>, expected_retries: usize) -> Self {
            Self {
                exchanges,
                expected_retries,
                keep_alive: AtomicBool::new(false),
                reconnect_call_count: AtomicUsize::new(0),
                write_call_count: AtomicUsize::new(0),
                responses_len: AtomicUsize::new(0),
                read_call_count: AtomicUsize::new(0),
            }
        }
    }

    impl Reconnect for MockSocket {
        fn reconnect(&self) -> Result<(), Error> {
            let reconnect_call_count = self.reconnect_call_count.load(Ordering::SeqCst);

            if reconnect_call_count == self.expected_retries {
                return Ok(());
            }

            self.reconnect_call_count.fetch_add(1, Ordering::SeqCst);
            Err(mock_socket_error(ErrorKind::ConnectionRefused))
        }
        fn sleep(&self, _duration: std::time::Duration) {}
    }

    impl Stream for MockSocket {}

    impl Io for MockSocket {
        fn read_message(&self) -> Result<Vec<u8>, Error> {
            trace!("===== mock read =====");

            if self.keep_alive.load(Ordering::SeqCst) {
                return Err(mock_socket_error(ErrorKind::WouldBlock));
            }

            // if response_index > responses len (too many reads for the given exchange)
            // the next read executed before the next write
            // and happens if the mock socket is used with the dispatcher thread
            // this blocks the dispatcher thread until the write has executed
            while self.read_call_count.load(Ordering::SeqCst) >= self.responses_len.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // The state may have changed while waiting
            let write_call_count = self.write_call_count.load(Ordering::SeqCst);
            let read_call_count = self.read_call_count.load(Ordering::SeqCst);
            let exchange = &self.exchanges[write_call_count - 1];
            let responses = &exchange.responses;

            trace!(
                "mock read: responses.len(): {}, read_call_count: {}, write_call_count: {}, exchange_index: {}",
                responses.len(),
                read_call_count,
                write_call_count,
                write_call_count - 1
            );

            let response = responses.get(read_call_count).unwrap();

            // disconnect if a null byte response is encountered
            if response.fields[0] == "\0" {
                return Err(mock_socket_error(ErrorKind::ConnectionReset));
            }

            // if there are no more remaining exchanges or responses
            // set keep_alive - so the client can gracefully disconnect
            if write_call_count >= self.exchanges.len() && read_call_count >= responses.len() - 1 {
                self.keep_alive.store(true, Ordering::SeqCst);
            }

            self.read_call_count.fetch_add(1, Ordering::SeqCst);

            // process the declared response in the test with transport read_message()
            // to force any errors
            let encoded = response.encode();
            debug!("mock read {:?}", &encoded);
            let expected = encode_length(&encoded);
            read_message(&mut expected.as_slice())
        }

        fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
            trace!("===== mock write =====");
            let write_call_count = self.write_call_count.load(Ordering::SeqCst);
            trace!("mock write: write_call_count: {write_call_count}");

            let exchange = self.exchanges.get(write_call_count).unwrap();
            let request = &exchange.request;

            let is_handshake = buf.starts_with(b"API\0");

            // strip API\0 if handshake
            let buf = if is_handshake {
                &buf[4..] // strip prefix
            } else {
                buf
            };

            // the handshake does not include the trailing null byte
            // Message encode() cannot be used to encode the handshake
            let expected = if is_handshake {
                assert_eq!(request.len(), 1);
                &encode_length(&request.fields[0])
            } else {
                &encode_length(&request.encode())
            };

            let raw_string = std::str::from_utf8(&buf[4..]).unwrap(); // strip length
            debug!("mock write {:?}", raw_string);

            assert_eq!(
                expected,
                buf,
                "assertion left == right failed\nexpected: {:?}\nbuf: {:?}\n",
                std::str::from_utf8(expected).unwrap(),
                std::str::from_utf8(buf).unwrap()
            );

            self.read_call_count.store(0, Ordering::SeqCst);
            self.write_call_count.fetch_add(1, Ordering::SeqCst);
            self.responses_len.store(exchange.responses.len(), Ordering::SeqCst);

            Ok(())
        }
    }

    #[derive(Debug)]
    struct Exchange {
        request: RequestMessage,
        responses: VecDeque<ResponseMessage>,
    }

    impl Exchange {
        fn new(request: RequestMessage, responses: Vec<ResponseMessage>) -> Self {
            Self {
                request,
                responses: VecDeque::from(responses),
            }
        }
        fn simple(request: &str, responses: &[&str]) -> Self {
            let responses = responses
                .iter()
                .map(|s| ResponseMessage::from_simple(s))
                .collect::<Vec<ResponseMessage>>();
            Self::new(RequestMessage::from_simple(request), responses)
        }
        fn request(request: RequestMessage, responses: &[&str]) -> Self {
            let responses = responses
                .iter()
                .map(|s| ResponseMessage::from_simple(s))
                .collect::<Vec<ResponseMessage>>();
            Self::new(request, responses)
        }
    }

    #[test]
    fn test_bus_send_order_request() -> Result<(), Error> {
        let order = order_builder::market_order(Action::Buy, 100.0);
        let contract = &Contract::stock("AAPL").build();
        let request = encode_place_order(176, 5, contract, &order)?;

        let events = vec![
            Exchange::simple("v100..200", &["200|20250415 19:38:30 British Summer Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|5|"]),
            Exchange::request(request.clone(),
                &[
                    "5|5|265598|AAPL|STK||0|?||SMART|USD|AAPL|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|600745656|0|0|0||600745656.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|PreSubmitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||100|0.02|||",
                    "3|5|PreSubmitted|0|100|0|600745656|0|0|100||0|",
                    "11|-1|5|265598|AAPL|STK||0.0|||IEX|USD|AAPL|NMS|0000e0d5.67fe667b.01.01|20250415  19:38:31|DU1234567|IEX|BOT|100|201.94|600745656|100|0|100|201.94|||||2|",
                    "5|5|265598|AAPL|STK||0|?||SMART|USD|AAPL|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|600745656|0|0|0||600745656.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||100|0.02|||",
                    "3|5|Filled|100|0|201.94|600745656|0|201.94|100||0|"
                ]),
        ];

        let stream = MockSocket::new(events, 0);
        let connection = Connection::connect(stream, 28)?;
        let server_version = connection.server_version();
        let bus = Arc::new(TcpMessageBus::new(connection)?);

        let subscription = bus.send_order_request(5, &request)?;

        bus.dispatch(server_version)?;
        bus.dispatch(server_version)?;
        bus.dispatch(server_version)?;
        bus.dispatch(server_version)?;
        bus.dispatch(server_version)?;

        subscription.next().unwrap()?;

        Ok(())
    }

    #[test]
    fn test_connection_establish_connection() -> Result<(), Error> {
        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple(
                "71|2|28||",
                &[
                    "15|1|DU1234567|",
                    "9|1|1|",
                    "4|2|-1|2104|Market data farm connection is OK:usfarm||",
                    "4|2|-1|2107|HMDS data farm connection is inactive but should be available upon demand.ushmds||",
                    "4|2|-1|2158|Sec-def data farm connection is OK:secdefil||",
                ],
            ),
        ];
        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;

        Ok(())
    }

    #[test]
    fn test_reconnect_failed() -> Result<(), Error> {
        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        ];
        let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize + 1);

        let connection = Connection::stubbed(socket, 28);
        connection.establish_connection(None)?;

        // simulated dispatcher thread read to trigger disconnection
        let _ = connection.read_message();

        match connection.reconnect() {
            Err(Error::ConnectionFailed) => Ok(()),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_reconnect_success() -> Result<(), Error> {
        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        ];
        let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize - 1);

        let connection = Connection::stubbed(socket, 28);
        connection.establish_connection(None)?;

        // simulated dispatcher thread read to trigger disconnection
        let _ = connection.read_message();

        connection.reconnect()
    }

    #[test]
    fn test_client_reconnect() -> Result<(), Error> {
        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
            Exchange::simple("17|1|", &["\0"]), // ManagedAccounts RESTART
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
            Exchange::simple("17|1|", &["15|1|DU1234567|"]), // ManagedAccounts
        ];
        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;
        let server_version = connection.server_version();
        let bus = Arc::new(TcpMessageBus::new(connection)?);
        bus.process_messages(server_version, std::time::Duration::from_secs(0))?;
        let client = Client::stubbed(bus.clone(), server_version);

        client.managed_accounts()?;

        Ok(())
    }

    const AAPL_CONTRACT_RESPONSE: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";

    #[test]
    fn test_send_request_after_disconnect() -> Result<(), Error> {
        let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL").build())?;

        let expected_response = &format!("10|9000|{AAPL_CONTRACT_RESPONSE}");

        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
            Exchange::request(packet.clone(), &[expected_response, "52|1|9001|"]),
        ];

        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;
        let server_version = connection.server_version();
        let bus = TcpMessageBus::new(connection)?;

        bus.dispatch(server_version)?;

        let subscription = bus.send_request(9000, &packet)?;

        bus.dispatch(server_version)?;
        bus.dispatch(server_version)?;

        let result = subscription.next().unwrap()?;

        assert_eq!(&result.encode_simple(), expected_response);

        Ok(())
    }

    // If a request is sent before a restart
    // the waiter should receive Error::ConnectionReset
    #[test]
    fn test_request_before_disconnect_raises_error() -> Result<(), Error> {
        let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL").build())?;

        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
            Exchange::request(packet.clone(), &["\0"]), // RESTART
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        ];

        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;
        let server_version = connection.server_version();
        let bus = TcpMessageBus::new(connection)?;

        let subscription = bus.send_request(9000, &packet)?;

        bus.dispatch(server_version)?;

        match subscription.next() {
            Some(Err(Error::ConnectionReset)) => {}
            _ => panic!(),
        }

        Ok(())
    }

    // If a request is sent during a restart
    // the waiter should receive Error::ConnectionReset
    #[test]
    fn test_request_during_disconnect_raises_error() -> Result<(), Error> {
        let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL").build())?;

        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::request(packet.clone(), &[]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        ];

        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;

        match connection.read_message() {
            Ok(_) => panic!(""),
            Err(_) => {
                connection.socket.reconnect()?;
                connection.handshake()?;
                connection.write_message(&packet)?;
                connection.start_api()?;
                connection.receive_account_info(None)?;
            }
        };

        Ok(())
    }

    #[test]
    fn test_contract_details_disconnect_raises_error() -> Result<(), Error> {
        let contract = &Contract::stock("AAPL").build();

        let packet = encode_request_contract_data(173, 9000, contract)?;

        let events = vec![
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
            Exchange::request(packet.clone(), &["\0"]),
            Exchange::simple("v100..200", &["200|20250323 22:21:01 Greenwich Mean Time|"]),
            Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        ];

        let stream = MockSocket::new(events, 0);
        let connection = Connection::stubbed(stream, 28);
        connection.establish_connection(None)?;
        let server_version = connection.server_version();
        let bus = Arc::new(TcpMessageBus::new(connection)?);
        bus.process_messages(server_version, std::time::Duration::from_secs(0))?;
        let client = Client::stubbed(bus.clone(), server_version);

        match client.contract_details(contract) {
            Err(Error::ConnectionReset) => {}
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn test_request_simple_encoding_roundtrip() {
        let expected = "17|1|";
        let req = RequestMessage::from_simple(expected);
        assert_eq!(req.fields, vec!["17", "1"]);
        let simple_encoded = req.encode_simple();
        assert_eq!(simple_encoded, expected);
    }

    #[test]
    fn test_request_encoding_roundtrip() {
        let expected = "17\01\0";
        let req = RequestMessage::from(expected);
        assert_eq!(req.fields, vec!["17", "1"]);
        let encoded = req.encode();
        assert_eq!(encoded, expected);
    }
}
