use anyhow::{anyhow, Result};
use time::OffsetDateTime;

use crate::domain::Contract;

#[derive(Debug)]
pub struct BasicClient<'a> {
    host: &'a str,
    port: i32,
    client_id: i32,
}

pub struct Packet {}
pub struct PacketIterator {}

pub trait ToPacket {
    fn to_packet(&self) -> String;
}

impl Packet {
    pub fn add_field<T: ToPacket>(&self, val: T) {
        val.to_packet();
    }

    pub fn next_int(&self) -> Result<i32> {
        Err(anyhow!("not implemented!"))
    }

    pub fn next_date_time(&self) -> Result<OffsetDateTime> {
        Err(anyhow!("not implemented!"))
    }
}

pub trait Client {
    fn next_request_id(&self) -> i32;
    fn send_packet(&self, packet: &Packet) -> i32;
    fn receive_packet(&self, request_id: i32) -> Packet;
    fn receive_packets(&self, request_id: i32) -> PacketIterator;
    fn check_server_version(&self, version: i32, message: &str) -> Result<()>;
}

// fn check_server_version(version: i32) -> Result<()> {

// }

pub fn connect(host: &str, port: i32, client_id: i32) -> anyhow::Result<BasicClient> {
    println!("Connect, world!");
    Ok(BasicClient {
        host,
        port,
        client_id,
    })
}

impl ToPacket for bool {
    fn to_packet(&self) -> String {
        "bool".to_string()
    }
}

impl ToPacket for String {
    fn to_packet(&self) -> String {
        self.clone()
    }
}

impl ToPacket for i32 {
    fn to_packet(&self) -> String {
        "i32".to_string()
    }
}

impl ToPacket for &str {
    fn to_packet(&self) -> String {
        self.clone().to_string()
    }
}

impl ToPacket for &Contract {
    fn to_packet(&self) -> String {
        "contract".to_string()
    }
}
