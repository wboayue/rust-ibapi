use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, error, warn};
use time::{Date, OffsetDateTime};
use time_tz::Tz;

use crate::accounts::{AccountSummaries, AccountUpdate, AccountUpdateMulti, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
use crate::contracts::{Contract, OptionComputation, SecurityType};
use crate::errors::Error;
use crate::market_data::historical::{self, HistogramEntry};
use crate::market_data::realtime::{self, Bar, BarSize, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, WhatToShow};
use crate::market_data::MarketDataType;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::news::NewsArticle;
use crate::orders::{CancelOrder, Executions, ExerciseOptions, Order, Orders, PlaceOrder};
use crate::scanner::ScannerData;
use crate::transport::{Connection, ConnectionMetadata, InternalSubscription, MessageBus, TcpMessageBus, TcpSocket};
use crate::wsh::AutoFill;
use crate::{accounts, contracts, market_data, news, orders, scanner, wsh};

#[cfg(test)]
mod tests;

// Client

/// TWS API Client. Manages the connection to TWS or Gateway.
/// Tracks some global information such as server version and server time.
/// Supports generation of order ids
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    pub(crate) message_bus: Arc<dyn MessageBus>,

    client_id: i32,             // ID of client.
    next_request_id: AtomicI32, // Next available request_id.
    order_id: AtomicI32,        // Next available order_id. Starts with value returned on connection.
}

impl Client {
    /// Establishes connection to TWS or Gateway
    ///
    /// Connects to server using the given connection string
    ///
    /// # Arguments
    /// * `address`   - address of server. e.g. 127.0.0.1:4002
    /// * `client_id` - id of client. e.g. 100
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// println!("server_version: {}", client.server_version());
    /// println!("connection_time: {:?}", client.connection_time());
    /// println!("next_order_id: {}", client.next_order_id());
    /// ```
    pub fn connect(address: &str, client_id: i32) -> Result<Client, Error> {
        let stream = TcpStream::connect(address)?;
        let socket = TcpSocket::new(stream, address)?;

        let connection = Connection::connect(socket, client_id)?;
        let connection_metadata = connection.connection_metadata();

        let message_bus = Arc::new(TcpMessageBus::new(connection)?);

        // Starts thread to read messages from TWS
        message_bus.process_messages(connection_metadata.server_version, Duration::from_secs(1))?;

        Client::new(connection_metadata, message_bus)
    }

    fn new(connection_metadata: ConnectionMetadata, message_bus: Arc<dyn MessageBus>) -> Result<Client, Error> {
        let client = Client {
            server_version: connection_metadata.server_version,
            connection_time: connection_metadata.connection_time,
            time_zone: connection_metadata.time_zone,
            message_bus,
            client_id: connection_metadata.client_id,
            next_request_id: AtomicI32::new(9000),
            order_id: AtomicI32::new(connection_metadata.next_order_id),
        };

        Ok(client)
    }

    /// Returns the ID assigned to the [Client].
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the next request ID.
    pub fn next_request_id(&self) -> i32 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns and increments the order ID.
    ///
    /// The client maintains a sequence of order IDs. This function returns the next order ID in the sequence.
    pub fn next_order_id(&self) -> i32 {
        self.order_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Gets the next valid order ID from the TWS server.
    ///
    /// Unlike [Self::next_order_id], this function requests the next valid order ID from the TWS server and updates the client's internal order ID sequence.
    /// This can be for ensuring that order IDs are unique across multiple clients.
    ///
    /// Use this method when coordinating order IDs across multiple client instances or when you need to synchronize with the server's order ID sequence at the start of a session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// // Connect to the TWS server at the given address with client ID.
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Request the next valid order ID from the server.
    /// let next_valid_order_id = client.next_valid_order_id().expect("request failed");
    /// println!("next_valid_order_id: {next_valid_order_id}");
    /// ```
    pub fn next_valid_order_id(&self) -> Result<i32, Error> {
        orders::next_valid_order_id(self)
    }

    /// Sets the current value of order ID.
    pub(crate) fn set_next_order_id(&self, order_id: i32) {
        self.order_id.store(order_id, Ordering::Relaxed)
    }

    /// Returns the version of the TWS API server to which the client is connected.
    /// This version is determined during the initial connection handshake.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let server_version = client.server_version();
    /// println!("Connected to TWS server version: {}", server_version);
    /// ```
    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    /// The time of the server when the client connected
    pub fn connection_time(&self) -> Option<OffsetDateTime> {
        self.connection_time
    }

    // === Accounts ===

    /// TWS's current time. TWS is synchronized with the server (not local computer) using NTP and this function will receive the current time in TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let server_time = client.server_time().expect("error requesting server time");
    /// println!("server time: {server_time:?}");
    /// ```
    pub fn server_time(&self) -> Result<OffsetDateTime, Error> {
        accounts::server_time(self)
    }

    /// Subscribes to [PositionUpdate]s for all accessible accounts.
    /// All positions sent initially, and then only updates as positions change.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::PositionUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.positions().expect("error requesting positions");
    /// for position_response in subscription.iter() {
    ///     match position_response {
    ///         PositionUpdate::Position(position) => println!("{position:?}"),
    ///         PositionUpdate::PositionEnd => println!("initial set of positions received"),
    ///     }
    /// }
    /// ```
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        accounts::positions(self)
    }

    /// Subscribes to [PositionUpdateMulti] updates for account and/or model.
    /// Initially all positions are returned, and then updates are returned for any position changes in real time.
    ///
    /// # Arguments
    /// * `account`    - If an account Id is provided, only the account’s positions belonging to the specified model will be delivered.
    /// * `model_code` - The code of the model’s positions we are interested in.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let account = "U1234567";
    /// let subscription = client.positions_multi(Some(account), None).expect("error requesting positions by model");
    /// for position in subscription.iter() {
    ///     println!("{position:?}")
    /// }
    /// ```
    pub fn positions_multi(&self, account: Option<&str>, model_code: Option<&str>) -> Result<Subscription<PositionUpdateMulti>, Error> {
        accounts::positions_multi(self, account, model_code)
    }

    /// Creates subscription for real time daily PnL and unrealized PnL updates.
    ///
    /// # Arguments
    /// * `account`    - account for which to receive PnL updates
    /// * `model_code` - specify to request PnL updates for a specific model
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let account = "account id";
    /// let subscription = client.pnl(account, None).expect("error requesting pnl");
    /// for pnl in subscription.iter() {
    ///     println!("{pnl:?}")
    /// }
    /// ```
    pub fn pnl(&self, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
        accounts::pnl(self, account, model_code)
    }

