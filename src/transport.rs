//! This module implements a message bus for handling communications with TWS.
//! It provides functionality for routing requests from the Client to TWS,
//! and responses from TWS back to the Client.

use std::collections::HashMap;
use std::io::{prelude::*, Cursor};
use std::iter::Iterator;
use std::net::TcpStream;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info};

use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::{server_versions, Error};
use recorder::MessageRecorder;

mod recorder;

pub(crate) trait MessageBus: Send + Sync {
    fn read_message(&mut self) -> Result<ResponseMessage, Error>;

    fn write_message(&mut self, packet: &RequestMessage) -> Result<(), Error>;

    fn send_generic_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<BusSubscription, Error>;
    fn send_durable_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<BusSubscription, Error>;
    fn send_order_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<BusSubscription, Error>;

    fn send_shared_message(&mut self, message_id: OutgoingMessages, packet: &RequestMessage) -> Result<BusSubscription, Error>;

    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error>;
    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error>;
    fn request_positions(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error>;
    fn request_family_codes(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error>;

    fn write(&mut self, packet: &str) -> Result<(), Error>;

    fn process_messages(&mut self, server_version: i32) -> Result<(), Error>;

    // Exists for testing when request are stubbed
    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
    }
}

// For requests without an identifier, shared channels are created
// to route request/response pairs based on message type.
#[derive(Debug)]
struct SharedChannels {
    // Maps an inbound reply to channel used to send responses.
    senders: HashMap<IncomingMessages, Arc<Sender<ResponseMessage>>>,
    // Maps an outbound request to channel used to receive responses.
    receivers: HashMap<OutgoingMessages, Arc<Receiver<ResponseMessage>>>,
}

impl SharedChannels {
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

        instance
    }

    // Maps an outgoing message to incoming message(s)
    fn register(&mut self, outbound: OutgoingMessages, inbounds: &[IncomingMessages]) {
        let (sender, receiver) = channel::unbounded::<ResponseMessage>();

        self.receivers.insert(outbound, Arc::new(receiver));

        let sender = &Arc::new(sender);

        for inbound in inbounds {
            self.senders.insert(*inbound, Arc::clone(sender));
        }
    }

    pub fn get_receiver(&self, message_id: OutgoingMessages) -> Arc<Receiver<ResponseMessage>> {
        let receiver = self
            .receivers
            .get(&message_id)
            .expect(&format!("unsupported request message {:?}", message_id));
        Arc::clone(receiver)
    }

    pub fn get_sender(&self, message_id: IncomingMessages) -> Arc<Sender<ResponseMessage>> {
        let sender = self
            .senders
            .get(&message_id)
            .expect(&format!("unsupported response message {:?}", message_id));
        Arc::clone(sender)
    }
}

#[derive(Debug)]
pub struct TcpMessageBus {
    reader: Arc<TcpStream>,
    writer: Arc<Mutex<TcpStream>>,
    handles: Vec<JoinHandle<i32>>,
    requests: Arc<SenderHash<i32, ResponseMessage>>,
    orders: Arc<SenderHash<i32, ResponseMessage>>,
    recorder: MessageRecorder,
    globals: Arc<GlobalChannels>,
    shared_channels: Arc<SharedChannels>,
    signals_send: Sender<Signal>,
    signals_recv: Receiver<Signal>,
}

pub enum Signal {
    Request(i32),
    Order(i32),
}

#[derive(Debug)]
struct GlobalChannels {
    order_ids_in: Arc<Sender<ResponseMessage>>,
    order_ids_out: Arc<Receiver<ResponseMessage>>,
    open_orders_in: Arc<Sender<ResponseMessage>>,
    open_orders_out: Arc<Receiver<ResponseMessage>>,
    send_market_rule: Arc<Sender<ResponseMessage>>,
    recv_market_rule: Arc<Receiver<ResponseMessage>>,
    send_positions: Arc<Sender<ResponseMessage>>,
    recv_positions: Arc<Receiver<ResponseMessage>>,
    send_family_codes: Arc<Sender<ResponseMessage>>,
    recv_family_codes: Arc<Receiver<ResponseMessage>>,
}

