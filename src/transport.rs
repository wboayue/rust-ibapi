use std::io::prelude::*;
use std::io::Cursor;
use std::net::TcpStream;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, info};

use crate::client::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    fn read_packet(&mut self) -> Result<ResponsePacket>;
    fn write_packet(&mut self, packet: &RequestPacket) -> Result<()>;
    fn write(&mut self, packet: &str) -> Result<()>;
    fn process_messages(&self) -> Result<()>;
}

#[derive(Debug)]
pub struct TcpMessageBus {
    stream: Box<TcpStream>,
}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        // .set_read_timeout(Some(Duration::new(0, 0)));
//        stream.set_nonblocking(true)
        let stream = Box::new(TcpStream::connect(connection_string)?);
        // stream.set_nonblocking(true);
        Ok(TcpMessageBus {
            stream: stream,
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

    fn process_messages(&self) -> Result<()> {
        // let message_bus = Arc::clone(&self.message_bus);

        // let handle = thread::spawn(move || loop {
        //     || -> () {
        //         debug!("read next packet");
        //         let packet = message_bus.lock().unwrap().read_packet();
        //         info!("next packet: {:?}", packet);
        //         thread::sleep(std_time::Duration::from_secs(1));    
        //     }();
        // });
        Ok(())
    }
}