    /// Requests real time updates for daily PnL of individual positions.
    ///
    /// # Arguments
    /// * `account`     - Account in which position exists
    /// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return response if invalid conId is entered.
    /// * `model_code`  - Model in which position exists
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let account = "<account id>";
    /// let contract_id = 1001;
    ///
    /// let subscription = client.pnl_single(account, contract_id, None).expect("error requesting pnl");
    /// for pnl in &subscription {
    ///     println!("{pnl:?}")
    /// }
    /// ```
    pub fn pnl_single<'a>(&'a self, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<Subscription<'a, PnLSingle>, Error> {
        accounts::pnl_single(self, account, contract_id, model_code)
    }

    /// Requests a specific account’s summary. Subscribes to the account summary as presented in the TWS’ Account Summary tab. Data received is specified by using a specific tags value.
    ///
    /// # Arguments
    /// * `group` - Set to “All” to return account summary data for all accounts, or set to a specific Advisor Account Group name that has already been created in TWS Global Configuration.
    /// * `tags`  - List of the desired tags.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountSummaryTags;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let group = "All";
    ///
    /// let subscription = client.account_summary(group, AccountSummaryTags::ALL).expect("error requesting account summary");
    /// for summary in &subscription {
    ///     println!("{summary:?}")
    /// }
    /// ```
    pub fn account_summary<'a>(&'a self, group: &str, tags: &[&str]) -> Result<Subscription<'a, AccountSummaries>, Error> {
        accounts::account_summary(self, group, tags)
    }

    /// Subscribes to a specific account’s information and portfolio.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change in a position, or to an account value every 3 minutes if it has changed. Only one account can be subscribed at a time.
    ///
    /// # Arguments
    /// * `account` - The account id (i.e. U1234567) for which the information is requested.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let account = "U1234567";
    ///
    /// let subscription = client.account_updates(account).expect("error requesting account updates");
    /// for update in &subscription {
    ///     println!("{update:?}");
    ///
    ///     // stop after full initial update
    ///     if let AccountUpdate::End = update {
    ///         subscription.cancel();
    ///     }
    /// }
    /// ```
    pub fn account_updates<'a>(&'a self, account: &str) -> Result<Subscription<'a, AccountUpdate>, Error> {
        accounts::account_updates(self, account)
    }

    /// Requests account updates for account and/or model.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change in a position, or to an account value every 3 minutes if it has changed. Only one account can be subscribed at a time.
    ///
    /// # Arguments
    /// * `account`        - Account values can be requested for a particular account.
    /// * `model_code`     - Account values can also be requested for a model.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountUpdateMulti;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let account = Some("U1234567");
    ///
    /// let subscription = client.account_updates_multi(account, None).expect("error requesting account updates multi");
    /// for update in &subscription {
    ///     println!("{update:?}");
    ///
    ///     // stop after full initial update
    ///     if let AccountUpdateMulti::End = update {
    ///         subscription.cancel();
    ///     }
    /// }
    /// ```
    pub fn account_updates_multi<'a>(
        &'a self,
        account: Option<&str>,
        model_code: Option<&str>,
    ) -> Result<Subscription<'a, AccountUpdateMulti>, Error> {
        accounts::account_updates_multi(self, account, model_code)
    }

    /// Requests the accounts to which the logged user has access to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let accounts = client.managed_accounts().expect("error requesting managed accounts");
    /// println!("managed accounts: {accounts:?}")
    /// ```
    pub fn managed_accounts(&self) -> Result<Vec<String>, Error> {
        accounts::managed_accounts(self)
    }

    // === Contracts ===

    /// Requests contract information.
    ///
    /// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use [Client::option_chain] for that purpose.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    /// let results = client.contract_details(&contract).expect("request failed");
    /// for contract_detail in results {
    ///     println!("contract: {:?}", contract_detail);
    /// }
    /// ```
    pub fn contract_details(&self, contract: &Contract) -> Result<Vec<contracts::ContractDetails>, Error> {
        contracts::contract_details(self, contract)
    }

    /// Get current [FamilyCode]s for all accessible accounts.
    pub fn family_codes(&self) -> Result<Vec<FamilyCode>, Error> {
        accounts::family_codes(self)
    }

    /// Requests details about a given market rule
    ///
    /// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
    /// A list of market rule ids can be obtained by invoking [Self::contract_details()] for a particular contract.
    /// The returned market rule ID list will provide the market rule ID for the instrument in the correspond valid exchange list in [contracts::ContractDetails].
    pub fn market_rule(&self, market_rule_id: i32) -> Result<contracts::MarketRule, Error> {
        contracts::market_rule(self, market_rule_id)
    }

    /// Requests matching stock symbols.
    ///
    /// # Arguments
    /// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contracts = client.matching_symbols("IB").expect("request failed");
    /// for contract in contracts {
    ///     println!("contract: {:?}", contract);
    /// }
    /// ```
    pub fn matching_symbols(&self, pattern: &str) -> Result<impl Iterator<Item = contracts::ContractDescription>, Error> {
        Ok(contracts::matching_symbols(self, pattern)?.into_iter())
    }

    /// Calculates an option’s price based on the provided volatility and its underlying’s price.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] object representing the option for which the calculation is being requested.
    /// * `volatility`      - Hypothetical volatility as a percentage (e.g., 20.0 for 20%).
    /// * `underlying_price` - Hypothetical price of the underlying asset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::option("AAPL", "20251219", 150.0, "C");
    /// let calculation = client.calculate_option_price(&contract, 100.0, 235.0).expect("request failed");
    /// println!("calculation: {:?}", calculation);
    /// ```
    pub fn calculate_option_price(&self, contract: &Contract, volatility: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        contracts::calculate_option_price(self, contract, volatility, underlying_price)
    }

    /// Calculates the implied volatility based on the hypothetical option price and underlying price.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] object representing the option for which the calculation is being requested.
    /// * `option_price`    - Hypothetical option price.
    /// * `underlying_price` - Hypothetical price of the underlying asset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::option("AAPL", "20230519", 150.0, "C");
    /// let calculation = client.calculate_implied_volatility(&contract, 25.0, 235.0).expect("request failed");
    /// println!("calculation: {:?}", calculation);
    /// ```
    pub fn calculate_implied_volatility(&self, contract: &Contract, option_price: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        contracts::calculate_implied_volatility(self, contract, option_price, underlying_price)
    }

    /// Requests security definition option parameters for viewing a contract’s option chain.
    ///
    /// # Arguments
    /// `symbol`   - Contract symbol of the underlying.
    /// `exchange` - The exchange on which the returned options are trading. Can be set to the empty string for all exchanges.
    /// `security_type` - The type of the underlying security, i.e. STK
    /// `contract_id`   - The contract ID of the underlying security.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{contracts::SecurityType, Client};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let symbol = "AAPL";
    /// let exchange = ""; // all exchanges
    /// let security_type = SecurityType::Stock;
    /// let contract_id = 265598;
    ///
    /// let subscription = client
    ///     .option_chain(symbol, exchange, security_type, contract_id)
    ///     .expect("request option chain failed!");
    ///
    /// for option_chain in &subscription {
    ///     println!("{option_chain:?}")
    /// }
    /// ```
    pub fn option_chain(
        &self,
        symbol: &str,
        exchange: &str,
        security_type: SecurityType,
        contract_id: i32,
    ) -> Result<Subscription<contracts::OptionChain>, Error> {
        contracts::option_chain(self, symbol, exchange, security_type, contract_id)
    }

    // === Orders ===

    /// Requests all *current* open orders in associated accounts at the current moment.
    /// Open orders are returned once; this function does not initiate a subscription.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.all_open_orders().expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn all_open_orders(&self) -> Result<Subscription<Orders>, Error> {
        orders::all_open_orders(self)
    }

    /// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
    ///
    /// # Arguments
    /// * `auto_bind` - if set to true, the newly created orders will be assigned an API order ID and implicitly associated with this client. If set to false, future orders will not be.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 0).expect("connection failed");
    ///
    /// let subscription = client.auto_open_orders(false).expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn auto_open_orders(&self, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
        orders::auto_open_orders(self, auto_bind)
    }

    /// Cancels an active [Order] placed by the same API client ID.
    ///
    /// # Arguments
    /// * `order_id` - ID of the [Order] to cancel.
    /// * `manual_order_cancel_time` - Optional timestamp to specify the cancellation time. Use an empty string to use the current time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let order_id = 15;
    /// let subscription = client.cancel_order(order_id, "").expect("request failed");
    /// for result in subscription {
    ///    println!("{result:?}");
    /// }
    /// ```
    pub fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        orders::cancel_order(self, order_id, manual_order_cancel_time)
    }

    /// Requests completed [Order]s.
    ///
    /// # Arguments
    /// * `api_only` - request only orders placed by the API.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.completed_orders(false).expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn completed_orders(&self, api_only: bool) -> Result<Subscription<Orders>, Error> {
        orders::completed_orders(self, api_only)
    }

    /// Requests current day's (since midnight) executions matching the filter.
    ///
    /// Only the current day's executions can be retrieved.
    /// Along with the [orders::ExecutionData], the [orders::CommissionReport] will also be returned.
    /// When requesting executions, a filter can be specified to receive only a subset of them
    ///
    /// # Arguments
    /// * `filter` - filter criteria used to determine which execution reports are returned
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::orders::ExecutionFilter;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = ExecutionFilter{
    ///    side: "BUY".to_owned(),
    ///    ..ExecutionFilter::default()
    /// };
    ///
    /// let subscription = client.executions(filter).expect("request failed");
    /// for execution_data in &subscription {
    ///    println!("{execution_data:?}")
    /// }
    /// ```
    pub fn executions(&self, filter: orders::ExecutionFilter) -> Result<Subscription<Executions>, Error> {
        orders::executions(self, filter)
    }

    /// Cancels all open [Order]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// client.global_cancel().expect("request failed");
    /// ```
    pub fn global_cancel(&self) -> Result<(), Error> {
        orders::global_cancel(self)
    }

    /// Requests all open orders places by this specific API client (identified by the API client id).
    /// For client ID 0, this will bind previous manual TWS orders.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.open_orders().expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn open_orders(&self) -> Result<Subscription<Orders>, Error> {
        orders::open_orders(self)
    }

    /// Places or modifies an [Order].
    ///
    /// Submits an [Order] using [Client] for the given [Contract].
    /// Upon successful submission, the client will start receiving events related to the order's activity via the subscription, including order status updates and execution reports.
    ///
    /// # Arguments
    /// * `order_id` - ID for [Order]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [Order] to submit.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::{order_builder, Action, PlaceOrder};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("MSFT");
    /// let order = order_builder::market_order(Action::Buy, 100.0);
    /// let order_id = client.next_order_id();
    ///
    /// let events = client.place_order(order_id, &contract, &order).expect("request failed");
    ///
    /// for event in &events {
    ///     match event {
    ///         PlaceOrder::OrderStatus(order_status) => {
    ///             println!("order status: {order_status:?}")
    ///         }
    ///         PlaceOrder::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///         PlaceOrder::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///         PlaceOrder::CommissionReport(report) => println!("commission report: {report:?}"),
    ///         PlaceOrder::Message(message) => println!("message: {message:?}"),
    ///    }
    /// }
    /// ```
    pub fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error> {
        orders::place_order(self, order_id, contract, order)
    }

    /// Exercises an options contract.
    ///
    /// Note: this function is affected by a TWS setting which specifies if an exercise request must be finalized.
    ///
    /// # Arguments
    /// * `contract`          - The option [Contract] to be exercised.
    /// * `exercise_action`   - Exercise option. ExerciseAction::Exercise or ExerciseAction::Lapse.
    /// * `exercise_quantity` - Number of contracts to be exercised.
    /// * `account`           - Destination account.
    /// * `ovrd`              - Specifies whether your setting will override the system’s natural action. For example, if your action is "exercise" and the option is not in-the-money, by natural action the option would not exercise. If you have override set to true the natural action would be overridden and the out-of-the money option would be exercised.
    /// * `manual_order_time` - Specify the time at which the options should be exercised. If `None`, the current time will be used. Requires TWS API 10.26 or higher.
    pub fn exercise_options<'a>(
        &'a self,
        contract: &Contract,
        exercise_action: orders::ExerciseAction,
        exercise_quantity: i32,
        account: &str,
        ovrd: bool,
        manual_order_time: Option<OffsetDateTime>,
    ) -> Result<Subscription<'a, ExerciseOptions>, Error> {
        orders::exercise_options(self, contract, exercise_action, exercise_quantity, account, ovrd, manual_order_time)
    }

    // === Historical Market Data ===

    /// Returns the timestamp of earliest available historical data for a contract and data type.
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{self, WhatToShow};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("MSFT");
    /// let what_to_show = WhatToShow::Trades;
    /// let use_rth = true;
    ///
    /// let result = client.head_timestamp(&contract, what_to_show, use_rth).expect("head timestamp failed");
    ///
    /// print!("head_timestamp: {result:?}");
    /// ```
    pub fn head_timestamp(&self, contract: &Contract, what_to_show: historical::WhatToShow, use_rth: bool) -> Result<OffsetDateTime, Error> {
        historical::head_timestamp(self, contract, what_to_show, use_rth)
    }

    /// Requests interval of historical data ending at specified time for [Contract].
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [historical::HistoricalData] for.
    /// * `interval_end` - optional end date of interval to retrieve [historical::HistoricalData] for. If `None` current time or last trading of contract is implied.
    /// * `duration`     - duration of interval to retrieve [historical::HistoricalData] for.
    /// * `bar_size`     - [historical::BarSize] to return.
    /// * `what_to_show` - requested bar type: [historical::WhatToShow].
    /// * `use_rth`      - use regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    ///
    /// let historical_data = client
    ///     .historical_data(&contract, Some(datetime!(2023-04-15 0:00 UTC)), 7.days(), BarSize::Day, WhatToShow::Trades, true)
    ///     .expect("historical data request failed");
    ///
    /// println!("start_date: {}, end_date: {}", historical_data.start, historical_data.end);
    ///
    /// for bar in &historical_data.bars {
    ///     println!("{bar:?}");
    /// }
    /// ```
    pub fn historical_data(
        &self,
        contract: &Contract,
        interval_end: Option<OffsetDateTime>,
        duration: historical::Duration,
        bar_size: historical::BarSize,
        what_to_show: historical::WhatToShow,
        use_rth: bool,
    ) -> Result<historical::HistoricalData, Error> {
        historical::historical_data(self, contract, interval_end, duration, bar_size, Some(what_to_show), use_rth)
    }

    /// Requests interval of historical data ending now for [Contract].
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [historical::HistoricalData] for.
    /// * `duration`     - duration of interval to retrieve [historical::HistoricalData] for.
    /// * `bar_size`     - [historical::BarSize] to return.
    /// * `what_to_show` - requested bar type: [historical::WhatToShow].
    /// * `use_rth`      - use regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    ///
    /// let historical_data = client
    ///     .historical_data_ending_now(&contract, 7.days(), BarSize::Day, WhatToShow::Trades, true)
    ///     .expect("historical data request failed");
    ///
    /// println!("start_date: {}, end_date: {}", historical_data.start, historical_data.end);
    ///
    /// for bar in &historical_data.bars {
    ///     println!("{bar:?}");
    /// }
    /// ```
    #[deprecated(since = "1.1.0", note = "use `historical_data` instead")]
    pub fn historical_data_ending_now(
        &self,
        contract: &Contract,
        duration: historical::Duration,
        bar_size: historical::BarSize,
        what_to_show: historical::WhatToShow,
        use_rth: bool,
    ) -> Result<historical::HistoricalData, Error> {
        historical::historical_data(self, contract, None, duration, bar_size, Some(what_to_show), use_rth)
    }

    /// Requests [Schedule](historical::Schedule) for an interval of given duration
    /// ending at specified date.
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [Schedule](historical::Schedule) for.
    /// * `interval_end` - end date of interval to retrieve [Schedule](historical::Schedule) for.
    /// * `duration`     - duration of interval to retrieve [Schedule](historical::Schedule) for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM");
    ///
    /// let historical_data = client
    ///     .historical_schedules(&contract, datetime!(2023-04-15 0:00 UTC), 30.days())
    ///     .expect("historical schedule request failed");
    ///
    /// println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);
    ///
    /// for session in &historical_data.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn historical_schedules(
        &self,
        contract: &Contract,
        interval_end: OffsetDateTime,
        duration: historical::Duration,
    ) -> Result<historical::Schedule, Error> {
        historical::historical_schedule(self, contract, Some(interval_end), duration)
    }

    /// Requests [historical::Schedule] for interval ending at current time.
    ///
    /// # Arguments
    /// * `contract` - [Contract] to retrieve [historical::Schedule] for.
    /// * `duration` - [historical::Duration] for interval to retrieve [historical::Schedule] for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM");
    ///
    /// let historical_data = client
    ///     .historical_schedules_ending_now(&contract, 30.days())
    ///     .expect("historical schedule request failed");
    ///
    /// println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);
    ///
    /// for session in &historical_data.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn historical_schedules_ending_now(&self, contract: &Contract, duration: historical::Duration) -> Result<historical::Schedule, Error> {
        historical::historical_schedule(self, contract, None, duration)
    }

    /// Requests historical time & sales data (Bid/Ask) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `use_rth`         - Data from regular trading hours (true), or all available hours (false)
    /// * `ignore_size`     - A filter only used when the source price is Bid_Ask
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    ///
    /// let ticks = client
    ///     .historical_ticks_bid_ask(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, true, false)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_bid_ask(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
        ignore_size: bool,
    ) -> Result<historical::TickSubscription<historical::TickBidAsk>, Error> {
        historical::historical_ticks_bid_ask(self, contract, start, end, number_of_ticks, use_rth, ignore_size)
    }

    /// Requests historical time & sales data (Midpoint) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `use_rth`         - Data from regular trading hours (true), or all available hours (false)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    ///
    /// let ticks = client
    ///     .historical_ticks_mid_point(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, true)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_mid_point(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
    ) -> Result<historical::TickSubscription<historical::TickMidpoint>, Error> {
        historical::historical_ticks_mid_point(self, contract, start, end, number_of_ticks, use_rth)
    }

    /// Requests historical time & sales data (Trades) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `use_rth`         - Data from regular trading hours (true), or all available hours (false)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    ///
    /// let ticks = client
    ///     .historical_ticks_trade(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, true)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_trade(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
    ) -> Result<historical::TickSubscription<historical::TickLast>, Error> {
        historical::historical_ticks_trade(self, contract, start, end, number_of_ticks, use_rth)
    }

    /// Requests data histogram of specified contract.
    ///
    /// # Arguments
    /// * `contract`  - [Contract] to retrieve [Histogram Entries](historical::HistogramEntry) for.
    /// * `use_rth`   - Data from regular trading hours (true), or all available hours (false).
    /// * `period`    - The time period of each histogram bar (e.g., `BarSize::Day`, `BarSize::Week`, `BarSize::Month`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    //
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::BarSize;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM");
    ///
    /// let histogram = client
    ///     .histogram_data(&contract, true, BarSize::Week)
    ///     .expect("histogram request failed");
    ///
    /// for item in &histogram {
    ///     println!("{item:?}");
    /// }
    /// ```
    pub fn histogram_data(&self, contract: &Contract, use_rth: bool, period: historical::BarSize) -> Result<Vec<HistogramEntry>, Error> {
        historical::histogram_data(self, contract, use_rth, period)
    }

    // === Realtime Market Data ===

    /// Requests realtime bars.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA");
    /// let subscription = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).expect("request failed");
    ///
    /// for (i, bar) in subscription.iter().enumerate().take(60) {
    ///     println!("bar[{i}]: {bar:?}");
    /// }
    /// ```
    pub fn realtime_bars<'a>(
        &'a self,
        contract: &Contract,
        bar_size: BarSize,
        what_to_show: WhatToShow,
        use_rth: bool,
    ) -> Result<Subscription<'a, Bar>, Error> {
        realtime::realtime_bars(self, contract, &bar_size, &what_to_show, use_rth, Vec::default())
    }

    /// Requests tick by tick AllLast ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_all_last(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick all last data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("All Last Tick: {:?}", tick);
    /// }
    /// ```
    pub fn tick_by_tick_all_last<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<'a, realtime::Trade>, Error> {
        realtime::tick_by_tick_all_last(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick BidAsk ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored. (typically true for BidAsk ticks to get changes based on price).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick bid/ask data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("BidAsk Tick: {:?}", tick);
    /// }
    /// ```
    pub fn tick_by_tick_bid_ask<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<'a, realtime::BidAsk>, Error> {
        realtime::tick_by_tick_bid_ask(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick Last ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored (typically false for Last ticks).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_last(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick last data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("Last Tick: {:?}", tick);
    /// }
    /// ```
    pub fn tick_by_tick_last<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<'a, realtime::Trade>, Error> {
        realtime::tick_by_tick_last(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick MidPoint ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick mid-point data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("MidPoint Tick: {:?}", tick);
    /// }
    /// ```
    pub fn tick_by_tick_midpoint<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<'a, MidPoint>, Error> {
        realtime::tick_by_tick_midpoint(self, contract, number_of_ticks, ignore_size)
    }

    /// Switches market data type returned from request_market_data requests to Live, Frozen, Delayed, or FrozenDelayed.
    ///
    /// # Arguments
    /// * `market_data_type` - Type of market data to retrieve.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::market_data::{MarketDataType};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let market_data_type = MarketDataType::Live;
    /// client.switch_market_data_type(market_data_type).expect("request failed");
    /// println!("market data switched: {:?}", market_data_type);
    /// ```
    pub fn switch_market_data_type(&self, market_data_type: MarketDataType) -> Result<(), Error> {
        market_data::switch_market_data_type(self, market_data_type)
    }

    /// Requests the contract's market depth (order book).
    ///
    /// # Arguments
    ///
    /// * `contract` - The Contract for which the depth is being requested.
    /// * `number_of_rows` - The number of rows on each side of the order book.
    /// * `is_smart_depth` - Flag indicates that this is smart depth request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    ///
    /// let subscription = client.market_depth(&contract, 5, true).expect("error requesting market depth");
    /// for row in &subscription {
    ///     println!("row: {row:?}");
    /// }
    ///
    /// if let Some(error) = subscription.error() {
    ///     println!("error: {:?}", error);
    /// }
    /// ```
    pub fn market_depth<'a>(
        &'a self,
        contract: &Contract,
        number_of_rows: i32,
        is_smart_depth: bool,
    ) -> Result<Subscription<'a, MarketDepths>, Error> {
        realtime::market_depth(self, contract, number_of_rows, is_smart_depth)
    }

    /// Requests venues for which market data is returned to market_depth (those with market makers)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let exchanges = client.market_depth_exchanges().expect("error requesting market depth exchanges");
    /// for exchange in &exchanges {
    ///     println!("{exchange:?}");
    /// }
    /// ```
    pub fn market_depth_exchanges(&self) -> Result<Vec<DepthMarketDataDescription>, Error> {
        realtime::market_depth_exchanges(self)
    }

    /// Requests real time market data.
    ///
    /// Returns market data for an instrument either in real time or 10-15 minutes delayed data.
    ///
    /// # Arguments
    ///
    /// * `contract` - Contract for which the data is being requested.
    /// * `generic_ticks` - IDs of the available generic ticks:
    ///   - 100 Option Volume (currently for stocks)
    ///   - 101 Option Open Interest (currently for stocks)
    ///   - 104 Historical Volatility (currently for stocks)
    ///   - 105 Average Option Volume (currently for stocks)
    ///   - 106 Option Implied Volatility (currently for stocks)
    ///   - 162 Index Future Premium
    ///   - 165 Miscellaneous Stats
    ///   - 221 Mark Price (used in TWS P&L computations)
    ///   - 225 Auction values (volume, price and imbalance)
    ///   - 233 RTVolume - contains the last trade price, last trade size, last trade time, total volume, VWAP, and single trade flag.
    ///   - 236 Shortable
    ///   - 256 Inventory
    ///   - 258 Fundamental Ratios
    ///   - 411 Realtime Historical Volatility
    ///   - 456 IBDividends
    /// * `snapshot` - for users with corresponding real time market data subscriptions. A true value will return a one-time snapshot, while a false value will provide streaming data.
    /// * `regulatory_snapshot` - snapshot for US stocks requests NBBO snapshots for users which have "US Securities Snapshot Bundle" subscription but not corresponding Network A, B, or C subscription necessary for streaming market data. One-time snapshot of current market price that will incur a fee of 1 cent to the account per snapshot.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{contracts::Contract, market_data::realtime::TickTypes, Client};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    ///
    /// // https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#available-tick-types
    /// let generic_ticks = &["233", "293"];
    /// let snapshot = false;
    /// let regulatory_snapshot = false;
    ///
    /// let subscription = client
    ///     .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
    ///     .expect("error requesting market data");
    ///
    /// for tick in &subscription {
    ///     match tick {
    ///         TickTypes::Price(tick_price) => println!("{:?}", tick_price),
    ///         TickTypes::Size(tick_size) => println!("{:?}", tick_size),
    ///         TickTypes::PriceSize(tick_price_size) => println!("{:?}", tick_price_size),
    ///         TickTypes::Generic(tick_generic) => println!("{:?}", tick_generic),
    ///         TickTypes::String(tick_string) => println!("{:?}", tick_string),
    ///         TickTypes::EFP(tick_efp) => println!("{:?}", tick_efp),
    ///         TickTypes::OptionComputation(option_computation) => println!("{:?}", option_computation),
    ///         TickTypes::RequestParameters(tick_request_parameters) => println!("{:?}", tick_request_parameters),
    ///         TickTypes::Notice(notice) => println!("{:?}", notice),
    ///         TickTypes::SnapshotEnd => subscription.cancel(),
    ///     }
    /// }
    /// ```
    pub fn market_data(
        &self,
        contract: &Contract,
        generic_ticks: &[&str],
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<TickTypes>, Error> {
        realtime::market_data(self, contract, generic_ticks, snapshot, regulatory_snapshot)
    }

    // === News ===

    /// Requests news providers which the user has subscribed to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let news_providers = client.news_providers().expect("request news providers failed");
    /// for news_provider in &news_providers {
    ///   println!("news provider {:?}", news_provider);
    /// }
    /// ```
    pub fn news_providers(&self) -> Result<Vec<news::NewsProvider>, Error> {
        news::news_providers(self)
    }

    /// Subscribes to IB's News Bulletins.
    ///
    /// # Arguments
    ///
    /// * `all_messages` - If set to true, will return all the existing bulletins for the current day, set to false to receive only the new bulletins.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let news_bulletins = client.news_bulletins(true).expect("request news providers failed");
    /// for news_bulletin in &news_bulletins {
    ///   println!("news bulletin {:?}", news_bulletin);
    /// }
    /// ```
    pub fn news_bulletins(&self, all_messages: bool) -> Result<Subscription<news::NewsBulletin>, Error> {
        news::news_bulletins(self, all_messages)
    }

    /// Requests historical news headlines.
    ///
    /// # Arguments
    ///
    /// * `contract_id`    - Contract ID of ticker. See [contract_details](Client::contract_details) for how to retrieve contract ID.
    /// * `provider_codes` - A list of provider codes.
    /// * `start_time`     - Marks the (exclusive) start of the date range.
    /// * `end_time`       - Marks the (inclusive) end of the date range.
    /// * `total_results`  - The maximum number of headlines to fetch (1 – 300)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract; // Or remove if conId is always known
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Example: Fetch historical news for a known contract ID (e.g., AAPL's conId)
    /// let contract_id = 265598;
    /// let provider_codes = &["DJNL", "BRFG"]; // Example provider codes
    /// // Define a past date range for the news query
    /// let start_time = datetime!(2023-01-01 0:00 UTC);
    /// let end_time = datetime!(2023-01-02 0:00 UTC);
    /// let total_results = 5u8; // Request a small number of articles for the example
    ///
    /// let articles_subscription = client.historical_news(
    ///     contract_id,
    ///     provider_codes,
    ///     start_time,
    ///     end_time,
    ///     total_results,
    /// ).expect("request historical news failed");
    ///
    /// println!("Requested historical news articles:");
    /// for article in articles_subscription.iter().take(total_results as usize) {
    ///     println!("- Headline: {}, ID: {}, Provider: {}, Time: {}",
    ///              article.headline, article.article_id, article.provider_code, article.time);
    /// }
    /// ```
    pub fn historical_news(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u8,
    ) -> Result<Subscription<news::NewsArticle>, Error> {
        news::historical_news(self, contract_id, provider_codes, start_time, end_time, total_results)
    }

    /// Requests news article body given articleId.
    ///
    /// # Arguments
    ///
    /// * `provider_code` - Short code indicating news provider, e.g. FLY.
    /// * `article_id`    - ID of the specific article.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // can get these using the historical_news method
    /// let provider_code = "DJ-N";
    /// let article_id = "DJ-N$1915168d";
    ///
    /// let article = client.news_article(provider_code, article_id).expect("request news article failed");
    /// println!("{:?}", article);
    /// ```
    pub fn news_article(&self, provider_code: &str, article_id: &str) -> Result<news::NewsArticleBody, Error> {
        news::news_article(self, provider_code, article_id)
    }

    /// Requests realtime contract specific news
    ///
    /// # Arguments
    ///
    /// * `contract`       - Contract for which news is being requested.
    /// * `provider_codes` - Short codes indicating news providers, e.g. DJ-N.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL");
    /// let provider_codes = ["DJ-N"];
    ///
    /// let subscription = client.contract_news(&contract, &provider_codes).expect("request contract news failed");
    /// for article in &subscription {
    ///     println!("{:?}", article);
    /// }
    /// ```
    pub fn contract_news(&self, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
        news::contract_news(self, contract, provider_codes)
    }

    /// Requests realtime BroadTape News
    ///
    /// # Arguments
    ///
    /// * `provider_code` - Short codes indicating news provider, e.g. DJ-N.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let provider_code = "BRFG";
    ///
    /// let subscription = client.broad_tape_news(provider_code).expect("request broad tape news failed");
    /// for article in &subscription {
    ///     println!("{:?}", article);
    /// }
    /// ```
    pub fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
        news::broad_tape_news(self, provider_code)
    }

    // === Scanner ===

    /// Requests an XML list of scanner parameters valid in TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::scanner::ScannerSubscription;
    /// use ibapi::orders::TagValue; // Or ensure common::TagValue is the correct path
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let mut sub = ScannerSubscription::default();
    /// sub.instrument = Some("STK".to_string());
    /// sub.location_code = Some("STK.US.MAJOR".to_string());
    /// sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    /// // Further customize the subscription object as needed, for example:
    /// // sub.above_price = Some(1.0);
    /// // sub.below_price = Some(100.0);
    /// // sub.number_of_rows = Some(20);
    ///
    /// // Filter options are advanced and not always needed. Pass an empty Vec if not used.
    /// let filter_options: Vec<TagValue> = Vec::new();
    /// // Example of adding a filter:
    /// // filter_options.push(TagValue { tag: "marketCapAbove".to_string(), value: "1000000000".to_string() });
    ///
    /// match client.scanner_subscription(&sub, &filter_options) {
    ///     Ok(subscription) => {
    ///         // Iterate over received scanner data.
    ///         // Note: Scanner subscriptions can be continuous or return a snapshot.
    ///         // This example just takes the first batch if available.
    ///         if let Some(scanner_results_vec) = subscription.iter().next() {
    ///             println!("Scanner Results (first batch):");
    ///             for data in scanner_results_vec {
    ///                 println!("  Rank: {}, Symbol: {}",
    ///                          data.rank,
    ///                          data.contract_details.contract.symbol);
    ///             }
    ///         } else {
    ///             println!("No scanner results received in the first check.");
    ///         }
    ///         // In a real application, you might continuously iterate or handle updates.
    ///         // Remember to cancel the subscription when no longer needed if it's continuous.
    ///         // subscription.cancel();
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to start scanner subscription: {}", e);
    ///     }
    /// };
    /// ```
    pub fn scanner_parameters(&self) -> Result<String, Error> {
        scanner::scanner_parameters(self)
    }

    /// Starts a subscription to market scan results based on the provided parameters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::scanner::ScannerSubscription;
    /// use ibapi::orders::TagValue;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let mut sub = ScannerSubscription::default();
    /// sub.instrument = Some("STK".to_string());
    /// sub.location_code = Some("STK.US.MAJOR".to_string());
    /// sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    /// // Further customize the subscription object as needed, for example:
    /// // sub.above_price = Some(1.0);
    /// // sub.below_price = Some(100.0);
    /// // sub.number_of_rows = Some(20);
    ///
    /// // Filter options are advanced and not always needed. Pass an empty Vec if not used.
    /// let mut filter_options: Vec<TagValue> = Vec::new();
    /// // Example of adding a filter:
    /// // filter_options.push(TagValue { tag: "marketCapAbove".to_string(), value: "1000000000".to_string() });
    ///
    /// match client.scanner_subscription(&sub, &filter_options) {
    ///     Ok(subscription) => {
    ///         // Iterate over received scanner data.
    ///         // Note: Scanner subscriptions can be continuous or return a snapshot.
    ///         // This example just takes the first batch if available.
    ///         if let Some(scanner_results_vec) = subscription.iter().next() {
    ///             println!("Scanner Results (first batch):");
    ///             for data in scanner_results_vec {
    ///                 println!("  Rank: {}, Symbol: {}",
    ///                          data.rank,
    ///                          data.contract_details.contract.symbol);
    ///             }
    ///         } else {
    ///             println!("No scanner results received in the first check.");
    ///         }
    ///         // In a real application, you might continuously iterate or handle updates.
    ///         // Remember to cancel the subscription when no longer needed if it's continuous.
    ///         // subscription.cancel();
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to start scanner subscription: {}", e);
    ///     }
    /// };
    /// ```
    pub fn scanner_subscription(
        &self,
        subscription: &scanner::ScannerSubscription,
        filter: &Vec<orders::TagValue>,
    ) -> Result<Subscription<Vec<ScannerData>>, Error> {
        scanner::scanner_subscription(self, subscription, filter)
    }

    // == Wall Street Horizon

    /// Requests metadata from the WSH calendar.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let metadata = client.wsh_metadata().expect("request wsh metadata failed");
    /// println!("{:?}", metadata);
    /// ```
    pub fn wsh_metadata(&self) -> Result<wsh::WshMetadata, Error> {
        wsh::wsh_metadata(self)
    }

    /// Requests event data for a specified contract from the Wall Street Horizons (WSH) calendar.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    /// * `start_date`  - Start date of the event request.
    /// * `end_date`    - End date of the event request.
    /// * `limit`       - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill`   - Fields to automatically fill in. See [AutoFill] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract_id = 76792991; // TSLA
    /// let event_data = client.wsh_event_data_by_contract(contract_id, None, None, None, None).expect("request wsh event data failed");
    /// println!("{:?}", event_data);
    /// ```
    pub fn wsh_event_data_by_contract(
        &self,
        contract_id: i32,
        start_date: Option<Date>,
        end_date: Option<Date>,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<wsh::WshEventData, Error> {
        wsh::wsh_event_data_by_contract(self, contract_id, start_date, end_date, limit, auto_fill)
    }

    /// Requests event data from the Wall Street Horizons (WSH) calendar using a JSON filter.
    ///
    /// # Arguments
    ///
    /// * `filter`    - Json-formatted string containing all filter values.
    /// * `limit`     - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill` - Fields to automatically fill in. See [AutoFill] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = ""; // see https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    /// let event_data = client.wsh_event_data_by_filter(filter, None, None).expect("request wsh event data failed");
    /// println!("{:?}", event_data);
    /// ```
    pub fn wsh_event_data_by_filter(
        &self,
        filter: &str,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<Subscription<wsh::WshEventData>, Error> {
        wsh::wsh_event_data_by_filter(self, filter, limit, auto_fill)
    }

    // == Internal Use ==

    #[cfg(test)]
    pub(crate) fn stubbed(message_bus: Arc<dyn MessageBus>, server_version: i32) -> Client {
        Client {
            server_version: server_version,
            connection_time: None,
            time_zone: None,
            message_bus,
            client_id: 100,
            next_request_id: AtomicI32::new(9000),
            order_id: AtomicI32::new(-1),
        }
    }

    pub(crate) fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<InternalSubscription, Error> {
        debug!("send_message({:?}, {:?})", request_id, message);
        self.message_bus.send_request(request_id, &message)
    }

    pub(crate) fn send_order(&self, order_id: i32, message: RequestMessage) -> Result<InternalSubscription, Error> {
        debug!("send_order({:?}, {:?})", order_id, message);
        self.message_bus.send_order_request(order_id, &message)
    }

    /// Sends request for the next valid order id.
    pub(crate) fn send_shared_request(&self, message_id: OutgoingMessages, message: RequestMessage) -> Result<InternalSubscription, Error> {
        self.message_bus.send_shared_request(message_id, &message)
    }

    pub(crate) fn check_server_version(&self, version: i32, message: &str) -> Result<(), Error> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(Error::ServerVersion(version, self.server_version, message.into()))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        debug!("dropping basic client");
        self.message_bus.ensure_shutdown();
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("server_version", &self.server_version)
            .field("server_time", &self.connection_time)
            .field("client_id", &self.client_id)
            .finish()
    }
}

/// Subscriptions facilitate handling responses from TWS that may be delayed or delivered periodically.
///
/// They offer both blocking and non-blocking methods for retrieving data.
///
/// In the simplest case a subscription can be implicitly converted to blocking iterator
/// that cancels the subscription when it goes out of scope.
///
/// ```no_run
/// use ibapi::contracts::Contract;
/// use ibapi::market_data::realtime::{BarSize, WhatToShow};
/// use ibapi::Client;
///
/// let connection_url = "127.0.0.1:4002";
/// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
///
/// // Request real-time bars data for AAPL with 5-second intervals
/// let contract = Contract::stock("AAPL");
/// let subscription = client
///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
///     .expect("realtime bars request failed!");
///
/// // Use the subscription as a blocking iterator
/// for bar in subscription {
///     // Process each bar here (e.g., print or use in calculations)
///     println!("Received bar: {bar:?}");
/// }
/// // The subscription goes out of scope and is automatically cancelled.
/// ```
///
/// Subscriptions can be explicitly canceled using the [cancel](Subscription::cancel) method.
///
/// You can convert subscriptions into blocking or non-blocking iterators using the [iter](Subscription::iter), [try_iter](Subscription::try_iter) or [timeout_iter](Subscription::timeout_iter) methods.
///
/// Alternatively, you may poll subscriptions in a blocking or non-blocking manner using the [next](Subscription::next), [try_next](Subscription::try_next) or [next_timeout](Subscription::next_timeout) methods.
#[allow(private_bounds)]
#[derive(Debug)]
pub struct Subscription<'a, T: DataStream<T> + 'static> {
    client: &'a Client,
    request_id: Option<i32>,
    order_id: Option<i32>,
    message_type: Option<OutgoingMessages>,
    phantom: PhantomData<T>,
    cancelled: AtomicBool,
    snapshot_ended: AtomicBool,
    subscription: InternalSubscription,
    response_context: ResponseContext,
    error: Mutex<Option<Error>>,
}

// Extra metadata that might be need
#[derive(Debug, Default)]
pub(crate) struct ResponseContext {
    pub(crate) request_type: Option<OutgoingMessages>,
    pub(crate) is_smart_depth: bool,
}

#[allow(private_bounds)]
impl<'a, T: DataStream<T> + 'static> Subscription<'a, T> {
    pub(crate) fn new(client: &'a Client, subscription: InternalSubscription, context: ResponseContext) -> Self {
        if let Some(request_id) = subscription.request_id {
            Subscription {
                client,
                request_id: Some(request_id),
                order_id: None,
                message_type: None,
                subscription,
                response_context: context,
                phantom: PhantomData,
                cancelled: AtomicBool::new(false),
                snapshot_ended: AtomicBool::new(false),
                error: Mutex::new(None),
            }
        } else if let Some(order_id) = subscription.order_id {
            Subscription {
                client,
                request_id: None,
                order_id: Some(order_id),
                message_type: None,
                subscription,
                response_context: context,
                phantom: PhantomData,
                cancelled: AtomicBool::new(false),
                snapshot_ended: AtomicBool::new(false),
                error: Mutex::new(None),
            }
        } else if let Some(message_type) = subscription.message_type {
            Subscription {
                client,
                request_id: None,
                order_id: None,
                message_type: Some(message_type),
                subscription,
                response_context: context,
                phantom: PhantomData,
                cancelled: AtomicBool::new(false),
                snapshot_ended: AtomicBool::new(false),
                error: Mutex::new(None),
            }
        } else {
            panic!("unsupported internal subscription: {:?}", subscription)
        }
    }

    /// Polls the subscription for the next item and blocks until the next item is available.
    ///
    /// This method will wait indefinitely until either:
    /// - A new item becomes available
    /// - The subscription encounters an error
    /// - The subscription is canceled
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// // Process bars blocking until the next bar is available
    /// while let Some(bar) = subscription.next() {
    ///     println!("Received bar: {bar:?}");
    /// }
    ///
    /// // When the loop exits, check if it was due to an error
    /// if let Some(err) = subscription.error() {
    ///     eprintln!("subscription error: {err}");
    /// }
    /// ```
    /// # Returns
    /// * `Some(T)` - The next available item from the subscription
    /// * `None` - If the subscription has ended or encountered an error
    pub fn next(&self) -> Option<T> {
        match self.process_response(self.subscription.next()) {
            Some(val) => Some(val),
            None => match self.error() {
                Some(Error::UnexpectedResponse(m)) => {
                    debug!("error in subscription: {m:?}");
                    self.next()
                }
                _ => None,
            },
        }
    }

    fn process_response(&self, response: Option<Result<ResponseMessage, Error>>) -> Option<T> {
        self.clear_error();

        match response {
            Some(Ok(message)) => self.process_message(message),
            Some(Err(e)) => {
                let mut error = self.error.lock().unwrap();
                *error = Some(e);
                None
            }
            None => None,
        }
    }

    fn process_message(&self, mut message: ResponseMessage) -> Option<T> {
        match T::decode(self.client, &mut message) {
            Ok(val) => {
                // Check if this decoded value represents the end of a snapshot subscription
                if val.is_snapshot_end() {
                    self.snapshot_ended.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Some(val)
            }
            Err(Error::EndOfStream) => None,
            Err(err) => {
                error!("error decoding message: {err}");
                let mut error = self.error.lock().unwrap();
                *error = Some(err);
                None
            }
        }
    }

    /// Polls the subscription for the next item, returns immediately if no data is available.
    ///
    /// Unlike [next](Subscription::next) which blocks waiting for data, this method provides
    /// non-blocking access to the subscription data. It returns immediately with:
    /// - The next item if one is available
    /// - None if no data is currently available
    /// - None if an error occurred
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// loop {
    ///     // Process all currently available data
    ///     while let Some(bar) = subscription.try_next() {
    ///         println!("Received bar: {bar:?}");
    ///     }
    ///
    ///     // Check if we stopped due to an error
    ///     if let Some(err) = subscription.error() {
    ///         eprintln!("subscription error: {err}");
    ///        break;
    ///     }
    ///
    ///     // No data currently available, perform other work
    ///     // The subscription remains active and can be checked again
    /// }
    ///
    /// ```
    /// # Returns
    /// * `Some(T)` - The next available item from the subscription
    /// * `None` - If no data is immediately available or if an error occurred
    pub fn try_next(&self) -> Option<T> {
        self.process_response(self.subscription.try_next())
    }

    /// Polls the subscription for the next item, waiting up to the specified timeout duration.
    ///
    /// This method provides a middle ground between [try_next](Subscription::try_next) and
    /// [next](Subscription::next):
    /// - Unlike try_next, it will wait for data to arrive
    /// - Unlike next, it will only wait for a specified duration
    /// - Returns None if no data arrives within the timeout period
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    /// use std::time::Duration;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// loop {
    ///     // Wait up to 1 second for each new piece of data
    ///     while let Some(bar) = subscription.next_timeout(Duration::from_secs(1)) {
    ///         println!("Received bar: {bar:?}");
    ///     }
    ///
    ///     // Check if we stopped due to an error
    ///     if let Some(err) = subscription.error() {
    ///         eprintln!("subscription error: {err}");
    ///        break;
    ///     }
    ///
    ///     // Either timeout occurred or no more data available
    ///     // Perform other work before checking again
    /// }
    ///
    /// ```
    ///
    /// # Arguments
    /// * `timeout` - Maximum duration to wait for the next item
    ///
    /// # Returns
    /// * `Some(T)` - The next available item from the subscription
    /// * `None` - If no data arrives within the timeout period or if an error occurred
    ///
    /// # See also
    /// - [Subscription::next] - For blocking access without timeout
    /// - [Subscription::try_next] - For immediate non-blocking access
    /// - [Subscription::error] - For checking error status
    pub fn next_timeout(&self, timeout: Duration) -> Option<T> {
        self.process_response(self.subscription.next_timeout(timeout))
    }

    /// Cancel the subscription
    pub fn cancel(&self) {
        // Only cancel if snapshot hasn't ended (for market data snapshots)
        // For streaming subscriptions, snapshot_ended will remain false
        if self.snapshot_ended.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }

        self.cancelled.store(true, Ordering::Relaxed);

        if let Some(request_id) = self.request_id {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id, &self.response_context) {
                if let Err(e) = self.client.message_bus.cancel_subscription(request_id, &message) {
                    warn!("error cancelling subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else if let Some(order_id) = self.order_id {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id, &self.response_context) {
                if let Err(e) = self.client.message_bus.cancel_order_subscription(order_id, &message) {
                    warn!("error cancelling order subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else if let Some(message_type) = self.message_type {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id, &self.response_context) {
                if let Err(e) = self.client.message_bus.cancel_shared_subscription(message_type, &message) {
                    warn!("error cancelling shared subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else {
            debug!("Could not determine cancel method")
        }
    }

    /// Creates an iterator from the [Subscription] that blocks until the next item is available.
    ///
    /// The iterator does not consume the [Subscription], allowing you to explicitly cancel the subscription at any time using the [cancel](Subscription::cancel) method.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// // Create an iterator that blocks until the next item is available.
    /// for bar in subscription.iter() {
    ///     // Process each bar here (e.g., print or use in calculations)
    ///     println!("Received bar: {bar:?}");
    /// }
    /// // The subscription is still in scope and can be explicitly canceled.
    /// ```
    /// # Returns
    /// A [SubscriptionIter] that yields items as they become available, blocking if necessary.
    pub fn iter(&self) -> SubscriptionIter<T> {
        SubscriptionIter { subscription: self }
    }

    /// Creates an iterator from the [Subscription] that returns the next bar if available without waiting.
    ///
    /// The iterator does not consume the [Subscription], allowing you to explicitly cancel the subscription at any time using the [cancel](Subscription::cancel) method.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// // Process data in a non-blocking way.
    /// loop {
    ///     // Create an iterator that returns the next bar without waiting.
    ///     for bar in subscription.try_iter() {
    ///         // Process all available data here.
    ///     }
    ///
    ///     // Perform other work between checking for data.
    ///     // The subscription remains active and can be cancelled when needed.
    ///
    ///     // Optional: Add a small delay to prevent excessive CPU usage
    ///     thread::sleep(Duration::from_secs(1));
    /// }
    /// ```
    /// # Returns
    /// A [SubscriptionTryIter] that yields items if they are available, without waiting.
    pub fn try_iter(&self) -> SubscriptionTryIter<T> {
        SubscriptionTryIter { subscription: self }
    }

    /// Creates an iterator from the [Subscription] that waits until specified timeout for available data.
    ///
    /// Similar to [try_iter](Subscription::try_iter), this iterator does not consume the [Subscription],
    /// allowing you to explicitly cancel the subscription at any time using the [cancel](Subscription::cancel) method.
    /// However, unlike try_iter which returns immediately, this iterator will wait up to the specified timeout
    /// duration before yielding data or returning.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::Client;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let connection_url = "127.0.0.1:4002";
    /// let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    ///
    /// // Request real-time bars data for AAPL with 5-second intervals
    /// let contract = Contract::stock("AAPL");
    /// let subscription = client
    ///     .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///     .expect("realtime bars request failed!");
    ///
    /// // Process data with a 1-second timeout between checks
    /// loop {
    ///     // Iterator will wait up to 1 second for new data before continuing
    ///     for bar in subscription.timeout_iter(Duration::from_secs(1)) {
    ///         // Process all available data here.
    ///     }
    ///
    ///     // If no data arrives within timeout, loop continues here
    ///     // Perform other work between checking for data.
    ///     // The subscription remains active and can be cancelled when needed.
    /// }
    /// ```
    ///
    /// # Arguments
    /// * `timeout` - Maximum duration to wait for data before continuing iteration
    ///
    /// # Returns
    /// A [SubscriptionTimeoutIter] that waits for the specified timeout duration for available data.
    pub fn timeout_iter(&self, timeout: Duration) -> SubscriptionTimeoutIter<T> {
        SubscriptionTimeoutIter { subscription: self, timeout }
    }

    /// Returns any error that caused the [Subscription] to stop receiving data.
    ///
    /// A [Subscription] may stop yielding items either because there is no more data available
    /// or because it encountered an error condition (e.g., network disconnection).
    /// This method allows checking if an error occurred and retrieving the error details.
    ///
    /// # Returns
    /// * `Some(Error)` - If an error occurred that stopped the subscription
    /// * `None` - If no error has occurred (subscription may still be active or completed normally)
    pub fn error(&self) -> Option<Error> {
        let error = self.error.lock().unwrap();
        error.clone()
    }

    fn clear_error(&self) {
        let mut error = self.error.lock().unwrap();
        *error = None;
    }
}

impl<T: DataStream<T> + 'static> Drop for Subscription<'_, T> {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Internal trait for types that can be streamed from TWS/Gateway responses.
///
/// Implementors must provide:
/// - A decode method to convert response messages into the target type
/// - Optionally, a cancel message generator for cleaning up subscriptions
///
/// This trait is used internally by the Subscription system to handle
/// different types of streaming data from IB.
pub(crate) trait DataStream<T> {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<T, Error>;
    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        Err(Error::NotImplemented)
    }

    /// Returns true if this decoded value represents the end of a snapshot subscription
    fn is_snapshot_end(&self) -> bool {
        false
    }
}

/// An iterator that yields items as they become available, blocking if necessary.
#[allow(private_bounds)]
pub struct SubscriptionIter<'a, T: DataStream<T> + 'static> {
    subscription: &'a Subscription<'a, T>,
}

impl<T: DataStream<T> + 'static> Iterator for SubscriptionIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: DataStream<T> + 'static> IntoIterator for &'a Subscription<'a, T> {
    type Item = T;
    type IntoIter = SubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[allow(private_bounds)]
pub struct SubscriptionOwnedIter<'a, T: DataStream<T> + 'static> {
    subscription: Subscription<'a, T>,
}

impl<T: DataStream<T> + 'static> Iterator for SubscriptionOwnedIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: DataStream<T> + 'static> IntoIterator for Subscription<'a, T> {
    type Item = T;
    type IntoIter = SubscriptionOwnedIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        SubscriptionOwnedIter { subscription: self }
    }
}

/// An iterator that yields items if they are available, without waiting.
#[allow(private_bounds)]
pub struct SubscriptionTryIter<'a, T: DataStream<T> + 'static> {
    subscription: &'a Subscription<'a, T>,
}

impl<T: DataStream<T> + 'static> Iterator for SubscriptionTryIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// An iterator that waits for the specified timeout duration for available data.
#[allow(private_bounds)]
pub struct SubscriptionTimeoutIter<'a, T: DataStream<T> + 'static> {
    subscription: &'a Subscription<'a, T>,
    timeout: Duration,
}

impl<T: DataStream<T> + 'static> Iterator for SubscriptionTimeoutIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

/// Marker trait for shared channels
pub trait SharesChannel {}
