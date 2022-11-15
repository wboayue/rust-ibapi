use std::io::prelude::*;
use std::net::TcpStream;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, WriteBytesExt};
use log::{info};

use super::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    // fn connect(connection_string: &str) -> Result<Self>;
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &mut RequestPacket) -> Result<()>;
}

#[derive(Debug)]
pub struct TcpMessageBus {
    stream: Box<TcpStream>,
}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        Ok(TcpMessageBus{stream: Box::new(TcpStream::connect(connection_string)?)})
    }
}

impl MessageBus for TcpMessageBus {
    // set read timeout
    fn read_packet(&mut self) -> Result<ResponsePacket> {
        info!("reading packet");

        let mut buf = &mut [0 as u8; 4];
        self.stream.read(buf)?;
        info!("read {:?}", buf);
        Err(anyhow!("TcpMessageBus::read_packet() not implemented"))
    }

    fn write_packet(&mut self, packet: &mut RequestPacket) -> Result<()> {
        let encoded = packet.encode();

        let mut wtr = vec![];
        wtr.write_u32::<BigEndian>(encoded.len().try_into().unwrap())?;

        info!("write packet {:?}", encoded);

        self.stream.write(&wtr)?;
        self.stream.write(&encoded)?;

        Ok(())
    }
}