impl GlobalChannels {
    pub fn new() -> Self {
        let (order_ids_in, order_ids_out) = channel::unbounded();
        let (open_orders_in, open_orders_out) = channel::unbounded();
        let (send_market_rule, recv_market_rule) = channel::unbounded();
        let (send_positions, recv_positions) = channel::unbounded();
        let (send_family_codes, recv_family_codes) = channel::unbounded();

        GlobalChannels {
            order_ids_in: Arc::new(order_ids_in),
            order_ids_out: Arc::new(order_ids_out),
            open_orders_in: Arc::new(open_orders_in),
            open_orders_out: Arc::new(open_orders_out),
            send_market_rule: Arc::new(send_market_rule),
            recv_market_rule: Arc::new(recv_market_rule),
            send_positions: Arc::new(send_positions),
            recv_positions: Arc::new(recv_positions),
            send_family_codes: Arc::new(send_family_codes),
            recv_family_codes: Arc::new(recv_family_codes),
        }
    }
}

impl TcpMessageBus {
    // establishes TCP connection to server
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus, Error> {
        let stream = TcpStream::connect(connection_string)?;

        let reader = Arc::new(stream.try_clone()?);
        let writer = Arc::new(Mutex::new(stream));
        let requests = Arc::new(SenderHash::new());
        let orders = Arc::new(SenderHash::new());

        let (signals_send, signals_recv) = channel::unbounded();

        Ok(TcpMessageBus {
            reader,
            writer,
            handles: Vec::default(),
            requests,
            orders,
            recorder: MessageRecorder::new(),
            globals: Arc::new(GlobalChannels::new()),
            shared_channels: Arc::new(SharedChannels::new()),
            signals_send,
            signals_recv,
        })
    }

    fn add_request(&mut self, request_id: i32, sender: Sender<ResponseMessage>) -> Result<(), Error> {
        self.requests.insert(request_id, sender);
        Ok(())
    }

    fn add_order(&mut self, order_id: i32, sender: Sender<ResponseMessage>) -> Result<(), Error> {
        self.orders.insert(order_id, sender);
        Ok(())
    }
}

const UNSPECIFIED_REQUEST_ID: i32 = -1;

impl MessageBus for TcpMessageBus {
    fn read_message(&mut self) -> Result<ResponseMessage, Error> {
        read_packet(&self.reader)
    }

    fn send_generic_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<BusSubscription, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_request(request_id, sender)?;
        self.write_message(packet)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .signaler(self.signals_send.clone())
            .request_id(request_id)
            .build();

