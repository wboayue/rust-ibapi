//! This module implements a message bus for handling communications with TWS.
//! It provides functionality for routing requests from the Client to TWS,
//! and responses from TWS back to the Client.

use std::collections::HashMap;
use std::io::{prelude::*, Cursor, ErrorKind};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::{server_versions, Error};
use recorder::MessageRecorder;

mod recorder;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const MAX_RETRIES: i32 = 20;

pub(crate) trait MessageBus: Send + Sync {
    // Sends formatted message to TWS and creates a reply channel by request id.
    fn send_request(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    // Sends formatted message to TWS and creates a reply channel by request id.
    fn cancel_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error>;

    // Sends formatted message to TWS and creates a reply channel by message type.
    fn send_shared_request(&self, message_id: OutgoingMessages, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    // Sends formatted message to TWS and creates a reply channel by message type.
    fn cancel_shared_subscription(&self, message_id: OutgoingMessages, packet: &RequestMessage) -> Result<(), Error>;

    // Sends formatted order specific message to TWS and creates a reply channel by order id.
    fn send_order_request(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error>;

    fn cancel_order_subscription(&self, request_id: i32, packet: &RequestMessage) -> Result<(), Error>;

    fn ensure_shutdown(&self) {}

    // Testing interface. Tracks requests sent messages when Bus is stubbed.
    #[cfg(test)]
    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Response {
    Message(ResponseMessage),
    Cancelled,
    Disconnected,
}

impl From<ResponseMessage> for Response {
    fn from(val: ResponseMessage) -> Self {
        Response::Message(val)
    }
}

// For requests without an identifier, shared channels are created
// to route request/response pairs based on message type.
#[derive(Debug)]
struct SharedChannels {
    // Maps an inbound reply to channel used to send responses.
    senders: HashMap<IncomingMessages, Arc<Sender<Response>>>,
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
        instance.register(OutgoingMessages::RequestIds, &[IncomingMessages::NextValidId]);
        instance.register(OutgoingMessages::RequestFamilyCodes, &[IncomingMessages::FamilyCodes]);
        instance.register(OutgoingMessages::RequestMarketRule, &[IncomingMessages::MarketRule]);
        instance.register(
            OutgoingMessages::RequestPositions,
            &[IncomingMessages::Position, IncomingMessages::PositionEnd],
        );
        instance.register(
            OutgoingMessages::RequestPositionsMulti,
            &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd],
        );
        instance.register(
            OutgoingMessages::RequestOpenOrders,
            &[IncomingMessages::OpenOrder, IncomingMessages::OpenOrderEnd],
        );
        instance.register(OutgoingMessages::RequestManagedAccounts, &[IncomingMessages::ManagedAccounts]);

        instance
    }

    // Maps an outgoing message to incoming message(s)
    fn register(&mut self, outbound: OutgoingMessages, inbounds: &[IncomingMessages]) {
        let (sender, receiver) = channel::unbounded::<Response>();

        self.receivers.insert(outbound, Arc::new(receiver));

        let sender = &Arc::new(sender);

        for inbound in inbounds {
            self.senders.insert(*inbound, Arc::clone(sender));
        }
    }

    // Get receiver for specified message type. Panics if receiver not found.
    pub fn get_receiver(&self, message_type: OutgoingMessages) -> Arc<Receiver<Response>> {
        let receiver = self.receivers.get(&message_type).unwrap_or_else(|| {
            panic!("unsupported request message {message_type:?}. check mapping in SharedChannels::new() located in transport.rs")
        });

        Arc::clone(receiver)
    }

    // Get sender for specified message type. Panics if sender not found.
    pub fn get_sender(&self, message_type: IncomingMessages) -> Arc<Sender<Response>> {
        let sender = self
            .senders
            .get(&message_type)
            .unwrap_or_else(|| panic!("unsupported response message {message_type:?}"));

        Arc::clone(sender)
    }

    fn contains_sender(&self, message_type: IncomingMessages) -> bool {
        self.senders.contains_key(&message_type)
    }
}

// Signals are used to notify the backend when a subscriber is dropped.
// This facilitates the cleanup of the SenderHashes.
pub enum Signal {
    Request(i32),
    Order(i32),
}

#[derive(Debug)]
pub struct TcpMessageBus {
    connection: Connection,
    handles: Vec<JoinHandle<i32>>,
    requests: SenderHash<i32, Response>,
    orders: SenderHash<i32, Response>,
    executions: SenderHash<String, Response>,
    recorder: MessageRecorder,
    shared_channels: SharedChannels,
    signals_send: Sender<Signal>,
    signals_recv: Receiver<Signal>,
    shutdown_requested: AtomicBool,
    is_alive: bool,
}

impl TcpMessageBus {
    pub fn new(connection: Connection) -> Result<TcpMessageBus, Error> {
        let (signals_send, signals_recv) = channel::unbounded();

        Ok(TcpMessageBus {
            connection,
            handles: Vec::default(),
            requests: SenderHash::new(),
            orders: SenderHash::new(),
            executions: SenderHash::new(),
            recorder: MessageRecorder::new(),
            shared_channels: SharedChannels::new(),
            signals_send,
            signals_recv,
            shutdown_requested: AtomicBool::new(false),
            is_alive: true,
        })
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    fn reset(&self) {}

    fn clean_request(&self, request_id: i32) {
        self.requests.remove(&request_id);
        debug!("released request_id {}, requests.len()={}", request_id, self.requests.len());
    }

    fn clean_order(&self, order_id: i32) {
        self.orders.remove(&order_id);
        debug!("released order_id {}, orders.len()={}", order_id, self.orders.len());
    }

    fn read_message(&self) -> Result<ResponseMessage, Error> {
        self.connection.read_message()
    }

    // Dispatcher thread reads messages from TWS and dispatches them to
    // appropriate channel.
    fn start_dispatcher_thread(self: &Arc<Self>, server_version: i32) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);
        let recorder = self.recorder.clone();

        const RECONNECT_ERRORS: &[ErrorKind] = &[ErrorKind::ConnectionReset, ErrorKind::UnexpectedEof];
        const RETRY_ERRORS: &[ErrorKind] = &[ErrorKind::Interrupted];

        thread::spawn(move || {
            let mut backoff = FibonacciBackoff::new(30);
            let mut retry_attempt = 0;

            loop {
                match message_bus.read_message() {
                    Ok(message) => {
                        recorder.record_response(&message);
                        message_bus.dispatch_message(server_version, message);

                        backoff.reset();
                        retry_attempt = 1;
                    }
                    Err(Error::Io(e)) if RECONNECT_ERRORS.contains(&e.kind()) => {
                        error!("error reading packet: {:?}", e);
                        // reset hashes
                        if let Err(e) = message_bus.connection.reconnect() {
                            error!("error reconnecting: {:?}", e);
                            message_bus.request_shutdown();
                            return;
                        }
                        info!("reconnected");
                        message_bus.reset();
                        continue;
                    }
                    Err(Error::Io(e)) if RETRY_ERRORS.contains(&e.kind()) => {
                        error!("error reading packet: {:?}", e);
                        let next_delay = backoff.next_delay();
                        if retry_attempt > MAX_RETRIES {
                            message_bus.request_shutdown();
                            return;
                        }
                        error!("retry read attempt {retry_attempt} of {MAX_RETRIES}");
                        thread::sleep(next_delay);
                        retry_attempt += 1;
                        continue;
                    }
                    Err(err) => {
                        error!("error reading packet: {:?}", err);
                        message_bus.request_shutdown();
                        return;
                    }
                };

                if message_bus.is_shutting_down() {
                    return;
                }
            }
        })
    }

    fn dispatch_message(&self, server_version: i32, message: ResponseMessage) {
        match message.message_type() {
            IncomingMessages::Error => {
                let request_id = message.peek_int(2).unwrap_or(-1);

                if request_id == UNSPECIFIED_REQUEST_ID {
                    error_event(server_version, message).unwrap();
                } else {
                    self.process_response(message);
                }
            }
            IncomingMessages::OrderStatus
            | IncomingMessages::OpenOrder
            | IncomingMessages::OpenOrderEnd
            | IncomingMessages::CompletedOrder
            | IncomingMessages::CompletedOrdersEnd
            | IncomingMessages::ExecutionData
            | IncomingMessages::ExecutionDataEnd
            | IncomingMessages::CommissionsReport => self.process_orders(message),
            _ => self.process_response(message),
        };
    }

    fn process_response(&self, message: ResponseMessage) {
        let request_id = message.request_id().unwrap_or(-1); // pass in request id?
        if self.requests.contains(&request_id) {
            self.requests.send(&request_id, Response::Message(message)).unwrap();
        } else if self.orders.contains(&request_id) {
            self.orders.send(&request_id, Response::Message(message)).unwrap();
        } else if self.shared_channels.contains_sender(message.message_type()) {
            self.shared_channels
                .get_sender(message.message_type())
                .send(Response::Message(message))
                .unwrap()
        } else {
            info!("no recipient found for: {:?}", message)
        }
    }

    fn process_orders(&self, message: ResponseMessage) {
        match message.message_type() {
            IncomingMessages::ExecutionData => {
                match (message.order_id(), message.request_id()) {
                    // First check matching orders channel
                    (Some(order_id), _) if self.orders.contains(&order_id) => {
                        if let Err(e) = self.orders.send(&order_id, Response::Message(message)) {
                            error!("error routing message for order_id({order_id}): {e}");
                        }
                    }
                    (_, Some(request_id)) if self.requests.contains(&request_id) => {
                        if let Some(sender) = self.requests.copy_sender(request_id) {
                            if let Some(execution_id) = message.execution_id() {
                                self.executions.insert(execution_id, sender);
                            }
                        }

                        if let Err(e) = self.requests.send(&request_id, Response::Message(message)) {
                            error!("error routing message for request_id({request_id}): {e}");
                        }
                    }
                    _ => {
                        error!("could not route message {message:?}");
                    }
                }
            }
            IncomingMessages::ExecutionDataEnd => {
                match (message.order_id(), message.request_id()) {
                    // First check matching orders channel
                    (Some(order_id), _) if self.orders.contains(&order_id) => {
                        if let Err(e) = self.orders.send(&order_id, Response::from(message)) {
                            error!("error routing message for order_id({order_id}): {e}");
                        }
                    }
                    (_, Some(request_id)) if self.requests.contains(&request_id) => {
                        if let Err(e) = self.requests.send(&request_id, Response::from(message)) {
                            error!("error routing message for request_id({request_id}): {e}");
                        }
                    }
                    _ => {
                        error!("could not route message {message:?}");
                    }
                }
            }
            IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => {
                if let Some(order_id) = message.order_id() {
                    if self.orders.contains(&order_id) {
                        if let Err(e) = self.orders.send(&order_id, Response::from(message)) {
                            error!("error routing message for order_id({order_id}): {e}");
                        }
                    } else if let Err(e) = self.shared_channels.get_sender(IncomingMessages::OpenOrder).send(Response::from(message)) {
                        error!("error sending IncomingMessages::OpenOrder: {e}");
                    }
                }
            }
            IncomingMessages::CompletedOrder => {
                if let Err(e) = self.shared_channels.get_sender(message.message_type()).send(Response::from(message)) {
                    error!("error sending IncomingMessages::CompletedOrder: {e}");
                }
            }
            IncomingMessages::OpenOrderEnd => {
                if let Err(e) = self.shared_channels.get_sender(message.message_type()).send(Response::from(message)) {
                    error!("error sending IncomingMessages::OpenOrderEnd: {e}");
                }
            }
            IncomingMessages::CompletedOrdersEnd => {
                if let Err(e) = self.shared_channels.get_sender(message.message_type()).send(Response::from(message)) {
                    error!("error sending IncomingMessages::CompletedOrdersEnd: {e}");
                }
            }
            IncomingMessages::CommissionsReport => {
                if let Some(execution_id) = message.execution_id() {
                    if let Err(e) = self.executions.send(&execution_id, Response::from(message)) {
                        error!("error sending commission report for execution {}: {}", execution_id, e);
                    }
                }
            }
            _ => (),
        }
    }

    // The cleanup thread receives signals as subscribers are dropped and
    // releases the sender channels
    fn start_cleanup_thread(self: &Arc<Self>) -> JoinHandle<()> {
        let message_bus = Arc::clone(self);

        thread::spawn(move || loop {
            let signal_recv = message_bus.signals_recv.clone();

            for signal in &signal_recv {
                match signal {
                    Signal::Request(request_id) => {
                        message_bus.clean_request(request_id);
                    }
                    Signal::Order(order_id) => {
                        message_bus.clean_order(order_id);
                    }
                }

                if message_bus.is_shutting_down() {
                    return;
                }
            }
        })
    }

    pub(crate) fn process_messages(self: &Arc<Self>, server_version: i32) -> Result<(), Error> {
        let handle = self.start_dispatcher_thread(server_version);
        self.add_join_handle(handle);

        let handle = self.start_cleanup_thread();
        self.add_join_handle(handle);

        Ok(())
    }

    fn add_join_handle(&self, handle: JoinHandle<()>) {}
}

const UNSPECIFIED_REQUEST_ID: i32 = -1;

impl MessageBus for TcpMessageBus {
    fn send_request(&self, request_id: i32, packet: &RequestMessage) -> Result<InternalSubscription, Error> {
        let (sender, receiver) = channel::unbounded();
        let sender_copy = sender.clone();

        self.requests.insert(request_id, sender);

        self.connection.write_message(packet)?;

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

        if let Err(e) = self.requests.send(&request_id, Response::Cancelled) {
            info!("error sending cancel notification: {e}");
        }

        self.requests.remove(&request_id);

        Ok(())
    }

