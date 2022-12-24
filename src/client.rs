use std::fmt;
use std::ops::Index;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use log::{debug, info};
use time::OffsetDateTime;

use self::transport::ResponsePacketIterator;
use self::transport::{MessageBus, ResponsePacketPromise, TcpMessageBus};
use crate::contracts::{ComboLegOpenClose, SecurityType};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::orders::{Action, OrderCondition, OrderOpenClose, Rule80A};
use crate::server_versions;

mod transport;
mod versions;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const START_API: i32 = 71;
const INFINITY_STR: &str = "Infinity";

pub trait Client {
    fn next_request_id(&mut self) -> i32;
    fn server_version(&self) -> i32;
    fn send_packet(&mut self, packet: RequestMessage) -> Result<()>;
    fn send_message(
        &mut self,
        request_id: i32,
        message: RequestMessage,
    ) -> Result<ResponsePacketPromise>;
    // fn receive_packet(&mut self, request_id: i32) -> Result<ResponsePacket>;
    fn receive_packets(&self, request_id: i32) -> Result<ResponsePacketIterator>;
    fn check_server_version(&self, version: i32, message: &str) -> Result<()>;
}

pub struct IBClient {
    /// IB server version
    pub server_version: i32,
    /// IB Server time
    //    pub server_time: OffsetDateTime,
    pub server_time: String,
    // Next valid order id
    pub next_valid_order_id: i32,
    // Ids of managed accounts
    pub managed_accounts: String,

    client_id: i32,
    message_bus: Box<dyn MessageBus>,
    next_request_id: i32,
}

impl IBClient {
    /// Opens connection to TWS workstation or gateway.
    pub fn connect(connection_string: &str) -> Result<IBClient> {
        let message_bus = Box::new(TcpMessageBus::connect(connection_string)?);
        IBClient::do_connect(connection_string, message_bus)
    }

    fn do_connect(connection_string: &str, message_bus: Box<dyn MessageBus>) -> Result<IBClient> {
        debug!("connecting to server with #{:?}", connection_string);

        let mut client = IBClient {
            server_version: 0,
            server_time: String::from("hello"),
            next_valid_order_id: 0,
            managed_accounts: String::from(""),
            message_bus,
            client_id: 100,
            next_request_id: 9000,
        };

        client.handshake()?;
        client.start_api()?;

        client.message_bus.process_messages(client.server_version)?;

        Ok(client)
    }

    fn handshake(&mut self) -> Result<()> {
        self.message_bus.write("API\x00")?;

        let prelude = &mut RequestMessage::default();
        prelude.push_field(&format!("v{}..{}", MIN_SERVER_VERSION, MAX_SERVER_VERSION));

        self.message_bus.write_packet(prelude)?;

        let mut status = self.message_bus.read_packet()?;
        self.server_version = status.next_int()?;
        self.server_time = status.next_string()?;

        Ok(())
    }

    fn start_api(&mut self) -> Result<()> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&START_API);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.message_bus.write_packet(prelude)?;

        Ok(())
    }
}

impl Drop for IBClient {
    fn drop(&mut self) {
        info!("dropping basic client")
    }
}

impl Client for IBClient {
    fn next_request_id(&mut self) -> i32 {
        self.next_request_id += 1;
        self.next_request_id
    }

    fn server_version(&self) -> i32 {
        self.server_version
    }

    fn send_packet(&mut self, packet: RequestMessage) -> Result<()> {
        debug!("send_packet({:?})", packet);
        self.message_bus.write_packet(&packet)
    }

    fn send_message(
        &mut self,
        request_id: i32,
        message: RequestMessage,
    ) -> Result<ResponsePacketPromise> {
        debug!("send_message({:?}, {:?})", request_id, message);
        self.message_bus
            .write_packet_for_request(request_id, &message)
    }

    // fn receive_packet(&mut self, request_id: i32) -> Result<ResponsePacket> {
    //     self.message_bus.read_packet_for_request(request_id)
    // }