        Ok(subscription)
    }

    fn send_durable_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<BusSubscription, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_request(request_id, sender)?;
        self.write_message(packet)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .signaler(self.signals_send.clone())
            .request_id(request_id)
            .build();

        Ok(subscription)
    }

    fn send_order_message(&mut self, order_id: i32, message: &RequestMessage) -> Result<BusSubscription, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_order(order_id, sender)?;
        self.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .signaler(self.signals_send.clone())
            .order_id(order_id)
            .build();

        Ok(subscription)
    }

    fn send_shared_message(&mut self, message_id: OutgoingMessages, message: &RequestMessage) -> Result<BusSubscription, Error> {
        self.write_message(message)?;

        let shared_receiver = self.shared_channels.get_receiver(message_id);

        let subscription = SubscriptionBuilder::new().shared_receiver(shared_receiver).build();

        Ok(subscription)
    }

    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error> {
        self.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(Arc::clone(&self.globals.open_orders_out))
            .build();

        Ok(subscription)
    }

    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error> {
        self.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(Arc::clone(&self.globals.recv_market_rule))
            .build();

        Ok(subscription)
    }

    fn request_positions(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error> {
        self.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(Arc::clone(&self.globals.recv_positions))
            .build();

        Ok(subscription)
    }

    fn request_family_codes(&mut self, message: &RequestMessage) -> Result<BusSubscription, Error> {
        self.write_message(message)?;

        let subscription = SubscriptionBuilder::new()
            .shared_receiver(Arc::clone(&self.globals.recv_family_codes))
            .build();

        Ok(subscription)
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<(), Error> {
        let data = message.encode();
        debug!("-> {data:?}");

        let data = data.as_bytes();

        let mut packet = Vec::with_capacity(data.len() + 4);

        packet.write_u32::<BigEndian>(data.len() as u32)?;
        packet.write_all(data)?;

        self.writer.lock()?.write_all(&packet)?;

        self.recorder.record_request(message);

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<(), Error> {
        debug!("{data:?} ->");
        self.writer.lock()?.write_all(data.as_bytes())?;
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> Result<(), Error> {
        let reader = Arc::clone(&self.reader);
        let requests = Arc::clone(&self.requests);
        let recorder = self.recorder.clone();
        let orders = Arc::clone(&self.orders);
        let globals = Arc::clone(&self.globals);
        let executions = SenderHash::<String, ResponseMessage>::new();

        let handle = thread::spawn(move || loop {
            match read_packet(&reader) {
                Ok(message) => {
                    recorder.record_response(&message);
                    dispatch_message(message, server_version, &requests, &orders, &globals, &executions);
                }
                Err(err) => {
                    error!("error reading packet: {:?}", err);
                    continue;
                }
            };
        });

        self.handles.push(handle);

        let requests = Arc::clone(&self.requests);
        let orders = Arc::clone(&self.orders);
        let signal_recv = self.signals_recv.clone();

        let handle = thread::spawn(move || loop {
            for signal in &signal_recv {
                match signal {
                    Signal::Request(request_id) => {
                        requests.remove(&request_id);
                        debug!("released request_id {}, requests.len()={}", request_id, requests.len());
                    }
                    Signal::Order(order_id) => {
                        orders.remove(&order_id);
                        debug!("released order_id {}, orders.len()={}", order_id, requests.len());
                    }
                }
            }
        });

        self.handles.push(handle);

        Ok(())
    }
}

fn dispatch_message(
    message: ResponseMessage,
    server_version: i32,
    requests: &Arc<SenderHash<i32, ResponseMessage>>,
    orders: &Arc<SenderHash<i32, ResponseMessage>>,
    globals: &Arc<GlobalChannels>,
    executions: &SenderHash<String, ResponseMessage>,
) {
    match message.message_type() {
        IncomingMessages::Error => {
            let request_id = message.peek_int(2).unwrap_or(-1);

            if request_id == UNSPECIFIED_REQUEST_ID {
                error_event(server_version, message).unwrap();
            } else {
                process_response(requests, orders, message);
            }
        }
        IncomingMessages::NextValidId => {
            globals.order_ids_in.send(message).unwrap();
        }
        IncomingMessages::MarketRule => {
            globals.send_market_rule.send(message).unwrap();
        }
        IncomingMessages::Position | IncomingMessages::PositionEnd => {
            globals.send_positions.send(message).unwrap();
        }
        IncomingMessages::FamilyCodes => {
            globals.send_family_codes.send(message).unwrap();
        }

        IncomingMessages::ManagedAccounts => process_managed_accounts(server_version, message),
        IncomingMessages::OrderStatus
        | IncomingMessages::OpenOrder
        | IncomingMessages::OpenOrderEnd
        | IncomingMessages::CompletedOrder
        | IncomingMessages::CompletedOrdersEnd
        | IncomingMessages::ExecutionData
        | IncomingMessages::ExecutionDataEnd
        | IncomingMessages::CommissionsReport => process_orders(message, requests, orders, executions, globals),
        _ => process_response(requests, orders, message),
    };
}

fn read_packet(mut reader: &TcpStream) -> Result<ResponseMessage, Error> {
    let message_size = read_header(reader)?;
    let mut data = vec![0_u8; message_size];

    reader.read_exact(&mut data)?;

    let raw_string = String::from_utf8(data)?;
    debug!("<- {:?}", raw_string);

    let packet = ResponseMessage::from(&raw_string);

    Ok(packet)
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

fn process_managed_accounts(_server_version: i32, mut packet: ResponseMessage) {
    packet.skip(); // message_id
    packet.skip(); // version

    let managed_accounts = packet.next_string().unwrap_or_else(|_| String::default());
    info!("managed accounts: {}", managed_accounts)
}

fn process_response(requests: &Arc<SenderHash<i32, ResponseMessage>>, orders: &Arc<SenderHash<i32, ResponseMessage>>, message: ResponseMessage) {
    let request_id = message.request_id().unwrap_or(-1); // pass in request id?
    if requests.contains(&request_id) {
        requests.send(&request_id, message).unwrap();
    } else if orders.contains(&request_id) {
        orders.send(&request_id, message).unwrap();
    } else {
        info!("no recipient found for: {:?}", message)
    }
}

fn process_orders(
    message: ResponseMessage,
    requests: &Arc<SenderHash<i32, ResponseMessage>>,
    orders: &Arc<SenderHash<i32, ResponseMessage>>,
    executions: &SenderHash<String, ResponseMessage>,
    globals: &Arc<GlobalChannels>,
) {
    match message.message_type() {
        IncomingMessages::ExecutionData => {
            match (message.order_id(), message.request_id()) {
                // First check matching orders channel
                (Some(order_id), _) if orders.contains(&order_id) => {
                    if let Err(e) = orders.send(&order_id, message) {
                        error!("error routing message for order_id({order_id}): {e}");
                    }
                }
                (_, Some(request_id)) if requests.contains(&request_id) => {
                    if let Some(sender) = requests.copy_sender(request_id) {
                        if let Some(execution_id) = message.execution_id() {
                            executions.insert(execution_id, sender);
                        }
                    }

                    if let Err(e) = requests.send(&request_id, message) {
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
                (Some(order_id), _) if orders.contains(&order_id) => {
                    if let Err(e) = orders.send(&order_id, message) {
                        error!("error routing message for order_id({order_id}): {e}");
                    }
                }
                (_, Some(request_id)) if requests.contains(&request_id) => {
                    if let Err(e) = requests.send(&request_id, message) {
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
                if orders.contains(&order_id) {
                    if let Err(e) = orders.send(&order_id, message) {
                        error!("error routing message for order_id({order_id}): {e}");
                    }
                } else if let Err(e) = globals.open_orders_in.send(message) {
                    error!("error sending IncomingMessages::OpenOrder: {e}");
                }
            }
        }
        IncomingMessages::CompletedOrder => {
            if let Err(e) = globals.open_orders_in.send(message) {
                error!("error sending IncomingMessages::CompletedOrder: {e}");
            }
        }
        IncomingMessages::OpenOrderEnd => {
            if let Err(e) = globals.open_orders_in.send(message) {
                error!("error sending IncomingMessages::OpenOrderEnd: {e}");
            }
        }
        IncomingMessages::CompletedOrdersEnd => {
            if let Err(e) = globals.open_orders_in.send(message) {
                error!("error sending IncomingMessages::CompletedOrdersEnd: {e}");
            }
        }
        IncomingMessages::CommissionsReport => {
            if let Some(execution_id) = message.execution_id() {
                if let Err(e) = executions.send(&execution_id, message) {
                    error!("error sending commission report for execution {}: {}", execution_id, e);
                }
            }
        }
        _ => (),
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
pub(crate) struct BusSubscription {
    receiver: Option<Receiver<ResponseMessage>>, // for client to receive incoming messages
    shared_receiver: Option<Arc<Receiver<ResponseMessage>>>,
    signaler: Option<Sender<Signal>>, // for client to signal termination
    request_id: Option<i32>,          // initiating request_id
    order_id: Option<i32>,            // initiating order_id
    timeout: Option<Duration>,        // How long to wait for next message
}

impl BusSubscription {
    pub(crate) fn new(
        messages: Receiver<ResponseMessage>,
        signals: Sender<Signal>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        timeout: Option<Duration>,
    ) -> Self {
        BusSubscription {
            receiver: Some(messages),
            shared_receiver: None,
            signaler: Some(signals),
            request_id,
            order_id,
            timeout,
        }
    }

    pub(crate) fn try_next(&mut self) -> Option<ResponseMessage> {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(message) => Some(message),
                Err(err) => {
                    debug!("try_next: {err}");
                    None
                }
            }
        } else if let Some(receiver) = &self.shared_receiver {
            match receiver.try_recv() {
                Ok(message) => Some(message),
                Err(err) => {
                    debug!("try_next: {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    pub(crate) fn next_timeout(&mut self, timeout: Duration) -> Option<ResponseMessage> {
        if let Some(receiver) = &self.receiver {
            match receiver.recv_timeout(timeout) {
                Ok(message) => Some(message),
                Err(err) => {
                    info!("timeout receiving message: {err}");
                    None
                }
            }
        } else {
            None
        }
    }
}

impl Drop for BusSubscription {
    fn drop(&mut self) {
        if let (Some(request_id), Some(signaler)) = (self.request_id, &self.signaler) {
            signaler.send(Signal::Request(request_id)).unwrap();
        }

        if let (Some(order_id), Some(signaler)) = (self.order_id, &self.signaler) {
            signaler.send(Signal::Order(order_id)).unwrap();
        }
    }
}

impl Iterator for BusSubscription {
    type Item = ResponseMessage;
    fn next(&mut self) -> Option<Self::Item> {
        if let (Some(timeout), Some(receiver)) = (self.timeout, &self.receiver) {
            match receiver.recv_timeout(timeout) {
                Ok(message) => Some(message),
                Err(err) => {
                    info!("timeout receiving message: {err}");
                    None
                }
            }
        } else if let Some(receiver) = &self.receiver {
            match receiver.recv() {
                Ok(message) => Some(message),
                Err(err) => {
                    error!("error receiving message: {err}");
                    None
                }
            }
        } else if let Some(receiver) = &self.shared_receiver {
            match receiver.recv() {
                Ok(message) => Some(message),
                Err(err) => {
                    error!("error receiving message: {err}");
                    None
                }
            }
        } else {
            None
        }
    }
}

pub(crate) struct SubscriptionBuilder {
    receiver: Option<Receiver<ResponseMessage>>,
    shared_receiver: Option<Arc<Receiver<ResponseMessage>>>,
    signaler: Option<Sender<Signal>>,
    order_id: Option<i32>,
    request_id: Option<i32>,
}

impl SubscriptionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            receiver: None,
            shared_receiver: None,
            signaler: None,
            order_id: None,
            request_id: None,
        }
    }

    pub(crate) fn receiver(mut self, receiver: Receiver<ResponseMessage>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub(crate) fn shared_receiver(mut self, shared_receiver: Arc<Receiver<ResponseMessage>>) -> Self {
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

    pub(crate) fn build(self) -> BusSubscription {
        if let (Some(receiver), Some(signaler)) = (self.receiver, self.signaler) {
            return BusSubscription {
                receiver: Some(receiver),
                shared_receiver: None,
                signaler: Some(signaler),
                request_id: self.request_id,
                order_id: self.order_id,
                timeout: None,
            };
        } else if let Some(receiver) = self.shared_receiver {
            return BusSubscription {
                receiver: None,
                shared_receiver: Some(receiver),
                signaler: None,
                request_id: self.request_id,
                order_id: self.order_id,
                timeout: None,
            };
        }

        panic!("bad configuration");
    }
}

#[cfg(test)]
mod tests;
