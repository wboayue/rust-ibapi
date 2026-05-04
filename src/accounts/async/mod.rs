//! Asynchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::{decoders, encoders};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

// DataStream implementations are now in common/stream_decoders.rs

impl Client {
    /// Subscribe to streaming position updates for all accessible accounts.
    ///
    /// The stream first replays the full position list and then sends incremental updates.
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
    ///     while let Some(position_response) = subscription.next_data().await {
    ///         match position_response {
    ///             Ok(PositionUpdate::Position(position)) => println!("{position:?}"),
    ///             Ok(PositionUpdate::PositionEnd) => println!("initial set of positions received"),
    ///             Err(e) => eprintln!("Error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        crate::common::request_helpers::shared_subscription(
            self,
            Features::POSITIONS,
            OutgoingMessages::RequestPositions,
            encoders::encode_request_positions,
        )
        .await
    }

    /// Subscribe to streaming position updates scoped by account and model code.
    ///
    /// Requires [Features::MODELS_SUPPORT] to be available on the connected gateway.
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
        check_version(self.server_version(), Features::MODELS_SUPPORT)?;

        let builder = self.request();
        let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

        builder.send::<PositionUpdateMulti>(request).await
    }

    /// Fetch the account family codes registered with the broker.
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
        crate::common::request_helpers::one_shot_request(
            self,
            Features::FAMILY_CODES,
            OutgoingMessages::RequestFamilyCodes,
            encoders::encode_request_family_codes,
            decoders::decode_family_codes,
            Vec::default,
        )
        .await
    }

    /// Subscribe to real-time daily and unrealized PnL updates for an account.
    ///
    /// Optionally filter by model code to scope the updates.
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
        crate::common::request_helpers::request_with_id(self, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code)).await
    }

    /// Subscribe to real-time daily PnL updates for a single contract.
    ///
    /// The stream includes realized and unrealized PnL information for the requested position.
    ///
    /// # Arguments
    /// * `account`     - Account in which position exists
    /// * `contract_id` - Contract ID of contract to receive daily PnL updates for.
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
        crate::common::request_helpers::request_with_id(self, Features::REALIZED_PNL, |id| {
            encoders::encode_request_pnl_single(id, account, contract_id, model_code)
        })
        .await
    }

    /// Subscribe to account summary updates for a group of accounts.
    ///
    /// # Arguments
    /// * `group` - Set to "All" to return account summary data for all accounts, or set to a specific Advisor Account Group name.
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
        crate::common::request_helpers::request_with_id(self, Features::ACCOUNT_SUMMARY, |id| {
            encoders::encode_request_account_summary(id, group, tags)
        })
        .await
    }

    /// Subscribe to detailed account updates for a specific account.
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
    ///     while let Some(update_result) = subscription.next_data().await {
    ///         match update_result {
    ///             Ok(update) => {
    ///                 println!("{update:?}");
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
        crate::common::request_helpers::shared_request(self, OutgoingMessages::RequestAccountData, || {
            encoders::encode_request_account_updates(true, account)
        })
        .await
    }

    /// Subscribe to account updates scoped by account and model code.
    ///
    /// Requires [Features::MODELS_SUPPORT] to be available on the connected gateway.
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
    ///     while let Some(update_result) = subscription.next_data().await {
    ///         match update_result {
    ///             Ok(update) => {
    ///                 println!("{update:?}");
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
        check_version(self.server_version(), Features::MODELS_SUPPORT)?;

        let builder = self.request();
        let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code)?;

        builder.send::<AccountUpdateMulti>(request).await
    }

    /// Fetch the list of accounts accessible to the current user.
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
        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::RequestManagedAccounts,
            encoders::encode_request_managed_accounts,
            |message| {
                message.skip(); // message type
                message.skip(); // message version
                let accounts = message.next_string()?;
                Ok(accounts.split(',').filter(|s| !s.is_empty()).map(String::from).collect())
            },
            || Ok(Vec::default()),
        )
        .await
    }

    /// Query the current server time reported by TWS or IB Gateway.
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
        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::RequestCurrentTime,
            encoders::encode_request_server_time,
            decoders::decode_server_time,
            || Err(Error::Simple("No response from server".to_string())),
        )
        .await
    }

    /// Query the current server time in milliseconds reported by TWS or IB Gateway.
    pub async fn server_time_millis(&self) -> Result<OffsetDateTime, Error> {
        check_version(self.server_version(), Features::CURRENT_TIME_IN_MILLIS)?;

        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::RequestCurrentTimeInMillis,
            encoders::encode_request_server_time_millis,
            decoders::decode_server_time_millis,
            || Err(Error::Simple("No response from server".to_string())),
        )
        .await
    }
}

#[cfg(test)]
mod tests;
