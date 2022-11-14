use std::ops::Index;

use anyhow::{anyhow, Result};
use time::OffsetDateTime;
// use time::time_macros::{datetime, format_description};

use crate::domain::Contract;

#[derive(Debug, Default)]
pub struct BasicClient<'a> {
    /// IB server version
    pub server_version: i32,
    /// IB Server time 
//    pub server_time: OffsetDateTime,
    // Next valid order id
    pub next_valid_order_id: i32,
    // Ids of managed accounts
    pub managed_accounts: String,

    host: &'a str,
    port: i32,
    client_id: i32,
}


// 	currentRequestId int                   // used to generate sequence of request Ids
// 	channels         map[int]chan []string // message exchange
// 	ready            chan struct{}

// 	mu                   sync.Mutex
// 	requestIdMutex       sync.Mutex
// 	contractDetailsMutex sync.Mutex
// 


#[derive(Default, Debug, PartialEq)]
pub struct RequestPacket {
    fields: Vec<String>,
}

#[derive(Default, Debug, PartialEq)]
pub struct ResponsePacket {
    i: usize,
    fields: Vec<String>,
}

pub struct ResponsePacketIterator {}

pub trait ToPacket {
    fn to_packet(&self) -> String;
}

impl RequestPacket {
    pub fn add_field<T: ToPacket>(&mut self, val: T) {
        let field = val.to_packet();
        self.fields.push(field);
    }
}

impl Index<usize> for RequestPacket {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
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

// fn check_server_version(version: i32) -> Result<()> {

// }

pub fn connect(host: &str, port: i32, client_id: i32) -> anyhow::Result<BasicClient> {
    println!("Connect, world!");
    Ok(BasicClient{
        host,
        port,
        client_id,
        ..BasicClient::default()
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
