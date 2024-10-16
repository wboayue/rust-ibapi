use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};
use time::OffsetDateTime;
use time_tz::Tz;

use crate::accounts::{AccountSummaries, AccountUpdates, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
use crate::contracts::Contract;
use crate::errors::Error;
use crate::market_data::historical;
use crate::market_data::realtime::{self, Bar, BarSize, MidPoint, WhatToShow};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::messages::{RequestMessage, ResponseMessage};
use crate::orders::{Order, OrderDataResult, OrderNotification};
use crate::transport::{Connection, ConnectionMetadata, InternalSubscription, MessageBus, Response, TcpMessageBus};
use crate::{accounts, contracts, orders};

// Client

/// TWS API Client. Manages the connection to TWS or Gateway.
/// Tracks some global information such as server version and server time.
/// Supports generation of order ids
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    /// IB Server time
    //    pub server_time: OffsetDateTime,
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
        let connection = Connection::connect(client_id, address)?;
        let connection_metadata = connection.connection_metadata();

        let message_bus = Arc::new(TcpMessageBus::new(connection)?);

        // Starts thread to read messages from TWS
        message_bus.process_messages(connection_metadata.server_version)?;

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
            order_id: AtomicI32::new(-1),
        };

        Ok(client)
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
    pub fn connection_time(&self) -> Option<OffsetDateTime> {
        self.connection_time
    }

    // === Accounts ===

    /// Subscribes to [PositionUpdate](accounts::PositionUpdate)s for all accessible accounts.
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
    pub fn positions(&self) -> core::result::Result<Subscription<PositionUpdate>, Error> {
        accounts::positions(self)
    }

    /// Subscribes to [PositionUpdateMulti](accounts::PositionUpdateMulti) updates for account and/or model.
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
    /// let subscription = client.account_summary(group, AccountSummaryTags::ALL).expect("error requesting pnl");
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
    /// use ibapi::accounts::AccountUpdates;
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
    ///     if let AccountUpdates::End = update {
    ///         subscription.cancel();
    ///     }
    /// }
    /// ```
    pub fn account_updates<'a>(&'a self, account: &str) -> Result<Subscription<'a, AccountUpdates>, Error> {
        accounts::account_updates(self, account)
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
    /// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
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
    pub fn contract_details(&self, contract: &Contract) -> Result<impl Iterator<Item = contracts::ContractDetails>, Error> {
        Ok(contracts::contract_details(self, contract)?.into_iter())
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
    /// let results = client.all_open_orders().expect("request failed");
    /// for order_data in results {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn all_open_orders(&self) -> Result<impl Iterator<Item = orders::OrderDataResult>, Error> {
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
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let results = client.auto_open_orders(false).expect("request failed");
    /// for order_data in results {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn auto_open_orders(&self, auto_bind: bool) -> Result<impl Iterator<Item = orders::OrderDataResult>, Error> {
        orders::auto_open_orders(self, auto_bind)
    }

    /// Cancels an open [Order].
    ///
    /// # Arguments
    /// * `order_id` - ID of [Order] to cancel.
    /// * `manual_order_cancel_time` - can't find documentation. leave blank.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let order_id = 15;
    /// let results = client.cancel_order(order_id, "").expect("request failed");
    /// for result in results {
    ///    println!("{result:?}");
    /// }
    /// ```
    pub fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<impl Iterator<Item = orders::CancelOrderResult>, Error> {
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
    /// let results = client.completed_orders(false).expect("request failed");
    /// for order_data in results {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn completed_orders(&self, api_only: bool) -> Result<impl Iterator<Item = orders::OrderDataResult>, Error> {
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
    /// let executions = client.executions(filter).expect("request failed");
    /// for execution_data in executions {
    ///    println!("{execution_data:?}")
    /// }
    /// ```
    pub fn executions(&self, filter: orders::ExecutionFilter) -> Result<impl Iterator<Item = orders::ExecutionDataResult>, Error> {
        orders::executions(self, filter)
    }

    /// Cancels all open [Order]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// let mut client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// client.global_cancel().expect("request failed");
    /// ```
    pub fn global_cancel(&self) -> Result<(), Error> {
        orders::global_cancel(self)
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
    /// let next_valid_order_id = client.next_valid_order_id().expect("request failed");
    /// println!("next_valid_order_id: {next_valid_order_id}");
    /// ```
    pub fn next_valid_order_id(&self) -> Result<i32, Error> {
        orders::next_valid_order_id(self)
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
    /// let results = client.open_orders().expect("request failed");
    /// for order_data in results {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn open_orders(&self) -> Result<impl Iterator<Item = OrderDataResult>, Error> {
        orders::open_orders(self)
    }

    /// Submits an [Order].
    ///
    /// Submits an [Order] using [Client] for the given [Contract].
    /// Immediately after the order was submitted correctly, the TWS will start sending events concerning the order's activity via IBApi.EWrapper.openOrder and IBApi.EWrapper.orderStatus
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
    /// use ibapi::orders::{order_builder, Action, OrderNotification};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("MSFT");
    /// let order = order_builder::market_order(Action::Buy, 100.0);
    /// let order_id = client.next_order_id();
    ///
    /// let notifications = client.place_order(order_id, &contract, &order).expect("request failed");
    ///
    /// for notification in notifications {
    ///     match notification {
    ///         OrderNotification::OrderStatus(order_status) => {
    ///             println!("order status: {order_status:?}")
    ///         }
    ///         OrderNotification::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///         OrderNotification::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///         OrderNotification::CommissionReport(report) => println!("commission report: {report:?}"),
    ///         OrderNotification::Message(message) => println!("message: {message:?}"),
    ///    }
    /// }
    /// ```
    pub fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<impl Iterator<Item = OrderNotification>, Error> {
        orders::place_order(self, order_id, contract, order)
    }

    // === Historical Market Data ===

    /// Returns the timestamp of earliest available historical data for a contract and data type.
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{self, WhatToShow};
    ///
    /// let mut client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
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
    /// * `interval_end` - end date of interval to retrieve [historical::HistoricalData] for.
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
    ///     .historical_data(&contract, datetime!(2023-04-15 0:00 UTC), 7.days(), BarSize::Day, WhatToShow::Trades, true)
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
        interval_end: OffsetDateTime,
        duration: historical::Duration,
        bar_size: historical::BarSize,
        what_to_show: historical::WhatToShow,
        use_rth: bool,
    ) -> Result<historical::HistoricalData, Error> {
        historical::historical_data(self, contract, Some(interval_end), duration, bar_size, Some(what_to_show), use_rth)
    }

    /// Requests interval of historical data end now for [Contract].
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
    //
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
    ) -> Result<impl Iterator<Item = historical::TickBidAsk>, Error> {
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
    ) -> Result<impl Iterator<Item = historical::TickMidpoint>, Error> {
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
    ) -> Result<impl Iterator<Item = historical::TickLast>, Error> {
        historical::historical_ticks_trade(self, contract, start, end, number_of_ticks, use_rth)
    }

    // === Realtime Market Data ===

    /// Requests realtime bars.
    ///
    /// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
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
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    /// * `number_of_ticks` - number of ticks.
    /// * `ignore_size` - ignore size flag.
    pub fn tick_by_tick_all_last<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<impl Iterator<Item = realtime::Trade> + 'a, Error> {
        realtime::tick_by_tick_all_last(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick BidAsk ticks.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    /// * `number_of_ticks` - number of ticks.
    /// * `ignore_size` - ignore size flag.
    pub fn tick_by_tick_bid_ask<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<impl Iterator<Item = realtime::BidAsk> + 'a, Error> {
        realtime::tick_by_tick_bid_ask(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick Last ticks.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    /// * `number_of_ticks` - number of ticks.
    /// * `ignore_size` - ignore size flag.
    pub fn tick_by_tick_last<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<impl Iterator<Item = realtime::Trade> + 'a, Error> {
        realtime::tick_by_tick_last(self, contract, number_of_ticks, ignore_size)
    }

    /// Requests tick by tick MidPoint ticks.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    /// * `number_of_ticks` - number of ticks.
    /// * `ignore_size` - ignore size flag.
    pub fn tick_by_tick_midpoint<'a>(
        &'a self,
        contract: &Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<'a, MidPoint>, Error> {
        realtime::tick_by_tick_midpoint(self, contract, number_of_ticks, ignore_size)
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

/// Server sends data until not required
/// Supports the handling of responses from TWS.
/// Cancelled with dropped if not already cancelled.
///
#[allow(private_bounds)]
pub struct Subscription<'a, T: Subscribable<T>> {
    pub(crate) client: &'a Client,
    pub(crate) request_id: Option<i32>,
    pub(crate) order_id: Option<i32>,
    pub(crate) message_type: Option<OutgoingMessages>,
    pub(crate) subscription: InternalSubscription,
    pub(crate) phantom: PhantomData<T>,
}

