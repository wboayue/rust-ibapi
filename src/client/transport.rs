use anyhow::{anyhow, Result};

use super::{RequestPacket, ResponsePacket};

pub trait MessageBus {
    fn connect(&self, connection_string: &str) -> Result<()>;
    fn read_packet(&self) -> Result<ResponsePacket>;
    fn write_packet(&self, packet: &RequestPacket) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct TcpMessageBus {}

impl MessageBus for TcpMessageBus {
    fn connect(&self, connection_string: &str) -> Result<()> {
        Err(anyhow!("TcpMessageBus::connect() not implemented"))
    }

    fn read_packet(&self) -> Result<ResponsePacket> {
        Err(anyhow!("TcpMessageBus::read_packet() not implemented"))
    }
    fn write_packet(&self, packet: &RequestPacket) -> Result<()> {
        Ok(())
    }
}