    fn send_order_request(&self, order_id: i32, message: &RequestMessage) -> Result<InternalSubscription, Error> {
        let (sender, receiver) = channel::unbounded();
        let sender_copy = sender.clone();

        self.orders.insert(order_id, sender);

        self.connection.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .sender(sender_copy)
            .signaler(self.signals_send.clone())
            .order_id(order_id)
            .build();

        Ok(subscription)
    }

    fn cancel_order_subscription(&self, request_id: i32, message: &RequestMessage) -> Result<(), Error> {
        self.connection.write_message(message)?;

        if let Err(e) = self.orders.send(&request_id, Response::Cancelled) {
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
}

fn read_header(mut reader: &TcpStream) -> Result<usize, Error> {
    let buffer = &mut [0_u8; 4];
    reader.read_exact(buffer)?;

    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;

    Ok(count as usize)
}

fn error_event(server_version: i32, mut packet: ResponseMessage) -> Result<(), Error> {
    packet.skip(); // message_id

    let version = packet.next_int()?;

    if version < 2 {
        let message = packet.next_string()?;
        error!("version 2 error: {}", message);
        Ok(())
    } else {
        let request_id = packet.next_int()?;
        let error_code = packet.next_int()?;
        let error_message = packet.next_string()?;

        // if 322 forward to market_rule_id

        let mut advanced_order_reject_json: String = "".to_string();
        if server_version >= server_versions::ADVANCED_ORDER_REJECT {
            advanced_order_reject_json = packet.next_string()?;
        }
        debug!(
            "request_id: {}, error_code: {}, error_message: {}, advanced_order_reject_json: {}",
            request_id, error_code, error_message, advanced_order_reject_json
        );
        println!("[{error_code}] {error_message}");
        Ok(())
    }
}

#[derive(Debug)]
struct SenderHash<K, V> {
    data: RwLock<HashMap<K, Sender<V>>>,
}

impl<K: std::hash::Hash + Eq + std::fmt::Debug, V: std::fmt::Debug> SenderHash<K, V> {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub fn send(&self, id: &K, message: V) -> Result<(), Error> {
        let senders = self.data.read().unwrap();
        debug!("senders: {senders:?}");
        if let Some(sender) = senders.get(id) {
            if let Err(err) = sender.send(message) {
                error!("error sending: {id:?}, {err}")
            }
        } else {
            error!("no recipient found for: {id:?}, {message:?}")
        }
        Ok(())
    }

    pub fn copy_sender(&self, id: K) -> Option<Sender<V>> {
        let senders = self.data.read().unwrap();
        senders.get(&id).cloned()
    }

    pub fn insert(&self, id: K, message: Sender<V>) -> Option<Sender<V>> {
        let mut senders = self.data.write().unwrap();
        senders.insert(id, message)
    }

    pub fn remove(&self, id: &K) -> Option<Sender<V>> {
        let mut senders = self.data.write().unwrap();
        senders.remove(id)
    }

    pub fn contains(&self, id: &K) -> bool {
        let senders = self.data.read().unwrap();
        senders.contains_key(id)
    }

    pub fn len(&self) -> usize {
        let senders = self.data.read().unwrap();
        senders.len()
    }
}

// Enables routing of response messages from TWS to Client
#[derive(Debug)]
pub(crate) struct InternalSubscription {
    receiver: Option<Receiver<Response>>,              // requests with request ids receive responses via this channel
    sender: Option<Sender<Response>>,                  // requests with request ids receive responses via this channel
    shared_receiver: Option<Arc<Receiver<Response>>>,  // this channel is for responses that share channel based on message type
    signaler: Option<Sender<Signal>>,                  // for client to signal termination
    pub(crate) request_id: Option<i32>,                // initiating request id
    pub(crate) order_id: Option<i32>,                  // initiating order id
    pub(crate) message_type: Option<OutgoingMessages>, // initiating message type
}

impl InternalSubscription {
    // Blocks until next message become available.
    pub(crate) fn next(&self) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::receive(receiver)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::receive(receiver)
        } else {
            None
        }
    }