#[allow(private_bounds)]
impl<'a, T: Subscribable<T>> Subscription<'a, T> {
    pub(crate) fn new(client: &'a Client, subscription: InternalSubscription) -> Self {
        if let Some(request_id) = subscription.request_id {
            Subscription {
                client,
                request_id: Some(request_id),
                order_id: None,
                message_type: None,
                subscription,
                phantom: PhantomData,
            }
        } else if let Some(order_id) = subscription.order_id {
            Subscription {
                client,
                request_id: None,
                order_id: Some(order_id),
                message_type: None,
                subscription,
                phantom: PhantomData,
            }
        } else if let Some(message_type) = subscription.message_type {
            Subscription {
                client,
                request_id: None,
                order_id: None,
                message_type: Some(message_type),
                subscription,
                phantom: PhantomData,
            }
        } else {
            panic!("unsupported internal subscription: {:?}", subscription)
        }
    }

    /// Blocks until the item become available.
    pub fn next(&self) -> Option<T> {
        loop {
            match self.subscription.next() {
                Some(Response::Message(mut message)) => {
                    if T::RESPONSE_MESSAGE_IDS.contains(&message.message_type()) {
                        match T::decode(self.client.server_version(), &mut message) {
                            Ok(val) => return Some(val),
                            Err(err) => {
                                error!("error decoding execution data: {err}");
                            }
                        }
                    } else if message.message_type() == IncomingMessages::Error {
                        let error_message = message.peek_string(4);
                        error!("{error_message}");
                        return None;
                    } else {
                        info!("subscription iterator unexpected message: {message:?}");
                    }
                }
                Some(Response::Cancelled) => {
                    debug!("subscription cancelled");
                    return None;
                }
                Some(Response::Disconnected) => {
                    debug!("server disconnected");
                    return None;
                }
                _ => {
                    return None;
                }
            }
        }
    }

