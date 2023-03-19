use std::fmt;
use std::ops::Index;
use std::str::FromStr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::cell::RefCell;

use anyhow::{anyhow, Result};
use log::{debug, error, info};
use time::OffsetDateTime;

use self::transport::{GlobalResponseIterator, MessageBus, ResponseIterator, TcpMessageBus};
use crate::messages::{order_id_index, request_id_index, IncomingMessages, OutgoingMessages};
use crate::{server_versions, ToField};

pub(crate) mod transport;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = server_versions::HISTORICAL_SCHEDULE;
const INFINITY_STR: &str = "Infinity";
const UNSET_DOUBLE: &str = "1.7976931348623157E308";
const UNSET_INTEGER: &str = "2147483647";
const UNSET_LONG: &str = "9223372036854775807";

/// TWS API Client. Manages the connection to TWS or Gateway.
/// Tracks some global information such as server version and server time.
/// Supports generation of order ids
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    /// IB Server time
    //    pub server_time: OffsetDateTime,
    pub(crate) server_time: String,

    managed_accounts: String,
    client_id: i32, // ID of client.
    pub(crate) message_bus: RefCell<Box<dyn MessageBus>>,
    next_request_id: AtomicI32, // Next available request_id.
    order_id: AtomicI32,        // Next available order_id. Starts with value returned on connection.
}

impl Client {
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
    /// use ibapi::client::{Client};
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let mut client = Client::connect("localhost:4002")?;
    ///
    ///     println!("server_version: {}", client.server_version());
    ///     println!("server_time: {}", client.server_time());
    ///     println!("managed_accounts: {}", client.managed_accounts());
    ///     println!("next_order_id: {}", client.next_order_id());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn connect(connection_string: &str) -> Result<Client> {
        debug!("connecting to server with #{:?}", connection_string);

        let message_bus = RefCell::new(Box::new(TcpMessageBus::connect(connection_string)?));
        Client::do_connect(message_bus)
    }

    fn do_connect(message_bus: RefCell<Box<dyn MessageBus>>) -> Result<Client> {
        let mut client = Client {
            server_version: 0,
            server_time: String::from(""),
            managed_accounts: String::from(""),
            message_bus,
            client_id: 100,
            next_request_id: AtomicI32::new(9000),
            order_id: AtomicI32::new(-1),
        };

        client.handshake()?;
        client.start_api()?;
        client.receive_account_info()?;

        client.message_bus.borrow_mut().process_messages(client.server_version)?;

        Ok(client)
    }

    #[cfg(test)]
    pub(crate) fn stubbed(message_bus: RefCell<Box<dyn MessageBus>>, server_version: i32) -> Client {
        Client {
            server_version: server_version,
            server_time: String::from(""),
            managed_accounts: String::from(""),
            message_bus,
            client_id: 100,
            next_request_id: AtomicI32::new(9000),
            order_id: AtomicI32::new(-1),
        }
    }

    // sends server handshake
    fn handshake(&mut self) -> Result<()> {
        self.message_bus.borrow_mut().write("API\x00")?;

        let prelude = &mut RequestMessage::new();
        prelude.push_field(&format!("v{MIN_SERVER_VERSION}..{MAX_SERVER_VERSION}"));

        self.message_bus.borrow_mut().write_message(prelude)?;

        let mut status = self.message_bus.borrow_mut().read_message()?;

        self.server_version = status.next_int()?;
        self.server_time = status.next_string()?;

        Ok(())
    }

    // asks server to start processing messages
    fn start_api(&mut self) -> Result<()> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&OutgoingMessages::StartApi);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.message_bus.borrow_mut().write_message(prelude)?;

        Ok(())
    }

    // Fetches next order id and managed accounts.
    fn receive_account_info(&mut self) -> Result<()> {
        let mut saw_next_order_id: bool = false;
        let mut saw_managed_accounts: bool = false;

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.message_bus.borrow_mut().read_message()?;

            match message.message_type() {
                IncomingMessages::NextValidId => {
                    saw_next_order_id = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    self.order_id.store(message.next_int()?, Ordering::Relaxed);
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

    // Old Client interface
    /// Returns the next request ID.
    pub fn next_request_id(&self) -> i32 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns and increments the order ID.
    pub fn next_order_id(&self) -> i32 {
        self.order_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Sets the current value of order ID.
    pub(crate) fn set_next_order_id(&self, order_id: i32) {
        self.order_id.store(order_id, Ordering::Relaxed)
    }

    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    /// The time of the server when the client connected
    pub fn server_time(&self) -> String {
        self.server_time.to_owned()
    }

    /// Returns the managed accounts.
    pub fn managed_accounts(&self) -> String {
        self.managed_accounts.to_owned()
    }

    pub(crate) fn send_message(&self, packet: RequestMessage) -> Result<()> {
        self.message_bus.borrow_mut().write_message(&packet)
    }

    pub(crate) fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<ResponseIterator> {
        debug!("send_message({:?}, {:?})", request_id, message);
        self.message_bus.borrow_mut().send_generic_message(request_id, &message)
    }

    pub(crate) fn send_order(&self, order_id: i32, message: RequestMessage) -> Result<ResponseIterator> {
        debug!("send_order({:?}, {:?})", order_id, message);
        self.message_bus.borrow_mut().send_order_message(order_id, &message)
    }

    /// Sends request for the next valid order id.
    pub(crate) fn request_next_order_id(&mut self, message: RequestMessage) -> Result<GlobalResponseIterator> {
        self.message_bus.borrow_mut().request_next_order_id(&message)
    }

    /// Sends request for open orders.
    pub(crate) fn request_order_data(&mut self, message: RequestMessage) -> Result<GlobalResponseIterator> {
        self.message_bus.borrow_mut().request_open_orders(&message)
    }

    /// Sends request for market rule.
    pub(crate) fn request_market_rule(&mut self, message: RequestMessage) -> Result<GlobalResponseIterator> {
        self.message_bus.borrow_mut().request_market_rule(&message)
    }

    pub(crate) fn check_server_version(&self, version: i32, message: &str) -> Result<()> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(anyhow!("server version {} required, got {}: {}", version, self.server_version, message))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        info!("dropping basic client")
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("server_version", &self.server_version)
            .field("server_time", &self.server_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct RequestMessage {
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

    pub(crate) fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }
}

impl Index<usize> for RequestMessage {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}

#[derive(Clone, Default, Debug)]
pub(crate) struct ResponseMessage {
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

    pub fn execution_id(&self) -> Option<String> {
        match self.message_type() {
            IncomingMessages::ExecutionData => Some(self.peek_string(14)),
            IncomingMessages::CommissionsReport => Some(self.peek_string(2)),
            _ => None,
        }
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

    pub fn next_optional_long(&mut self) -> Result<Option<i64>> {
        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_LONG {
            return Ok(None);
        }

        match field.parse::<i64>() {
            Ok(val) => Ok(Some(val)),
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

#[cfg(test)]
mod tests;
