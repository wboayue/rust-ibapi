use anyhow::{anyhow, Result};
use time::OffsetDateTime;

use crate::domain::Contract;

#[derive(Debug)]
pub struct BasicClient<'a> {
    host: &'a str,
    port: i32,
    client_id: i32,
}

#[derive(Default, Debug, PartialEq)]
pub struct RequestPacket {
    fields: Vec<String>
}

#[derive(Default, Debug, PartialEq)]
pub struct ResponsePacket {
    fields: Vec<String>
}

pub struct ResponsePacketIterator {}

pub trait ToPacket {
    fn to_packet(&self) -> String;
}

impl RequestPacket {
    pub fn add_field<T: ToPacket>(& mut self, val: T) {
        let field = val.to_packet();
        self.fields.push(field);
    }
}

impl ResponsePacket {
    pub fn next_int(&self) -> Result<i32> {
        Err(anyhow!("ResponsePacket.next_int not implemented!"))
    }

    pub fn next_date_time(&self) -> Result<OffsetDateTime> {
        Err(anyhow!("ResponsePacket.next_date_time not implemented!"))
    }
}

pub trait Client {
    fn next_request_id(&self) -> i32;
    fn server_version(&self) -> i32;
    fn send_packet(& mut self, packet: RequestPacket) -> Result<()>;
    fn receive_packet(&self, request_id: i32) -> ResponsePacket;
    fn receive_packets(&self, request_id: i32) -> ResponsePacketIterator;
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

#[cfg(test)]
pub mod tests;