    // Returns message if available or immediately returns None.
    pub(crate) fn try_next(&self) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::try_receive(receiver)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::try_receive(receiver)
        } else {
            None
        }
    }

    // Waits for next message until specified timeout.
    pub(crate) fn next_timeout(&self, timeout: Duration) -> Option<Response> {
        if let Some(receiver) = &self.receiver {
            Self::timeout_receive(receiver, timeout)
        } else if let Some(receiver) = &self.shared_receiver {
            Self::timeout_receive(receiver, timeout)
        } else {
            None
        }
    }

    // pub(crate) fn cancel(&self) -> Result<(), Error> {
    //     if let Some(receiver) = &self.receiver {
    //         receiver.
    //     } else if let Some(receiver) = &self.shared_receiver {
    //         Self::timeout_receive(receiver, timeout)
    //     } else {
    //         None
    //     }

    //     if let (Some(request_id), Some(signaler)) = (self.request_id, &self.signaler) {
    //         signaler.send(Signal::Ca).unwrap();
    //     }

    //     if let (Some(order_id), Some(signaler)) = (self.order_id, &self.signaler) {
    //         signaler.send(Signal::Order(order_id)).unwrap();
    //     }

    //     Ok(())
    // }

    fn receive(receiver: &Receiver<Response>) -> Option<Response> {
        receiver.recv().ok()
    }

    fn try_receive(receiver: &Receiver<Response>) -> Option<Response> {
        receiver.try_recv().ok()
    }

    fn timeout_receive(receiver: &Receiver<Response>, timeout: Duration) -> Option<Response> {
        receiver.recv_timeout(timeout).ok()
    }
}

