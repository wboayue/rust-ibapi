use std::collections::HashMap;
use std::io::prelude::*;
use std::io::Cursor;
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error, info};

use crate::client::{RequestPacket, ResponsePacket};
use crate::messages::IncomingMessage;
use crate::server_versions;

pub trait MessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn read_packet_for_request(&mut self, request_id: i32) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()>;
    fn write_packet_for_request(
        &mut self,
        request_id: i32,
        packet: &RequestPacket,
    ) -> Result<ResponsePacketPromise>;
    fn write(&mut self, packet: &str) -> Result<()>;
    fn process_messages(&mut self, server_version: i32) -> Result<()>;
}

#[derive(Debug)]
struct Outbox(Sender<ResponsePacket>);

#[derive(Debug)]
pub struct TcpMessageBus {
    reader: Arc<TcpStream>,
    writer: Box<TcpStream>,
    handles: Vec<JoinHandle<i32>>,
    requests: Arc<RwLock<HashMap<i32, Outbox>>>,
}

unsafe impl Send for Outbox {}
unsafe impl Sync for Outbox {}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        let stream = TcpStream::connect(connection_string)?;

        let reader = Arc::new(stream.try_clone()?);
        let writer = Box::new(stream);
        let requests = Arc::new(RwLock::new(HashMap::default()));

        Ok(TcpMessageBus {
            reader,
            writer,
            handles: Vec::default(),
            requests,
        })
    }

    fn add_sender(&mut self, request_id: i32, sender: Sender<ResponsePacket>) -> Result<()> {
        let requests = Arc::clone(&self.requests);

        match requests.write() {
            Ok(mut hash) => {
                hash.insert(request_id, Outbox(sender));
            }
            Err(e) => {
                return Err(anyhow!("{}", e));
            }
        }

        Ok(())
    }
}

// impl read/write?

const UNSPECIFIED_REQUEST_ID: i32 = -1;

impl MessageBus for TcpMessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket> {
        read_packet(&self.reader)
    }

    fn read_packet_for_request(&mut self, request_id: i32) -> Result<ResponsePacket> {
        debug!("read message for request_id {:?}", request_id);

        let requests = Arc::clone(&self.requests);

        let collection = requests.read().unwrap();
        let request = match collection.get(&request_id) {
            Some(request) => request,
            _ => {
                return Err(anyhow!("no request found for request_id {:?}", request_id));
            }
        };

        // debug!("found request {:?}", request);
        // // FIXME still conviluted
        // let data = request.rx.recv()?;

        // let mut mut_collection = requests.write().unwrap();
        // mut_collection.remove(&request_id);

        // Ok(request.rx.recv()?)
        Err(anyhow!("no way"))
    }

    fn write_packet_for_request(
        &mut self,
        request_id: i32,
        packet: &RequestPacket,
    ) -> Result<ResponsePacketPromise> {
        let (sender, receiver) = mpsc::channel();

        self.add_sender(request_id, sender)?;
        self.write_packet(packet)?;

        Ok(ResponsePacketPromise::new(receiver))
    }

    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()> {
        let encoded = packet.encode();
        debug!("{:?} ->", encoded);

        let data = encoded.as_bytes();
        let mut header = vec![];
        header.write_u32::<BigEndian>(data.len() as u32)?;

        self.writer.write_all(&header)?;
        self.writer.write_all(data)?;

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        debug!("{:?} ->", data);
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> Result<()> {
        let reader = Arc::clone(&self.reader);
        let requests = Arc::clone(&self.requests);

        let handle = thread::spawn(move || loop {
            info!("tick");

            let mut packet = match read_packet(&reader) {
                Ok(packet) => packet,
                Err(err) => {
                    error!("error reading packet: {:?}", err);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };

            match packet.message_type() {
                IncomingMessage::Error => {
                    let request_id = packet.peek_int(2).unwrap_or(-1);

                    if request_id == UNSPECIFIED_REQUEST_ID {
                        error_event(server_version, &mut packet).unwrap();
                    } else {
                        process_response(&requests, packet);
                    }
                }
                IncomingMessage::NextValidId => process_next_valid_id(server_version, &mut packet),
                IncomingMessage::ManagedAccounts => {
                    process_managed_accounts(server_version, &mut packet)
                }
                _ => process_response(&requests, packet),
            };

            thread::sleep(Duration::from_secs(1));
        });

        self.handles.push(handle);

        Ok(())
    }
}

fn read_packet(mut reader: &TcpStream) -> Result<ResponsePacket> {
    let message_size = read_header(reader)?;
    let mut data = vec![0_u8; message_size];

    reader.read_exact(&mut data)?;

    let packet = ResponsePacket::from(&String::from_utf8(data)?);

    Ok(packet)
}

fn read_header(mut reader: &TcpStream) -> Result<usize> {
    let buffer = &mut [0_u8; 4];
    reader.read_exact(buffer)?;

    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;

    Ok(count as usize)
}

fn error_event(server_version: i32, packet: &mut ResponsePacket) -> Result<()> {
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

fn process_next_valid_id(server_version: i32, packet: &mut ResponsePacket) {
    packet.skip(); // message_id
    packet.skip(); // version

    let order_id = packet.next_string().unwrap_or_else(|_| String::default());
    info!("next_valid_order_id: {}", order_id)
}

fn process_managed_accounts(server_version: i32, packet: &mut ResponsePacket) {
    packet.skip(); // message_id
    packet.skip(); // version

    let managed_accounts = packet.next_string().unwrap_or_else(|_| String::default());
    info!("managed accounts: {}", managed_accounts)
}

fn process_response(requests: &Arc<RwLock<HashMap<i32, Outbox>>>, packet: ResponsePacket) {
    let collection = requests.read().unwrap();

    let request_id = packet.request_id().unwrap_or(-1);
    let outbox = match collection.get(&request_id) {
        Some(outbox) => outbox,
        _ => {
            debug!(
                "no request found for request_id {:?} - {:?}",
                request_id, packet
            );
            return;
        }
    };

    outbox.0.send(packet).unwrap();
}

#[derive(Debug)]
pub struct ResponsePacketPromise {
    receiver: Receiver<ResponsePacket>,
}

impl ResponsePacketPromise {
    fn new(receiver: Receiver<ResponsePacket>) -> ResponsePacketPromise {
        ResponsePacketPromise { receiver }
    }

    pub fn message(&self) -> Result<ResponsePacket> {
        // Duration::from_millis(100)

        Ok(self.receiver.recv_timeout(Duration::from_millis(20000))?)
        // return Err(anyhow!("no message"));
    }
}
