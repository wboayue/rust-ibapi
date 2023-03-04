use std::collections::HashMap;
use std::io::prelude::*;
use std::io::Cursor;
use std::iter::Iterator;
use std::net::TcpStream;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error, info};

use crate::client::{RequestMessage, ResponseMessage};
use crate::messages::IncomingMessages;
use crate::server_versions;
use recorder::MessageRecorder;

mod recorder;

pub trait MessageBus {
    fn read_message(&mut self) -> Result<ResponseMessage>;

    fn write_message(&mut self, packet: &RequestMessage) -> Result<()>;
    fn write_message_for_request(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponsePacketPromise>;
    fn send_order_message(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponsePacketPromise>;
    fn send_order_id_message(&mut self, message: &RequestMessage) -> Result<GlobalResponsePacketPromise>;
    fn send_open_orders_message(&mut self, message: &RequestMessage) -> Result<GlobalResponsePacketPromise>;

    fn write(&mut self, packet: &str) -> Result<()>;

    fn process_messages(&mut self, server_version: i32) -> Result<()>;
}

#[derive(Debug)]
pub struct TcpMessageBus {
    reader: Arc<TcpStream>,
    writer: Box<TcpStream>,
    handles: Vec<JoinHandle<i32>>,
    requests: Arc<SenderHash<ResponseMessage>>,
    orders: Arc<SenderHash<ResponseMessage>>,
    recorder: MessageRecorder,
    globals: GlobalChannels,
}

#[derive(Debug)]
struct GlobalChannels {
    order_ids_in: Arc<Sender<ResponseMessage>>,
    order_ids_out: Arc<Mutex<Receiver<ResponseMessage>>>,
    open_orders_in: Arc<Sender<ResponseMessage>>,
    open_orders_out: Arc<Mutex<Receiver<ResponseMessage>>>,
}

impl GlobalChannels {
    pub fn new() -> Self {
        let (order_ids_in, order_ids_out) = channel::unbounded();
        let (open_orders_in, open_orders_out) = channel::unbounded();

        GlobalChannels {
            order_ids_in: Arc::new(order_ids_in),
            order_ids_out: Arc::new(Mutex::new(order_ids_out)),
            open_orders_in: Arc::new(open_orders_in),
            open_orders_out: Arc::new(Mutex::new(open_orders_out)),
        }
    }
}

impl TcpMessageBus {
    // establishes TCP connection to server
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        let stream = TcpStream::connect(connection_string)?;

        let reader = Arc::new(stream.try_clone()?);
        let writer = Box::new(stream);
        let requests = Arc::new(SenderHash::new());
        let orders = Arc::new(SenderHash::new());

        Ok(TcpMessageBus {
            reader,
            writer,
            handles: Vec::default(),
            requests,
            orders,
            recorder: MessageRecorder::new(),
            globals: GlobalChannels::new(),
        })
    }

    fn add_request(&mut self, request_id: i32, sender: Sender<ResponseMessage>) -> Result<()> {
        self.requests.insert(request_id, sender);
        Ok(())
    }

    fn add_order(&mut self, order_id: i32, sender: Sender<ResponseMessage>) -> Result<()> {
        self.orders.insert(order_id, sender);
        Ok(())
    }
}

// impl read/write?

const UNSPECIFIED_REQUEST_ID: i32 = -1;

impl MessageBus for TcpMessageBus {
    fn read_message(&mut self) -> Result<ResponseMessage> {
        read_packet(&self.reader)
    }

    fn write_message_for_request(&mut self, request_id: i32, packet: &RequestMessage) -> Result<ResponsePacketPromise> {
        let (sender, receiver) = channel::unbounded();
        let (signals_out, signals_in) = channel::unbounded();

        self.add_request(request_id, sender)?;
        self.write_message(packet)?;

        Ok(ResponsePacketPromise::new(receiver, signals_out, Some(request_id), None))
    }

    fn send_order_message(&mut self, order_id: i32, message: &RequestMessage) -> Result<ResponsePacketPromise> {
        let (sender, receiver) = channel::unbounded();
        let (signals_out, signals_in) = channel::unbounded();

        self.add_order(order_id, sender)?;
        self.write_message(message)?;

        Ok(ResponsePacketPromise::new(receiver, signals_out, None, Some(order_id)))
    }