impl Drop for InternalSubscription {
    fn drop(&mut self) {
        if let (Some(request_id), Some(signaler)) = (self.request_id, &self.signaler) {
            signaler.send(Signal::Request(request_id)).unwrap();
        }

        if let (Some(order_id), Some(signaler)) = (self.order_id, &self.signaler) {
            signaler.send(Signal::Order(order_id)).unwrap();
        }
    }
}

pub(crate) struct SubscriptionBuilder {
    receiver: Option<Receiver<Response>>,
    sender: Option<Sender<Response>>,
    shared_receiver: Option<Arc<Receiver<Response>>>,
    signaler: Option<Sender<Signal>>,
    order_id: Option<i32>,
    request_id: Option<i32>,
    message_type: Option<OutgoingMessages>,
}

impl SubscriptionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            shared_receiver: None,
            signaler: None,
            order_id: None,
            request_id: None,
            message_type: None,
        }
    }

    pub(crate) fn receiver(mut self, receiver: Receiver<Response>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub(crate) fn sender(mut self, sender: Sender<Response>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub(crate) fn shared_receiver(mut self, shared_receiver: Arc<Receiver<Response>>) -> Self {
        self.shared_receiver = Some(shared_receiver);
        self
    }

    pub(crate) fn signaler(mut self, signaler: Sender<Signal>) -> Self {
        self.signaler = Some(signaler);
        self
    }

    pub(crate) fn order_id(mut self, order_id: i32) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub(crate) fn request_id(mut self, request_id: i32) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub(crate) fn message_type(mut self, message_type: OutgoingMessages) -> Self {
        self.message_type = Some(message_type);
        self
    }

    pub(crate) fn build(self) -> InternalSubscription {
        if let (Some(receiver), Some(signaler)) = (self.receiver, self.signaler) {
            InternalSubscription {
                receiver: Some(receiver),
                sender: self.sender,
                shared_receiver: None,
                signaler: Some(signaler),
                request_id: self.request_id,
                order_id: self.order_id,
                message_type: self.message_type,
            }
        } else if let Some(receiver) = self.shared_receiver {
            InternalSubscription {
                receiver: None,
                sender: None,
                shared_receiver: Some(receiver),
                signaler: None,
                request_id: self.request_id,
                order_id: self.order_id,
                message_type: self.message_type,
            }
        } else {
            panic!("bad configuration");
        }
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct ConnectionMetadata {
    next_order_id: i32,
    pub(crate) client_id: i32,
    pub(crate) server_version: i32,
    pub(crate) managed_accounts: String,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
}

#[derive(Debug)]
pub(crate) struct Connection {
    client_id: i32,
    connection_url: String,
    reader: Mutex<TcpStream>,
    writer: Mutex<TcpStream>,
    connection_metadata: Mutex<ConnectionMetadata>,
    max_retries: i32,
}

impl Connection {
    pub fn connect(client_id: i32, connection_url: &str) -> Result<Self, Error> {
        let reader = TcpStream::connect(connection_url)?;
        let writer = reader.try_clone()?;

        let connection = Self {
            client_id,
            connection_url: connection_url.into(),
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            connection_metadata: Mutex::new(ConnectionMetadata::default()),
            max_retries: MAX_RETRIES,
        };

        connection.establish_connection()?;

        Ok(connection)
    }

    pub fn connection_metadata(&self) -> ConnectionMetadata {
        let metadata = self.connection_metadata.lock().unwrap();
        metadata.clone()
    }

    pub fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);

        for i in 0..self.max_retries {
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            thread::sleep(next_delay);

            match TcpStream::connect(&self.connection_url) {
                Ok(stream) => {
                    {
                        let mut reader = self.reader.lock()?;
                        let mut writer = self.writer.lock()?;

                        *reader = stream.try_clone()?;
                        *writer = stream;
                    }

                    info!("reconnected !!!");
                    self.establish_connection()?;

                    return Ok(());
                }
                Err(e) => {
                    error!("reconnection attempt {i} of {} failed: {e}", self.max_retries);
                }
            }
        }

        Err(Error::ConnectionFailed)
    }

    fn establish_connection(&self) -> Result<(), Error> {
        self.handshake()?;
        self.start_api()?;
        self.receive_account_info()?;
        Ok(())
    }

    fn write(&self, data: &str) -> Result<(), Error> {
        let mut writer = self.writer.lock()?;
        writer.write_all(data.as_bytes())?;
        Ok(())
    }

    fn write_message(&self, message: &RequestMessage) -> Result<(), Error> {
        let mut writer = self.writer.lock()?;

        let data = message.encode();
        debug!("-> {data:?}");

        let data = data.as_bytes();

        let mut packet = Vec::with_capacity(data.len() + 4);

        packet.write_u32::<BigEndian>(data.len() as u32)?;
        packet.write_all(data)?;

        writer.write_all(&packet)?;

        Ok(())
    }

    fn read_message(&self) -> Result<ResponseMessage, Error> {
        let mut reader = self.reader.lock()?;

        let message_size = read_header(&reader)?;
        let mut data = vec![0_u8; message_size];

        reader.read_exact(&mut data)?;

        let raw_string = String::from_utf8(data)?;
        debug!("<- {:?}", raw_string);

        let packet = ResponseMessage::from(&raw_string);

        Ok(packet)
    }

    // sends server handshake
    fn handshake(&self) -> Result<(), Error> {
        let prefix = "API\0";
        let version = format!("v{MIN_SERVER_VERSION}..{MAX_SERVER_VERSION}");

        let packet = prefix.to_owned() + &encode_packet(&version);
        self.write(&packet)?;

        let ack = self.read_message();

        let mut connection_metadata = self.connection_metadata.lock()?;

        match ack {
            Ok(mut response) => {
                connection_metadata.server_version = response.next_int()?;

                let time = response.next_string()?;
                (connection_metadata.connection_time, connection_metadata.time_zone) = parse_connection_time(time.as_str());
            }
            Err(Error::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::Simple(format!("The server may be rejecting connections from this host: {err}")));
            }
            Err(err) => {
                return Err(err);
            }
        }
        Ok(())
    }

    // asks server to start processing messages
    fn start_api(&self) -> Result<(), Error> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&OutgoingMessages::StartApi);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version() > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.write_message(prelude)?;

        Ok(())
    }

    fn server_version(&self) -> i32 {
        let connection_metadata = self.connection_metadata.lock().unwrap();
        connection_metadata.server_version
    }

    // Fetches next order id and managed accounts.
    fn receive_account_info(&self) -> Result<(), Error> {
        let mut saw_next_order_id: bool = false;
        let mut saw_managed_accounts: bool = false;

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message()?;

            match message.message_type() {
                IncomingMessages::NextValidId => {
                    saw_next_order_id = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock()?;
                    connection_metadata.next_order_id = message.next_int()?;
                }
                IncomingMessages::ManagedAccounts => {
                    saw_managed_accounts = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock()?;
                    connection_metadata.managed_accounts = message.next_string()?;
                }
                IncomingMessages::Error => {
                    error!("message: {message:?}")
                }
                _ => info!("message: {message:?}"),
            }

            attempts += 1;
            if (saw_next_order_id && saw_managed_accounts) || attempts > MAX_ATTEMPTS {
                break;
            }
        }

        Ok(())
    }
}

