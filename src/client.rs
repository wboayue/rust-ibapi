use std::fmt;
use std::ops::Index;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use log::{debug, error, info};
use time::OffsetDateTime;

use self::transport::{GlobalResponsePacketPromise, MessageBus, ResponsePacketPromise, TcpMessageBus};
use crate::contracts::{ComboLegOpenClose, SecurityType};
use crate::messages::{order_id_index, request_id_index, IncomingMessages, OutgoingMessages};
use crate::orders::{Action, OrderCondition, OrderOpenClose, Rule80A, TagValue};
use crate::server_versions;

pub(crate) mod transport;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const START_API: i32 = 71;
const INFINITY_STR: &str = "Infinity";
const UNSET_DOUBLE: &str = "1.7976931348623157E308";
const UNSET_INTEGER: &str = "2147483647";

pub trait Client {
    /// Returns the next request ID.
    fn next_request_id(&mut self) -> i32;
    /// Returns the next order ID. Set at connection time then incremented on each call.
    fn next_order_id(&mut self) -> i32;
    /// Sets the current value of order ID.
    fn set_next_order_id(&mut self, order_id: i32) -> i32;
    /// Returns the server version.
    fn server_version(&self) -> i32;
    /// Returns the server time at connection time.
    fn server_time(&self) -> String;
    /// Returns the managed accounts.
    fn managed_accounts(&self) -> String;
    /// Sends a message without an expected reply.
    fn send_message(&mut self, packet: RequestMessage) -> Result<()>;
    /// Sends a request and waits for reply.
    fn send_request(&mut self, request_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise>;
    /// Submits an order and waits for reply.
    fn send_order(&mut self, order_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise>;
    /// Sends request for the next valid order id.
    fn request_next_order_id(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise>;
    /// Sends request for open orders.
    fn request_open_orders(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise>;
    /// Ensures server is at least the requested version.
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

    managed_accounts: String,
    client_id: i32, // ID of client.
    message_bus: Box<dyn MessageBus>,
    next_request_id: i32, // Next available request_id.
    order_id: i32,        // Next available order_id. Starts with value returned on connection.
}

impl IBClient {
    /// Establishes connection to TWS or Gateway
    ///
    /// Connects to server using the given connection string
    ///
    /// # Arguments
    /// * `connection_string` - connection string in the following format [host]:[port]:[client_id].
    ///                         client id is optional and defaults to 100.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::{IBClient, Client};
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let mut client = IBClient::connect("localhost:4002")?;
    ///
    ///     println!("server_version: {}", client.server_version());
    ///     println!("server_time: {}", client.server_time());
    ///     println!("managed_accounts: {}", client.managed_accounts());
    ///     println!("next_order_id: {}", client.next_order_id());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn connect(connection_string: &str) -> Result<IBClient> {
        debug!("connecting to server with #{:?}", connection_string);

        let message_bus = Box::new(TcpMessageBus::connect(connection_string)?);
        IBClient::do_connect(connection_string, message_bus)
    }

    fn do_connect(connection_string: &str, message_bus: Box<dyn MessageBus>) -> Result<IBClient> {
        let mut client = IBClient {
            server_version: 0,
            server_time: String::from(""),
            next_valid_order_id: 0,
            managed_accounts: String::from(""),
            message_bus,
            client_id: 100,
            next_request_id: 9000,
            order_id: -1,
        };

        client.handshake()?;
        client.start_api()?;
        client.receive_account_info()?;

        client.message_bus.process_messages(client.server_version)?;

        Ok(client)
    }

    // sends server handshake
    fn handshake(&mut self) -> Result<()> {
        self.message_bus.write("API\x00")?;

        let prelude = &mut RequestMessage::new();
        prelude.push_field(&format!("v{MIN_SERVER_VERSION}..{MAX_SERVER_VERSION}"));

        self.message_bus.write_message(prelude)?;

        let mut status = self.message_bus.read_message()?;

        self.server_version = status.next_int()?;
        self.server_time = status.next_string()?;

        Ok(())
    }

    // asks server to start processing messages
    fn start_api(&mut self) -> Result<()> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&START_API);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.message_bus.write_message(prelude)?;

        Ok(())
    }

    // Fetches next order id and managed accounts.
    fn receive_account_info(&mut self) -> Result<()> {
        let mut saw_next_order_id: bool = false;
        let mut saw_managed_accounts: bool = false;

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.message_bus.read_message()?;

            match message.message_type() {
                IncomingMessages::NextValidId => {
                    saw_next_order_id = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    self.order_id = message.next_int()?;
                }
                IncomingMessages::ManagedAccounts => {
                    saw_managed_accounts = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    self.managed_accounts = message.next_string()?;
                }
                IncomingMessages::Error => {
                    error!("message: {message:?}")
                }
                _ => info!("message: {message:?}"),
            }

            attempts += 1;
            if (saw_next_order_id && saw_managed_accounts) || attempts > MAX_ATTEMPTS {
                break;
            }
        }

        Ok(())
    }
}

