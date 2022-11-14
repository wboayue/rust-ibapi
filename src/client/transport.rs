use anyhow::{anyhow, Result};

use super::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    fn read_packet(&self) -> Result<ResponsePacket>;
    fn write_packet(&self, packet: &RequestPacket) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct TcpMessageBus {
}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        Err(anyhow!("TcpMessageBus::connect() not implemented"))
    }
}

impl MessageBus for TcpMessageBus {
    fn read_packet(&self) -> Result<ResponsePacket> {
        Err(anyhow!("TcpMessageBus::read_packet() not implemented"))
    }
    fn write_packet(&self, packet: &RequestPacket) -> Result<()> {
        Ok(())
    }
}