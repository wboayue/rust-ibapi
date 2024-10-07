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

use crate::messages::IncomingMessages;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::{server_versions, Error};
use recorder::MessageRecorder;

mod recorder;

pub(crate) trait MessageBus: Send + Sync {
    fn read_message(&mut self) -> Result<ResponseMessage, Error>;

    fn write_message(&mut self, packet: &RequestMessage) -> Result<(), Error>;

    fn send_generic_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponseIterator, Error>;
    fn send_durable_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponseIterator, Error>;
    fn send_order_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponseIterator, Error>;
    fn request_next_order_id(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error>;
    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error>;
    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error>;
    fn request_positions(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error>;
    fn request_family_codes(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error>;

    fn write(&mut self, packet: &str) -> Result<(), Error>;

    fn process_messages(&mut self, server_version: i32) -> Result<(), Error>;

    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
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

    fn send_generic_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponseIterator, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_request(request_id, sender)?;
        self.write_message(packet)?;

        Ok(ResponseIterator::new(
            receiver,
            self.signals_send.clone(),
            Some(request_id),
            None,
            Some(Duration::from_secs(10)),
        ))
    }

    fn send_durable_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponseIterator, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_request(request_id, sender)?;
        self.write_message(packet)?;

        Ok(ResponseIterator::new(receiver, self.signals_send.clone(), Some(request_id), None, None))
    }

    fn send_order_message(&mut self, order_id: i32, message: &RequestMessage) -> Result<ResponseIterator, Error> {
        let (sender, receiver) = channel::unbounded();

        self.add_order(order_id, sender)?;
        self.write_message(message)?;

        Ok(ResponseIterator::new(
            receiver,
            self.signals_send.clone(),
            None,
            Some(order_id),
            Some(Duration::from_secs(10)),
        ))
    }

    fn request_next_order_id(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.write_message(message)?;
        Ok(GlobalResponseIterator::new(Arc::clone(&self.globals.order_ids_out)))
    }

    fn request_open_orders(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.write_message(message)?;
        Ok(GlobalResponseIterator::new(Arc::clone(&self.globals.open_orders_out)))
    }

    fn request_market_rule(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.write_message(message)?;
        Ok(GlobalResponseIterator::new(Arc::clone(&self.globals.recv_market_rule)))
    }

    fn request_positions(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.write_message(message)?;
        Ok(GlobalResponseIterator::new(Arc::clone(&self.globals.recv_positions)))
    }

    fn request_family_codes(&mut self, message: &RequestMessage) -> Result<GlobalResponseIterator, Error> {
        self.write_message(message)?;
        Ok(GlobalResponseIterator::new(Arc::clone(&self.globals.recv_family_codes)))
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<(), Error> {
        let data = message.encode();
        debug!("-> {data:?}");

        let data = data.as_bytes();

        let mut packet = Vec::with_capacity(data.len() + 4);

        packet.write_u32::<BigEndian>(data.len() as u32)?;
        packet.write_all(data)?;

        self.writer.lock().expect("MessageBus writer is poisoned").write_all(&packet)?;

        self.recorder.record_request(message);

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<(), Error> {
        debug!("{data:?} ->");
        self.writer.lock().expect("MessageBus writer is poisoned").write_all(data.as_bytes())?;
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

            // thread::sleep(Duration::from_secs(1));
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

#[derive(Debug)]
pub(crate) struct ResponseIterator {
    messages: Receiver<ResponseMessage>, // for client to receive incoming messages
    signals: Sender<Signal>,             // for client to signal termination
    request_id: Option<i32>,             // initiating request_id
    order_id: Option<i32>,               // initiating order_id
    timeout: Option<Duration>,           // How long to wait for next message
}

impl ResponseIterator {
    pub(crate) fn new(
        messages: Receiver<ResponseMessage>,
        signals: Sender<Signal>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        timeout: Option<Duration>,
    ) -> Self {
        ResponseIterator {
            messages,
            signals,
            request_id,
            order_id,
            timeout,
        }
    }

    pub(crate) fn try_next(&mut self) -> Option<ResponseMessage> {
        match self.messages.try_recv() {
            Ok(message) => Some(message),
            Err(err) => {
                debug!("try_next: {err}");
                None
            }
        }
    }

    pub(crate) fn next_timeout(&mut self, timeout: Duration) -> Option<ResponseMessage> {
        match self.messages.recv_timeout(timeout) {
            Ok(message) => Some(message),
            Err(err) => {
                info!("timeout receiving message: {err}");
                None
            }
        }
    }
}

impl Drop for ResponseIterator {
    fn drop(&mut self) {
        if let Some(request_id) = self.request_id {
            self.signals.send(Signal::Request(request_id)).unwrap();
        }

        if let Some(order_id) = self.order_id {
            self.signals.send(Signal::Order(order_id)).unwrap();
        }
    }
}

impl Iterator for ResponseIterator {
    type Item = ResponseMessage;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(timeout) = self.timeout {
            match self.messages.recv_timeout(timeout) {
                Ok(message) => Some(message),
                Err(err) => {
                    info!("timeout receiving message: {err}");
                    None
                }
            }
        } else {
            match self.messages.recv() {
                Ok(message) => Some(message),
                Err(err) => {
                    error!("error receiving message: {err}");
                    None
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct GlobalResponseIterator {
    messages: Arc<Receiver<ResponseMessage>>,
}

impl GlobalResponseIterator {
    pub fn new(messages: Arc<Receiver<ResponseMessage>>) -> Self {
        Self { messages }
    }
}

impl Iterator for GlobalResponseIterator {
    type Item = ResponseMessage;
    fn next(&mut self) -> Option<Self::Item> {
        match self.messages.recv_timeout(Duration::from_secs(5)) {
            Err(err) => {
                info!("timeout receiving packet: {err}");
                None
            }
            Ok(message) => Some(message),
        }
    }
}

#[cfg(test)]
mod tests;
