use std::default;
use std::fmt;
use std::ops::Index;
use std::net::TcpStream;

use anyhow::{anyhow, Result};
use time::OffsetDateTime;

use crate::client::transport::{MessageBus, TcpMessageBus};
use crate::domain::Contract;
use crate::server_versions;

pub struct BasicClient<'a> {
    /// IB server version
    pub server_version: i32,
    /// IB Server time
    //    pub server_time: OffsetDateTime,
    pub server_time: String,
    // Next valid order id
    pub next_valid_order_id: i32,
    // Ids of managed accounts
    pub managed_accounts: String,

    message_bus: Box<dyn MessageBus>,

    host: &'a str,
    port: i32,
    client_id: i32,
}

const CLIENT_VERSION: i32 = 2;
const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const START_API: i32 = 2;


impl BasicClient<'_> {
    pub fn connect(connection_string: &str) -> Result<BasicClient<'_>> {
        let message_bus = Box::new(TcpMessageBus::connect(connection_string)?);
        BasicClient::do_connect(connection_string, message_bus)
    }

    pub fn do_connect(connection_string: &str, message_bus: Box<dyn MessageBus>) -> Result<BasicClient<'_>> {
        let mut client = BasicClient {
            server_version: 0,
            server_time: String::from("hello"),
            next_valid_order_id: 0,
            managed_accounts: String::from(""),
            message_bus: message_bus,
            host: "",
            port: 0,
            client_id: 0,
        };

        client.handshake()?;
        client.start_api()?;

        Ok(client)
    }

    fn handshake(&mut self) -> Result<()> {
        let prelude = &mut RequestPacket::default();
        prelude.add_field("API");
        prelude.add_field(format!("v{}..{}", MIN_SERVER_VERSION, MAX_SERVER_VERSION));

        self.message_bus.write_packet(prelude);

        let mut status = self.message_bus.read_packet()?;
        // if status.len() != 2 {
        //     return Err(!anyhow("hello"));
        // }

        self.server_version = status.next_int()?;
        self.server_time = status.next_string()?;

        Ok(())
    }
    
    fn start_api(&mut self) -> Result<()> {
        let mut prelude = &mut RequestPacket::default();
        prelude.add_field(START_API);
        prelude.add_field(CLIENT_VERSION);
        prelude.add_field(self.client_id);

        if self.server_version > server_versions::OPTIONAL_CAPABILITIES {
            prelude.add_field("");
        }

        self.message_bus.write_packet(prelude);

        Ok(())
    }
}

impl fmt::Debug for BasicClient<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IbClient")
         .field("server_version", &self.server_version)
         .field("server_time", &self.server_time)
         .field("client_id", &self.client_id)
         .finish()
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct RequestPacket {
    fields: Vec<String>,
    encoded: String,
}

#[derive(Default, Debug, PartialEq)]
pub struct ResponsePacket {
    i: usize,
    fields: Vec<String>,
}

impl RequestPacket {
    pub fn from(fields: &[Box<dyn ToPacket>]) -> RequestPacket {
        RequestPacket::default()
    }

    pub fn add_field<T: ToPacket>(&mut self, val: T) -> &RequestPacket {
        let field = val.to_packet();
        self.fields.push(field);
        self
    }

    pub fn encode(&mut self, ) -> &[u8] {
        self.encoded = self.fields.join("\x00");
        self.encoded.as_bytes()
    } 
}

impl Index<usize> for RequestPacket {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}

pub struct ResponsePacketIterator {}

pub trait ToPacket {
    fn to_packet(&self) -> String;
}

impl ResponsePacket {
    pub fn next_int(&mut self) -> Result<i32> {
        let field = &self.fields[self.i];
        match field.parse() {
            Ok(val) => {
                self.i += 1;
                return Ok(val);
            }
            Err(err) => return Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        };
    }

    pub fn next_date_time(&mut self) -> Result<OffsetDateTime> {
        let field = &self.fields[self.i];
        // from_unix_timestamp
        let timestamp: i64 = field.parse()?;
        match OffsetDateTime::from_unix_timestamp(timestamp) {
            Ok(val) => {
                self.i += 1;
                return Ok(val);
            }
            Err(err) => return Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        };
    }

    pub fn next_string(&mut self) -> Result<String> {
        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

    pub fn from(fields: Vec<String>) -> ResponsePacket {
        ResponsePacket {
            i: 0,
            fields: fields,
        }
    }
}

pub trait Client {
    fn next_request_id(&self) -> i32;
    fn server_version(&self) -> i32;
    fn send_packet(&mut self, packet: RequestPacket) -> Result<()>;
    fn receive_packet(&mut self, request_id: i32) -> Result<ResponsePacket>;
    fn receive_packets(&self, request_id: i32) -> ResponsePacketIterator;
    fn check_server_version(&self, version: i32, message: &str) -> Result<()>;
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
pub mod transport;
