use std::io::prelude::*;
use std::io::Cursor;
use std::net::TcpStream;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, info};

use super::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    // fn connect(connection_string: &str) -> Result<Self>;
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()>;
    fn write(&mut self, packet: &str) -> Result<()>;
}

#[derive(Debug)]
pub struct TcpMessageBus {
    stream: Box<TcpStream>,
}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        Ok(TcpMessageBus {
            stream: Box::new(TcpStream::connect(connection_string)?),
        })
    }
}

impl MessageBus for TcpMessageBus {
    // set read timeout
    fn read_packet(&mut self) -> Result<ResponsePacket> {
        let buf = &mut [0 as u8; 4];

        self.stream.read(buf)?;

        let mut rdr = Cursor::new(buf);
        let count = rdr.read_u32::<BigEndian>()?;

        let mut data = vec![0 as u8; count as usize];
        self.stream.read(&mut data)?;

        let packet = ResponsePacket::from(&String::from_utf8(data)?);
        debug!("read packet {:?}", packet);

        Ok(packet)
    }

    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()> {
        let encoded = packet.encode();

        let mut wtr = vec![];
        wtr.write_u32::<BigEndian>(encoded.len().try_into().unwrap())?;

        info!("outbound request {:?}", encoded);

        self.stream.write(&wtr)?;
        self.stream.write(&encoded.as_bytes())?;

        Ok(())
    }

    fn write(&mut self, packet: &str) -> Result<()> {
        info!("write_packet: {:?}", packet);
        self.stream.write(&packet.as_bytes())?;
        Ok(())
    }
}