    fn receive_packets(&self, request_id: i32) -> Result<ResponsePacketIterator> {
        Err(anyhow!(
            "received_packets not implemented: {:?}",
            request_id
        ))
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(anyhow!(
                "server version {} required, got {}: {}",
                version,
                self.server_version,
                message
            ))
        }
    }
}

impl fmt::Debug for IBClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IbClient")
            .field("server_version", &self.server_version)
            .field("server_time", &self.server_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct RequestMessage {
    fields: Vec<String>,
}

impl RequestMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(_fields: &[Box<dyn ToField>]) -> RequestMessage {
        RequestMessage::default()
    }

    pub fn push_field<T: ToField>(&mut self, val: &T) -> &RequestMessage {
        let field = val.to_field();
        self.fields.push(field);
        self
    }

    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
    }
}

impl Index<usize> for RequestMessage {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}

pub trait ToField {
    fn to_field(&self) -> String;
}

#[derive(Default, Debug)]
pub struct ResponseMessage {
    i: usize,
    fields: Vec<String>,
}

impl ResponseMessage {
    pub fn message_type(&self) -> IncomingMessages {
        if self.fields.is_empty() {
            IncomingMessages::NotValid
        } else {
            let message_id = i32::from_str(&self.fields[0]).unwrap_or(-1);
            IncomingMessages::from(message_id)
        }
    }

    pub fn request_id(&self) -> Result<i32> {
        match self.message_type() {
            IncomingMessages::ContractData
            | IncomingMessages::TickByTick
            | IncomingMessages::SymbolSamples => self.peek_int(1),
            IncomingMessages::ContractDataEnd | IncomingMessages::RealTimeBars => self.peek_int(2),
            _ => Err(anyhow!(
                "error parsing field request id {:?}: {:?}",
                self.message_type(),
                self
            )),
        }
    }

    pub fn peek_int(&self, i: usize) -> Result<i32> {
        let field = &self.fields[i];
        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", i, field, err)),
        }
    }

    pub fn next_int(&mut self) -> Result<i32> {
        let field = &self.fields[self.i];
        self.i += 1;

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_date_time(&mut self) -> Result<OffsetDateTime> {
        let field = &self.fields[self.i];
        self.i += 1;

        // from_unix_timestamp
        let timestamp: i64 = field.parse()?;
        match OffsetDateTime::from_unix_timestamp(timestamp) {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_string(&mut self) -> Result<String> {
        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

    pub fn next_double(&mut self) -> Result<f64> {
        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == "0" {
            return Ok(0.0);
        }

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_double_max(&mut self) -> Result<f64> {
        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == "0" {
            return Ok(f64::MAX);
        }
        if field == INFINITY_STR {
            return Ok(f64::INFINITY);
        }

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn from(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split('\x00').map(|x| x.to_string()).collect(),
        }
    }

    pub fn skip(&mut self) {
        self.i += 1;
    }

    pub fn reset(&mut self) {
        self.i = 0;
    }
}

impl ToField for bool {
    fn to_field(&self) -> String {
        if *self {
            String::from("1")
        } else {
            String::from("0")
        }
    }
}

impl ToField for String {
    fn to_field(&self) -> String {
        self.clone()
    }
}

impl ToField for &str {
    fn to_field(&self) -> String {
        <&str>::clone(self).to_string()
    }
}

impl ToField for usize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for i32 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<i32> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for f64 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<f64> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for SecurityType {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<SecurityType> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for OutgoingMessages {
    fn to_field(&self) -> String {
        (*self as i32).to_string()
    }
}

impl ToField for Action {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for ComboLegOpenClose {
    fn to_field(&self) -> String {
        (*self as u8).to_string()
    }
}

impl ToField for OrderOpenClose {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<OrderOpenClose> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for Rule80A {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<Rule80A> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for OrderCondition {
    fn to_field(&self) -> String {
        (*self as u8).to_string()
    }
}

impl ToField for Option<OrderCondition> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

fn encode_option_field<T: ToField>(val: &Option<T>) -> String {
    match val {
        Some(val) => val.to_field(),
        None => String::from(""),
    }
}

#[cfg(test)]
pub mod tests;
