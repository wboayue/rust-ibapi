//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

use log::debug;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::connection::{r#async::AsyncConnection, ConnectionMetadata};
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::transport::{
    r#async::{AsyncInternalSubscription, AsyncTcpMessageBus},
    AsyncMessageBus,
};
use crate::Error;

use super::id_generator::ClientIdManager;
use crate::accounts;
use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
use crate::accounts::{AccountSummaryResult, AccountUpdate, AccountUpdateMulti, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
use crate::subscriptions::Subscription;

/// Asynchronous TWS API Client
#[derive(Clone)]
pub struct Client {
    /// IB server version
    pub(crate) server_version: i32,
    pub(crate) connection_time: Option<OffsetDateTime>,
    pub(crate) time_zone: Option<&'static Tz>,
    pub(crate) message_bus: Arc<dyn AsyncMessageBus>,

    client_id: i32,                   // ID of client.
    id_manager: Arc<ClientIdManager>, // Manages request and order ID generation
}

impl Drop for Client {
    fn drop(&mut self) {
        debug!("dropping async client");
        // Request shutdown of the message bus synchronously
        self.message_bus.request_shutdown_sync();
    }
}

impl Client {
    /// Establishes async connection to TWS or Gateway
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
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     println!("server_version: {}", client.server_version());
    ///     println!("connection_time: {:?}", client.connection_time());
    ///     println!("next_order_id: {}", client.next_order_id());
    /// }
    /// ```
    pub async fn connect(address: &str, client_id: i32) -> Result<Client, Error> {
        let connection = AsyncConnection::connect(address, client_id).await?;
        let connection_metadata = connection.connection_metadata();

        let message_bus = Arc::new(AsyncTcpMessageBus::new(connection)?);

        // Start background task to read messages from TWS
        message_bus
            .clone()
            .process_messages(connection_metadata.server_version, Duration::from_secs(1))?;

        Client::new(connection_metadata, message_bus)
    }

    fn new(connection_metadata: ConnectionMetadata, message_bus: Arc<dyn AsyncMessageBus>) -> Result<Client, Error> {
        let client = Client {
            server_version: connection_metadata.server_version,
            connection_time: connection_metadata.connection_time,
            time_zone: connection_metadata.time_zone,
            message_bus,
            client_id: connection_metadata.client_id,
            id_manager: Arc::new(ClientIdManager::new(connection_metadata.next_order_id)),
        };

        Ok(client)
    }

    /// Returns the server version
    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    /// Returns the connection time
    pub fn connection_time(&self) -> Option<OffsetDateTime> {
        self.connection_time
    }

    /// Returns the ID assigned to the [Client].
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id()
    }

    /// Returns the next request ID
    pub fn next_request_id(&self) -> i32 {
        self.id_manager.next_request_id()
    }

    /// Sets the current value of order ID.
    pub(crate) fn set_next_order_id(&self, order_id: i32) {
        self.id_manager.set_order_id(order_id);
    }

    /// Check server version requirement
    pub fn check_server_version(&self, required_version: i32, feature: &str) -> Result<(), Error> {
        if self.server_version < required_version {
            return Err(Error::Simple(format!(
                "Server version {} is too old. {} requires version {}",
                self.server_version, feature, required_version
            )));
        }
        Ok(())
    }

    /// Send a request with a specific request ID
    pub async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_request(request_id, message).await
    }

    /// Send a shared request (no ID)
    pub async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_shared_request(message_type, message).await
    }

    /// Send an order request
    pub async fn send_order(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_order_request(order_id, message).await
    }

    /// Create order update subscription
    pub async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.create_order_update_subscription().await
    }

    /// Send a message without expecting a response
    pub async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
        self.message_bus.send_message(message).await
    }

    // === Account Management ===

    /// Requests the current server time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let server_time = client.server_time().await.expect("error requesting server time");
    ///     println!("server time: {server_time:?}");
    /// }
    /// ```
    pub async fn server_time(&self) -> Result<OffsetDateTime, Error> {
        accounts::server_time(self).await
    }

    /// Subscribes to position updates for all accessible accounts.
    /// All positions sent initially, and then only updates as positions change.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::PositionUpdate;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut subscription = client.positions().await.expect("error requesting positions");
    ///     
    ///     while let Some(position_response) = subscription.next().await {
    ///         match position_response {
    ///             Ok(PositionUpdate::Position(position)) => println!("{position:?}"),
    ///             Ok(PositionUpdate::PositionEnd) => println!("initial set of positions received"),
    ///             Err(e) => eprintln!("Error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        accounts::positions(self).await
    }

    /// Subscribes to position updates for account and/or model.
    /// Initially all positions are returned, and then updates are returned for any position changes in real time.
    ///
    /// # Arguments
    /// * `account`    - If an account Id is provided, only the account's positions belonging to the specified model will be delivered.
    /// * `model_code` - The code of the model's positions we are interested in.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::types::AccountId;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = AccountId("U1234567".to_string());
    ///     let mut subscription = client.positions_multi(Some(&account), None).await.expect("error requesting positions by model");
    ///     
    ///     while let Some(position) = subscription.next().await {
    ///         println!("{position:?}")
    ///     }
    /// }
    /// ```
    pub async fn positions_multi(
        &self,
        account: Option<&AccountId>,
        model_code: Option<&ModelCode>,
    ) -> Result<Subscription<PositionUpdateMulti>, Error> {
        accounts::positions_multi(self, account, model_code).await
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
    /// use ibapi::accounts::types::AccountId;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let account = AccountId("account id".to_string());
    ///     let mut subscription = client.pnl(&account, None).await.expect("error requesting pnl");
    ///     
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl(&self, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Subscription<PnL>, Error> {
        accounts::pnl(self, account, model_code).await
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
    /// use ibapi::accounts::types::{AccountId, ContractId};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = AccountId("<account id>".to_string());
    ///     let contract_id = ContractId(1001);
    ///
    ///     let mut subscription = client.pnl_single(&account, contract_id, None).await.expect("error requesting pnl");
    ///     
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl_single(
        &self,
        account: &AccountId,
        contract_id: ContractId,
        model_code: Option<&ModelCode>,
    ) -> Result<Subscription<PnLSingle>, Error> {
        accounts::pnl_single(self, account, contract_id, model_code).await
    }

    /// Requests a specific account's summary. Subscribes to the account summary as presented in the TWS' Account Summary tab.
    /// Data received is specified by using a specific tags value.
    ///
    /// # Arguments
    /// * `group` - Set to "All" to return account summary data for all accounts, or set to a specific Advisor Account Group name that has already been created in TWS Global Configuration.
    /// * `tags`  - List of the desired tags.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountSummaryTags;
    /// use ibapi::accounts::types::AccountGroup;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let group = AccountGroup("All".to_string());
    ///
    ///     let mut subscription = client.account_summary(&group, AccountSummaryTags::ALL).await.expect("error requesting account summary");
    ///     
    ///     while let Some(summary) = subscription.next().await {
    ///         println!("{summary:?}")
    ///     }
    /// }
    /// ```
    pub async fn account_summary(&self, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<AccountSummaryResult>, Error> {
        accounts::account_summary(self, group, tags).await
    }

    /// Subscribes to a specific account's information and portfolio.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change
    /// in a position, or to an account value every 3 minutes if it has changed. Only one account can be subscribed at a time.
    ///
    /// # Arguments
    /// * `account` - The account id (i.e. U1234567) for which the information is requested.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountUpdate;
    /// use ibapi::accounts::types::AccountId;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = AccountId("U1234567".to_string());
    ///
    ///     let mut subscription = client.account_updates(&account).await.expect("error requesting account updates");
    ///     
    ///     while let Some(update_result) = subscription.next().await {
    ///         match update_result {
    ///             Ok(update) => {
    ///                 println!("{update:?}");
    ///
    ///                 // stop after full initial update
    ///                 if let AccountUpdate::End = update {
    ///                     break;
    ///                 }
    ///             }
    ///             Err(e) => eprintln!("Error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn account_updates(&self, account: &AccountId) -> Result<Subscription<AccountUpdate>, Error> {
        accounts::account_updates(self, account).await
    }

    /// Requests account updates for account and/or model.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change
    /// in a position, or to an account value every 3 minutes if it has changed. Only one account can be subscribed at a time.
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
    /// use ibapi::accounts::types::AccountId;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = AccountId("U1234567".to_string());
    ///
    ///     let mut subscription = client.account_updates_multi(Some(&account), None).await.expect("error requesting account updates multi");
    ///     
    ///     while let Some(update_result) = subscription.next().await {
    ///         match update_result {
    ///             Ok(update) => {
    ///                 println!("{update:?}");
    ///
    ///                 // stop after full initial update
    ///                 if let AccountUpdateMulti::End = update {
    ///                     break;
    ///                 }
    ///             }
    ///             Err(e) => eprintln!("Error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn account_updates_multi(
        &self,
        account: Option<&AccountId>,
        model_code: Option<&ModelCode>,
    ) -> Result<Subscription<AccountUpdateMulti>, Error> {
        accounts::account_updates_multi(self, account, model_code).await
    }

    /// Requests the accounts to which the logged user has access to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let accounts = client.managed_accounts().await.expect("error requesting managed accounts");
    ///     println!("managed accounts: {accounts:?}")
    /// }
    /// ```
    pub async fn managed_accounts(&self) -> Result<Vec<String>, Error> {
        accounts::managed_accounts(self).await
    }

    /// Get current family codes for all accessible accounts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let codes = client.family_codes().await.expect("error requesting family codes");
    ///     println!("family codes: {codes:?}")
    /// }
    /// ```
    pub async fn family_codes(&self) -> Result<Vec<FamilyCode>, Error> {
        accounts::family_codes(self).await
    }

    // === Market Data ===

    /// Requests real time market data.
    /// Returns market data for an instrument either in real time or 10-15 minutes delayed (depending on the market data type specified,
    /// see `switch_market_data_type`).
    ///
    /// # Arguments
    /// * `contract` - The Contract for which the data is being requested
    /// * `generic_ticks` - comma separated ids of the available generic ticks: https://interactivebrokers.github.io/tws-api/tick_types.html
    /// * `snapshot` - Check to return a single snapshot of Market data and have the market data subscription cancel.
    /// * `regulatory_snapshot` - snapshot for US stocks requests NBBO snapshots for users which have "US Securities Snapshot Bundle" subscription but not corresponding Network A, B, or C subscription necessary for streaming market data. One-time snapshot of current market price that will incur a fee of 1 cent to the account per snapshot.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{contracts::Contract, market_data::realtime::TickTypes, Client};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL");
    ///
    ///     let generic_ticks = &["233", "293"];
    ///     let snapshot = false;
    ///     let regulatory_snapshot = false;
    ///
    ///     let mut subscription = client
    ///         .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
    ///         .await
    ///         .expect("error requesting market data");
    ///
    ///     while let Some(tick_result) = subscription.next().await {
    ///         match tick_result {
    ///             Ok(tick) => match tick {
    ///                 TickTypes::Price(tick_price) => println!("{tick_price:?}"),
    ///                 TickTypes::Size(tick_size) => println!("{tick_size:?}"),
    ///                 TickTypes::String(tick_string) => println!("{tick_string:?}"),
    ///                 TickTypes::SnapshotEnd => {
    ///                     println!("Snapshot completed");
    ///                     break;
    ///                 }
    ///                 _ => {}
    ///             },
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn market_data(
        &self,
        contract: &crate::contracts::Contract,
        generic_ticks: &[&str],
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<crate::market_data::realtime::TickTypes>, Error> {
        crate::market_data::realtime::market_data(self, contract, generic_ticks, snapshot, regulatory_snapshot).await
    }

    /// Requests real time bars
    /// Currently, only 5 seconds bars are provided.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which the depth is being requested
    /// * `bar_size` - Currently being ignored
    /// * `what_to_show` - The nature of the data being retrieved (TRADES, MIDPOINT, BID, ASK)
    /// * `use_rth` - Set to false to obtain the data which was also generated outside of the Regular Trading Hours, set to true to obtain only the RTH data
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("TSLA");
    ///     let mut subscription = client
    ///         .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false)
    ///         .await
    ///         .expect("request failed");
    ///
    ///     while let Some(bar_result) = subscription.next().await {
    ///         match bar_result {
    ///             Ok(bar) => println!("{bar:?}"),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn realtime_bars(
        &self,
        contract: &crate::contracts::Contract,
        bar_size: crate::market_data::realtime::BarSize,
        what_to_show: crate::market_data::realtime::WhatToShow,
        use_rth: bool,
    ) -> Result<Subscription<crate::market_data::realtime::Bar>, Error> {
        crate::market_data::realtime::realtime_bars(self, contract, &bar_size, &what_to_show, use_rth, vec![]).await
    }

    /// Requests tick by tick AllLast ticks.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which tick-by-tick data is requested.
    /// * `number_of_ticks` - Number of historical ticks to return from the TWS's historical database. Max value is 1000.
    /// * `ignore_size` - Ignore size flag.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL");
    ///     let mut subscription = client
    ///         .tick_by_tick_all_last(&contract, 0, false)
    ///         .await
    ///         .expect("request failed");
    ///
    ///     while let Some(trade_result) = subscription.next().await {
    ///         match trade_result {
    ///             Ok(trade) => println!("Trade: {} - ${} x {} on {}",
    ///                 trade.time, trade.price, trade.size, trade.exchange),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn tick_by_tick_all_last(
        &self,
        contract: &crate::contracts::Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<crate::market_data::realtime::Trade>, Error> {
        crate::market_data::realtime::tick_by_tick_all_last(self, contract, number_of_ticks, ignore_size).await
    }

    /// Requests tick by tick Last ticks.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which tick-by-tick data is requested.
    /// * `number_of_ticks` - Number of historical ticks to return from the TWS's historical database. Max value is 1000.
    /// * `ignore_size` - Ignore size flag.
    pub async fn tick_by_tick_last(
        &self,
        contract: &crate::contracts::Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<crate::market_data::realtime::Trade>, Error> {
        crate::market_data::realtime::tick_by_tick_last(self, contract, number_of_ticks, ignore_size).await
    }

    /// Requests tick by tick BidAsk ticks.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which tick-by-tick data is requested.
    /// * `number_of_ticks` - Number of historical ticks to return from the TWS's historical database. Max value is 1000.
    /// * `ignore_size` - Ignore size flag.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL");
    ///     let mut subscription = client
    ///         .tick_by_tick_bid_ask(&contract, 0, false)
    ///         .await
    ///         .expect("request failed");
    ///
    ///     while let Some(quote_result) = subscription.next().await {
    ///         match quote_result {
    ///             Ok(quote) => println!("Quote: {} - Bid: ${} x {} | Ask: ${} x {}",
    ///                 quote.time, quote.bid_price, quote.bid_size,
    ///                 quote.ask_price, quote.ask_size),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn tick_by_tick_bid_ask(
        &self,
        contract: &crate::contracts::Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<crate::market_data::realtime::BidAsk>, Error> {
        crate::market_data::realtime::tick_by_tick_bid_ask(self, contract, number_of_ticks, ignore_size).await
    }

    /// Requests tick by tick MidPoint ticks.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which tick-by-tick data is requested.
    /// * `number_of_ticks` - Number of historical ticks to return from the TWS's historical database. Max value is 1000.
    /// * `ignore_size` - Ignore size flag.
    pub async fn tick_by_tick_midpoint(
        &self,
        contract: &crate::contracts::Contract,
        number_of_ticks: i32,
        ignore_size: bool,
    ) -> Result<Subscription<crate::market_data::realtime::MidPoint>, Error> {
        crate::market_data::realtime::tick_by_tick_midpoint(self, contract, number_of_ticks, ignore_size).await
    }

    /// Requests the contract's market depth (order book).
    ///
    /// This request returns the full available market depth and updates whenever there's a change in the order book.
    /// Market depth data is not available for all instruments. Check the TWS Contract Details under "Market Data Availability" - "Deep Book" field
    /// before requesting market depth.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which the depth is being requested
    /// * `number_of_rows` - The number of rows on each side of the order book (max 50)
    /// * `is_smart_depth` - Flag indicates that this is smart depth request
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::MarketDepths;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL");
    ///     let mut subscription = client
    ///         .market_depth(&contract, 5, false)
    ///         .await
    ///         .expect("request failed");
    ///
    ///     while let Some(depth_result) = subscription.next().await {
    ///         match depth_result {
    ///             Ok(MarketDepths::MarketDepth(depth)) => {
    ///                 let side = if depth.side == 1 { "Bid" } else { "Ask" };
    ///                 let operation = match depth.operation {
    ///                     0 => "Insert",
    ///                     1 => "Update",
    ///                     2 => "Delete",
    ///                     _ => "Unknown",
    ///                 };
    ///                 println!("{} {} at position {} - Price: ${}, Size: {}",
    ///                     operation, side, depth.position, depth.price, depth.size);
    ///             }
    ///             Ok(MarketDepths::Notice(notice)) => println!("Notice: {}", notice.message),
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn market_depth(
        &self,
        contract: &crate::contracts::Contract,
        number_of_rows: i32,
        is_smart_depth: bool,
    ) -> Result<Subscription<crate::market_data::realtime::MarketDepths>, Error> {
        crate::market_data::realtime::market_depth(self, contract, number_of_rows, is_smart_depth).await
    }

    /// Requests venues for which market data is returned to market_depth (those with market makers)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let exchanges = client.market_depth_exchanges().await.expect("request failed");
    ///     for exchange in exchanges {
    ///         println!("{} - {} ({})",
    ///             exchange.exchange_name, exchange.security_type, exchange.service_data_type);
    ///     }
    /// }
    /// ```
    pub async fn market_depth_exchanges(&self) -> Result<Vec<crate::market_data::realtime::DepthMarketDataDescription>, Error> {
        crate::market_data::realtime::market_depth_exchanges(self).await
    }

    /// Switches market data type returned from market data request.
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
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let market_data_type = MarketDataType::Live;
    ///     client.switch_market_data_type(market_data_type).await.expect("request failed");
    ///     println!("market data switched: {market_data_type:?}");
    /// }
    /// ```
    pub async fn switch_market_data_type(&self, market_data_type: crate::market_data::MarketDataType) -> Result<(), Error> {
        crate::market_data::switch_market_data_type(self, market_data_type).await
    }

    /// Returns the timestamp of earliest available historical data for a contract and data type.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::WhatToShow;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("MSFT");
    ///     let what_to_show = WhatToShow::Trades;
    ///     let use_rth = true;
    ///
    ///     let timestamp = client
    ///         .head_timestamp(&contract, what_to_show, use_rth)
    ///         .await
    ///         .expect("error requesting head timestamp");
    ///     println!("Earliest data available: {timestamp:?}");
    /// }
    /// ```
    pub async fn head_timestamp(
        &self,
        contract: &crate::contracts::Contract,
        what_to_show: crate::market_data::historical::WhatToShow,
        use_rth: bool,
    ) -> Result<OffsetDateTime, Error> {
        crate::market_data::historical::head_timestamp(self, contract, what_to_show, use_rth).await
    }

    /// Requests historical bars data.
    ///
    /// When requesting historical data, a finishing time and date is required along with a duration string.
    /// For example, having: end_date = 20130701 23:59:59 GMT and duration = 3 D
    /// will return three days of data counting backwards from July 1st 2013 at 23:59:59 GMT resulting in all the
    /// available bars of the last three days until the date and time specified.
    ///
    /// # Arguments
    /// * `contract` - The contract for which we want to retrieve the data.
    /// * `end_date` - Request's ending time. If None, current time is used.
    /// * `duration` - The amount of time for which the data needs to be retrieved.
    /// * `bar_size` - The bar size.
    /// * `what_to_show` - The kind of information being retrieved.
    /// * `use_rth` - Set to false to obtain the data which was also generated outside of the Regular Trading Hours, set to true to obtain only the RTH data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("TSLA");
    ///
    ///     let interval_end = Some(datetime!(2023-04-11 20:00 UTC));
    ///     let duration = 5.days();
    ///     let bar_size = BarSize::Hour;
    ///     let what_to_show = Some(WhatToShow::Trades);
    ///     let use_rth = true;
    ///
    ///     let historical_data = client
    ///         .historical_data(&contract, interval_end, duration, bar_size, what_to_show, use_rth)
    ///         .await
    ///         .expect("historical bars request failed");
    ///
    ///     println!("start: {}, end: {}", historical_data.start, historical_data.end);
    ///     for bar in &historical_data.bars {
    ///         println!("{bar:?}")
    ///     }
    /// }
    /// ```
    pub async fn historical_data(
        &self,
        contract: &crate::contracts::Contract,
        end_date: Option<OffsetDateTime>,
        duration: crate::market_data::historical::Duration,
        bar_size: crate::market_data::historical::BarSize,
        what_to_show: Option<crate::market_data::historical::WhatToShow>,
        use_rth: bool,
    ) -> Result<crate::market_data::historical::HistoricalData, Error> {
        crate::market_data::historical::historical_data(self, contract, end_date, duration, bar_size, what_to_show, use_rth).await
    }

    /// Requests historical schedule.
    ///
    /// # Arguments
    /// * `contract` - Contract object for which trading schedule is requested.
    /// * `end_date` - Request's ending date. If None, current time is used.
    /// * `duration` - The amount of time for which the data needs to be retrieved.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM");
    ///
    ///     let end_date = Some(datetime!(2022-11-21 00:00 UTC));
    ///     let duration = 30.days();
    ///
    ///     let schedule = client
    ///         .historical_schedule(&contract, end_date, duration)
    ///         .await
    ///         .expect("error requesting historical schedule");
    ///     
    ///     println!("Trading schedule from {} to {}", schedule.start, schedule.end);
    ///     for session in &schedule.sessions {
    ///         println!("  {} - Trading: {} to {}",
    ///             session.reference, session.start, session.end);
    ///     }
    /// }
    /// ```
    pub async fn historical_schedule(
        &self,
        contract: &crate::contracts::Contract,
        end_date: Option<OffsetDateTime>,
        duration: crate::market_data::historical::Duration,
    ) -> Result<crate::market_data::historical::Schedule, Error> {
        crate::market_data::historical::historical_schedule(self, contract, end_date, duration).await
    }

    /// Requests historical bid/ask tick data.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `start` - Start timestamp. Either start or end must be specified.
    /// * `end` - End timestamp. Either start or end must be specified.
    /// * `number_of_ticks` - Number of ticks to retrieve
    /// * `use_rth` - Data from regular trading hours (true), or all available hours (false)
    /// * `ignore_size` - Ignore size flag
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM");
    ///
    ///     let start = Some(datetime!(2022-11-07 16:00 UTC));
    ///     let end = Some(datetime!(2022-11-07 17:00 UTC));
    ///     let number_of_ticks = 1000;
    ///     let use_rth = true;
    ///     let ignore_size = false;
    ///
    ///     let mut subscription = client
    ///         .historical_ticks_bid_ask(&contract, start, end, number_of_ticks, use_rth, ignore_size)
    ///         .await
    ///         .expect("error requesting historical ticks");
    ///
    ///     while let Some(tick) = subscription.next().await {
    ///         println!("Bid/Ask tick: {} - Bid: ${} x {} | Ask: ${} x {}",
    ///             tick.timestamp, tick.price_bid, tick.size_bid,
    ///             tick.price_ask, tick.size_ask);
    ///     }
    /// }
    /// ```
    pub async fn historical_ticks_bid_ask(
        &self,
        contract: &crate::contracts::Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
        ignore_size: bool,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickBidAsk>, Error> {
        crate::market_data::historical::historical_ticks_bid_ask(self, contract, start, end, number_of_ticks, use_rth, ignore_size).await
    }

    /// Requests historical midpoint tick data.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `start` - Start timestamp. Either start or end must be specified.
    /// * `end` - End timestamp. Either start or end must be specified.
    /// * `number_of_ticks` - Number of ticks to retrieve
    /// * `use_rth` - Data from regular trading hours (true), or all available hours (false)
    pub async fn historical_ticks_mid_point(
        &self,
        contract: &crate::contracts::Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickMidpoint>, Error> {
        crate::market_data::historical::historical_ticks_mid_point(self, contract, start, end, number_of_ticks, use_rth).await
    }

    /// Requests historical trade tick data.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `start` - Start timestamp. Either start or end must be specified.
    /// * `end` - End timestamp. Either start or end must be specified.
    /// * `number_of_ticks` - Number of ticks to retrieve
    /// * `use_rth` - Data from regular trading hours (true), or all available hours (false)
    pub async fn historical_ticks_trade(
        &self,
        contract: &crate::contracts::Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        use_rth: bool,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickLast>, Error> {
        crate::market_data::historical::historical_ticks_trade(self, contract, start, end, number_of_ticks, use_rth).await
    }

    /// Returns histogram of market data for a contract.
    ///
    /// # Arguments
    /// * `contract` - Contract object for which histogram is being requested
    /// * `use_rth` - Data from regular trading hours (true), or all available hours (false)
    /// * `period` - Period of which data is being requested
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::BarSize;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM");
    ///
    ///     let use_rth = true;
    ///     let period = BarSize::Week;
    ///
    ///     let histogram = client
    ///         .histogram_data(&contract, use_rth, period)
    ///         .await
    ///         .expect("error requesting histogram");
    ///
    ///     for entry in &histogram {
    ///         println!("Price: ${} - Count: {}", entry.price, entry.size);
    ///     }
    /// }
    /// ```
    pub async fn histogram_data(
        &self,
        contract: &crate::contracts::Contract,
        use_rth: bool,
        period: crate::market_data::historical::BarSize,
    ) -> Result<Vec<crate::market_data::historical::HistogramEntry>, Error> {
        crate::market_data::historical::histogram_data(self, contract, use_rth, period).await
    }

    // === Wall Street Horizon (WSH) Data ===

    /// Requests Wall Street Horizon metadata information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let metadata = client.wsh_metadata().await.expect("error requesting wsh metadata");
    ///     println!("wsh metadata: {metadata:?}")
    /// }
    /// ```
    pub async fn wsh_metadata(&self) -> Result<crate::wsh::WshMetadata, Error> {
        crate::wsh::wsh_metadata(self).await
    }

    /// Requests event data for a specified contract from the Wall Street Horizons (WSH) calendar.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    /// * `start_date`  - Start date of the event request.
    /// * `end_date`    - End date of the event request.
    /// * `limit`       - Number of events to return.
    /// * `auto_fill`   - Autofill configuration for watchlist, portfolio, and position.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use time::macros::date;
    /// use ibapi::wsh::AutoFill;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract_id = 12345;
    ///     let start_date = Some(date!(2024-01-01));
    ///     let end_date = Some(date!(2024-12-31));
    ///     let limit = Some(100);
    ///     let auto_fill = Some(AutoFill {
    ///         competitors: true,
    ///         portfolio: false,
    ///         watchlist: false,
    ///     });
    ///
    ///     let event_data = client
    ///         .wsh_event_data_by_contract(contract_id, start_date, end_date, limit, auto_fill)
    ///         .await
    ///         .expect("error requesting wsh event data");
    ///     println!("wsh event data: {event_data:?}")
    /// }
    /// ```
    pub async fn wsh_event_data_by_contract(
        &self,
        contract_id: i32,
        start_date: Option<time::Date>,
        end_date: Option<time::Date>,
        limit: Option<i32>,
        auto_fill: Option<crate::wsh::AutoFill>,
    ) -> Result<crate::wsh::WshEventData, Error> {
        crate::wsh::wsh_event_data_by_contract(self, contract_id, start_date, end_date, limit, auto_fill).await
    }

    /// Requests event data using a filter from the Wall Street Horizons (WSH) calendar.
    ///
    /// # Arguments
    ///
    /// * `filter`    - Filter for the event request (e.g. JSON-encoded string).
    /// * `limit`     - Number of events to return.
    /// * `auto_fill` - Autofill configuration for watchlist, portfolio, and position.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::wsh::AutoFill;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let filter = r#"{"country": "US"}"#;
    ///     let limit = Some(100);
    ///     let auto_fill = Some(AutoFill {
    ///         competitors: true,
    ///         portfolio: false,
    ///         watchlist: false,
    ///     });
    ///
    ///     let mut event_data_subscription = client
    ///         .wsh_event_data_by_filter(filter, limit, auto_fill)
    ///         .await
    ///         .expect("error requesting wsh event data");
    ///     
    ///     while let Some(event_data) = event_data_subscription.next().await {
    ///         println!("{event_data:?}")
    ///     }
    /// }
    /// ```
    pub async fn wsh_event_data_by_filter(
        &self,
        filter: &str,
        limit: Option<i32>,
        auto_fill: Option<crate::wsh::AutoFill>,
    ) -> Result<Subscription<crate::wsh::WshEventData>, Error> {
        crate::wsh::wsh_event_data_by_filter(self, filter, limit, auto_fill).await
    }

    // === Contract Management ===

    /// Requests detailed contract information for matching contracts.
    ///
    /// This function returns all contracts that match the provided contract sample.
    /// It can be used to retrieve complete options and futures chains.
    ///
    /// # Arguments
    /// * `contract` - The Contract used as a sample to query available contracts
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract = Contract::stock("AAPL");
    ///     let details = client.contract_details(&contract).await.expect("request failed");
    ///     
    ///     for detail in details {
    ///         println!("Contract: {} - Exchange: {}", detail.contract.symbol, detail.contract.exchange);
    ///     }
    /// }
    /// ```
    pub async fn contract_details(&self, contract: &crate::contracts::Contract) -> Result<Vec<crate::contracts::ContractDetails>, Error> {
        crate::contracts::contract_details(self, contract).await
    }

    /// Searches for stock contracts matching the provided pattern.
    ///
    /// # Arguments
    /// * `pattern` - Either start of ticker symbol or (for larger strings) company name
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let symbols = client.matching_symbols("AAP").await.expect("request failed");
    ///     for symbol in symbols {
    ///         println!("{} - {} ({})", symbol.contract.symbol,
    ///                  symbol.contract.primary_exchange, symbol.contract.currency);
    ///     }
    /// }
    /// ```
    pub async fn matching_symbols(&self, pattern: &str) -> Result<Vec<crate::contracts::ContractDescription>, Error> {
        crate::contracts::matching_symbols(self, pattern).await
    }

    /// Retrieves market rule details for a specific market rule ID.
    ///
    /// Market rules define how minimum price increments change with price.
    /// Rule IDs can be obtained from contract details.
    ///
    /// # Arguments
    /// * `market_rule_id` - The market rule ID to query
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let rule = client.market_rule(26).await.expect("request failed");
    ///     for increment in rule.price_increments {
    ///         println!("Above ${}: increment ${}", increment.low_edge, increment.increment);
    ///     }
    /// }
    /// ```
    pub async fn market_rule(&self, market_rule_id: i32) -> Result<crate::contracts::MarketRule, Error> {
        crate::contracts::market_rule(self, market_rule_id).await
    }

    /// Calculates option price based on volatility and underlying price.
    ///
    /// # Arguments
    /// * `contract` - The option contract
    /// * `volatility` - Hypothetical volatility
    /// * `underlying_price` - Hypothetical underlying price
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let option = Contract::option("AAPL", "20240119", 150.0, "C");
    ///     let computation = client.calculate_option_price(&option, 0.3, 145.0).await
    ///         .expect("calculation failed");
    ///         
    ///     if let Some(price) = computation.option_price {
    ///         println!("Option price: ${:.2}", price);
    ///     }
    /// }
    /// ```
    pub async fn calculate_option_price(
        &self,
        contract: &crate::contracts::Contract,
        volatility: f64,
        underlying_price: f64,
    ) -> Result<crate::contracts::OptionComputation, Error> {
        crate::contracts::calculate_option_price(self, contract, volatility, underlying_price).await
    }

    /// Calculates implied volatility based on option and underlying prices.
    ///
    /// # Arguments
    /// * `contract` - The option contract
    /// * `option_price` - Hypothetical option price
    /// * `underlying_price` - Hypothetical underlying price
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let option = Contract::option("AAPL", "20240119", 150.0, "C");
    ///     let computation = client.calculate_implied_volatility(&option, 7.5, 148.0).await
    ///         .expect("calculation failed");
    ///         
    ///     if let Some(iv) = computation.implied_volatility {
    ///         println!("Implied volatility: {:.2}%", iv * 100.0);
    ///     }
    /// }
    /// ```
    pub async fn calculate_implied_volatility(
        &self,
        contract: &crate::contracts::Contract,
        option_price: f64,
        underlying_price: f64,
    ) -> Result<crate::contracts::OptionComputation, Error> {
        crate::contracts::calculate_implied_volatility(self, contract, option_price, underlying_price).await
    }

    /// Requests option chain data for an underlying instrument.
    ///
    /// Returns option expiration dates and strikes available for the specified underlying.
    ///
    /// # Arguments
    /// * `symbol` - The underlying symbol
    /// * `exchange` - The exchange
    /// * `security_type` - The underlying security type
    /// * `contract_id` - The underlying contract ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::SecurityType;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut chain = client.option_chain("AAPL", "SMART", SecurityType::Stock, 265598).await
    ///         .expect("request failed");
    ///         
    ///     while let Some(result) = chain.next().await {
    ///         match result {
    ///             Ok(data) => {
    ///                 println!("Expirations: {:?}", data.expirations);
    ///                 println!("Strikes: {:?}", data.strikes);
    ///             }
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn option_chain(
        &self,
        symbol: &str,
        exchange: &str,
        security_type: crate::contracts::SecurityType,
        contract_id: i32,
    ) -> Result<Subscription<crate::contracts::OptionChain>, Error> {
        crate::contracts::option_chain(self, symbol, exchange, security_type, contract_id).await
    }

    // === Order Management ===

    /// Subscribes to order update events. Only one subscription can be active at a time.
    ///
    /// This function returns a subscription that will receive updates of activity for all orders placed by the client.
    /// Use this when you need a global view of all order activity, especially with submit_order().
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::Client;
    /// use ibapi::orders::OrderUpdate;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut stream = client.order_update_stream().await.expect("failed to create stream");
    ///     
    ///     while let Some(update) = stream.next().await {
    ///         match update {
    ///             Ok(OrderUpdate::OrderStatus(status)) => {
    ///                 println!("Order {} status: {}", status.order_id, status.status);
    ///             }
    ///             Ok(OrderUpdate::ExecutionData(exec)) => {
    ///                 println!("Execution: {} shares @ {}", exec.execution.shares, exec.execution.price);
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn order_update_stream(&self) -> Result<Subscription<crate::orders::OrderUpdate>, Error> {
        crate::orders::order_update_stream(self).await
    }

    /// Submits an Order (fire-and-forget).
    ///
    /// After the order is submitted correctly, events will be returned through the order_update_stream().
    /// This is a fire-and-forget method that does not wait for confirmation or return a subscription.
    ///
    /// # Arguments
    /// * `order_id` - Unique order identifier
    /// * `contract` - Contract to submit order for
    /// * `order` - Order details
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::{Contract, SecurityType};
    /// use ibapi::orders::{order_builder, Action};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut contract = Contract::default();
    ///     contract.symbol = "AAPL".to_string();
    ///     contract.security_type = SecurityType::Stock;
    ///     contract.exchange = "SMART".to_string();
    ///     contract.currency = "USD".to_string();
    ///     
    ///     let order = order_builder::limit_order(Action::Buy, 100.0, 150.0);
    ///     let order_id = client.next_order_id();
    ///     
    ///     client.submit_order(order_id, &contract, &order).await.expect("failed to submit order");
    /// }
    /// ```
    pub async fn submit_order(&self, order_id: i32, contract: &crate::contracts::Contract, order: &crate::orders::Order) -> Result<(), Error> {
        crate::orders::submit_order(self, order_id, contract, order).await
    }

    /// Submits an Order with a subscription for updates.
    ///
    /// After the order is submitted correctly, events will be returned concerning the order's activity
    /// through the returned subscription.
    ///
    /// # Arguments
    /// * `order_id` - Unique order identifier
    /// * `contract` - Contract to submit order for
    /// * `order` - Order details
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::Client;
    /// use ibapi::contracts::{Contract, SecurityType};
    /// use ibapi::orders::{order_builder, PlaceOrder, Action};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut contract = Contract::default();
    ///     contract.symbol = "AAPL".to_string();
    ///     contract.security_type = SecurityType::Stock;
    ///     contract.exchange = "SMART".to_string();
    ///     contract.currency = "USD".to_string();
    ///     
    ///     let order = order_builder::limit_order(Action::Buy, 100.0, 150.0);
    ///     let order_id = client.next_order_id();
    ///     
    ///     let mut subscription = client.place_order(order_id, &contract, &order).await
    ///         .expect("failed to place order");
    ///         
    ///     while let Some(update) = subscription.next().await {
    ///         match update {
    ///             Ok(PlaceOrder::OrderStatus(status)) => {
    ///                 println!("Status: {}", status.status);
    ///                 if status.status == "Filled" { break; }
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn place_order(
        &self,
        order_id: i32,
        contract: &crate::contracts::Contract,
        order: &crate::orders::Order,
    ) -> Result<Subscription<crate::orders::PlaceOrder>, Error> {
        crate::orders::place_order(self, order_id, contract, order).await
    }

    /// Cancels an open Order.
    ///
    /// # Arguments
    /// * `order_id` - Order ID to cancel
    /// * `manual_order_cancel_time` - Time of manual order cancellation (empty string for API cancellations)
    pub async fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<crate::orders::CancelOrder>, Error> {
        crate::orders::cancel_order(self, order_id, manual_order_cancel_time).await
    }

    /// Cancels all open Orders.
    pub async fn global_cancel(&self) -> Result<(), Error> {
        crate::orders::global_cancel(self).await
    }

    /// Gets next valid order id.
    pub async fn next_valid_order_id(&self) -> Result<i32, Error> {
        crate::orders::next_valid_order_id(self).await
    }

    /// Requests completed Orders.
    ///
    /// # Arguments
    /// * `api_only` - If true, only orders placed through the API are returned
    pub async fn completed_orders(&self, api_only: bool) -> Result<Subscription<crate::orders::Orders>, Error> {
        crate::orders::completed_orders(self, api_only).await
    }

    /// Requests all open orders placed by this specific API client (identified by the API client id).
    /// For client ID 0, this will bind previous manual TWS orders.
    pub async fn open_orders(&self) -> Result<Subscription<crate::orders::Orders>, Error> {
        crate::orders::open_orders(self).await
    }

    /// Requests all *current* open orders in associated accounts at the current moment.
    /// Open orders are returned once; this function does not initiate a subscription.
    pub async fn all_open_orders(&self) -> Result<Subscription<crate::orders::Orders>, Error> {
        crate::orders::all_open_orders(self).await
    }

    /// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
    ///
    /// # Arguments
    /// * `auto_bind` - If true, newly submitted orders will be implicitly associated with this client
    pub async fn auto_open_orders(&self, auto_bind: bool) -> Result<Subscription<crate::orders::Orders>, Error> {
        crate::orders::auto_open_orders(self, auto_bind).await
    }

    /// Requests current day's (since midnight) executions matching the filter.
    ///
    /// Only the current day's executions can be retrieved.
    /// Along with the ExecutionData, the CommissionReport will also be returned.
    ///
    /// # Arguments
    /// * `filter` - Filter criteria used to determine which execution reports are returned
    pub async fn executions(&self, filter: crate::orders::ExecutionFilter) -> Result<Subscription<crate::orders::Executions>, Error> {
        crate::orders::executions(self, filter).await
    }

    /// Exercises an options contract.
    ///
    /// # Arguments
    /// * `contract` - Option contract to exercise
    /// * `exercise_action` - Whether to exercise (1) or lapse (2)
    /// * `exercise_quantity` - Number of contracts to exercise
    /// * `account` - Account for which to exercise
    /// * `ovrd` - Override default handling action
    /// * `manual_order_time` - Time of manual order entry
    pub async fn exercise_options(
        &self,
        contract: &crate::contracts::Contract,
        exercise_action: crate::orders::ExerciseAction,
        exercise_quantity: i32,
        account: &str,
        ovrd: bool,
        manual_order_time: Option<OffsetDateTime>,
    ) -> Result<Subscription<crate::orders::ExerciseOptions>, Error> {
        crate::orders::exercise_options(self, contract, exercise_action, exercise_quantity, account, ovrd, manual_order_time).await
    }

    // === News Management ===

    /// Requests available news providers.
    ///
    /// Returns a list of news providers that the user has subscribed to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let providers = client.news_providers().await.expect("request failed");
    ///     for provider in providers {
    ///         println!("{} - {}", provider.code, provider.name);
    ///     }
    /// }
    /// ```
    pub async fn news_providers(&self) -> Result<Vec<crate::news::NewsProvider>, Error> {
        crate::news::news_providers(self).await
    }

    /// Subscribes to IB News Bulletins.
    ///
    /// # Arguments
    /// * `all_messages` - If true, returns all messages including exchange availability
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut bulletins = client.news_bulletins(true).await.expect("request failed");
    ///     while let Some(result) = bulletins.next().await {
    ///         match result {
    ///             Ok(bulletin) => println!("{}: {}", bulletin.exchange, bulletin.message),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn news_bulletins(&self, all_messages: bool) -> Result<Subscription<crate::news::NewsBulletin>, Error> {
        crate::news::news_bulletins(self, all_messages).await
    }

    /// Requests historical news headlines.
    ///
    /// # Arguments
    /// * `contract_id` - Contract ID to get news for
    /// * `provider_codes` - List of provider codes to filter by
    /// * `start_time` - Start of the time period
    /// * `end_time` - End of the time period
    /// * `total_results` - Maximum number of headlines to return
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract_id = 265598; // AAPL
    ///     let providers = &["BRFG", "DJNL"];
    ///     let end_time = time::OffsetDateTime::now_utc();
    ///     let start_time = end_time - time::Duration::days(7);
    ///     
    ///     let mut news = client
    ///         .historical_news(contract_id, providers, start_time, end_time, 100)
    ///         .await
    ///         .expect("request failed");
    ///         
    ///     while let Some(result) = news.next().await {
    ///         match result {
    ///             Ok(article) => println!("{}: {}", article.time, article.headline),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn historical_news(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u8,
    ) -> Result<Subscription<crate::news::NewsArticle>, Error> {
        crate::news::historical_news(self, contract_id, provider_codes, start_time, end_time, total_results).await
    }

    /// Requests the body of a news article.
    ///
    /// # Arguments
    /// * `provider_code` - The news provider code
    /// * `article_id` - The article ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let article = client.news_article("BRFG", "BRFG$12345").await.expect("request failed");
    ///     println!("Article type: {:?}", article.article_type);
    ///     println!("Content: {}", article.article_text);
    /// }
    /// ```
    pub async fn news_article(&self, provider_code: &str, article_id: &str) -> Result<crate::news::NewsArticleBody, Error> {
        crate::news::news_article(self, provider_code, article_id).await
    }

    /// Subscribes to real-time news for a specific contract.
    ///
    /// # Arguments
    /// * `contract` - The contract to monitor
    /// * `provider_codes` - List of provider codes to subscribe to
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract = Contract::stock("AAPL");
    ///     let providers = &["BRFG", "DJNL"];
    ///     
    ///     let mut news = client.contract_news(&contract, providers).await.expect("request failed");
    ///     while let Some(result) = news.next().await {
    ///         match result {
    ///             Ok(article) => println!("{}: {}", article.time, article.headline),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn contract_news(
        &self,
        contract: &crate::contracts::Contract,
        provider_codes: &[&str],
    ) -> Result<Subscription<crate::news::NewsArticle>, Error> {
        crate::news::contract_news(self, contract, provider_codes).await
    }

    /// Subscribes to broad tape news from a specific provider.
    ///
    /// # Arguments
    /// * `provider_code` - The news provider code
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let mut news = client.broad_tape_news("BRFG").await.expect("request failed");
    ///     while let Some(result) = news.next().await {
    ///         match result {
    ///             Ok(article) => println!("{}: {}", article.time, article.headline),
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<crate::news::NewsArticle>, Error> {
        crate::news::broad_tape_news(self, provider_code).await
    }

    // === Scanner ===

    /// Requests scanner parameters available in TWS.
    ///
    /// Returns an XML string containing all available scanner parameters including
    /// scan types, locations, instruments, and filters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let xml = client.scanner_parameters().await.expect("request failed");
    ///     println!("Scanner parameters XML: {} bytes", xml.len());
    /// }
    /// ```
    pub async fn scanner_parameters(&self) -> Result<String, Error> {
        crate::scanner::scanner_parameters(self).await
    }

    /// Starts a subscription to market scanner results.
    ///
    /// Scans the market based on the specified criteria and returns matching contracts.
    ///
    /// # Arguments
    /// * `subscription` - Scanner subscription parameters defining the scan criteria
    /// * `filter` - Additional filters to apply to the scan
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::scanner::ScannerSubscription;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let subscription = ScannerSubscription {
    ///         number_of_rows: 10,
    ///         instrument: Some("STK".to_string()),
    ///         location_code: Some("STK.US.MAJOR".to_string()),
    ///         scan_code: Some("TOP_PERC_GAIN".to_string()),
    ///         above_price: Some(5.0),
    ///         ..Default::default()
    ///     };
    ///     
    ///     let mut scanner = client.scanner_subscription(&subscription, &vec![]).await
    ///         .expect("request failed");
    ///         
    ///     while let Some(result) = scanner.next().await {
    ///         match result {
    ///             Ok(data_list) => {
    ///                 for data in data_list {
    ///                     println!("Rank {}: {}", data.rank,
    ///                              data.contract_details.contract.symbol);
    ///                 }
    ///             }
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn scanner_subscription(
        &self,
        subscription: &crate::scanner::ScannerSubscription,
        filter: &Vec<crate::orders::TagValue>,
    ) -> Result<Subscription<Vec<crate::scanner::ScannerData>>, Error> {
        crate::scanner::scanner_subscription(self, subscription, filter).await
    }

    /// Creates a stubbed client for testing
    #[cfg(test)]
    pub fn stubbed(message_bus: Arc<dyn AsyncMessageBus>, server_version: i32) -> Self {
        use crate::connection::ConnectionMetadata;

        let connection_metadata = ConnectionMetadata {
            client_id: 100,
            next_order_id: 9000,
            server_version,
            managed_accounts: String::new(),
            connection_time: None,
            time_zone: None,
        };

        Client::new(connection_metadata, message_bus).expect("Failed to create stubbed client")
    }

    /// Get a reference to the message bus for testing
    #[cfg(test)]
    pub fn message_bus(&self) -> &Arc<dyn AsyncMessageBus> {
        &self.message_bus
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::client::common::tests::*;

    const CLIENT_ID: i32 = 100;

    #[tokio::test]
    async fn test_connect() {
        let gateway = setup_connect();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        assert_eq!(client.client_id(), CLIENT_ID);
        assert_eq!(client.server_version(), gateway.server_version());
        assert_eq!(client.time_zone, gateway.time_zone());

        assert_eq!(gateway.requests().len(), 0, "No requests should be sent on connect");
    }

    #[tokio::test]
    async fn test_server_time() {
        let (gateway, expectations) = setup_server_time();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let server_time = client.server_time().await.unwrap();
        assert_eq!(server_time, expectations.server_time);

        let requests = gateway.requests();
        assert_eq!(requests[0], "49\01\0");
    }

    #[tokio::test]
    async fn test_next_valid_order_id() {
        let (gateway, expectations) = setup_next_valid_order_id();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let next_valid_order_id = client.next_valid_order_id().await.unwrap();
        assert_eq!(next_valid_order_id, expectations.next_valid_order_id);

        let requests = gateway.requests();
        assert_eq!(requests[0], "8\01\00\0");
    }

    #[tokio::test]
    async fn test_managed_accounts() {
        let (gateway, expectations) = setup_managed_accounts();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let accounts = client.managed_accounts().await.unwrap();
        assert_eq!(accounts, expectations.accounts);

        let requests = gateway.requests();
        assert_eq!(requests[0], "17\01\0");
    }

    #[tokio::test]
    async fn test_positions() {
        let gateway = setup_positions();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let mut positions = client.positions().await.unwrap();
        let mut position_count = 0;

        while let Some(position_update) = positions.next().await {
            match position_update.unwrap() {
                crate::accounts::PositionUpdate::Position(position) => {
                    assert_eq!(position.account, "DU1234567");
                    assert_eq!(position.contract.symbol, "AAPL");
                    assert_eq!(position.position, 500.0);
                    assert_eq!(position.average_cost, 150.25);
                    position_count += 1;
                }
                crate::accounts::PositionUpdate::PositionEnd => {
                    break;
                }
            }
        }

        assert_eq!(position_count, 1);
        let requests = gateway.requests();
        assert_eq!(requests[0], "61\01\0");
    }

    #[tokio::test]
    async fn test_positions_multi() {
        use crate::accounts::types::AccountId;

        let gateway = setup_positions_multi();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let account = AccountId("DU1234567".to_string());
        let mut positions = client.positions_multi(Some(&account), None).await.unwrap();
        let mut position_count = 0;

        while let Some(position_update) = positions.next().await {
            match position_update.unwrap() {
                crate::accounts::PositionUpdateMulti::Position(position) => {
                    position_count += 1;
                    if position_count == 1 {
                        assert_eq!(position.account, "DU1234567");
                        assert_eq!(position.contract.symbol, "AAPL");
                        assert_eq!(position.position, 500.0);
                        assert_eq!(position.average_cost, 150.25);
                        assert_eq!(position.model_code, "MODEL1");
                    } else if position_count == 2 {
                        assert_eq!(position.account, "DU1234568");
                        assert_eq!(position.contract.symbol, "GOOGL");
                        assert_eq!(position.position, 200.0);
                        assert_eq!(position.average_cost, 2500.00);
                        assert_eq!(position.model_code, "MODEL1");
                    }
                }
                crate::accounts::PositionUpdateMulti::PositionEnd => {
                    break;
                }
            }
        }

        assert_eq!(position_count, 2);
        let requests = gateway.requests();
        assert_eq!(requests[0], "74\01\09000\0DU1234567\0\0");
    }

    #[tokio::test]
    async fn test_account_summary() {
        use crate::accounts::types::AccountGroup;

        let gateway = setup_account_summary();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let group = AccountGroup("All".to_string());
        let tags = vec!["NetLiquidation", "TotalCashValue"];

        let mut summaries = client.account_summary(&group, &tags).await.unwrap();
        let mut summary_count = 0;

        while let Some(summary_result) = summaries.next().await {
            match summary_result.unwrap() {
                crate::accounts::AccountSummaryResult::Summary(summary) => {
                    assert_eq!(summary.account, "DU1234567");
                    assert_eq!(summary.currency, "USD");

                    if summary.tag == "NetLiquidation" {
                        assert_eq!(summary.value, "25000.00");
                    } else if summary.tag == "TotalCashValue" {
                        assert_eq!(summary.value, "15000.00");
                    }
                    summary_count += 1;
                }
                crate::accounts::AccountSummaryResult::End => {
                    break;
                }
            }
        }

        assert_eq!(summary_count, 2);
        let requests = gateway.requests();
        assert_eq!(requests[0], "62\01\09000\0All\0NetLiquidation,TotalCashValue\0");
    }

    #[tokio::test]
    async fn test_pnl() {
        use crate::accounts::types::AccountId;

        let gateway = setup_pnl();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let account = AccountId("DU1234567".to_string());
        let mut pnl = client.pnl(&account, None).await.unwrap();

        let first_pnl = pnl.next().await.unwrap().unwrap();
        assert_eq!(first_pnl.daily_pnl, 250.50);
        assert_eq!(first_pnl.unrealized_pnl, Some(1500.00));
        assert_eq!(first_pnl.realized_pnl, Some(750.00));

        let requests = gateway.requests();
        assert_eq!(requests[0], "92\09000\0DU1234567\0\0");
    }

    #[tokio::test]
    async fn test_pnl_single() {
        use crate::accounts::types::{AccountId, ContractId};

        let gateway = setup_pnl_single();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let account = AccountId("DU1234567".to_string());
        let contract_id = ContractId(12345);
        let mut pnl_single = client.pnl_single(&account, contract_id, None).await.unwrap();

        let first_pnl = pnl_single.next().await.unwrap().unwrap();
        assert_eq!(first_pnl.position, 100.0);
        assert_eq!(first_pnl.daily_pnl, 150.25);
        assert_eq!(first_pnl.unrealized_pnl, 500.00);
        assert_eq!(first_pnl.realized_pnl, 250.00);
        assert_eq!(first_pnl.value, 1000.00);

        let requests = gateway.requests();
        assert_eq!(requests[0], "94\09000\0DU1234567\0\012345\0");
    }

    #[tokio::test]
    async fn test_account_updates() {
        use crate::accounts::types::AccountId;

        let gateway = setup_account_updates();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let account = AccountId("DU1234567".to_string());
        let mut updates = client.account_updates(&account).await.unwrap();

        let mut value_count = 0;
        let mut portfolio_count = 0;
        let mut has_time_update = false;
        let mut has_end = false;

        while let Some(update) = updates.next().await {
            match update.unwrap() {
                crate::accounts::AccountUpdate::AccountValue(value) => {
                    assert_eq!(value.key, "NetLiquidation");
                    assert_eq!(value.value, "25000.00");
                    assert_eq!(value.currency, "USD");
                    assert_eq!(value.account, Some("DU1234567".to_string()));
                    value_count += 1;
                }
                crate::accounts::AccountUpdate::PortfolioValue(portfolio) => {
                    assert_eq!(portfolio.contract.symbol, "AAPL");
                    assert_eq!(portfolio.position, 500.0);
                    assert_eq!(portfolio.market_price, 151.50);
                    assert_eq!(portfolio.market_value, 75750.00);
                    assert_eq!(portfolio.average_cost, 150.25);
                    assert_eq!(portfolio.unrealized_pnl, 375.00);
                    assert_eq!(portfolio.realized_pnl, 125.00);
                    assert_eq!(portfolio.account, Some("DU1234567".to_string()));
                    portfolio_count += 1;
                }
                crate::accounts::AccountUpdate::UpdateTime(time) => {
                    assert_eq!(time.timestamp, "20240122 15:30:00");
                    has_time_update = true;
                }
                crate::accounts::AccountUpdate::End => {
                    has_end = true;
                    break;
                }
            }
        }

        assert!(has_end, "Expected End message");
        assert_eq!(value_count, 1);
        assert_eq!(portfolio_count, 1);
        assert!(has_time_update);

        let requests = gateway.requests();
        assert_eq!(requests[0], "6\02\01\0DU1234567\0");
    }

    #[tokio::test]
    async fn test_family_codes() {
        let gateway = setup_family_codes();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let family_codes = client.family_codes().await.unwrap();

        assert_eq!(family_codes.len(), 2);
        assert_eq!(family_codes[0].account_id, "DU1234567");
        assert_eq!(family_codes[0].family_code, "FAM001");
        assert_eq!(family_codes[1].account_id, "DU1234568");
        assert_eq!(family_codes[1].family_code, "FAM002");

        let requests = gateway.requests();
        assert_eq!(requests[0], "80\01\0");
    }

    #[tokio::test]
    async fn test_account_updates_multi() {
        use crate::accounts::types::{AccountId, ModelCode};

        let gateway = setup_account_updates_multi();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let account = AccountId("DU1234567".to_string());
        let model_code: Option<ModelCode> = None;
        let mut updates = client.account_updates_multi(Some(&account), model_code.as_ref()).await.unwrap();

        let mut cash_balance_found = false;
        let mut currency_found = false;
        let mut stock_market_value_found = false;
        let mut has_end = false;

        while let Some(update) = updates.next().await {
            match update.unwrap() {
                crate::accounts::AccountUpdateMulti::AccountMultiValue(value) => {
                    assert_eq!(value.account, "DU1234567");
                    assert_eq!(value.model_code, "");

                    match value.key.as_str() {
                        "CashBalance" => {
                            assert_eq!(value.value, "94629.71");
                            assert_eq!(value.currency, "USD");
                            cash_balance_found = true;
                        }
                        "Currency" => {
                            assert_eq!(value.value, "USD");
                            assert_eq!(value.currency, "USD");
                            currency_found = true;
                        }
                        "StockMarketValue" => {
                            assert_eq!(value.value, "0.00");
                            assert_eq!(value.currency, "BASE");
                            stock_market_value_found = true;
                        }
                        _ => panic!("Unexpected key: {}", value.key),
                    }
                }
                crate::accounts::AccountUpdateMulti::End => {
                    has_end = true;
                    break;
                }
            }
        }

        assert!(cash_balance_found, "Expected CashBalance update");
        assert!(currency_found, "Expected Currency update");
        assert!(stock_market_value_found, "Expected StockMarketValue update");
        assert!(has_end, "Expected End message");

        let requests = gateway.requests();
        assert_eq!(requests[0], "76\01\09000\0DU1234567\0\01\0");
    }
}
