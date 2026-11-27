//! Asynchronous implementation of account management functionality

use std::time::Duration;

use time::OffsetDateTime;

use crate::client::{ClientRequestBuilders, SubscriptionBuilderExt};
use crate::common::request_helpers::{self, empty_on_end_of_stream, expect_proto};
use crate::messages::OutgoingMessages;
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::snapshots::SnapshotBuilder;
use super::common::{decoders, encoders};
use super::types::{AccountGroup, AccountId};
use super::*;
use crate::contracts::ContractId;

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
        let request = encoders::encode_request_positions()?;
        self.subscription().send_shared(OutgoingMessages::RequestPositions, request).await
    }

    /// Subscribe to streaming position updates, optionally narrowed to one account or model.
    /// TWS sends the current positions, then updates as positions change.
    ///
    /// Returns a builder: set [`account`](PositionsMultiBuilder::account) and
    /// [`model_code`](PositionsMultiBuilder::model_code), then call
    /// [`subscribe`](PositionsMultiBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///     let mut positions = client
    ///         .positions_multi()
    ///         .account("DU1234567")
    ///         .model_code("TARGET2024")
    ///         .subscribe()
    ///         .await?
    ///         .filter_data();
    ///     while let Some(position) = positions.next().await {
    ///         println!("{:?}", position?);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn positions_multi(&self) -> PositionsMultiBuilder<'_, Self> {
        PositionsMultiBuilder::new(self)
    }

    /// Fetch the account family codes registered with the broker.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestFamilyCodes,
            encoders::encode_request_family_codes,
            expect_proto(decoders::decode_family_codes_proto),
        )
        .await
        .or_else(empty_on_end_of_stream)
    }

    /// Subscribe to real-time daily and unrealized PnL updates for `account`.
    ///
    /// Returns a builder: narrow to one model with [`model_code`](PnLBuilder::model_code), then
    /// call [`subscribe`](PnLBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///     let mut updates = client.pnl("DU1234567").subscribe().await?.filter_data();
    ///     while let Some(pnl) = updates.next().await {
    ///         println!("{:?}", pnl?);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn pnl(&self, account: impl Into<AccountId>) -> PnLBuilder<'_, Self> {
        PnLBuilder::new(self, account)
    }

    /// Subscribe to real-time daily PnL for one position: `contract_id` held in `account`.
    ///
    /// Returns a builder: name the model the position is held in with
    /// [`model_code`](PnLSingleBuilder::model_code), then call
    /// [`subscribe`](PnLSingleBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///     let mut updates = client.pnl_single("DU1234567", 265598).subscribe().await?.filter_data(); // AAPL
    ///     while let Some(pnl) = updates.next().await {
    ///         println!("{:?}", pnl?);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn pnl_single(&self, account: impl Into<AccountId>, contract_id: impl Into<ContractId>) -> PnLSingleBuilder<'_, Self> {
        PnLSingleBuilder::new(self, account, contract_id)
    }

    /// Subscribe to account summary updates for a group of accounts.
    ///
    /// # Subscription lifetime
    ///
    /// The subscription stays open until it is dropped or cancelled. TWS first sends the requested
    /// tags followed by [`AccountSummaryResult::End`], then keeps pushing
    /// [`AccountSummaryResult::Summary`] rows without another `End`: when values change, and
    /// periodically, when a push can resend rows whose values did not change. Dropping the subscription cancels
    /// the request.
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
        let builder = self.request();
        let request = encoders::encode_request_account_summary(builder.request_id(), group, tags)?;
        builder.send(request).await
    }

    /// Subscribe to account summary updates as complete snapshots.
    ///
    /// Wraps [`account_summary`](Self::account_summary) and keeps the latest value of every row, so
    /// each [`AccountSummarySnapshot`] holds the whole account rather than only the rows TWS pushed
    /// last. TWS sends one `End` marker after the initial snapshot and none after the rows it pushes
    /// later, so the first snapshot is emitted at the first `End`, even when empty, and each later
    /// one once no row has arrived for `quiet`. Pushes arrive when values change and periodically
    /// (IB documents every three minutes), and a push can resend unchanged rows. A later snapshot is
    /// emitted only when a row changed a value since the previous one. Choose `quiet` longer than
    /// the gap between the rows of one push, which arrive within milliseconds of each other.
    ///
    /// To fold the rows with your own batching instead, use [`AccountSummarySnapshot::apply`].
    ///
    /// Notices that arrive on the subscription are logged at `warn!`. For other streams that dump
    /// then push, batch with [`Subscription::next_batch`](crate::subscriptions::Subscription::next_batch).
    ///
    /// Dropping the returned stream cancels the subscription in a background task. Call
    /// [`cancel`](AccountSummarySnapshots::cancel) before resubscribing: TWS allows two account
    /// summary requests at a time.
    ///
    /// # Arguments
    /// * `group` - Set to "All" to return account summary data for all accounts, or set to a specific Advisor Account Group name.
    /// * `tags`  - List of the desired tags.
    /// * `quiet` - How long without a row completes a pushed update.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountSummaryTags;
    /// use ibapi::accounts::types::AccountGroup;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let group = AccountGroup("All".to_string());
    ///     let tags = &[AccountSummaryTags::NET_LIQUIDATION];
    ///
    ///     let mut snapshots = client
    ///         .account_summary_snapshots(&group, tags, Duration::from_secs(1))
    ///         .await
    ///         .expect("error requesting account summary");
    ///
    ///     while let Some(snapshot) = snapshots.next().await {
    ///         let snapshot = snapshot.expect("account summary error");
    ///         println!("{} rows", snapshot.len());
    ///     }
    /// }
    /// ```
    pub async fn account_summary_snapshots(&self, group: &AccountGroup, tags: &[&str], quiet: Duration) -> Result<AccountSummarySnapshots, Error> {
        let subscription = self.account_summary(group, tags).await?;

        Ok(AccountSummarySnapshots {
            subscription,
            builder: SnapshotBuilder::default(),
            quiet,
        })
    }

    /// Subscribe to detailed account updates for a specific account.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change in a position, or to an account value every 3 minutes if it has changed.
    ///
    /// TWS streams one account at a time per connection. Subscribing to the same
    /// account again shares the stream; another account is refused until every
    /// subscription to the first has been cancelled. Call
    /// [`cancel`](Subscription::cancel)`().await` before switching accounts: a
    /// dropped subscription is released by a spawned task, so a request made right
    /// after the drop can still be refused. To stream several accounts at once, use
    /// [`account_updates_multi`](Client::account_updates_multi).
    ///
    /// # Arguments
    /// * `account` - The account id (i.e. U1234567) for which the information is requested.
    ///
    /// # Errors
    /// [`Error::AccountUpdatesInUse`] while another account's updates are live.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::AccountUpdate;
    /// use ibapi::subscriptions::SubscriptionItem;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let mut subscription = client.account_updates("U1234567").await.expect("error requesting account updates");
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
    pub async fn account_updates(&self, account: impl Into<AccountId>) -> Result<Subscription<AccountUpdate>, Error> {
        let account = &account.into();
        let request = encoders::encode_request_account_updates(true, account)?;
        let subscription = self.message_bus.send_account_updates_request(account, request).await?;
        Ok(Subscription::new_from_internal(
            subscription,
            self.message_bus.clone(),
            None,
            None,
            self.decoder_context(),
        ))
    }

    /// Subscribe to account updates scoped by account and model code.
    ///
    /// Each subscription is routed by its own request id, so several accounts or models can stream at once.
    /// Per-currency ledger values arrive keyed `$LEDGER-<key>` when the TWS setting is on; see
    /// [`AccountMultiValue::base_key`].
    ///
    /// # Arguments
    /// * `ledger_only` - Which keys arrive: [`LedgerOnly::Yes`] for only the per-currency
    ///   ledger keys, [`LedgerOnly::No`] for those plus the account-level keys
    ///   (`NetLiquidation`, `BuyingPower`, ...). See [`LedgerOnly`] for the `$LEDGER-` prefix.
    ///
    /// Returns a builder: narrow with [`account`](AccountUpdatesMultiBuilder::account) and
    /// [`model_code`](AccountUpdatesMultiBuilder::model_code), then call
    /// [`subscribe`](AccountUpdatesMultiBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///     let mut subscription = client.account_updates_multi(LedgerOnly::No).account("DU1234567").subscribe().await?;
    ///
    ///     while let Some(item) = subscription.next().await {
    ///         match item? {
    ///             SubscriptionItem::Data(update) => {
    ///                 println!("{update:?}");
    ///                 if let AccountUpdateMulti::End = update {
    ///                     break;
    ///                 }
    ///             }
    ///             SubscriptionItem::Notice(n) => eprintln!("notice: {n}"),
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn account_updates_multi(&self, ledger_only: LedgerOnly) -> AccountUpdatesMultiBuilder<'_, Self> {
        AccountUpdatesMultiBuilder::new(self, ledger_only)
    }

    /// Fetch the list of accounts accessible to the current user.
    ///
    /// For the list TWS sent at connect, with no round trip, see
    /// [`handshake_managed_accounts`](Self::handshake_managed_accounts).
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestManagedAccounts,
            encoders::encode_request_managed_accounts,
            expect_proto(decoders::decode_managed_accounts_proto),
        )
        .await
        .or_else(empty_on_end_of_stream)
    }

    /// Query the current server time reported by TWS or IB Gateway.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestCurrentTime,
            encoders::encode_request_server_time,
            expect_proto(decoders::decode_server_time_proto),
        )
        .await
    }

    /// Query the current server time in milliseconds reported by TWS or IB Gateway.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let server_time = client.server_time_millis().await.expect("error requesting server time");
    ///     println!("server time to the millisecond: {server_time:?}");
    /// }
    /// ```
    pub async fn server_time_millis(&self) -> Result<OffsetDateTime, Error> {
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestCurrentTimeInMillis,
            encoders::encode_request_server_time_millis,
            expect_proto(decoders::decode_server_time_millis_proto),
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
        request_helpers::one_shot_by_request_id(
            self,
            encoders::encode_request_soft_dollar_tiers,
            expect_proto(decoders::decode_soft_dollar_tiers_proto),
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
        request_helpers::one_shot_by_request_id(self, encoders::encode_request_user_info, expect_proto(decoders::decode_user_info_proto)).await
    }

    /// Request the current Financial Advisor configuration as an XML string.
    ///
    /// # Arguments
    /// * `fa_data_type` - which FA dataset to fetch.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared_matching(
            self,
            OutgoingMessages::RequestFA,
            move || encoders::encode_request_fa(fa_data_type),
            decoders::receive_fa_for(fa_data_type),
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
        request_helpers::one_shot_by_request_id(
            self,
            move |request_id| encoders::encode_replace_fa(request_id, fa_data_type, xml),
            expect_proto(decoders::decode_replace_fa_end_proto),
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
        let message = encoders::encode_set_server_log_level(log_level)?;
        self.send_message(message).await?;
        Ok(())
    }

    /// Initiate a TWS extension verification handshake.
    ///
    /// Most users will not call this directly; it is part of the IB Linking flow.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::VerifyRequest,
            move || encoders::encode_verify_request(api_name, api_version),
            expect_proto(decoders::decode_verify_message_api_proto),
        )
        .await
    }

    /// Continue a TWS extension verification handshake by sending the API response data.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::VerifyMessage,
            move || encoders::encode_verify_message(api_data),
            expect_proto(decoders::decode_verify_completed_proto),
        )
        .await
    }
}