    fn send_order_id_message(&mut self, message: &RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.write_message(message)?;
        Ok(GlobalResponsePacketPromise::new(Arc::clone(&self.globals.order_ids_out)))
    }

    fn send_open_orders_message(&mut self, message: &RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.write_message(message)?;
        Ok(GlobalResponsePacketPromise::new(Arc::clone(&self.globals.open_orders_out)))
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<()> {
        let encoded = message.encode();
        debug!("{encoded:?} ->");

        let data = encoded.as_bytes();
        let mut header = Vec::with_capacity(data.len());
        header.write_u32::<BigEndian>(data.len() as u32)?;

        self.writer.write_all(&header)?;
        self.writer.write_all(data)?;

        self.recorder.record_request(message);

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        debug!("{data:?} ->");
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> Result<()> {
        let reader = Arc::clone(&self.reader);
        let requests = Arc::clone(&self.requests);
        let recorder = self.recorder.clone();
        let orders = Arc::clone(&self.orders);

        let handle = thread::spawn(move || loop {
            match read_packet(&reader) {
                Ok(message) => {
                    recorder.record_response(&message);
                    dispatch_message(message, server_version, &requests, &orders);
                }
                Err(err) => {
                    error!("error reading packet: {:?}", err);
                    // thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };

            // FIXME - does read block?
            // thread::sleep(Duration::from_secs(1));
        });

        self.handles.push(handle);

        Ok(())
    }
}

fn dispatch_message(
    message: ResponseMessage,
    server_version: i32,
    requests: &Arc<SenderHash<ResponseMessage>>,
    orders: &Arc<SenderHash<ResponseMessage>>,
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
        IncomingMessages::NextValidId => process_next_valid_id(server_version, message),
        IncomingMessages::ManagedAccounts => process_managed_accounts(server_version, message),
        IncomingMessages::OrderStatus
        | IncomingMessages::OpenOrder
        | IncomingMessages::OpenOrderEnd
        | IncomingMessages::ExecutionData
        | IncomingMessages::ExecutionDataEnd
        | IncomingMessages::CommissionsReport => process_order_notifications(message, requests, orders),
        _ => process_response(requests, orders, message),
    };
}

fn read_packet(mut reader: &TcpStream) -> Result<ResponseMessage> {
    let message_size = read_header(reader)?;
    debug!("message size: {message_size}");
    let mut data = vec![0_u8; message_size];

    reader.read_exact(&mut data)?;

    let packet = ResponseMessage::from(&String::from_utf8(data)?);
    debug!("raw string: {:?}", packet);

    Ok(packet)
}

fn read_header(mut reader: &TcpStream) -> Result<usize> {
    let buffer = &mut [0_u8; 4];
    reader.read_exact(buffer)?;

    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;

    Ok(count as usize)
}

fn error_event(server_version: i32, mut packet: ResponseMessage) -> Result<()> {
    packet.skip(); // message_id

    let version = packet.next_int()?;

    if version < 2 {
        let message = packet.next_string()?;
        error!("version 2 erorr: {}", message);
        Ok(())
    } else {
        let request_id = packet.next_int()?;
        let error_code = packet.next_int()?;
        let error_message = packet.next_string()?;
        // let error_message = if server_version >= server_versions::ENCODE_MSG_ASCII7 {
        //     // Regex.Unescape(ReadString()) : ReadString();
        //     packet.next_string()?
        // } else {
        //     packet.next_string()?
        // };

        let mut advanced_order_reject_json: String = "".to_string();
        if server_version >= server_versions::ADVANCED_ORDER_REJECT {
            advanced_order_reject_json = packet.next_string()?;
            // if (!Util.StringIsEmpty(tempStr))
            // {
            //     advancedOrderRejectJson = Regex.Unescape(tempStr);
            // }
        }
        error!(
            "request_id: {}, error_code: {}, error_message: {}, advanced_order_reject_json: {}",
            request_id, error_code, error_message, advanced_order_reject_json
        );
        Ok(())
    }
}

fn process_next_valid_id(_server_version: i32, mut packet: ResponseMessage) {
    packet.skip(); // message_id
    packet.skip(); // version

    let order_id = packet.next_string().unwrap_or_else(|_| String::default());
    info!("next_valid_order_id: {}", order_id)
}

fn process_managed_accounts(_server_version: i32, mut packet: ResponseMessage) {
    packet.skip(); // message_id
    packet.skip(); // version

    let managed_accounts = packet.next_string().unwrap_or_else(|_| String::default());
    info!("managed accounts: {}", managed_accounts)
}

fn process_response(requests: &Arc<SenderHash<ResponseMessage>>, orders: &Arc<SenderHash<ResponseMessage>>, message: ResponseMessage) {
    let request_id = message.request_id().unwrap_or(-1); // pass in request id?
    if requests.contains(request_id) {
        requests.send(request_id, message).unwrap();
    } else if orders.contains(request_id) {
        orders.send(request_id, message).unwrap();
    }
}

fn process_order_notifications(message: ResponseMessage, requests: &Arc<SenderHash<ResponseMessage>>, orders: &Arc<SenderHash<ResponseMessage>>) {
    match message.message_type() {
        IncomingMessages::OrderStatus | IncomingMessages::OpenOrder | IncomingMessages::ExecutionData => {
            if let Some(order_id) = message.order_id() {
                if let Err(e) = orders.send(order_id, message) {
                    error!("error routing message for order_id({order_id}): {e}");
                }
                return;
            }

            if let Some(request_id) = message.request_id() {
                if let Err(e) = requests.send(request_id, message) {
                    error!("error routing message for request_id({request_id}): {e}");
                }
                return;
            }

            error!("message has no order_id: {message:?}");
        }
        _ => (),
    }
    // | IncomingMessages::OpenOrderEnd
    // | IncomingMessages::ExecutionDataEnd
    // | IncomingMessages::CommissionsReport => process_order_notifications(message, requests, orders),
}

#[derive(Debug)]
struct SenderHash<T> {
    data: RwLock<HashMap<i32, Sender<T>>>,
}

impl<T: std::fmt::Debug> SenderHash<T> {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub fn send(&self, id: i32, message: T) -> Result<()> {
        let senders = self.data.read().unwrap();
        debug!("senders: {senders:?}");
        if let Some(sender) = senders.get(&id) {
            if let Err(err) = sender.send(message) {
                error!("error sending: {id}, {err}")
            }
        } else {
            error!("no recipient found for: {id}, {message:?}")
        }
        Ok(())
    }

    pub fn insert(&self, id: i32, message: Sender<T>) -> Option<Sender<T>> {
        let mut senders = self.data.write().unwrap();
        senders.insert(id, message)
    }

    pub fn remove(&self, id: i32) -> Option<Sender<T>> {
        let mut senders = self.data.write().unwrap();
        senders.remove(&id)
    }

    pub fn contains(&self, id: i32) -> bool {
        let senders = self.data.read().unwrap();
        senders.contains_key(&id)
    }
}

#[derive(Debug)]
pub struct ResponsePacketPromise {
    messages: Receiver<ResponseMessage>, // for client to receive incoming messages
    signals: Sender<i32>,                // for client to signal termination
    request_id: Option<i32>,             // initiating request_id
    order_id: Option<i32>,               // initiating order_id
}

impl ResponsePacketPromise {
    pub fn new(messages: Receiver<ResponseMessage>, signals: Sender<i32>, request_id: Option<i32>, order_id: Option<i32>) -> Self {
        ResponsePacketPromise {
            messages,
            signals,
            request_id,
            order_id,
        }
    }

    #[deprecated]
    pub fn message(&self) -> Result<ResponseMessage> {
        // Duration::from_millis(100)

        Ok(self.messages.recv_timeout(Duration::from_secs(20))?)
        // return Err(anyhow!("no message"));
    }

    pub fn signal(&self, id: i32) {
        self.signals.send(id).unwrap();
    }
}

impl Iterator for ResponsePacketPromise {
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

#[derive(Debug)]
pub struct GlobalResponsePacketPromise {
    messages: Arc<Mutex<Receiver<ResponseMessage>>>,
}

impl GlobalResponsePacketPromise {
    pub fn new(messages: Arc<Mutex<Receiver<ResponseMessage>>>) -> Self {
        Self { messages }
    }
}
#[cfg(test)]
mod tests;
