use std::fmt;
use std::ops::Index;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use log::{debug, info};
use time::OffsetDateTime;

use crate::domain::Contract;
use crate::domain::SecurityType;
use crate::messages::IncomingMessage;
use crate::messages::OutgoingMessage;
use crate::server_versions;
use crate::transport::{MessageBus, TcpMessageBus};

mod versions;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const START_API: i32 = 71;

pub trait Client {
    fn next_request_id(&mut self) -> i32;
    fn server_version(&self) -> i32;
    fn send_packet(&mut self, packet: RequestPacket) -> Result<()>;
    fn receive_packet(&mut self, request_id: i32) -> Result<ResponsePacket>;
    fn receive_packets(&self, request_id: i32) -> Result<ResponsePacketIterator>;
    fn check_server_version(&self, version: i32, message: &str) -> Result<()>;
}

pub struct BasicClient {
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

impl BasicClient {
    /// Opens connection to TWS workstation or gateway.
    pub fn connect(connection_string: &str) -> Result<BasicClient> {
        let message_bus = Box::new(TcpMessageBus::connect(connection_string)?);
        BasicClient::do_connect(connection_string, message_bus)
    }

    fn do_connect(
        connection_string: &str,
        message_bus: Box<dyn MessageBus>,
    ) -> Result<BasicClient> {
        debug!("connecting to server with #{:?}", connection_string);

        let mut client = BasicClient {
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

        let prelude = &mut RequestPacket::default();
        prelude.add_field(&format!("v{}..{}", MIN_SERVER_VERSION, MAX_SERVER_VERSION));

        self.message_bus.write_packet(prelude)?;

        let mut status = self.message_bus.read_packet()?;
        self.server_version = status.next_int()?;
        self.server_time = status.next_string()?;

        Ok(())
    }

    fn start_api(&mut self) -> Result<()> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestPacket::default();

        prelude.add_field(&START_API);
        prelude.add_field(&VERSION);
        prelude.add_field(&self.client_id);

        if self.server_version > server_versions::OPTIONAL_CAPABILITIES {
            prelude.add_field(&"");
        }

        self.message_bus.write_packet(prelude)?;

        Ok(())
    }
}

impl Drop for BasicClient {
    fn drop(&mut self) {
        info!("dropping basic client")
    }
}

impl Client for BasicClient {
    fn next_request_id(&mut self) -> i32 {
        self.next_request_id += 1;
        self.next_request_id
    }

    fn server_version(&self) -> i32 {
        self.server_version
    }

    fn send_packet(&mut self, packet: RequestPacket) -> Result<()> {
        debug!("send_packet({:?})", packet);
        self.message_bus.write_packet(&packet)
    }

    fn receive_packet(&mut self, request_id: i32) -> Result<ResponsePacket> {
        Err(anyhow!("received_packet not implemented: {:?}", request_id))
    }

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

impl fmt::Debug for BasicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IbClient")
            .field("server_version", &self.server_version)
            .field("server_time", &self.server_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct RequestPacket {
    fields: Vec<String>,
}

impl RequestPacket {
    pub fn from(_fields: &[Box<dyn ToPacket>]) -> RequestPacket {
        RequestPacket::default()
    }

    pub fn add_field<T: ToPacket>(&mut self, val: &T) -> &RequestPacket {
        let field = val.to_packet();
        self.fields.push(field);
        self
    }

    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\x00");
        data.push_str("\0");
        data
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

#[derive(Default, Debug)]
pub struct ResponsePacket {
    i: usize,
    fields: Vec<String>,
}

impl ResponsePacket {
    pub fn message_type(&self) -> IncomingMessage {
        if self.fields.is_empty() {
            IncomingMessage::NotValid
        } else {
            let message_id = i32::from_str(&self.fields[0]).unwrap_or(-1);
            IncomingMessage::from(message_id)
        }
    }

    pub fn next_int(&mut self) -> Result<i32> {
        let field = &self.fields[self.i];
        match field.parse() {
            Ok(val) => {
                self.i += 1;
                Ok(val)
            }
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_date_time(&mut self) -> Result<OffsetDateTime> {
        let field = &self.fields[self.i];
        // from_unix_timestamp
        let timestamp: i64 = field.parse()?;
        match OffsetDateTime::from_unix_timestamp(timestamp) {
            Ok(val) => {
                self.i += 1;
                Ok(val)
            }
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_string(&mut self) -> Result<String> {
        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

    pub fn from(fields: &str) -> ResponsePacket {
        ResponsePacket {
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

impl ToPacket for bool {
    fn to_packet(&self) -> String {
        if *self {
            String::from("1")
        } else {
            String::from("0")
        }
    }
}

impl ToPacket for String {
    fn to_packet(&self) -> String {
        self.clone()
    }
}

impl ToPacket for i32 {
    fn to_packet(&self) -> String {
        self.to_string()
    }
}

impl ToPacket for f64 {
    fn to_packet(&self) -> String {
        self.to_string()
    }
}

impl ToPacket for SecurityType {
    fn to_packet(&self) -> String {
        self.to_string()
    }
}

impl ToPacket for &str {
    fn to_packet(&self) -> String {
        <&str>::clone(self).to_string()
    }
}

impl ToPacket for &Contract {
    fn to_packet(&self) -> String {
        format!("{:?}", self)
    }
}

impl ToPacket for OutgoingMessage {
    fn to_packet(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::VecDeque;

    use anyhow::{anyhow, Result};

    use super::*;

    #[derive(Default, Debug)]
    pub struct ClientStub {
        pub request_packets: Vec<RequestPacket>,
        pub response_packets: VecDeque<ResponsePacket>,
    }

    impl Client for ClientStub {
        fn next_request_id(&mut self) -> i32 {
            1
        }

        fn server_version(&self) -> i32 {
            1
        }

        fn send_packet(&mut self, packet: RequestPacket) -> Result<()> {
            self.request_packets.push(packet);
            Ok(())
        }

        fn receive_packet(&mut self, _request_id: i32) -> Result<ResponsePacket> {
            match self.response_packets.pop_front() {
                Some(packet) => Ok(packet),
                None => Err(anyhow!("ClientStub::receive_packet no packet")),
            }
        }

        fn receive_packets(&self, request_id: i32) -> Result<ResponsePacketIterator> {
            Ok(ResponsePacketIterator {})
        }

        fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn request_packet_from_fields() {
        // let mut packet = RequestPacket::default();
        // packet.add_field(32);

        let packet = || -> RequestPacket {
            let mut packet = RequestPacket::default();
            packet.add_field(&32);
            packet
        }();

        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
