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
    /// use ibapi::subscriptions::SubscriptionItem;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut subscription = client.positions().await.expect("error requesting positions");
    ///
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(PositionUpdate::Position(position))) => println!("{position:?}"),
    ///             Ok(SubscriptionItem::Data(PositionUpdate::PositionEnd))        => println!("initial set of positions received"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
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
    /// use ibapi::subscriptions::SubscriptionItem;
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
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(update)) => {
    ///                 println!("{update:?}");
    ///                 if let AccountUpdate::End = update {
    ///                     break;
    ///                 }
    ///             }
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
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
    /// use ibapi::subscriptions::SubscriptionItem;
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
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(update)) => {
    ///                 println!("{update:?}");
    ///                 if let AccountUpdateMulti::End = update {
    ///                     break;
    ///                 }
    ///             }
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
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
            decoders::decode_managed_accounts,
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
            || Err(Error::UnexpectedEndOfStream),
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
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Request the configured soft dollar tiers available to the account.
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
    ///     let tiers = client.soft_dollar_tiers().await.expect("request failed");
    ///     for tier in &tiers {
    ///         println!("{}: {}", tier.name, tier.display_name);
    ///     }
    /// }
    /// ```
    pub async fn soft_dollar_tiers(&self) -> Result<Vec<crate::orders::SoftDollarTier>, Error> {
        check_version(self.server_version(), Features::SOFT_DOLLAR_TIER)?;

        crate::common::request_helpers::one_shot_request_with_retry(
            self,
            encoders::encode_request_soft_dollar_tiers,
            decoders::decode_soft_dollar_tiers_message,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Request white-branding identity information for the logged-in user.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let info = client.user_info().await.expect("request failed");
    ///     println!("white branding id: {}", info.white_branding_id);
    /// }
    /// ```
    pub async fn user_info(&self) -> Result<UserInfo, Error> {
        check_version(self.server_version(), Features::USER_INFO)?;

        crate::common::request_helpers::one_shot_request_with_retry(
            self,
            encoders::encode_request_user_info,
            decoders::decode_user_info_message,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Request the current Financial Advisor configuration as an XML string.
    ///
    /// # Arguments
    /// * `fa_data_type` - which FA dataset to fetch.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::FaDataType;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let cfg = client.request_fa(FaDataType::Groups).await.expect("request failed");
    ///     println!("{}", cfg.xml);
    /// }
    /// ```
    pub async fn request_fa(&self, fa_data_type: FaDataType) -> Result<FaConfig, Error> {
        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::RequestFA,
            move || encoders::encode_request_fa(fa_data_type as i32),
            decoders::decode_receive_fa,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Replace the Financial Advisor configuration on the server.
    ///
    /// # Arguments
    /// * `fa_data_type` - which FA dataset to replace.
    /// * `xml`          - the replacement configuration as an XML string.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::FaDataType;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let result = client.replace_fa(FaDataType::Groups, "<xml/>").await.expect("request failed");
    ///     println!("{}", result.text);
    /// }
    /// ```
    pub async fn replace_fa(&self, fa_data_type: FaDataType, xml: &str) -> Result<ReplaceFaResult, Error> {
        check_version(self.server_version(), Features::REPLACE_FA_END)?;

        crate::common::request_helpers::one_shot_request_with_retry(
            self,
            move |request_id| encoders::encode_replace_fa(request_id, fa_data_type as i32, xml),
            decoders::decode_replace_fa_end_message,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Set the verbosity level for server-side TWS API diagnostics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::ServerLogLevel;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     client.set_server_log_level(ServerLogLevel::Detail).await.expect("request failed");
    /// }
    /// ```
    pub async fn set_server_log_level(&self, log_level: ServerLogLevel) -> Result<(), Error> {
        let message = encoders::encode_set_server_log_level(log_level as i32)?;
        self.send_message(message).await?;
        Ok(())
    }

    /// Initiate a TWS extension verification handshake.
    ///
    /// Most users will not call this directly; it is part of the IB Linking flow.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let challenge = client.verify_request("MyApp", "1.0").await.expect("request failed");
    ///     println!("{}", challenge.api_data);
    /// }
    /// ```
    pub async fn verify_request(&self, api_name: &str, api_version: &str) -> Result<VerificationChallenge, Error> {
        check_version(self.server_version(), Features::LINKING)?;

        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::VerifyRequest,
            move || encoders::encode_verify_request(api_name, api_version),
            decoders::decode_verify_message_api,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Continue a TWS extension verification handshake by sending the API response data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let result = client.verify_message("signed-challenge").await.expect("request failed");
    ///     if result.is_successful {
    ///         println!("verified");
    ///     } else {
    ///         eprintln!("{}", result.error_text);
    ///     }
    /// }
    /// ```
    pub async fn verify_message(&self, api_data: &str) -> Result<VerificationResult, Error> {
        check_version(self.server_version(), Features::LINKING)?;

        crate::common::request_helpers::one_shot_with_retry(
            self,
            OutgoingMessages::VerifyMessage,
            move || encoders::encode_verify_message(api_data),
            decoders::decode_verify_completed,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
