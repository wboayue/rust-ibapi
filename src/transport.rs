use std::io::prelude::*;
use std::io::Cursor;
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error, info};

use crate::client::{RequestPacket, ResponsePacket};
use crate::messages::IncomingMessage;
use crate::server_versions;

pub trait MessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()>;
    fn write(&mut self, packet: &str) -> Result<()>;
    fn process_messages(&mut self, server_version: i32) -> Result<()>;
}

#[derive(Debug)]
pub struct TcpMessageBus {
    reader: Arc<TcpStream>,
    writer: Box<TcpStream>,
    handles: Vec<JoinHandle<i32>>,
}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        let stream = TcpStream::connect(connection_string)?;

        let reader = Arc::new(stream.try_clone()?);
        let writer = Box::new(stream);

        Ok(TcpMessageBus {
            reader,
            writer,
            handles: Vec::default(),
        })
    }
}

// impl read/write?

impl MessageBus for TcpMessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket> {
        read_packet(&self.reader)
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
                IncomingMessage::Error => error_event(server_version, &mut packet).unwrap(),
                IncomingMessage::NextValidId => process_next_valid_id(server_version, &mut packet),
                IncomingMessage::ManagedAccounts => process_managed_accounts(server_version, &mut packet),
                _ => info!(
                    "application message: {:?} {:?}",
                    packet.message_type(),
                    packet
                ),
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
        let id = packet.next_int()?;
        let error_code = packet.next_int()?;
        let error_message = if server_version >= server_versions::ENCODE_MSG_ASCII7 {
            // Regex.Unescape(ReadString()) : ReadString();
            packet.next_string()?
        } else {
            packet.next_string()?
        };

        let mut advanced_order_reject_json: String = "".to_string();
        if server_version >= server_versions::ADVANCED_ORDER_REJECT {
            advanced_order_reject_json = packet.next_string()?;
            // if (!Util.StringIsEmpty(tempStr))
            // {
            //     advancedOrderRejectJson = Regex.Unescape(tempStr);
            // }
        }
        error!(
            "id: {}, error_code: {}, error_message: {}, advanced_order_reject_json: {}",
            id, error_code, error_message, advanced_order_reject_json
        );
        Ok(())
    }
}

fn process_next_valid_id(server_version: i32, packet: &mut ResponsePacket) {
    packet.skip(); // message_id
    packet.skip(); // version

    let order_id = packet.next_string().unwrap_or(String::from(""));
    info!("next_valid_order_id: {}", order_id)
}

fn process_managed_accounts(server_version: i32, packet: &mut ResponsePacket) {
    packet.skip(); // message_id
    packet.skip(); // version

    let managed_accounts = packet.next_string().unwrap_or(String::from(""));
    info!("managed accounts: {}", managed_accounts)
}
