//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)&ensp;[![license]](https://opensource.org/licenses/MIT)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [license]: https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge&labelColor=555555
//!
//! <br>
//!
//! An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust. The official TWS API is an event driven API. This implementation provides a synchronous API that simplifies the development of trading strategies.
//!
//! This is a work in progress and targets support for TWS API 10.20. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).
//!
//! The initial release focuses on APIs for [contracts](crate::contracts), [realtime data](crate::market_data::realtime) and [order management](crate::orders).
//!
//! The list of open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). If you run into an issue or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) first and then report the issue if it is not already tracked.
//!
//!```no_run
//!     use anyhow;
//!     use ibapi::Client;     
//!     
//!     fn main() -> anyhow::Result<()> {
//!         let client = Client::connect("localhost:4002:100")?;
//!         println!("Client: {:?}", client);
//!         Ok(())
//!     }
//!```

mod accounts;
/// TSW API Client.
///
/// The Client establishes the connection to TWS or the Gateway.
/// It manages the routing of messages between TWS and the application.
pub mod client;
mod constants;
/// A [Contract](crate::contracts::Contract) object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;
/// Describes primary data structures used by the model.
pub(crate) mod domain;
/// APIs for retrieving market data
pub mod market_data;
mod messages;
pub(crate) mod news;
/// APIs for placing orders
pub mod orders;
mod server_versions;
pub(crate) mod stubs;

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Debug;
use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{anyhow, Result};
use contracts::Contract;
use log::{debug, error, info};
use market_data::realtime::RealTimeBarIterator;
use market_data::{BarSize, WhatToShow};

use crate::accounts::Position;
use crate::client::transport::{GlobalResponseIterator, MessageBus, ResponseIterator, TcpMessageBus};
use crate::client::RequestMessage;
use crate::market_data::realtime;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::orders::{Order, OrderDataResult, OrderNotification};

pub type IbApiError = Box<dyn Error + Send>;

// Client

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
    /// use ibapi::Client;
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

    // === Accounts ===

    pub fn positions(&self) -> core::result::Result<impl Iterator<Item = Position>, IbApiError> {
        accounts::positions(self)
    }

    // === Orders ===

    /// Submits an [Order].
    ///
    /// Submits an [Order] using [Client] for the given [Contract].
    /// Immediately after the order was submitted correctly, the TWS will start sending events concerning the order's activity via IBApi.EWrapper.openOrder and IBApi.EWrapper.orderStatus
    ///
    /// # Arguments
    /// * `client` - [Client] used to communicate with server.
    /// * `order_id` - ID for [Order]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [Order] to sumbit.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Client};
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::{order_builder, Action, OrderNotification};
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let client = Client::connect("localhost:4002")?;
    ///
    ///     let contract = Contract::stock("MSFT");
    ///     let order = order_builder::market_order(Action::Buy, 100.0);
    ///     let order_id = client.next_order_id();
    ///
    ///     let notifications = client.place_order(order_id, &contract, &order)?;
    ///
    ///     for notification in notifications {
    ///         match notification {
    ///             OrderNotification::OrderStatus(order_status) => {
    ///                 println!("order status: {order_status:?}")
    ///             }
    ///             OrderNotification::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///             OrderNotification::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///             OrderNotification::CommissionReport(report) => println!("commision report: {report:?}"),
    ///             OrderNotification::Message(message) => println!("message: {message:?}"),
    ///        }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<impl Iterator<Item = OrderNotification>> {
        orders::place_order(self, order_id, contract, order)
    }

    /// Requests all open orders places by this specific API client (identified by the API client id).
    /// For client ID 0, this will bind previous manual TWS orders.
    ///
    /// # Arguments
    /// * `client` - [Client] used to communicate with server.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    ///
    /// use ibapi::Client;
    ///
    /// fn main() -> Result<(), Box<dyn Error>> {
    ///     let client = Client::connect("localhost:4002")?;
    ///
    ///     let results = client.open_orders()?;
    ///     for order_data in results {
    ///        println!("{order_data:?}")
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn open_orders(&self) -> Result<impl Iterator<Item = OrderDataResult>> {
        orders::open_orders(self)
    }

    // === Market Data ===

    /// Requests realtime bars.
    ///
    /// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
    ///
    /// # Arguments
    /// * `client` - [Client] with an active connection to gateway.
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::{self, Contract};
    /// use ibapi::market_data::{realtime, BarSize, WhatToShow};
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let client = Client::connect("localhost:4002")?;
    ///
    ///     let contract = Contract::stock("TSLA");
    ///     let bars = client.realtime_bars(&contract, &BarSize::Sec5, &WhatToShow::Trades, false)?;
    ///
    ///     for (i, bar) in bars.enumerate().take(60) {
    ///         println!("bar[{i}]: {bar:?}");
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn realtime_bars<'a>(
        &'a self,
        contract: &Contract,
        bar_size: &BarSize,
        what_to_show: &WhatToShow,
        use_rth: bool,
    ) -> Result<RealTimeBarIterator<'a>> {
        realtime::realtime_bars_with_options(self, contract, bar_size, what_to_show, use_rth, Vec::default())
    }

    // == Internal Use == 

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
    pub(crate) fn request_order_data(&self, message: RequestMessage) -> Result<GlobalResponseIterator> {
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

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("server_version", &self.server_version)
            .field("server_time", &self.server_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

// ToField

pub(crate) trait ToField {
    fn to_field(&self) -> String;
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

fn encode_option_field<T: ToField>(val: &Option<T>) -> String {
    match val {
        Some(val) => val.to_field(),
        None => String::from(""),
    }
}
