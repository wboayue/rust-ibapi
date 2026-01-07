//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

use log::debug;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::connection::common::StartupMessageCallback;
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
use crate::contracts::Contract;
use crate::display_groups;
use crate::display_groups::DisplayGroupUpdate;
use crate::market_data::builder::MarketDataBuilder;
use crate::market_data::TradingHours;
use crate::orders::OrderBuilder;
use crate::subscriptions::Subscription;

/// Asynchronous TWS API Client
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
        Self::connect_with_callback(address, client_id, None).await
    }

    /// Establishes async connection to TWS or Gateway with a callback for startup messages
    ///
    /// This is similar to [`connect`](Self::connect), but allows you to provide a callback
    /// that will be invoked for any unsolicited messages received during the connection
    /// handshake (e.g., OpenOrder, OrderStatus).
    ///
    /// Note: The callback is only invoked during the initial connection, not during
    /// automatic reconnections.
    ///
    /// # Arguments
    /// * `address`          - address of server. e.g. 127.0.0.1:4002
    /// * `client_id`        - id of client. e.g. 100
    /// * `startup_callback` - optional callback for unsolicited messages during connection
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Client, StartupMessageCallback};
    /// use ibapi::messages::IncomingMessages;
    /// use std::sync::{Arc, Mutex};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let orders = Arc::new(Mutex::new(Vec::new()));
    ///     let orders_clone = orders.clone();
    ///
    ///     let callback: StartupMessageCallback = Box::new(move |msg| {
    ///         match msg.message_type() {
    ///             IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => {
    ///                 orders_clone.lock().unwrap().push(msg);
    ///             }
    ///             _ => {}
    ///         }
    ///     });
    ///
    ///     let client = Client::connect_with_callback("127.0.0.1:4002", 100, Some(callback))
    ///         .await
    ///         .expect("connection failed");
    ///
    ///     println!("Received {} startup orders", orders.lock().unwrap().len());
    /// }
    /// ```
    pub async fn connect_with_callback(address: &str, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Client, Error> {
        let connection = AsyncConnection::connect_with_callback(address, client_id, startup_callback).await?;
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

    /// Returns true if the client is currently connected to TWS/IB Gateway.
    ///
    /// This method checks if the underlying connection to TWS or IB Gateway is active.
    /// Returns false if the connection has been lost, shut down, or reset.
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
    ///     if client.is_connected() {
    ///         println!("Client is connected to TWS/Gateway");
    ///     } else {
    ///         println!("Client is not connected");
    ///     }
    /// }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.message_bus.is_connected()
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

    /// Start building an order for the given contract
    ///
    /// This is the primary API for creating orders, providing a fluent interface
    /// that guides you through the order creation process.
    ///
    /// # Example
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     
    ///     let order_id = client.order(&contract)
    ///         .buy(100)
    ///         .limit(50.0)
    ///         .submit().await.expect("order submission failed");
    /// }
    /// ```
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<'a, Self> {
        OrderBuilder::new(self, contract)
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
    pub(crate) async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_request(request_id, message).await
    }

    /// Send a shared request (no ID)
    pub(crate) async fn send_shared_request(
        &self,
        message_type: OutgoingMessages,
        message: RequestMessage,
    ) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_shared_request(message_type, message).await
    }

    /// Send an order request
    pub(crate) async fn send_order(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Use atomic subscribe + send
        self.message_bus.send_order_request(order_id, message).await
    }

    /// Create order update subscription
    pub(crate) async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        self.message_bus.create_order_update_subscription().await
    }

    /// Send a message without expecting a response
    pub(crate) async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
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

    /// Subscribes to TWS's Display Groups.
    ///
    /// Display Groups are a TWS-only feature (not available in IB Gateway).
    /// They allow organizing contracts into color-coded groups in the TWS UI.
    /// When subscribed, you receive updates whenever the user changes the contract
    /// displayed in that group within TWS.
    ///
    /// # Arguments
    /// * `group_id` - The ID of the group to subscribe to (1-9)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:7497", 100).await.expect("connection failed");
    ///
    ///     let mut subscription = client.subscribe_to_group_events(1).await.expect("error subscribing to group events");
    ///
    ///     while let Some(event) = subscription.next().await {
    ///         println!("Received group event: {:?}", event);
    ///     }
    /// }
    /// ```
    pub async fn subscribe_to_group_events(&self, group_id: i32) -> Result<Subscription<DisplayGroupUpdate>, Error> {
        display_groups::r#async::subscribe_to_group_events(self, group_id).await
    }

    /// Updates the contract displayed in a TWS display group.
    ///
    /// This function changes the contract shown in the specified display group within TWS.
    /// You must first subscribe to the group using [`subscribe_to_group_events`](Self::subscribe_to_group_events)
    /// before calling this function.
    ///
    /// # Arguments
    /// * `request_id` - The request ID from the subscription (use `subscription.request_id()`)
    /// * `contract_info` - Contract to display:
    ///   - `"contractID@exchange"` for individual contracts (e.g., "265598@SMART")
    ///   - `"none"` for empty selection
    ///   - `"combo"` for combination contracts
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:7497", 100).await.expect("connection failed");
    ///
    ///     // First subscribe to the display group
    ///     let subscription = client.subscribe_to_group_events(1).await.expect("subscription failed");
    ///     let request_id = subscription.request_id().expect("no request ID");
    ///
    ///     // Update the display group to show AAPL
    ///     client.update_display_group(request_id, "265598@SMART").await.expect("update failed");
    /// }
    /// ```
    pub async fn update_display_group(&self, request_id: i32, contract_info: &str) -> Result<(), Error> {
        display_groups::r#async::update_display_group(self, request_id, contract_info).await
    }

    // === Market Data ===

    /// Creates a market data subscription builder with a fluent interface.
    ///
    /// This is the preferred way to subscribe to market data, providing a more
    /// intuitive and discoverable API than the raw method.
    ///
    /// # Arguments
    /// * `contract` - The contract to receive market data for
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::client::r#async::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await
    ///         .expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     // Subscribe to real-time streaming data with specific tick types
    ///     let mut subscription = client.market_data(&contract)
    ///         .generic_ticks(&["233", "236"])  // RTVolume and Shortable
    ///         .subscribe()
    ///         .await
    ///         .expect("subscription failed");
    ///
    ///     while let Some(tick) = subscription.next().await {
    ///         match tick {
    ///             Ok(TickTypes::Price(price)) => println!("Price: {price:?}"),
    ///             Ok(TickTypes::Size(size)) => println!("Size: {size:?}"),
    ///             Ok(TickTypes::SnapshotEnd) => break,
    ///             Err(e) => eprintln!("Error: {e:?}"),
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub fn market_data<'a>(&'a self, contract: &'a Contract) -> MarketDataBuilder<'a, Self> {
        MarketDataBuilder::new(self, contract)
    }

    /// Requests real time bars
    /// Currently, only 5 seconds bars are provided.
    ///
    /// # Arguments
    /// * `contract` - The Contract for which the depth is being requested
    /// * `bar_size` - Currently being ignored
    /// * `what_to_show` - The nature of the data being retrieved (TRADES, MIDPOINT, BID, ASK)
    /// * `trading_hours` - Use TradingHours::Regular for data generated only during regular trading hours, or TradingHours::Extended to include data from outside regular trading hours
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("TSLA").build();
    ///     let mut subscription = client
    ///         .realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended)
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
        trading_hours: TradingHours,
    ) -> Result<Subscription<crate::market_data::realtime::Bar>, Error> {
        crate::market_data::realtime::realtime_bars(self, contract, &bar_size, &what_to_show, trading_hours, vec![]).await
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
    ///     let contract = Contract::stock("AAPL").build();
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
    ///     let contract = Contract::stock("AAPL").build();
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
    ///     let contract = Contract::stock("AAPL").build();
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
    ///     let market_data_type = MarketDataType::Realtime;
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
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("MSFT").build();
    ///     let what_to_show = WhatToShow::Trades;
    ///     let trading_hours = TradingHours::Regular;
    ///
    ///     let timestamp = client
    ///         .head_timestamp(&contract, what_to_show, trading_hours)
    ///         .await
    ///         .expect("error requesting head timestamp");
    ///     println!("Earliest data available: {timestamp:?}");
    /// }
    /// ```
    pub async fn head_timestamp(
        &self,
        contract: &crate::contracts::Contract,
        what_to_show: crate::market_data::historical::WhatToShow,
        trading_hours: TradingHours,
    ) -> Result<OffsetDateTime, Error> {
        crate::market_data::historical::head_timestamp(self, contract, what_to_show, trading_hours).await
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
    /// * `trading_hours` - Use TradingHours::Regular for data generated only during regular trading hours, or TradingHours::Extended to include data from outside regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     let interval_end = Some(datetime!(2023-04-11 20:00 UTC));
    ///     let duration = 5.days();
    ///     let bar_size = BarSize::Hour;
    ///     let what_to_show = Some(WhatToShow::Trades);
    ///     let trading_hours = TradingHours::Regular;
    ///
    ///     let historical_data = client
    ///         .historical_data(&contract, interval_end, duration, bar_size, what_to_show, trading_hours)
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
        trading_hours: TradingHours,
    ) -> Result<crate::market_data::historical::HistoricalData, Error> {
        crate::market_data::historical::historical_data(self, contract, end_date, duration, bar_size, what_to_show, trading_hours).await
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
    ///     let contract = Contract::stock("GM").build();
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
    /// * `trading_hours` - Regular trading hours only, or include extended hours
    /// * `ignore_size` - Ignore size flag
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM").build();
    ///
    ///     let start = Some(datetime!(2022-11-07 16:00 UTC));
    ///     let end = Some(datetime!(2022-11-07 17:00 UTC));
    ///     let number_of_ticks = 1000;
    ///     let trading_hours = TradingHours::Regular;
    ///     let ignore_size = false;
    ///
    ///     let mut subscription = client
    ///         .historical_ticks_bid_ask(&contract, start, end, number_of_ticks, trading_hours, ignore_size)
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
        trading_hours: TradingHours,
        ignore_size: bool,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickBidAsk>, Error> {
        crate::market_data::historical::historical_ticks_bid_ask(self, contract, start, end, number_of_ticks, trading_hours, ignore_size).await
    }

    /// Requests historical midpoint tick data.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `start` - Start timestamp. Either start or end must be specified.
    /// * `end` - End timestamp. Either start or end must be specified.
    /// * `number_of_ticks` - Number of ticks to retrieve
    /// * `trading_hours` - Regular trading hours only, or include extended hours
    pub async fn historical_ticks_mid_point(
        &self,
        contract: &crate::contracts::Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        trading_hours: TradingHours,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickMidpoint>, Error> {
        crate::market_data::historical::historical_ticks_mid_point(self, contract, start, end, number_of_ticks, trading_hours).await
    }

    /// Requests historical trade tick data.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `start` - Start timestamp. Either start or end must be specified.
    /// * `end` - End timestamp. Either start or end must be specified.
    /// * `number_of_ticks` - Number of ticks to retrieve
    /// * `trading_hours` - Regular trading hours only, or include extended hours
    pub async fn historical_ticks_trade(
        &self,
        contract: &crate::contracts::Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        trading_hours: TradingHours,
    ) -> Result<crate::market_data::historical::TickSubscription<crate::market_data::historical::TickLast>, Error> {
        crate::market_data::historical::historical_ticks_trade(self, contract, start, end, number_of_ticks, trading_hours).await
    }

    /// Returns histogram of market data for a contract.
    ///
    /// # Arguments
    /// * `contract` - Contract object for which histogram is being requested
    /// * `trading_hours` - Regular trading hours only, or include extended hours
    /// * `period` - Period of which data is being requested
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    /// use ibapi::market_data::historical::BarSize;
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM").build();
    ///
    ///     let trading_hours = TradingHours::Regular;
    ///     let period = BarSize::Week;
    ///
    ///     let histogram = client
    ///         .histogram_data(&contract, trading_hours, period)
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
        trading_hours: TradingHours,
        period: crate::market_data::historical::BarSize,
    ) -> Result<Vec<crate::market_data::historical::HistogramEntry>, Error> {
        crate::market_data::historical::histogram_data(self, contract, trading_hours, period).await
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
    ///     let contract = Contract::stock("AAPL").build();
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
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract = Contract::stock("AAPL").build();
    ///     
    ///     let order = client.order(&contract)
    ///         .buy(100)
    ///         .limit(150.0)
    ///         .build_order()
    ///         .expect("failed to build order");
    ///     
    ///     let order_id = client.next_order_id();
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
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::PlaceOrder;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract = Contract::stock("AAPL").build();
    ///     
    ///     let order = client.order(&contract)
    ///         .buy(100)
    ///         .limit(150.0)
    ///         .build_order()
    ///         .expect("failed to build order");
    ///     
    ///     let order_id = client.next_order_id();
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
    ///     let contract = Contract::stock("AAPL").build();
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
    use crate::contracts::{Currency, Exchange, Symbol};
    use crate::market_data::TradingHours;

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
                    assert_eq!(position.contract.symbol, Symbol::from("AAPL"));
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
                        assert_eq!(position.contract.symbol, Symbol::from("AAPL"));
                        assert_eq!(position.position, 500.0);
                        assert_eq!(position.average_cost, 150.25);
                        assert_eq!(position.model_code, "MODEL1");
                    } else if position_count == 2 {
                        assert_eq!(position.account, "DU1234568");
                        assert_eq!(position.contract.symbol, Symbol::from("GOOGL"));
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
                    assert_eq!(portfolio.contract.symbol, Symbol::from("AAPL"));
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

    #[tokio::test]
    async fn test_contract_details() {
        let gateway = setup_contract_details();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = crate::contracts::Contract::stock("AAPL").build();
        let details = client.contract_details(&contract).await.expect("Failed to get contract details");

        assert_eq!(details.len(), 1);
        let detail = &details[0];

        // Verify contract fields
        assert_eq!(detail.contract.symbol, Symbol::from("AAPL"));
        assert_eq!(detail.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(detail.contract.currency, Currency::from("USD"));
        assert_eq!(detail.contract.exchange, Exchange::from("NASDAQ"));
        assert_eq!(detail.contract.local_symbol, "AAPL");
        assert_eq!(detail.contract.trading_class, "AAPL");
        assert_eq!(detail.contract.contract_id, 265598);
        assert_eq!(detail.contract.primary_exchange, Exchange::from("NASDAQ"));

        // Verify contract details fields
        assert_eq!(detail.market_name, "NMS");
        assert_eq!(detail.min_tick, 0.01);
        assert!(detail.order_types.contains(&"LMT".to_string()));
        assert!(detail.order_types.contains(&"MKT".to_string()));
        assert!(detail.valid_exchanges.contains(&"SMART".to_string()));
        assert_eq!(detail.long_name, "Apple Inc");
        assert_eq!(detail.industry, "Technology");
        assert_eq!(detail.category, "Computers");
        assert_eq!(detail.subcategory, "Computers");
        assert_eq!(detail.time_zone_id, "US/Eastern");
        assert_eq!(detail.stock_type, "NMS");
        assert_eq!(detail.min_size, 1.0);
        assert_eq!(detail.size_increment, 1.0);
        assert_eq!(detail.suggested_size_increment, 1.0);

        let requests = gateway.requests();
        // Request format: OutgoingMessages::RequestContractData(9), version(8), request_id, contract_id(0),
        // symbol, security_type, last_trade_date, strike, right, multiplier, exchange, primary_exchange,
        // currency, local_symbol, trading_class, include_expired, security_id_type, security_id, issuer_id
        assert_eq!(requests[0], "9\08\09000\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0");
    }

    #[tokio::test]
    async fn test_matching_symbols() {
        let gateway = setup_matching_symbols();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract_descriptions = client.matching_symbols("AAP").await.expect("Failed to get matching symbols");

        assert_eq!(contract_descriptions.len(), 2, "Should have 2 matching symbols");

        // First contract description
        assert_eq!(contract_descriptions[0].contract.contract_id, 265598);
        assert_eq!(contract_descriptions[0].contract.symbol, Symbol::from("AAPL"));
        assert_eq!(contract_descriptions[0].contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(contract_descriptions[0].contract.primary_exchange, Exchange::from("NASDAQ"));
        assert_eq!(contract_descriptions[0].contract.currency, Currency::from("USD"));
        assert_eq!(contract_descriptions[0].derivative_security_types.len(), 2);
        assert_eq!(contract_descriptions[0].derivative_security_types[0], "OPT");
        assert_eq!(contract_descriptions[0].derivative_security_types[1], "WAR");
        assert_eq!(contract_descriptions[0].contract.description, "Apple Inc.");
        assert_eq!(contract_descriptions[0].contract.issuer_id, "AAPL123");

        // Second contract description
        assert_eq!(contract_descriptions[1].contract.contract_id, 276821);
        assert_eq!(contract_descriptions[1].contract.symbol, Symbol::from("MSFT"));
        assert_eq!(contract_descriptions[1].contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(contract_descriptions[1].contract.primary_exchange, Exchange::from("NASDAQ"));
        assert_eq!(contract_descriptions[1].contract.currency, Currency::from("USD"));
        assert_eq!(contract_descriptions[1].derivative_security_types.len(), 1);
        assert_eq!(contract_descriptions[1].derivative_security_types[0], "OPT");
        assert_eq!(contract_descriptions[1].contract.description, "Microsoft Corporation");
        assert_eq!(contract_descriptions[1].contract.issuer_id, "MSFT456");

        // Verify request format
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have 1 request");
        // Request format: RequestMatchingSymbols(81), request_id, pattern
        assert!(requests[0].starts_with("81\0"), "Request should start with message type 81");
        assert!(requests[0].contains("\0AAP\0"), "Request should contain the pattern AAP");
    }

    #[tokio::test]
    async fn test_market_rule() {
        let gateway = setup_market_rule();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let market_rule = client.market_rule(26).await.expect("Failed to get market rule");

        // Verify market rule ID
        assert_eq!(market_rule.market_rule_id, 26, "Market rule ID should be 26");

        // Verify price increments
        assert_eq!(market_rule.price_increments.len(), 3, "Should have 3 price increments");

        // First increment: 0-100, increment 0.01
        assert_eq!(market_rule.price_increments[0].low_edge, 0.0, "First increment low edge");
        assert_eq!(market_rule.price_increments[0].increment, 0.01, "First increment value");

        // Second increment: 100-1000, increment 0.05
        assert_eq!(market_rule.price_increments[1].low_edge, 100.0, "Second increment low edge");
        assert_eq!(market_rule.price_increments[1].increment, 0.05, "Second increment value");

        // Third increment: 1000+, increment 0.10
        assert_eq!(market_rule.price_increments[2].low_edge, 1000.0, "Third increment low edge");
        assert_eq!(market_rule.price_increments[2].increment, 0.10, "Third increment value");

        // Verify request format
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have 1 request");
        // Request format: RequestMarketRule(91), market_rule_id
        assert_eq!(requests[0], "91\026\0", "Request should be message type 91 with market rule ID 26");
    }

    #[tokio::test]
    async fn test_calculate_option_price() {
        let gateway = setup_calculate_option_price();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create an option contract
        let contract = crate::contracts::Contract {
            symbol: Symbol::from("AAPL"),
            security_type: crate::contracts::SecurityType::Option,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            last_trade_date_or_contract_month: "20250120".to_string(),
            strike: 100.0,
            right: "C".to_string(),
            ..Default::default()
        };

        let volatility = 0.25;
        let underlying_price = 100.0;

        let computation = client
            .calculate_option_price(&contract, volatility, underlying_price)
            .await
            .expect("Failed to calculate option price");

        // Verify computation results
        assert_eq!(
            computation.field,
            crate::contracts::tick_types::TickType::ModelOption,
            "Should be ModelOption tick type"
        );
        assert_eq!(computation.tick_attribute, Some(0), "Tick attribute should be 0");
        assert_eq!(computation.implied_volatility, Some(0.25), "Implied volatility should match");
        assert_eq!(computation.delta, Some(0.5), "Delta should be 0.5");
        assert_eq!(computation.option_price, Some(12.75), "Option price should be 12.75");
        assert_eq!(computation.present_value_dividend, Some(0.0), "PV dividend should be 0");
        assert_eq!(computation.gamma, Some(0.05), "Gamma should be 0.05");
        assert_eq!(computation.vega, Some(0.02), "Vega should be 0.02");
        assert_eq!(computation.theta, Some(-0.01), "Theta should be -0.01");
        assert_eq!(computation.underlying_price, Some(100.0), "Underlying price should be 100");

        // Verify request format
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have 1 request");
        // Request format: ReqCalcImpliedVolat(54), version(3), request_id, contract fields, volatility, underlying_price
        assert!(
            requests[0].starts_with("54\03\0"),
            "Request should start with message type 54 and version 3"
        );
        assert!(requests[0].contains("\0AAPL\0"), "Request should contain symbol AAPL");
        assert!(requests[0].contains("\00.25\0"), "Request should contain volatility 0.25");
        assert!(requests[0].contains("\0100\0"), "Request should contain underlying price 100");
    }

    #[tokio::test]
    async fn test_calculate_implied_volatility() {
        let gateway = setup_calculate_implied_volatility();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create an option contract
        let contract = crate::contracts::Contract {
            symbol: Symbol::from("MSFT"),
            security_type: crate::contracts::SecurityType::Option,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            last_trade_date_or_contract_month: "20250220".to_string(),
            strike: 105.0,
            right: "P".to_string(), // Put option
            ..Default::default()
        };

        let option_price = 15.50;
        let underlying_price = 105.0;

        let computation = client
            .calculate_implied_volatility(&contract, option_price, underlying_price)
            .await
            .expect("Failed to calculate implied volatility");

        // Verify computation results
        assert_eq!(
            computation.field,
            crate::contracts::tick_types::TickType::ModelOption,
            "Should be ModelOption tick type"
        );
        assert_eq!(computation.tick_attribute, Some(1), "Tick attribute should be 1 (price-based)");
        assert_eq!(computation.implied_volatility, Some(0.35), "Implied volatility should be 0.35");
        assert_eq!(computation.delta, Some(0.45), "Delta should be 0.45");
        assert_eq!(computation.option_price, Some(15.50), "Option price should be 15.50");
        assert_eq!(computation.present_value_dividend, Some(0.0), "PV dividend should be 0");
        assert_eq!(computation.gamma, Some(0.04), "Gamma should be 0.04");
        assert_eq!(computation.vega, Some(0.03), "Vega should be 0.03");
        assert_eq!(computation.theta, Some(-0.02), "Theta should be -0.02");
        assert_eq!(computation.underlying_price, Some(105.0), "Underlying price should be 105");

        // Verify request format
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have 1 request");
        // Request format: ReqCalcImpliedVolat(54), version(3), request_id, contract fields, option_price, underlying_price
        assert!(
            requests[0].starts_with("54\03\0"),
            "Request should start with message type 54 and version 3"
        );
        assert!(requests[0].contains("\0MSFT\0"), "Request should contain symbol MSFT");
        assert!(requests[0].contains("\015.5\0"), "Request should contain option price 15.5");
        assert!(requests[0].contains("\0105\0"), "Request should contain underlying price 105");
    }

    #[tokio::test]
    async fn test_option_chain() {
        let gateway = setup_option_chain();

        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let subscription = client
            .option_chain("AAPL", "", crate::contracts::SecurityType::Stock, 0)
            .await
            .expect("Failed to get option chain");

        let mut chains = Vec::new();
        let mut subscription = subscription;
        while let Some(chain_result) = subscription.next().await {
            match chain_result {
                Ok(chain) => chains.push(chain),
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received the expected chains
        assert_eq!(chains.len(), 2, "Should have 2 option chains");

        // Verify first chain (SMART)
        assert_eq!(chains[0].exchange, "SMART", "First chain should be SMART");
        assert_eq!(chains[0].underlying_contract_id, 265598, "Should have correct contract ID");
        assert_eq!(chains[0].trading_class, "AAPL", "Should have correct trading class");
        assert_eq!(chains[0].multiplier, "100", "Should have correct multiplier");
        assert_eq!(chains[0].expirations.len(), 3, "SMART should have 3 expirations");
        assert_eq!(chains[0].expirations[0], "20250117", "First expiration should be 20250117");
        assert_eq!(chains[0].expirations[1], "20250221", "Second expiration should be 20250221");
        assert_eq!(chains[0].expirations[2], "20250321", "Third expiration should be 20250321");
        assert_eq!(chains[0].strikes.len(), 5, "SMART should have 5 strikes");
        assert_eq!(chains[0].strikes[0], 90.0, "First strike should be 90.0");
        assert_eq!(chains[0].strikes[4], 110.0, "Last strike should be 110.0");

        // Verify second chain (CBOE)
        assert_eq!(chains[1].exchange, "CBOE", "Second chain should be CBOE");
        assert_eq!(chains[1].underlying_contract_id, 265598, "Should have correct contract ID");
        assert_eq!(chains[1].trading_class, "AAPL", "Should have correct trading class");
        assert_eq!(chains[1].multiplier, "100", "Should have correct multiplier");
        assert_eq!(chains[1].expirations.len(), 2, "CBOE should have 2 expirations");
        assert_eq!(chains[1].strikes.len(), 4, "CBOE should have 4 strikes");

        // Verify request format
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have 1 request");
        // Request format: RequestSecurityDefinitionOptionalParameters(78), request_id, symbol, exchange, security_type, contract_id
        assert!(requests[0].starts_with("78\0"), "Request should start with message type 78");
        assert!(requests[0].contains("\0AAPL\0"), "Request should contain symbol AAPL");
        assert!(requests[0].contains("\0STK\0"), "Request should contain security type STK");
    }

    #[tokio::test]
    async fn test_place_order() {
        use crate::client::common::tests::setup_place_order;
        use crate::contracts::Contract;
        use crate::orders::{order_builder, Action, PlaceOrder};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_place_order();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create a stock contract
        let contract = Contract::stock("AAPL").build();

        // Create a market order
        let order = order_builder::market_order(Action::Buy, 100.0);

        // Use order ID 1001 to match the mock responses
        let order_id = 1001;

        // Place the order
        let mut subscription = client.place_order(order_id, &contract, &order).await.expect("Failed to place order");

        // Collect all events from the subscription
        let mut order_status_count = 0;
        let mut _open_order_count = 0;
        let mut execution_count = 0;
        let mut commission_count = 0;

        // We expect 6 messages total (3 order statuses, 1 open order, 1 execution, 1 commission)
        // Take only the expected number of events to avoid reading the shutdown message
        for _ in 0..6 {
            let event = match subscription.next().await {
                Some(Ok(event)) => event,
                Some(Err(crate::Error::EndOfStream)) => break,
                Some(Err(e)) => panic!("Unexpected error: {:?}", e),
                None => break,
            };

            match event {
                PlaceOrder::OrderStatus(status) => {
                    order_status_count += 1;
                    assert_eq!(status.order_id, order_id);

                    if order_status_count == 1 {
                        // First status: PreSubmitted
                        assert_eq!(status.status, "PreSubmitted");
                        assert_eq!(status.filled, 0.0);
                        assert_eq!(status.remaining, 100.0);
                    } else if order_status_count == 2 {
                        // Second status: Submitted
                        assert_eq!(status.status, "Submitted");
                        assert_eq!(status.filled, 0.0);
                        assert_eq!(status.remaining, 100.0);
                    } else if order_status_count == 3 {
                        // Third status: Filled
                        assert_eq!(status.status, "Filled");
                        assert_eq!(status.filled, 100.0);
                        assert_eq!(status.remaining, 0.0);
                        assert_eq!(status.average_fill_price, 150.25);
                    }
                }
                PlaceOrder::OpenOrder(order_data) => {
                    _open_order_count += 1;
                    assert_eq!(order_data.order_id, order_id);
                    assert_eq!(order_data.contract.symbol, Symbol::from("AAPL"));
                    assert_eq!(order_data.contract.contract_id, 265598);
                    assert_eq!(order_data.order.action, Action::Buy);
                    assert_eq!(order_data.order.total_quantity, 100.0);
                    assert_eq!(order_data.order.order_type, "LMT");
                    assert_eq!(order_data.order.limit_price, Some(1.0));
                }
                PlaceOrder::ExecutionData(exec_data) => {
                    execution_count += 1;
                    assert_eq!(exec_data.execution.order_id, order_id);
                    assert_eq!(exec_data.contract.symbol, Symbol::from("AAPL"));
                    assert_eq!(exec_data.execution.shares, 100.0);
                    assert_eq!(exec_data.execution.price, 150.25);
                }
                PlaceOrder::CommissionReport(report) => {
                    commission_count += 1;
                    assert_eq!(report.commission, 1.25);
                    assert_eq!(report.currency, "USD");
                }
                PlaceOrder::Message(_) => {
                    // Skip any messages
                }
            }
        }

        // Verify we received all expected events
        assert_eq!(order_status_count, 3, "Should receive 3 order status updates");
        assert_eq!(_open_order_count, 1, "Should receive 1 open order");
        assert_eq!(execution_count, 1, "Should receive 1 execution");
        assert_eq!(commission_count, 1, "Should receive 1 commission report");

        // Verify the request was sent
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        // PlaceOrder message type is 3
        assert!(requests[0].starts_with("3\0"), "Request should be a PlaceOrder message");
        assert!(requests[0].contains(&format!("\0{}\0", order_id)), "Request should contain order ID");
    }

    #[tokio::test]
    async fn test_submit_order_with_order_update_stream() {
        use crate::client::common::tests::setup_place_order;
        use crate::contracts::Contract;
        use crate::orders::{order_builder, Action, OrderUpdate};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_place_order();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create a stock contract
        let contract = Contract::stock("AAPL").build();

        // Create a market order
        let order = order_builder::market_order(Action::Buy, 100.0);

        // Use order ID 1001 to match the mock responses
        let order_id = 1001;

        // First, start the order update stream
        let mut update_stream = client.order_update_stream().await.expect("Failed to create order update stream");

        // Submit the order (fire and forget)
        client.submit_order(order_id, &contract, &order).await.expect("Failed to submit order");

        // Collect events from the update stream
        let mut order_status_count = 0;
        let mut _open_order_count = 0;
        let mut execution_count = 0;
        let mut commission_count = 0;

        // Read events from the update stream with timeout
        println!("Starting to read from update stream...");
        let timeout_duration = std::time::Duration::from_millis(500);
        let mut events_received = 0;

        while events_received < 6 {
            let update = match tokio::time::timeout(timeout_duration, update_stream.next()).await {
                Ok(Some(Ok(update))) => {
                    events_received += 1;
                    println!("Event {}: {:?}", events_received, &update);
                    update
                }
                Ok(Some(Err(e))) => panic!("Error receiving update: {}", e),
                Ok(None) => break, // Stream ended
                Err(_) => break,   // Timeout reached
            };

            match update {
                OrderUpdate::OrderStatus(status) => {
                    order_status_count += 1;
                    assert_eq!(status.order_id, order_id);

                    if order_status_count == 1 {
                        // First status: PreSubmitted
                        assert_eq!(status.status, "PreSubmitted");
                        assert_eq!(status.filled, 0.0);
                        assert_eq!(status.remaining, 100.0);
                    } else if order_status_count == 2 {
                        // Second status: Submitted
                        assert_eq!(status.status, "Submitted");
                        assert_eq!(status.filled, 0.0);
                        assert_eq!(status.remaining, 100.0);
                    } else if order_status_count == 3 {
                        // Third status: Filled
                        assert_eq!(status.status, "Filled");
                        assert_eq!(status.filled, 100.0);
                        assert_eq!(status.remaining, 0.0);
                        assert_eq!(status.average_fill_price, 150.25);
                    }
                }
                OrderUpdate::OpenOrder(order_data) => {
                    _open_order_count += 1;
                    assert_eq!(order_data.order_id, order_id);
                    assert_eq!(order_data.contract.symbol, Symbol::from("AAPL"));
                    assert_eq!(order_data.contract.contract_id, 265598);
                    assert_eq!(order_data.order.action, Action::Buy);
                    assert_eq!(order_data.order.total_quantity, 100.0);
                    assert_eq!(order_data.order.order_type, "LMT");
                    assert_eq!(order_data.order.limit_price, Some(1.0));
                }
                OrderUpdate::ExecutionData(exec_data) => {
                    execution_count += 1;
                    assert_eq!(exec_data.execution.order_id, order_id);
                    assert_eq!(exec_data.contract.symbol, Symbol::from("AAPL"));
                    assert_eq!(exec_data.execution.shares, 100.0);
                    assert_eq!(exec_data.execution.price, 150.25);
                }
                OrderUpdate::CommissionReport(report) => {
                    commission_count += 1;
                    assert_eq!(report.commission, 1.25);
                    assert_eq!(report.currency, "USD");
                }
                OrderUpdate::Message(_) => {
                    // Skip any messages
                }
            }
        }

        // Verify we received all expected events
        assert_eq!(order_status_count, 3, "Should receive 3 order status updates");
        assert_eq!(_open_order_count, 1, "Should receive 1 open order");
        assert_eq!(execution_count, 1, "Should receive 1 execution");
        assert_eq!(commission_count, 1, "Should receive 1 commission report");

        // Verify the request was sent
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        // PlaceOrder message type is 3
        assert!(requests[0].starts_with("3\0"), "Request should be a PlaceOrder message");
        assert!(requests[0].contains(&format!("\0{}\0", order_id)), "Request should contain order ID");
    }

    #[tokio::test]
    async fn test_open_orders() {
        use crate::client::common::tests::setup_open_orders;
        use crate::orders::{Action, Orders};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_open_orders();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request open orders
        let mut subscription = client.open_orders().await.expect("Failed to request open orders");

        // Collect orders from the subscription
        let mut orders = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(Orders::OrderData(order_data)) => {
                    orders.push(order_data);
                }
                Ok(Orders::OrderStatus(_)) => {
                    // Skip order status messages for this test
                }
                Ok(Orders::Notice(_)) => {
                    // Skip notices
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received 2 orders
        assert_eq!(orders.len(), 2, "Should receive 2 open orders");

        // Verify first order (AAPL)
        let order1 = &orders[0];
        assert_eq!(order1.order_id, 1001);
        assert_eq!(order1.contract.symbol, Symbol::from("AAPL"));
        assert_eq!(order1.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(order1.order.action, Action::Buy);
        assert_eq!(order1.order.total_quantity, 100.0);
        assert_eq!(order1.order.order_type, "MKT");
        assert_eq!(order1.order_state.status, "PreSubmitted");

        // Verify second order (MSFT)
        let order2 = &orders[1];
        assert_eq!(order2.order_id, 1002);
        assert_eq!(order2.contract.symbol, Symbol::from("MSFT"));
        assert_eq!(order2.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(order2.order.action, Action::Sell);
        assert_eq!(order2.order.total_quantity, 50.0);
        assert_eq!(order2.order.order_type, "LMT");
        assert_eq!(order2.order.limit_price, Some(350.0));
        assert_eq!(order2.order_state.status, "Submitted");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "5\01\0", "Request should be RequestOpenOrders with version 1");
    }

    #[tokio::test]
    async fn test_all_open_orders() {
        use crate::client::common::tests::setup_all_open_orders;
        use crate::orders::{Action, Orders};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_all_open_orders();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request all open orders
        let mut subscription = client.all_open_orders().await.expect("Failed to request all open orders");

        // Collect orders from the subscription
        let mut orders = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(Orders::OrderData(order_data)) => {
                    orders.push(order_data);
                }
                Ok(Orders::OrderStatus(_)) => {
                    // Skip order status messages for this test
                }
                Ok(Orders::Notice(_)) => {
                    // Skip notices
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received 3 orders (from different clients)
        assert_eq!(orders.len(), 3, "Should receive 3 open orders from all accounts");

        // Verify first order (TSLA from client 101)
        let order1 = &orders[0];
        assert_eq!(order1.order_id, 2001);
        assert_eq!(order1.contract.symbol, Symbol::from("TSLA"));
        assert_eq!(order1.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(order1.order.action, Action::Buy);
        assert_eq!(order1.order.total_quantity, 10.0);
        assert_eq!(order1.order.order_type, "LMT");
        assert_eq!(order1.order.limit_price, Some(420.0));
        assert_eq!(order1.order.account, "DU1236110");

        // Verify second order (AMZN from client 102)
        let order2 = &orders[1];
        assert_eq!(order2.order_id, 2002);
        assert_eq!(order2.contract.symbol, Symbol::from("AMZN"));
        assert_eq!(order2.order.action, Action::Sell);
        assert_eq!(order2.order.total_quantity, 5.0);
        assert_eq!(order2.order.order_type, "MKT");
        assert_eq!(order2.order.account, "DU1236111");

        // Verify third order (GOOGL from current client 100)
        let order3 = &orders[2];
        assert_eq!(order3.order_id, 1003);
        assert_eq!(order3.contract.symbol, Symbol::from("GOOGL"));
        assert_eq!(order3.order.action, Action::Buy);
        assert_eq!(order3.order.total_quantity, 20.0);
        assert_eq!(order3.order.order_type, "LMT");
        assert_eq!(order3.order.limit_price, Some(2800.0));
        assert_eq!(order3.order.account, "DU1236109");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "16\01\0", "Request should be RequestAllOpenOrders with version 1");
    }

    #[tokio::test]
    async fn test_auto_open_orders() {
        use crate::client::common::tests::setup_auto_open_orders;
        use crate::orders::Orders;

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_auto_open_orders();
        // Note: auto_open_orders usually requires client_id 0 for real TWS connections,
        // but for testing we use CLIENT_ID (100) to match the mock gateway expectation
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request auto open orders with auto_bind=true
        let mut subscription = client.auto_open_orders(true).await.expect("Failed to request auto open orders");

        // Collect messages from the subscription
        let mut order_statuses = Vec::new();
        let mut orders = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(Orders::OrderData(order_data)) => {
                    orders.push(order_data);
                }
                Ok(Orders::OrderStatus(status)) => {
                    order_statuses.push(status);
                }
                Ok(Orders::Notice(_)) => {
                    // Skip notices
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received order status updates
        assert_eq!(order_statuses.len(), 2, "Should receive 2 order status updates");

        // Verify first status (PreSubmitted)
        let status1 = &order_statuses[0];
        assert_eq!(status1.order_id, 3001);
        assert_eq!(status1.status, "PreSubmitted");

        // Verify second status (Submitted)
        let status2 = &order_statuses[1];
        assert_eq!(status2.order_id, 3001);
        assert_eq!(status2.status, "Submitted");

        // Verify we received 1 order
        assert_eq!(orders.len(), 1, "Should receive 1 order");

        // Verify the order (FB from TWS)
        let order = &orders[0];
        assert_eq!(order.order_id, 3001);
        assert_eq!(order.contract.symbol, Symbol::from("FB"));
        assert_eq!(order.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(order.order.action, crate::orders::Action::Buy);
        assert_eq!(order.order.total_quantity, 50.0);
        assert_eq!(order.order.order_type, "MKT");
        assert_eq!(order.order.account, "TWS");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(
            requests[0], "15\01\01\0",
            "Request should be RequestAutoOpenOrders with version 1 and auto_bind=true"
        );
    }

    #[tokio::test]
    async fn test_completed_orders() {
        use crate::client::common::tests::setup_completed_orders;
        use crate::orders::{Action, Orders};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_completed_orders();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request completed orders (api_only=false to get all completed orders)
        let mut subscription = client.completed_orders(false).await.expect("Failed to request completed orders");

        // Collect orders from the subscription
        let mut orders = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(Orders::OrderData(order_data)) => {
                    orders.push(order_data);
                }
                Ok(Orders::OrderStatus(_)) => {
                    // Skip order status messages
                }
                Ok(Orders::Notice(_)) => {
                    // Skip notices
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received 2 completed orders
        assert_eq!(orders.len(), 2, "Should receive 2 completed orders");

        // Verify first completed order (ES futures - based on captured data)
        let order1 = &orders[0];
        // CompletedOrder messages don't have order_id in the message, defaults to -1
        assert_eq!(order1.order_id, -1);
        assert_eq!(order1.contract.symbol, Symbol::from("ES"));
        assert_eq!(order1.contract.security_type, crate::contracts::SecurityType::Future);
        assert_eq!(order1.order.action, Action::Buy);
        assert_eq!(order1.order.total_quantity, 1.0);
        assert_eq!(order1.order.order_type, "LMT");
        assert_eq!(order1.order_state.status, "Cancelled");
        assert_eq!(order1.order.perm_id, 616088517);

        // Verify second completed order (AAPL)
        let order2 = &orders[1];
        assert_eq!(order2.order_id, -1); // CompletedOrder messages don't have order_id
        assert_eq!(order2.contract.symbol, Symbol::from("AAPL"));
        assert_eq!(order2.contract.security_type, crate::contracts::SecurityType::Stock);
        assert_eq!(order2.order.action, Action::Buy);
        assert_eq!(order2.order.total_quantity, 100.0);
        assert_eq!(order2.order.order_type, "MKT");
        assert_eq!(order2.order_state.status, "Filled");
        assert_eq!(order2.order.perm_id, 1377295418);

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "99\00\0", "Request should be RequestCompletedOrders with api_only=false");
    }

    #[tokio::test]
    async fn test_cancel_order() {
        use crate::client::common::tests::setup_cancel_order;
        use crate::messages::Notice;
        use crate::orders::CancelOrder;

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_cancel_order();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Cancel order with ID 1001
        let order_id = 1001;
        let manual_order_cancel_time = "";

        // Call cancel_order and get the result
        let result = client.cancel_order(order_id, manual_order_cancel_time).await;

        // Verify the result
        match result {
            Ok(mut cancel_stream) => {
                // Collect results from the stream
                let mut order_status_received = false;
                let mut notice_received = false;

                while let Some(result) = cancel_stream.next().await {
                    match result {
                        Ok(CancelOrder::OrderStatus(status)) => {
                            assert_eq!(status.order_id, order_id);
                            assert_eq!(status.status, "Cancelled");
                            assert_eq!(status.filled, 0.0);
                            assert_eq!(status.remaining, 100.0);
                            order_status_received = true;
                            println!("Received OrderStatus: {:?}", status);
                        }
                        Ok(CancelOrder::Notice(Notice { code, message })) => {
                            // Notice messages with code 202 are order cancellation confirmations
                            // The message should contain the order ID in the format
                            assert_eq!(code, 202);
                            assert!(message.contains("Order Cancelled"));
                            notice_received = true;
                            println!("Received Notice: code={}, message={}", code, message);
                        }
                        Err(e) => panic!("Error in cancel stream: {}", e),
                    }
                }

                assert!(order_status_received, "Should have received OrderStatus");
                assert!(notice_received, "Should have received Notice confirmation");
            }
            Err(e) => panic!("Failed to cancel order: {}", e),
        }

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert!(requests[0].starts_with("4\0"), "Request should be a CancelOrder message");
        assert!(requests[0].contains(&format!("{}\0", order_id)), "Request should contain order ID");
    }

    #[tokio::test]
    async fn test_global_cancel() {
        use crate::client::common::tests::setup_global_cancel;

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_global_cancel();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Call global_cancel
        let result = client.global_cancel().await;

        // Verify the result
        match result {
            Ok(()) => {
                println!("Global cancel request sent successfully");
            }
            Err(e) => panic!("Failed to send global cancel: {}", e),
        }

        // Give the gateway time to process the request
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "58\01\0", "Request should be a RequestGlobalCancel message with version 1");
    }

    #[tokio::test]
    async fn test_executions() {
        use crate::client::common::tests::setup_executions;
        use crate::contracts::SecurityType;
        use crate::orders::{ExecutionFilter, Executions};

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_executions();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create an execution filter
        let filter = ExecutionFilter {
            client_id: Some(CLIENT_ID),
            account_code: "DU1234567".to_string(),
            time: "".to_string(),          // Empty means all time
            symbol: "".to_string(),        // Empty means all symbols
            security_type: "".to_string(), // Empty means all types
            exchange: "".to_string(),      // Empty means all exchanges
            side: "".to_string(),          // Empty means all sides
        };

        // Request executions
        let mut subscription = client.executions(filter).await.expect("Failed to request executions");

        // Collect executions from the subscription
        let mut execution_data = Vec::new();
        let mut commission_reports = Vec::new();

        while let Some(result) = subscription.next().await {
            match result {
                Ok(Executions::ExecutionData(data)) => {
                    execution_data.push(data);
                }
                Ok(Executions::CommissionReport(report)) => {
                    commission_reports.push(report);
                }
                Ok(Executions::Notice(_)) => {
                    // Skip notices
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we received 3 executions and 3 commission reports
        assert_eq!(execution_data.len(), 3, "Should receive 3 execution data messages");
        assert_eq!(commission_reports.len(), 3, "Should receive 3 commission reports");

        // Verify first execution (AAPL stock)
        let exec1 = &execution_data[0];
        assert_eq!(exec1.request_id, 9000);
        assert_eq!(exec1.execution.order_id, 1001);
        assert_eq!(exec1.contract.symbol, Symbol::from("AAPL"));
        assert_eq!(exec1.contract.security_type, SecurityType::Stock);
        assert_eq!(exec1.execution.execution_id, "000e1a2b.67890abc.01.01");
        assert_eq!(exec1.execution.side, "BOT");
        assert_eq!(exec1.execution.shares, 100.0);
        assert_eq!(exec1.execution.price, 150.25);

        // Verify first commission report
        let comm1 = &commission_reports[0];
        assert_eq!(comm1.execution_id, "000e1a2b.67890abc.01.01");
        assert_eq!(comm1.commission, 1.25);
        assert_eq!(comm1.currency, "USD");

        // Verify second execution (ES futures)
        let exec2 = &execution_data[1];
        assert_eq!(exec2.request_id, 9000);
        assert_eq!(exec2.execution.order_id, 1002);
        assert_eq!(exec2.contract.symbol, Symbol::from("ES"));
        assert_eq!(exec2.contract.security_type, SecurityType::Future);
        assert_eq!(exec2.execution.execution_id, "000e1a2b.67890def.02.01");
        assert_eq!(exec2.execution.side, "SLD");
        assert_eq!(exec2.execution.shares, 5.0);
        assert_eq!(exec2.execution.price, 5050.25);

        // Verify second commission report
        let comm2 = &commission_reports[1];
        assert_eq!(comm2.execution_id, "000e1a2b.67890def.02.01");
        assert_eq!(comm2.commission, 2.50);
        assert_eq!(comm2.realized_pnl, Some(125.50));

        // Verify third execution (SPY options)
        let exec3 = &execution_data[2];
        assert_eq!(exec3.request_id, 9000);
        assert_eq!(exec3.execution.order_id, 1003);
        assert_eq!(exec3.contract.symbol, Symbol::from("SPY"));
        assert_eq!(exec3.contract.security_type, SecurityType::Option);
        assert_eq!(exec3.execution.execution_id, "000e1a2b.67890ghi.03.01");
        assert_eq!(exec3.execution.side, "BOT");
        assert_eq!(exec3.execution.shares, 10.0);
        assert_eq!(exec3.execution.price, 2.50);

        // Verify third commission report
        let comm3 = &commission_reports[2];
        assert_eq!(comm3.execution_id, "000e1a2b.67890ghi.03.01");
        assert_eq!(comm3.commission, 0.65);
        assert_eq!(comm3.realized_pnl, Some(250.00));

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        // Request format: RequestExecutions(7), version(3), request_id(9000), client_id, account_code, time, symbol, security_type, exchange, side
        assert_eq!(
            requests[0], "7\03\09000\0100\0DU1234567\0\0\0\0\0\0",
            "Request should be RequestExecutions with correct filter parameters"
        );
    }

    #[tokio::test]
    async fn test_exercise_options() {
        use crate::client::common::tests::setup_exercise_options;
        use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
        use crate::orders::{ExerciseAction, ExerciseOptions};
        use time::macros::datetime;

        // Initialize env_logger for debug output
        let _ = env_logger::try_init();

        let gateway = setup_exercise_options();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create option contract for SPY
        let contract = Contract {
            contract_id: 123456789,
            symbol: Symbol::from("SPY"),
            security_type: SecurityType::Option,
            last_trade_date_or_contract_month: "20240126".to_string(),
            strike: 450.0,
            right: "C".to_string(), // Call option
            multiplier: "100".to_string(),
            exchange: Exchange::from("CBOE"),
            currency: Currency::from("USD"),
            local_symbol: "SPY240126C00450000".to_string(),
            trading_class: "SPY".to_string(),
            ..Default::default()
        };

        // Exercise the option
        let exercise_action = ExerciseAction::Exercise;
        let exercise_quantity = 10;
        let account = "DU1234567";
        let ovrd = false;
        let manual_order_time = Some(datetime!(2024-01-25 10:30:00 UTC));

        let mut subscription = client
            .exercise_options(&contract, exercise_action, exercise_quantity, account, ovrd, manual_order_time)
            .await
            .expect("Failed to exercise options");

        // Collect results
        let mut order_statuses = Vec::new();
        let mut open_orders = Vec::new();

        while let Some(result) = subscription.next().await {
            match result {
                Ok(ExerciseOptions::OrderStatus(status)) => order_statuses.push(status),
                Ok(ExerciseOptions::OpenOrder(order)) => open_orders.push(order),
                Ok(ExerciseOptions::Notice(_notice)) => {
                    // Note: Warning messages (2100-2200) are currently not routed to subscriptions
                    // They are only logged. See TODO.md for future improvements.
                }
                Err(crate::Error::EndOfStream) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Verify we got the expected responses
        assert_eq!(order_statuses.len(), 3, "Should have 3 order status updates");
        assert_eq!(open_orders.len(), 1, "Should have 1 open order");

        // Verify order statuses
        assert_eq!(order_statuses[0].status, "PreSubmitted");
        assert_eq!(order_statuses[0].filled, 0.0);
        assert_eq!(order_statuses[0].remaining, 10.0);

        assert_eq!(order_statuses[1].status, "Submitted");
        assert_eq!(order_statuses[2].status, "Filled");
        assert_eq!(order_statuses[2].filled, 10.0);
        assert_eq!(order_statuses[2].remaining, 0.0);

        // Verify open order
        let open_order = &open_orders[0];
        assert_eq!(open_order.order.order_id, 90);
        assert_eq!(open_order.contract.symbol, Symbol::from("SPY"));
        assert_eq!(open_order.contract.security_type, SecurityType::Option);
        assert_eq!(open_order.order.order_type, "EXERCISE");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");

        // Request format: ExerciseOptions(21), version(2), order_id, contract fields, exercise_action, exercise_quantity, account, ovrd, manual_order_time
        let expected_request = format!(
            "21\02\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0",
            90, // order_id (using next_order_id from client)
            contract.contract_id,
            contract.symbol,
            contract.security_type,
            contract.last_trade_date_or_contract_month,
            contract.strike,
            contract.right,
            contract.multiplier,
            contract.exchange,
            contract.currency,
            contract.local_symbol,
            contract.trading_class,
            exercise_action as i32,
            exercise_quantity,
            account,
            if ovrd { 1 } else { 0 },
            "20240125 10:30:00 UTC" // manual_order_time formatted
        );

        assert_eq!(requests[0], expected_request, "Request should be ExerciseOptions with correct parameters");
    }

    // === Real-time Market Data Tests ===

    #[tokio::test]
    async fn test_market_data() {
        use crate::client::common::tests::setup_market_data;
        use crate::contracts::tick_types::TickType;
        use crate::contracts::Contract;
        use crate::market_data::realtime::TickTypes;

        let gateway = setup_market_data();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let generic_ticks = vec!["100", "101", "104"]; // Option volume, option open interest, historical volatility

        let mut subscription = client
            .market_data(&contract)
            .generic_ticks(&generic_ticks)
            .snapshot()
            .subscribe()
            .await
            .expect("Failed to request market data");

        let mut tick_count = 0;
        let mut has_bid_price = false;
        let mut has_bid_size = false;
        let mut has_ask_price = false;
        let mut has_ask_size = false;
        let mut has_last_price = false;
        let mut has_last_size = false;
        let mut has_volume = false;
        let mut has_snapshot_end = false;

        while let Some(tick_result) = subscription.next().await {
            let tick = tick_result.expect("Failed to get tick");
            tick_count += 1;
            match tick {
                TickTypes::PriceSize(price_size) => {
                    match price_size.price_tick_type {
                        TickType::Bid => {
                            assert_eq!(price_size.price, 150.50);
                            has_bid_price = true;
                        }
                        TickType::Ask => {
                            assert_eq!(price_size.price, 151.00);
                            has_ask_price = true;
                        }
                        TickType::Last => {
                            assert_eq!(price_size.price, 150.75);
                            has_last_price = true;
                        }
                        _ => {}
                    }
                    // Note: size_tick_type might be present but size value is 0 in PriceSize
                }
                TickTypes::Size(size_tick) => match size_tick.tick_type {
                    TickType::BidSize => {
                        assert_eq!(size_tick.size, 100.0);
                        has_bid_size = true;
                    }
                    TickType::AskSize => {
                        assert_eq!(size_tick.size, 200.0);
                        has_ask_size = true;
                    }
                    TickType::LastSize => {
                        assert_eq!(size_tick.size, 50.0);
                        has_last_size = true;
                    }
                    _ => {}
                },
                TickTypes::Generic(generic_tick) => {
                    if generic_tick.tick_type == TickType::Volume {
                        assert_eq!(generic_tick.value, 1500000.0);
                        has_volume = true;
                    }
                }
                TickTypes::String(_) => {
                    // Ignore string ticks like LastTimestamp
                }
                TickTypes::SnapshotEnd => {
                    has_snapshot_end = true;
                    break; // Snapshot complete
                }
                _ => {}
            }

            if tick_count > 20 {
                break; // Safety limit
            }
        }

        assert!(has_bid_price, "Should receive bid price");
        assert!(has_bid_size, "Should receive bid size");
        assert!(has_ask_price, "Should receive ask price");
        assert!(has_ask_size, "Should receive ask size");
        assert!(has_last_price, "Should receive last price");
        assert!(has_last_size, "Should receive last size");
        assert!(has_volume, "Should receive volume");
        assert!(has_snapshot_end, "Should receive snapshot end");

        let requests = gateway.requests();
        assert!(requests[0].starts_with("1\011\09000\0"), "Request should be RequestMarketData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("100,101,104\0"), "Request should contain generic ticks");
        assert!(requests[0].contains("\01\0"), "Request should have snapshot=true");
    }

    #[tokio::test]
    async fn test_realtime_bars() {
        use crate::client::common::tests::setup_realtime_bars;
        use crate::contracts::Contract;
        use crate::market_data::realtime::{BarSize, WhatToShow};

        let gateway = setup_realtime_bars();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let bar_size = BarSize::Sec5;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Extended;

        let mut subscription = client
            .realtime_bars(&contract, bar_size, what_to_show, trading_hours)
            .await
            .expect("Failed to request realtime bars");

        let mut bars = Vec::new();
        for _ in 0..3 {
            if let Some(bar_result) = subscription.next().await {
                bars.push(bar_result.expect("Failed to get bar"));
            }
        }

        assert_eq!(bars.len(), 3, "Should receive 3 bars");

        // Verify first bar
        let bar1 = &bars[0];
        assert_eq!(bar1.open, 150.25);
        assert_eq!(bar1.high, 150.75);
        assert_eq!(bar1.low, 150.00);
        assert_eq!(bar1.close, 150.50);
        assert_eq!(bar1.volume, 1000.0);
        assert_eq!(bar1.wap, 150.40);
        assert_eq!(bar1.count, 25);

        // Verify second bar
        let bar2 = &bars[1];
        assert_eq!(bar2.open, 150.50);
        assert_eq!(bar2.high, 151.00);
        assert_eq!(bar2.low, 150.40);
        assert_eq!(bar2.close, 150.90);
        assert_eq!(bar2.volume, 1200.0);

        // Verify third bar
        let bar3 = &bars[2];
        assert_eq!(bar3.open, 150.90);
        assert_eq!(bar3.high, 151.25);
        assert_eq!(bar3.low, 150.85);
        assert_eq!(bar3.close, 151.20);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("50\08\09000\0"), "Request should be RequestRealTimeBars");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(
            requests[0].contains("\00\0TRADES\00\0"),
            "Request should have bar_size=0 (5 sec) and TRADES"
        );
    }

    #[tokio::test]
    async fn test_tick_by_tick_last() {
        use crate::client::common::tests::setup_tick_by_tick_last;
        use crate::contracts::Contract;

        let gateway = setup_tick_by_tick_last();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let number_of_ticks = 0;
        let ignore_size = false;

        let mut subscription = client
            .tick_by_tick_last(&contract, number_of_ticks, ignore_size)
            .await
            .expect("Failed to request tick by tick last");

        let mut trades = Vec::new();
        for _ in 0..3 {
            if let Some(trade_result) = subscription.next().await {
                trades.push(trade_result.expect("Failed to get trade"));
            }
        }

        assert_eq!(trades.len(), 3, "Should receive 3 trades");

        // Verify first trade
        let trade1 = &trades[0];
        assert_eq!(trade1.tick_type, "1"); // 1 = Last
        assert_eq!(trade1.price, 150.75);
        assert_eq!(trade1.size, 100.0);
        assert_eq!(trade1.exchange, "NASDAQ");
        assert!(!trade1.trade_attribute.past_limit);
        assert!(!trade1.trade_attribute.unreported);

        // Verify second trade (unreported)
        let trade2 = &trades[1];
        assert_eq!(trade2.price, 150.80);
        assert_eq!(trade2.size, 50.0);
        assert_eq!(trade2.exchange, "NYSE");
        assert!(trade2.trade_attribute.unreported);

        // Verify third trade
        let trade3 = &trades[2];
        assert_eq!(trade3.price, 150.70);
        assert_eq!(trade3.size, 150.0);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("97\09000\0"), "Request should be RequestTickByTickData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("Last\0"), "Request should have Last tick type");
    }

    #[tokio::test]
    async fn test_tick_by_tick_all_last() {
        use crate::client::common::tests::setup_tick_by_tick_all_last;
        use crate::contracts::Contract;

        let gateway = setup_tick_by_tick_all_last();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let number_of_ticks = 0;
        let ignore_size = false;

        let mut subscription = client
            .tick_by_tick_all_last(&contract, number_of_ticks, ignore_size)
            .await
            .expect("Failed to request tick by tick all last");

        let mut trades = Vec::new();
        for _ in 0..3 {
            if let Some(trade_result) = subscription.next().await {
                trades.push(trade_result.expect("Failed to get trade"));
            }
        }

        assert_eq!(trades.len(), 3, "Should receive 3 trades");

        // Verify first trade
        let trade1 = &trades[0];
        assert_eq!(trade1.tick_type, "2"); // 2 = AllLast
        assert_eq!(trade1.price, 150.75);
        assert_eq!(trade1.exchange, "NASDAQ");

        // Verify second trade (unreported dark pool trade)
        let trade2 = &trades[1];
        assert_eq!(trade2.price, 150.80);
        assert_eq!(trade2.exchange, "DARK");
        assert_eq!(trade2.special_conditions, "ISO");
        assert!(trade2.trade_attribute.unreported);

        // Verify third trade
        let trade3 = &trades[2];
        assert_eq!(trade3.price, 150.70);
        assert_eq!(trade3.exchange, "NYSE");

        let requests = gateway.requests();
        assert!(requests[0].starts_with("97\09000\0"), "Request should be RequestTickByTickData");
        assert!(requests[0].contains("AllLast\0"), "Request should have AllLast tick type");
    }

    #[tokio::test]
    async fn test_tick_by_tick_bid_ask() {
        use crate::client::common::tests::setup_tick_by_tick_bid_ask;
        use crate::contracts::Contract;

        let gateway = setup_tick_by_tick_bid_ask();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let number_of_ticks = 0;
        let ignore_size = false;

        let mut subscription = client
            .tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size)
            .await
            .expect("Failed to request tick by tick bid ask");

        let mut bid_asks = Vec::new();
        for _ in 0..3 {
            if let Some(bid_ask_result) = subscription.next().await {
                bid_asks.push(bid_ask_result.expect("Failed to get bid/ask"));
            }
        }

        assert_eq!(bid_asks.len(), 3, "Should receive 3 bid/ask updates");

        // Verify first bid/ask
        let ba1 = &bid_asks[0];
        assert_eq!(ba1.bid_price, 150.50);
        assert_eq!(ba1.ask_price, 150.55);
        assert_eq!(ba1.bid_size, 100.0);
        assert_eq!(ba1.ask_size, 200.0);
        assert!(!ba1.bid_ask_attribute.bid_past_low);
        assert!(!ba1.bid_ask_attribute.ask_past_high);

        // Verify second bid/ask (bid past low)
        let ba2 = &bid_asks[1];
        assert_eq!(ba2.bid_price, 150.45);
        assert_eq!(ba2.ask_price, 150.55);
        assert!(ba2.bid_ask_attribute.bid_past_low);

        // Verify third bid/ask (ask past high)
        let ba3 = &bid_asks[2];
        assert_eq!(ba3.ask_price, 150.60);
        assert!(ba3.bid_ask_attribute.ask_past_high);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("97\09000\0"), "Request should be RequestTickByTickData");
        assert!(requests[0].contains("BidAsk\0"), "Request should have BidAsk tick type");
    }

    #[tokio::test]
    async fn test_tick_by_tick_midpoint() {
        use crate::client::common::tests::setup_tick_by_tick_midpoint;
        use crate::contracts::Contract;

        let gateway = setup_tick_by_tick_midpoint();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let number_of_ticks = 0;
        let ignore_size = false;

        let mut subscription = client
            .tick_by_tick_midpoint(&contract, number_of_ticks, ignore_size)
            .await
            .expect("Failed to request tick by tick midpoint");

        let mut midpoints = Vec::new();
        for _ in 0..3 {
            if let Some(midpoint_result) = subscription.next().await {
                midpoints.push(midpoint_result.expect("Failed to get midpoint"));
            }
        }

        assert_eq!(midpoints.len(), 3, "Should receive 3 midpoint updates");

        assert_eq!(midpoints[0].mid_point, 150.525);
        assert_eq!(midpoints[1].mid_point, 150.50);
        assert_eq!(midpoints[2].mid_point, 150.525);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("97\09000\0"), "Request should be RequestTickByTickData");
        assert!(requests[0].contains("MidPoint\0"), "Request should have MidPoint tick type");
    }

    #[tokio::test]
    async fn test_market_depth() {
        use crate::client::common::tests::setup_market_depth;
        use crate::contracts::Contract;
        use crate::market_data::realtime::MarketDepths;

        let gateway = setup_market_depth();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let num_rows = 5;
        let is_smart_depth = false;

        let mut subscription = client
            .market_depth(&contract, num_rows, is_smart_depth)
            .await
            .expect("Failed to request market depth");

        let mut updates = Vec::new();
        for _ in 0..4 {
            if let Some(update_result) = subscription.next().await {
                let update = update_result.expect("Failed to get depth update");
                if let MarketDepths::MarketDepth(depth) = update {
                    updates.push(depth);
                }
            }
        }

        assert_eq!(updates.len(), 4, "Should receive 4 depth updates");

        // Verify bid insert
        let update1 = &updates[0];
        assert_eq!(update1.position, 0);
        // MarketDepth (L1) doesn't have market_maker field
        assert_eq!(update1.operation, 0); // Insert
        assert_eq!(update1.side, 1); // Bid
        assert_eq!(update1.price, 150.50);
        assert_eq!(update1.size, 100.0);

        // Verify ask insert
        let update2 = &updates[1];
        assert_eq!(update2.operation, 0); // Insert
        assert_eq!(update2.side, 0); // Ask
        assert_eq!(update2.price, 150.55);
        assert_eq!(update2.size, 200.0);

        // Verify bid update
        let update3 = &updates[2];
        assert_eq!(update3.operation, 1); // Update
        assert_eq!(update3.price, 150.49);

        // Verify ask delete
        let update4 = &updates[3];
        assert_eq!(update4.operation, 2); // Delete

        let requests = gateway.requests();
        assert!(requests[0].starts_with("10\05\09000\0"), "Request should be RequestMarketDepth");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("5\00\0"), "Request should have 5 rows and smart_depth=false");
    }

    #[tokio::test]
    async fn test_market_depth_exchanges() {
        use crate::client::common::tests::setup_market_depth_exchanges;

        let gateway = setup_market_depth_exchanges();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let exchanges = client.market_depth_exchanges().await.expect("Failed to get market depth exchanges");

        assert_eq!(exchanges.len(), 3, "Should receive 3 exchange descriptions");

        // Verify first exchange
        let ex1 = &exchanges[0];
        assert_eq!(ex1.exchange_name, "ISLAND");
        assert_eq!(ex1.security_type, "STK");
        assert_eq!(ex1.listing_exchange, "NASDAQ");
        assert_eq!(ex1.service_data_type, "Deep2");
        assert_eq!(ex1.aggregated_group, Some("1".to_string()));

        // Verify second exchange
        let ex2 = &exchanges[1];
        assert_eq!(ex2.exchange_name, "NYSE");
        assert_eq!(ex2.security_type, "STK");
        assert_eq!(ex2.service_data_type, "Deep");
        assert_eq!(ex2.aggregated_group, Some("2".to_string()));

        // Verify third exchange
        let ex3 = &exchanges[2];
        assert_eq!(ex3.exchange_name, "ARCA");
        assert_eq!(ex3.aggregated_group, Some("2".to_string()));

        let requests = gateway.requests();
        assert_eq!(requests[0], "82\0", "Request should be RequestMktDepthExchanges");
    }

    #[tokio::test]
    async fn test_switch_market_data_type() {
        use crate::client::common::tests::setup_switch_market_data_type;
        use crate::market_data::MarketDataType;

        let gateway = setup_switch_market_data_type();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Test switching to delayed market data
        client
            .switch_market_data_type(MarketDataType::Delayed)
            .await
            .expect("Failed to switch market data type");

        // Give the mock gateway time to receive the request
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        // Verify request format: RequestMarketDataType(59), version(1), market_data_type(3=Delayed)
        assert_eq!(requests[0], "59\01\03\0", "Request should be RequestMarketDataType with Delayed(3)");
    }

    // === Historical Data Tests ===

    #[tokio::test]
    async fn test_head_timestamp() {
        use crate::client::common::tests::setup_head_timestamp;
        use crate::contracts::Contract;
        use crate::market_data::historical::WhatToShow;

        let gateway = setup_head_timestamp();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let timestamp = client
            .head_timestamp(&contract, what_to_show, trading_hours)
            .await
            .expect("Failed to get head timestamp");

        // Verify the timestamp is as expected (2024-01-15 09:30:00)
        assert_eq!(timestamp.year(), 2024);
        assert_eq!(timestamp.month() as u8, 1);
        assert_eq!(timestamp.day(), 15);
        assert_eq!(timestamp.hour(), 9);
        assert_eq!(timestamp.minute(), 30);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("87\0"), "Request should be RequestHeadTimestamp");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("TRADES\0"), "Request should contain TRADES");
    }

    #[tokio::test]
    async fn test_historical_data() {
        use crate::client::common::tests::setup_historical_data;
        use crate::contracts::Contract;
        use crate::market_data::historical::{BarSize, Duration, WhatToShow};
        use time::macros::datetime;

        let gateway = setup_historical_data();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let end_date_time = datetime!(2024-01-22 16:00:00).assume_utc();
        let duration = Duration::days(1);
        let bar_size = BarSize::Min5;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let historical_data = client
            .historical_data(&contract, Some(end_date_time), duration, bar_size, Some(what_to_show), trading_hours)
            .await
            .expect("Failed to get historical data");

        // Get bars from HistoricalData struct
        let bars = &historical_data.bars;

        assert_eq!(bars.len(), 3, "Should receive 3 bars");

        // Verify first bar
        assert_eq!(bars[0].open, 150.25);
        assert_eq!(bars[0].high, 150.75);
        assert_eq!(bars[0].low, 150.00);
        assert_eq!(bars[0].close, 150.50);
        assert_eq!(bars[0].volume, 1000.0);
        assert_eq!(bars[0].wap, 150.40);
        assert_eq!(bars[0].count, 25);

        // Verify second bar
        assert_eq!(bars[1].open, 150.50);
        assert_eq!(bars[1].high, 151.00);
        assert_eq!(bars[1].low, 150.40);
        assert_eq!(bars[1].close, 150.90);
        assert_eq!(bars[1].volume, 1200.0);

        // Verify third bar
        assert_eq!(bars[2].open, 150.90);
        assert_eq!(bars[2].high, 151.25);
        assert_eq!(bars[2].low, 150.85);
        assert_eq!(bars[2].close, 151.20);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("20\0"), "Request should be RequestHistoricalData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
    }

    #[tokio::test]
    async fn test_historical_schedule() {
        use crate::client::common::tests::setup_historical_schedules;
        use crate::contracts::Contract;
        use crate::market_data::historical::Duration;
        use time::macros::datetime;

        let gateway = setup_historical_schedules();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let end_date_time = datetime!(2024-01-22 16:00:00).assume_utc();
        let duration = Duration::days(1);

        let schedule = client
            .historical_schedule(&contract, Some(end_date_time), duration)
            .await
            .expect("Failed to get historical schedule");

        // Schedule has start and end as OffsetDateTime
        assert_eq!(schedule.time_zone, "US/Eastern");
        assert!(!schedule.sessions.is_empty(), "Should have at least one session");

        let requests = gateway.requests();
        assert!(requests[0].starts_with("20\0"), "Request should be RequestHistoricalData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("2\0"), "Request should contain formatDate=2 for schedule");
    }

    #[tokio::test]
    async fn test_historical_ticks_bid_ask() {
        use crate::client::common::tests::setup_historical_ticks_bid_ask;
        use crate::contracts::Contract;
        use time::macros::datetime;

        let gateway = setup_historical_ticks_bid_ask();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let start_date_time = datetime!(2024-01-22 09:30:00).assume_utc();
        let number_of_ticks = 100;
        let trading_hours = TradingHours::Regular;

        let mut tick_subscription = client
            .historical_ticks_bid_ask(&contract, Some(start_date_time), None, number_of_ticks, trading_hours, false)
            .await
            .expect("Failed to get historical ticks bid/ask");

        // Collect ticks from the subscription
        let mut ticks = Vec::new();
        while let Some(tick) = tick_subscription.next().await {
            ticks.push(tick);
        }

        assert_eq!(ticks.len(), 3, "Should receive 3 ticks");

        // Verify first tick
        assert_eq!(ticks[0].price_bid, 150.25);
        assert_eq!(ticks[0].price_ask, 150.50);
        assert_eq!(ticks[0].size_bid, 100);
        assert_eq!(ticks[0].size_ask, 200);

        // Verify second tick
        assert_eq!(ticks[1].price_bid, 150.30);
        assert_eq!(ticks[1].price_ask, 150.55);
        assert_eq!(ticks[1].size_bid, 150);
        assert_eq!(ticks[1].size_ask, 250);

        // Verify third tick
        assert_eq!(ticks[2].price_bid, 150.35);
        assert_eq!(ticks[2].price_ask, 150.60);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("96\0"), "Request should be RequestHistoricalTicks");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(requests[0].contains("BID_ASK\0"), "Request should contain BID_ASK");
    }

    #[tokio::test]
    async fn test_historical_ticks_mid_point() {
        use crate::client::common::tests::setup_historical_ticks_mid_point;
        use crate::contracts::Contract;
        use time::macros::datetime;

        let gateway = setup_historical_ticks_mid_point();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let start_date_time = datetime!(2024-01-22 09:30:00).assume_utc();
        let number_of_ticks = 100;
        let trading_hours = TradingHours::Regular;

        let mut tick_subscription = client
            .historical_ticks_mid_point(&contract, Some(start_date_time), None, number_of_ticks, trading_hours)
            .await
            .expect("Failed to get historical ticks midpoint");

        // Collect ticks from the subscription
        let mut ticks = Vec::new();
        while let Some(tick) = tick_subscription.next().await {
            ticks.push(tick);
        }

        assert_eq!(ticks.len(), 3, "Should receive 3 ticks");

        // Verify ticks
        assert_eq!(ticks[0].price, 150.375);
        assert_eq!(ticks[0].size, 0);
        assert_eq!(ticks[1].price, 150.425);
        assert_eq!(ticks[1].size, 0);
        assert_eq!(ticks[2].price, 150.475);
        assert_eq!(ticks[2].size, 0);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("96\0"), "Request should be RequestHistoricalTicks");
        assert!(requests[0].contains("MIDPOINT\0"), "Request should contain MIDPOINT");
    }

    #[tokio::test]
    async fn test_historical_ticks_trade() {
        use crate::client::common::tests::setup_historical_ticks_trade;
        use crate::contracts::Contract;
        use time::macros::datetime;

        let gateway = setup_historical_ticks_trade();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let start_date_time = datetime!(2024-01-22 09:30:00).assume_utc();
        let number_of_ticks = 100;
        let trading_hours = TradingHours::Regular;

        let mut tick_subscription = client
            .historical_ticks_trade(&contract, Some(start_date_time), None, number_of_ticks, trading_hours)
            .await
            .expect("Failed to get historical ticks trade");

        // Collect ticks from the subscription
        let mut ticks = Vec::new();
        while let Some(tick) = tick_subscription.next().await {
            ticks.push(tick);
        }

        assert_eq!(ticks.len(), 3, "Should receive 3 ticks");

        // Verify ticks
        assert_eq!(ticks[0].price, 150.50);
        assert_eq!(ticks[0].size, 100);
        assert_eq!(ticks[0].exchange, "NASDAQ");
        assert_eq!(ticks[0].special_conditions, "T");

        assert_eq!(ticks[1].price, 150.55);
        assert_eq!(ticks[1].size, 200);
        assert_eq!(ticks[1].exchange, "NYSE");

        assert_eq!(ticks[2].price, 150.60);
        assert_eq!(ticks[2].size, 150);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("96\0"), "Request should be RequestHistoricalTicks");
        assert!(requests[0].contains("TRADES\0"), "Request should contain TRADES");
    }

    #[tokio::test]
    async fn test_histogram_data() {
        use crate::client::common::tests::setup_histogram_data;
        use crate::contracts::Contract;
        use crate::market_data::historical::BarSize;

        let gateway = setup_histogram_data();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let contract = Contract::stock("AAPL").build();
        let trading_hours = TradingHours::Regular;
        let period = BarSize::Day;

        let entries = client
            .histogram_data(&contract, trading_hours, period)
            .await
            .expect("Failed to get histogram data");

        assert_eq!(entries.len(), 3, "Should receive 3 entries");

        // Verify entries
        assert_eq!(entries[0].price, 150.00);
        assert_eq!(entries[0].size, 1000);

        assert_eq!(entries[1].price, 150.50);
        assert_eq!(entries[1].size, 1500);

        assert_eq!(entries[2].price, 151.00);
        assert_eq!(entries[2].size, 800);

        let requests = gateway.requests();
        assert!(requests[0].starts_with("88\0"), "Request should be RequestHistogramData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
    }

    // === News Tests ===

    #[tokio::test]
    async fn test_news_providers() {
        use crate::client::common::tests::setup_news_providers;

        let gateway = setup_news_providers();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request news providers
        let providers = client.news_providers().await.expect("Failed to get news providers");

        // Verify we received 3 providers
        assert_eq!(providers.len(), 3, "Should receive 3 news providers");

        // Verify provider details
        assert_eq!(providers[0].code, "BRFG");
        assert_eq!(providers[0].name, "Briefing.com General Market Columns");

        assert_eq!(providers[1].code, "BRFUPDN");
        assert_eq!(providers[1].name, "Briefing.com Analyst Actions");

        assert_eq!(providers[2].code, "DJ-RT");
        assert_eq!(providers[2].name, "Dow Jones Real-Time News");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "85\0", "Request should be RequestNewsProviders");
    }

    #[tokio::test]
    async fn test_news_bulletins() {
        use crate::client::common::tests::setup_news_bulletins;

        let gateway = setup_news_bulletins();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request news bulletins with all_messages=true
        let mut subscription = client.news_bulletins(true).await.expect("Failed to get news bulletins");

        // Collect news bulletins
        let mut bulletins = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(b) => bulletins.push(b),
                Err(_) => break,
            }
            if bulletins.len() >= 2 {
                break; // We expect 2 bulletins
            }
        }

        // Verify we received 2 bulletins
        assert_eq!(bulletins.len(), 2, "Should receive 2 news bulletins");

        // Verify bulletin details
        assert_eq!(bulletins[0].message_id, 123);
        assert_eq!(bulletins[0].message_type, 1);
        assert_eq!(bulletins[0].message, "Important market announcement");
        assert_eq!(bulletins[0].exchange, "NYSE");

        assert_eq!(bulletins[1].message_id, 124);
        assert_eq!(bulletins[1].message_type, 2);
        assert_eq!(bulletins[1].message, "Trading halt on symbol XYZ");
        assert_eq!(bulletins[1].exchange, "NASDAQ");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(
            requests[0].starts_with("12\01\01\0"),
            "Request should be RequestNewsBulletins with version 1 and all_messages=true"
        );
    }

    #[tokio::test]
    async fn test_historical_news() {
        use crate::client::common::tests::setup_historical_news;
        use time::macros::datetime;

        let gateway = setup_historical_news();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request historical news
        let start_time = datetime!(2024-01-15 14:00:00 UTC);
        let end_time = datetime!(2024-01-15 15:00:00 UTC);
        let mut subscription = client
            .historical_news(
                1234,               // contract_id
                &["DJ-RT", "BRFG"], // provider_codes
                start_time,
                end_time,
                10, // total_results
            )
            .await
            .expect("Failed to get historical news");

        // Collect news articles
        let mut articles = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(a) => articles.push(a),
                Err(_) => break,
            }
            if articles.len() >= 2 {
                break; // We expect 2 articles
            }
        }

        // Verify we received 2 articles
        assert_eq!(articles.len(), 2, "Should receive 2 news articles");

        // Verify article details
        assert_eq!(articles[0].provider_code, "DJ-RT");
        assert_eq!(articles[0].article_id, "DJ001234");
        assert_eq!(articles[0].headline, "Market hits new highs amid positive earnings");

        assert_eq!(articles[1].provider_code, "BRFG");
        assert_eq!(articles[1].article_id, "BRF5678");
        assert_eq!(articles[1].headline, "Federal Reserve announces policy decision");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("86\0"), "Request should be RequestHistoricalNews");
        assert!(requests[0].contains("1234\0"), "Request should contain contract_id 1234");
        assert!(requests[0].contains("DJ-RT+BRFG\0"), "Request should contain provider codes");
    }

    #[tokio::test]
    async fn test_news_article() {
        use crate::client::common::tests::setup_news_article;

        let gateway = setup_news_article();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request news article
        let article = client
            .news_article(
                "DJ-RT",    // provider_code
                "DJ001234", // article_id
            )
            .await
            .expect("Failed to get news article");

        // Verify article details
        assert_eq!(article.article_type, crate::news::ArticleType::Text);
        assert_eq!(
            article.article_text,
            "This is the full text of the news article. It contains detailed information about the market event described in the headline."
        );

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("84\0"), "Request should be RequestNewsArticle");
        assert!(requests[0].contains("DJ-RT\0"), "Request should contain provider code");
        assert!(requests[0].contains("DJ001234\0"), "Request should contain article ID");
    }

    #[tokio::test]
    async fn test_scanner_parameters() {
        use crate::client::common::tests::setup_scanner_parameters;

        let gateway = setup_scanner_parameters();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request scanner parameters
        let xml = client.scanner_parameters().await.expect("Failed to get scanner parameters");

        // Verify we received XML content
        assert!(xml.contains("<ScanParameterResponse>"), "Should contain ScanParameterResponse");
        assert!(xml.contains("<Instrument>STK</Instrument>"), "Should contain STK instrument");
        assert!(xml.contains("<Instrument>OPT</Instrument>"), "Should contain OPT instrument");
        assert!(xml.contains("<Location>US</Location>"), "Should contain US location");
        assert!(
            xml.contains("<ScanType>TOP_PERC_GAIN</ScanType>"),
            "Should contain TOP_PERC_GAIN scan type"
        );

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert_eq!(requests.len(), 1, "Should have sent 1 request");
        assert_eq!(requests[0], "24\01\0", "Request should be RequestScannerParameters with version 1");
    }

    #[tokio::test]
    async fn test_scanner_subscription() {
        use crate::client::common::tests::setup_scanner_subscription;
        use crate::scanner::ScannerSubscription;

        let gateway = setup_scanner_subscription();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create scanner subscription parameters
        let scanner_subscription = ScannerSubscription {
            instrument: Some("STK".to_string()),
            location_code: Some("STK.US.MAJOR".to_string()),
            scan_code: Some("TOP_PERC_GAIN".to_string()),
            number_of_rows: 10,
            ..Default::default()
        };

        // Request scanner subscription
        let mut subscription = client
            .scanner_subscription(&scanner_subscription, &vec![])
            .await
            .expect("Failed to get scanner subscription");

        // Collect scanner data - subscription yields Vec<ScannerData>, not individual items
        let mut scan_data_vecs = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(d) => scan_data_vecs.push(d),
                Err(_) => break,
            }
            if !scan_data_vecs.is_empty() {
                break; // We expect 1 batch
            }
        }

        assert_eq!(scan_data_vecs.len(), 1, "Should receive 1 batch of scan data");
        let scan_data = &scan_data_vecs[0];

        // Verify we received 2 scan items
        assert_eq!(scan_data.len(), 2, "Should receive 2 scan data items");

        // Verify scan data details
        assert_eq!(scan_data[0].rank, 1);
        assert_eq!(scan_data[0].contract_details.contract.contract_id, 1234);
        assert_eq!(scan_data[0].contract_details.contract.symbol, Symbol::from("AAPL"));

        assert_eq!(scan_data[1].rank, 2);
        assert_eq!(scan_data[1].contract_details.contract.contract_id, 5678);
        assert_eq!(scan_data[1].contract_details.contract.symbol, Symbol::from("GOOGL"));

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("22\0"), "Request should be RequestScannerSubscription");
    }

    #[tokio::test]
    async fn test_wsh_metadata() {
        use crate::client::common::tests::setup_wsh_metadata;

        let gateway = setup_wsh_metadata();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request WSH metadata
        let metadata = client.wsh_metadata().await.expect("Failed to get WSH metadata");

        // Verify metadata
        assert_eq!(metadata.data_json, "{\"dataJson\":\"sample_metadata\"}");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("100\0"), "Request should be RequestWshMetaData");
    }

    #[tokio::test]
    async fn test_wsh_event_data() {
        use crate::client::common::tests::setup_wsh_event_data;

        let gateway = setup_wsh_event_data();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request WSH event data by contract_id - returns a single WshEventData
        let event_data = client
            .wsh_event_data_by_contract(1234, None, None, None, None)
            .await
            .expect("Failed to get WSH event data");

        // Verify we received the event data (only the first message is processed)
        assert_eq!(event_data.data_json, "{\"dataJson\":\"event1\"}");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("102\0"), "Request should be RequestWshEventData");
    }

    #[tokio::test]
    async fn test_contract_news() {
        use crate::client::common::tests::setup_contract_news;
        use crate::contracts::Contract;

        let gateway = setup_contract_news();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Create a contract for the request
        let contract = Contract::stock("AAPL").build();
        let provider_codes = &["DJ-RT", "BRFG"];

        // Request contract news
        let mut subscription = client
            .contract_news(&contract, provider_codes)
            .await
            .expect("Failed to get contract news");

        // Collect news articles
        let mut articles = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(a) => articles.push(a),
                Err(_) => break,
            }
            if articles.len() >= 2 {
                break; // We expect 2 articles
            }
        }

        // Verify we received 2 articles
        assert_eq!(articles.len(), 2, "Should receive 2 news articles");

        // Verify article details
        assert_eq!(articles[0].provider_code, "DJ-RT");
        assert_eq!(articles[0].article_id, "DJ001234");
        assert_eq!(articles[0].headline, "Stock rises on earnings beat");
        assert_eq!(articles[0].extra_data, "extraData1");

        assert_eq!(articles[1].provider_code, "BRFG");
        assert_eq!(articles[1].article_id, "BRF5678");
        assert_eq!(articles[1].headline, "Company announces expansion");
        assert_eq!(articles[1].extra_data, "extraData2");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("1\0"), "Request should be RequestMarketData");
        assert!(requests[0].contains("AAPL\0STK\0"), "Request should contain AAPL stock");
        assert!(
            requests[0].contains("mdoff,292:DJ-RT,292:BRFG\0"),
            "Request should contain news generic ticks"
        );
    }

    #[tokio::test]
    async fn test_broad_tape_news() {
        use crate::client::common::tests::setup_broad_tape_news;

        let gateway = setup_broad_tape_news();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request broad tape news
        let mut subscription = client.broad_tape_news("BRFG").await.expect("Failed to get broad tape news");

        // Collect news articles
        let mut articles = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(a) => articles.push(a),
                Err(_) => break,
            }
            if articles.len() >= 2 {
                break; // We expect 2 articles
            }
        }

        // Verify we received 2 articles
        assert_eq!(articles.len(), 2, "Should receive 2 news articles");

        // Verify article details
        assert_eq!(articles[0].provider_code, "BRFG");
        assert_eq!(articles[0].article_id, "BRF001");
        assert_eq!(articles[0].headline, "Market update: Tech sector rallies");
        assert_eq!(articles[0].extra_data, "extraData1");

        assert_eq!(articles[1].provider_code, "BRFG");
        assert_eq!(articles[1].article_id, "BRF002");
        assert_eq!(articles[1].headline, "Fed minutes released");
        assert_eq!(articles[1].extra_data, "extraData2");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("1\0"), "Request should be RequestMarketData");

        // Debug: print the actual request to understand the format
        if !requests[0].contains("BRFG") || !requests[0].contains("NEWS") {
            eprintln!("Actual request: {:?}", requests[0]);
        }

        // Check for the contract components (symbol, sec_type, exchange)
        assert!(requests[0].contains("BRFG:BRFG_ALL"), "Request should contain BRFG:BRFG_ALL symbol");
        assert!(requests[0].contains("NEWS"), "Request should contain NEWS security type");
        assert!(requests[0].contains("mdoff,292\0"), "Request should contain news generic ticks");
    }

    #[tokio::test]
    async fn test_wsh_event_data_by_filter() {
        use crate::client::common::tests::setup_wsh_event_data_by_filter;

        let gateway = setup_wsh_event_data_by_filter();
        let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        // Request WSH event data by filter (no limit param to avoid version check)
        let filter = "{\"watchlist\":[\"AAPL\"],\"country\":\"ALL\"}";
        let mut subscription = client
            .wsh_event_data_by_filter(filter, None, None)
            .await
            .expect("Failed to get WSH event data by filter");

        // Collect events
        let mut events = Vec::new();
        while let Some(result) = subscription.next().await {
            match result {
                Ok(e) => events.push(e),
                Err(_) => break,
            }
            if events.len() >= 2 {
                break; // We expect 2 events
            }
        }

        // Verify we received 2 events
        assert_eq!(events.len(), 2, "Should receive 2 WSH events");
        assert_eq!(events[0].data_json, "{\"dataJson\":\"filtered_event1\"}");
        assert_eq!(events[1].data_json, "{\"dataJson\":\"filtered_event2\"}");

        // Verify the request was sent correctly
        let requests = gateway.requests();
        assert!(requests[0].starts_with("102\0"), "Request should be RequestWshEventData");
        assert!(requests[0].contains(filter), "Request should contain the filter");
    }
}