impl Drop for IBClient {
    fn drop(&mut self) {
        info!("dropping basic client")
    }
}

impl Client for IBClient {
    /// Returns the next request ID.
    fn next_request_id(&mut self) -> i32 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        request_id
    }

    /// Returns and increments the order ID.
    fn next_order_id(&mut self) -> i32 {
        let order_id = self.order_id;
        self.order_id += 1;
        order_id
    }

    /// Sets the current value of order ID.
    fn set_next_order_id(&mut self, order_id: i32) -> i32 {
        self.order_id = order_id;
        self.order_id
    }

    fn server_version(&self) -> i32 {
        self.server_version
    }

    /// Returns the server version.
    fn server_time(&self) -> String {
        self.server_time.to_owned()
    }

    /// Returns the managed accounts.
    fn managed_accounts(&self) -> String {
        self.managed_accounts.to_owned()
    }

    fn send_message(&mut self, packet: RequestMessage) -> Result<()> {
        self.message_bus.write_message(&packet)
    }

    fn send_request(&mut self, request_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        debug!("send_message({:?}, {:?})", request_id, message);
        self.message_bus.send_generic_message(request_id, &message)
    }

    fn send_order(&mut self, order_id: i32, message: RequestMessage) -> Result<ResponsePacketPromise> {
        debug!("send_order({:?}, {:?})", order_id, message);
        self.message_bus.send_order_message(order_id, &message)
    }

    /// Sends request for the next valid order id.
    fn request_next_order_id(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.message_bus.request_next_order_id(&message)
    }

    /// Sends request for open orders.
    fn request_open_orders(&mut self, message: RequestMessage) -> Result<GlobalResponsePacketPromise> {
        self.message_bus.request_open_orders(&message)
    }

    fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(anyhow!("server version {} required, got {}: {}", version, self.server_version, message))
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

#[derive(Clone, Default, Debug)]
pub struct ResponseMessage {
    pub i: usize,
    pub fields: Vec<String>,
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

    pub fn request_id(&self) -> Option<i32> {
        if let Some(i) = request_id_index(self.message_type()) {
            if let Ok(request_id) = self.peek_int(i) {
                return Some(request_id);
            }
        }
        None
    }

    pub fn order_id(&self) -> Option<i32> {
        if let Some(i) = order_id_index(self.message_type()) {
            if let Ok(order_id) = self.peek_int(i) {
                return Some(order_id);
            }
        }
        None
    }

    pub fn peek_int(&self, i: usize) -> Result<i32> {
        let field = &self.fields[i];
        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", i, field, err)),
        }
    }

    pub fn peek_string(&self, i: usize) -> String {
        self.fields[i].to_owned()
    }

    pub fn next_int(&mut self) -> Result<i32> {
        let field = &self.fields[self.i];
        self.i += 1;

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_optional_int(&mut self) -> Result<Option<i32>> {
        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_INTEGER {
            return Ok(None);
        }

        match field.parse::<i32>() {
            Ok(val) => Ok(Some(val)),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_bool(&mut self) -> Result<bool> {
        let field = &self.fields[self.i];
        self.i += 1;

        Ok(field == "1")
    }

    pub fn next_long(&mut self) -> Result<i64> {
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

        if field.is_empty() || field == "0" || field == "0.0" {
            return Ok(0.0);
        }

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(anyhow!("error parsing field {} {}: {}", self.i, field, err)),
        }
    }

    pub fn next_optional_double(&mut self) -> Result<Option<f64>> {
        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_DOUBLE {
            return Ok(None);
        }

        if field == INFINITY_STR {
            return Ok(Some(f64::INFINITY));
        }

        match field.parse() {
            Ok(val) => Ok(Some(val)),
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

    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
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

impl ToField for Vec<TagValue> {
    fn to_field(&self) -> String {
        let mut values = Vec::new();
        for tag_value in self {
            values.push(format!("{}={};", tag_value.tag, tag_value.value))
        }
        values.concat()
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
pub(crate) mod stub;
