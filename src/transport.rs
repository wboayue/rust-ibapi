use std::io::prelude::*;
use std::io::Cursor;
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, info};

use crate::client::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()>;
    fn write(&mut self, packet: &str) -> Result<()>;
    fn process_messages(&mut self) -> Result<()>;
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
        self.writer.write_all(&data)?;

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        debug!("{:?} ->", data);
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }

    fn process_messages(&mut self) -> Result<()> {
        let reader = Arc::clone(&self.reader);
        let handle = thread::spawn(move || {

            loop {
                info!("tick");
                let packet = read_packet(&reader);
                info!("read packet: {:?}", packet);
                thread::sleep(Duration::from_secs(1));
            }
        });

        self.handles.push(handle);

        Ok(())
    }
}

fn read_packet(mut reader: &TcpStream) -> Result<ResponsePacket> {
    let message_size = read_header(reader)?;
    let mut data = vec![0_u8; message_size];

    reader.read(&mut data)?;

    let packet = ResponsePacket::from(&String::from_utf8(data)?);
    debug!("read packet {:?}", packet);

    Ok(packet)
}

fn read_header(mut reader: &TcpStream) -> Result<usize> {
    let buffer = &mut [0_u8; 4];
    reader.read(buffer)?;

    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;

    Ok(count as usize)
}