struct FibonacciBackoff {
    previous: u64,
    current: u64,
    max: u64,
}

impl FibonacciBackoff {
    fn new(max: u64) -> Self {
        FibonacciBackoff {
            previous: 0,
            current: 1,
            max,
        }
    }

    fn next_delay(&mut self) -> Duration {
        let next = self.previous + self.current;
        self.previous = self.current;
        self.current = next;

        if next > self.max {
            Duration::from_secs(self.max)
        } else {
            Duration::from_secs(next)
        }
    }

    fn reset(&mut self) {
        self.previous = 0;
        self.current = 1;
    }
}

// Parses following format: 20230405 22:20:39 PST
fn parse_connection_time(connection_time: &str) -> (Option<OffsetDateTime>, Option<&'static Tz>) {
    let parts: Vec<&str> = connection_time.split(' ').collect();

    let zones = timezones::find_by_name(parts[2]);
    if zones.is_empty() {
        error!("time zone not found for {}", parts[2]);
        return (None, None);
    }

    let timezone = zones[0];

    let format = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let date_str = format!("{} {}", parts[0], parts[1]);
    let date = time::PrimitiveDateTime::parse(date_str.as_str(), format);
    match date {
        Ok(connected_at) => match connected_at.assume_timezone(timezone) {
            OffsetResult::Some(date) => (Some(date), Some(timezone)),
            _ => {
                error!("error setting timezone");
                (None, Some(timezone))
            }
        },
        Err(err) => {
            error!("could not parse connection time from {date_str}: {err}");
            (None, Some(timezone))
        }
    }
}

fn encode_packet(message: &str) -> String {
    let data = message.as_bytes();

    let mut packet: Vec<u8> = Vec::with_capacity(data.len() + 4);

    packet.write_u32::<BigEndian>(data.len() as u32).unwrap();
    packet.write_all(data).unwrap();

    std::str::from_utf8(&packet).unwrap().into()
}

#[cfg(test)]
mod tests;