    /// To request the next bar in a non-blocking manner.
    ///
    /// ```text
    /// //loop {
    ///    // Check if the next bar is available without waiting
    ///    //if let Some(bar) = subscription.try_next() {
    ///        // Process the available bar (e.g., use it in calculations)
    ///    //}
    ///    // Perform other work before checking for the next bar
    /// //}
    /// ```
    pub fn try_next(&self) -> Option<T> {
        if let Some(Response::Message(mut message)) = self.subscription.try_next() {
            if message.message_type() == IncomingMessages::Error {
                error!("{}", message.peek_string(4));
                return None;
            }

            match T::decode(self.client.server_version(), &mut message) {
                Ok(val) => Some(val),
                Err(err) => {
                    error!("error decoding message: {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// To request the next bar in a non-blocking manner.
    ///
    /// ```text
    /// //loop {
    ///    // Check if the next bar is available without waiting
    ///   // if let Some(bar) = subscription.next_timeout() {
    ///        // Process the available bar (e.g., use it in calculations)
    ///   // }
    ///    // Perform other work before checking for the next bar
    /// //}
    /// ```
    pub fn next_timeout(&self, timeout: Duration) -> Option<T> {
        if let Some(Response::Message(mut message)) = self.subscription.next_timeout(timeout) {
            if message.message_type() == IncomingMessages::Error {
                error!("{}", message.peek_string(4));
                return None;
            }

            match T::decode(self.client.server_version(), &mut message) {
                Ok(val) => Some(val),
                Err(err) => {
                    error!("error decoding message: {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Cancel the subscription
    pub fn cancel(&self) -> Result<(), Error> {
        if let Some(request_id) = self.request_id {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id) {
                self.client.message_bus.cancel_subscription(request_id, &message)?;
                self.subscription.cancel();
            }
        } else if let Some(order_id) = self.order_id {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id) {
                self.client.message_bus.cancel_order_subscription(order_id, &message)?;
                self.subscription.cancel();
            }
        } else if let Some(message_type) = self.message_type {
            if let Ok(message) = T::cancel_message(self.client.server_version(), self.request_id) {
                self.client.message_bus.cancel_shared_subscription(message_type, &message)?;
                self.subscription.cancel();
            }
        } else {
            debug!("Could not determine cancel method")
        }
        Ok(())
    }

    pub fn iter(&self) -> SubscriptionIter<T> {
        SubscriptionIter { subscription: self }
    }

    pub fn try_iter(&self) -> SubscriptionTryIter<T> {
        SubscriptionTryIter { subscription: self }
    }

    pub fn timeout_iter(&self, timeout: Duration) -> SubscriptionTimeoutIter<T> {
        SubscriptionTimeoutIter { subscription: self, timeout }
    }
}

impl<'a, T: Subscribable<T>> Drop for Subscription<'a, T> {
    fn drop(&mut self) {
        if let Err(err) = self.cancel() {
            error!("error cancelling subscription: {err}");
        }
    }
}

pub(crate) trait Subscribable<T> {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages];
    const CANCEL_MESSAGE_ID: Option<IncomingMessages> = None;

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<T, Error>;
    fn cancel_message(_server_version: i32, _request_id: Option<i32>) -> Result<RequestMessage, Error> {
        Err(Error::Simple("not implemented".into()))
    }
}

/// Blocking iterator. Blocks until next item available.
#[allow(private_bounds)]
pub struct SubscriptionIter<'a, T: Subscribable<T>> {
    subscription: &'a Subscription<'a, T>,
}

impl<'a, T: Subscribable<T>> Iterator for SubscriptionIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: Subscribable<T>> IntoIterator for &'a Subscription<'a, T> {
    type Item = T;
    type IntoIter = SubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Non-Blocking iterator. Returns immediately if not available.
#[allow(private_bounds)]
pub struct SubscriptionTryIter<'a, T: Subscribable<T>> {
    subscription: &'a Subscription<'a, T>,
}

impl<'a, T: Subscribable<T>> Iterator for SubscriptionTryIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// Blocks and waits for timeout
#[allow(private_bounds)]
pub struct SubscriptionTimeoutIter<'a, T: Subscribable<T>> {
    subscription: &'a Subscription<'a, T>,
    timeout: Duration,
}

impl<'a, T: Subscribable<T>> Iterator for SubscriptionTimeoutIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

/// Marker trait for shared channels
pub trait SharesChannel {}

#[cfg(test)]
mod tests;