/// Complete account summary snapshots from [`Client::account_summary_snapshots`].
pub struct AccountSummarySnapshots {
    subscription: Subscription<AccountSummaryResult>,
    builder: SnapshotBuilder,
    quiet: Duration,
}

impl std::fmt::Debug for AccountSummarySnapshots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountSummarySnapshots")
            .field("quiet", &self.quiet)
            .finish_non_exhaustive()
    }
}

impl AccountSummarySnapshots {
    /// Cancels the underlying subscription. The next call to [`next`](Self::next) returns
    /// `Err(Error::Cancelled)`, then any rows not yet emitted, then `None`.
    pub async fn cancel(&self) {
        self.subscription.cancel().await;
    }

    /// Returns the next snapshot, or `None` once the subscription has ended.
    ///
    /// A snapshot completes at an `End` marker or after the `quiet` period without a row. Rows
    /// received before the subscription ends are returned as a final snapshot.
    ///
    /// Not cancel-safe: dropping the future, as `tokio::select!` does with the branches that lose,
    /// drops the rows of a batch still being read.
    ///
    /// If the consumer falls behind, frames are dropped and this returns `Error::Notice` with
    /// [`SUBSCRIPTION_LAG_CODE`](crate::SUBSCRIPTION_LAG_CODE). Later snapshots may then hold
    /// values the dropped frames updated, since TWS resends only changed tags: cancel and
    /// resubscribe for a fresh snapshot.
    pub async fn next(&mut self) -> Option<Result<AccountSummarySnapshot, Error>> {
        loop {
            match self.subscription.next_batch(self.quiet).await {
                Some(Ok(batch)) => {
                    if let Some(snapshot) = self.builder.fold(batch) {
                        return Some(Ok(snapshot));
                    }
                }
                Some(Err(e)) => return Some(Err(e)),
                None => return self.builder.flush().map(Ok),
            }
        }
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
