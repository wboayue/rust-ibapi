//! Asynchronous client implementation

use std::sync::Arc;
use std::time::Duration;

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
use crate::accounts::{AccountSummaries, AccountUpdate, AccountUpdateMulti, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
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

    /// Returns the next order ID
    pub fn next_order_id(&self) -> i32 {
        self.id_manager.next_order_id()
    }

    /// Returns the next request ID
    pub(crate) fn next_request_id(&self) -> i32 {
        self.id_manager.next_request_id()
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
        // First subscribe to the response channel
        let subscription = self.message_bus.subscribe(request_id).await;

        // Then send the request
        self.message_bus.send_request(message).await?;

        Ok(subscription)
    }

    /// Send a shared request (no ID)
    pub async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // First subscribe to the shared channel
        let subscription = self.message_bus.subscribe_shared(message_type).await;

        // Then send the request
        self.message_bus.send_request(message).await?;

        Ok(subscription)
    }

    /// Send an order request
    pub async fn send_order(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // First subscribe to the order channel
        let subscription = self.message_bus.subscribe_order(order_id).await;

        // Then send the request
        self.message_bus.send_request(message).await?;

        Ok(subscription)
    }

    /// Send a message without expecting a response
    pub async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
        self.message_bus.send_request(message).await
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = "U1234567";
    ///     let mut subscription = client.positions_multi(Some(account), None).await.expect("error requesting positions by model");
    ///     
    ///     while let Some(position) = subscription.next().await {
    ///         println!("{position:?}")
    ///     }
    /// }
    /// ```
    pub async fn positions_multi(&self, account: Option<&str>, model_code: Option<&str>) -> Result<Subscription<PositionUpdateMulti>, Error> {
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let account = "account id";
    ///     let mut subscription = client.pnl(account, None).await.expect("error requesting pnl");
    ///     
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl(&self, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = "<account id>";
    ///     let contract_id = 1001;
    ///
    ///     let mut subscription = client.pnl_single(account, contract_id, None).await.expect("error requesting pnl");
    ///     
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl_single(&self, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<Subscription<PnLSingle>, Error> {
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let group = "All";
    ///
    ///     let mut subscription = client.account_summary(group, AccountSummaryTags::ALL).await.expect("error requesting account summary");
    ///     
    ///     while let Some(summary) = subscription.next().await {
    ///         println!("{summary:?}")
    ///     }
    /// }
    /// ```
    pub async fn account_summary(&self, group: &str, tags: &[&str]) -> Result<Subscription<AccountSummaries>, Error> {
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = "U1234567";
    ///
    ///     let mut subscription = client.account_updates(account).await.expect("error requesting account updates");
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
    pub async fn account_updates(&self, account: &str) -> Result<Subscription<AccountUpdate>, Error> {
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let account = Some("U1234567");
    ///
    ///     let mut subscription = client.account_updates_multi(account, None).await.expect("error requesting account updates multi");
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
    pub async fn account_updates_multi(&self, account: Option<&str>, model_code: Option<&str>) -> Result<Subscription<AccountUpdateMulti>, Error> {
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
