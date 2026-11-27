//! Asynchronous implementation of account management functionality

use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use time::OffsetDateTime;
use tokio::task::JoinHandle;

use crate::client::builders::r#async::SubscriptionBuilder;
use crate::client::{ClientRequestBuilders, SubscriptionBuilderExt};
use crate::common::request_helpers::{self, empty_on_end_of_stream, expect_proto};
use crate::messages::{resubscribed_notice, Notice, OutgoingMessages};
use crate::subscriptions::Subscription;
use crate::transport::farm_watch::FarmMark;
use crate::transport::r#async::{CancelWrite, WriteAdmission};
use crate::{Client, Error};

use super::common::snapshots::SnapshotBuilder;
use super::common::{decoders, encoders};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

impl Client {
    /// Subscribe to streaming position updates for all accessible accounts.
    ///
    /// The stream first replays the full position list and then sends incremental updates.
    ///
    /// During an internet outage (after a 1100) TWS answers a new positions request only with
    /// the global notice 2151 "Positions info is not available yet." (request id -1, so it
    /// reaches `notice_stream()`, not this subscription) and then sends nothing, also after
    /// connectivity returns: live, no `PositionEnd` came in 1554 s and 348 s. Bound the wait
    /// for `PositionEnd` and request again after the 1102. The subscription does not end on
    /// 2151: positions share one channel per client, and a request opened before the outage
    /// resumes on its subscription, so a request-less 2151 could end a live stream
    /// (`docs/wire-facts/protocol.md#requests-held-during-an-internet-outage`).
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

    /// Subscribe to streaming position updates scoped by account and model code.
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
        let builder = self.request();
        let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

        builder.send::<PositionUpdateMulti>(request).await
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut subscription = client.pnl("DU1234567", None).await.expect("error requesting pnl");
    ///
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl(&self, account: impl Into<AccountId>, model_code: Option<&ModelCode>) -> Result<Subscription<PnL>, Error> {
        let account = account.into();
        let builder = self.request();
        let request = encoders::encode_request_pnl(builder.request_id(), &account, model_code)?;
        builder.send(request).await
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
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let mut subscription = client.pnl_single("DU1234567", 1001, None).await.expect("error requesting pnl");
    ///
    ///     while let Some(pnl) = subscription.next().await {
    ///         println!("{pnl:?}")
    ///     }
    /// }
    /// ```
    pub async fn pnl_single(
        &self,
        account: impl Into<AccountId>,
        contract_id: impl Into<ContractId>,
        model_code: Option<&ModelCode>,
    ) -> Result<Subscription<PnLSingle>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_pnl_single(builder.request_id(), &account.into(), contract_id.into(), model_code)?;
        builder.send(request).await
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
        self.account_summary_send(group, tags, None)?.await
    }

    fn account_summary_send(
        &self,
        group: &AccountGroup,
        tags: &[&str],
        admission: Option<WriteAdmission>,
    ) -> Result<impl std::future::Future<Output = Result<Subscription<AccountSummaryResult>, Error>> + Send + 'static, Error> {
        let request_id = self.mint_request_id();
        let request = encoders::encode_request_account_summary(request_id.raw(), group, tags)?;
        let builder = SubscriptionBuilder::new_with_components(self.decoder_context(), Arc::clone(&self.message_bus));
        Ok(builder.send_with_request_id_capped(request_id, request, None, admission))
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
    /// The stream ends when the transport reconnects; to resubscribe and keep going, use
    /// [`account_summary_snapshots_resubscribing`](Self::account_summary_snapshots_resubscribing).
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

        Ok(AccountSummarySnapshots::new(subscription, quiet))
    }

    /// [`account_summary_snapshots`](Self::account_summary_snapshots) that survives transport
    /// reconnects and TWS invalidation of account data (1101).
    ///
    /// When the connection drops, it waits for the reconnect and requests the summary again.
    /// On notice 1101, it cancels the invalidated request and opens a replacement. Notice
    /// 1102 keeps the request and its accumulated values. After opening the replacement,
    /// the wrapper returns an [`Error::Notice`] with `TRANSPORT_RECONNECT_CODE` or 1101, then
    /// builds the first snapshot from scratch at the new request's `End` marker.
    ///
    /// A broadcast lag notice, a failed reconnect, or another request error ends the stream
    /// after yielding the error. See [`cancel`](AccountSummarySnapshotsResubscribing::cancel)
    /// for the terminal items from explicit cancellation.
    ///
    /// The stream borrows the client. To move it into a spawned task, open it from an
    /// `Arc<Client>` with [`AccountSummarySnapshotsResubscribing::subscribe`].
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
    /// use ibapi::{Error, TRANSPORT_RECONNECT_CODE};
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
    ///         .account_summary_snapshots_resubscribing(&group, tags, Duration::from_secs(1))
    ///         .await
    ///         .expect("error requesting account summary");
    ///
    ///     while let Some(snapshot) = snapshots.next().await {
    ///         match snapshot {
    ///             Ok(snapshot) => println!("{} rows", snapshot.len()),
    ///             Err(Error::Notice(n)) if matches!(n.code, TRANSPORT_RECONNECT_CODE | 1101) => println!("gap: {n}"),
    ///             Err(e) => eprintln!("error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn account_summary_snapshots_resubscribing(
        &self,
        group: &AccountGroup,
        tags: &[&str],
        quiet: Duration,
    ) -> Result<AccountSummarySnapshotsResubscribing<&Self>, Error> {
        AccountSummarySnapshotsResubscribing::subscribe(self, group, tags, quiet).await
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
    /// * `account`        - Account values can be requested for a particular account.
    /// * `model_code`     - Account values can also be requested for a model.
    /// * `ledger_only`    - Which keys arrive: [`LedgerOnly::Yes`] for only the per-currency
    ///   ledger keys, [`LedgerOnly::No`] for those plus the account-level keys
    ///   (`NetLiquidation`, `BuyingPower`, ...). See [`LedgerOnly`] for the `$LEDGER-` prefix.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
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
    ///     let mut subscription = client.account_updates_multi(Some(&account), None, LedgerOnly::No).await.expect("error requesting account updates multi");
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
        ledger_only: LedgerOnly,
    ) -> Result<Subscription<AccountUpdateMulti>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code, ledger_only)?;

        builder.send::<AccountUpdateMulti>(request).await
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
    fn new(subscription: Subscription<AccountSummaryResult>, quiet: Duration) -> Self {
        Self {
            subscription,
            builder: SnapshotBuilder::default(),
            quiet,
        }
    }

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

/// Account summary snapshots that survive transport reconnects and 1101, from
/// [`Client::account_summary_snapshots_resubscribing`] or [`subscribe`](Self::subscribe).
///
/// `C` is how the stream holds the client it resubscribes with: `&Client` from the client
/// method, or an owned handle such as `Arc<Client>` from [`subscribe`](Self::subscribe), which
/// makes the stream `'static` so it can move into a spawned task.
pub struct AccountSummarySnapshotsResubscribing<C> {
    client: C,
    group: AccountGroup,
    tags: Vec<String>,
    quiet: Duration,
    /// `None` once the stream has ended, and while reconnecting.
    snapshots: Option<AccountSummarySnapshots>,
    mark: FarmMark,
    data_lost: Option<Notice>,
    invalidated: Option<AccountSummarySnapshots>,
    /// The connection dropped and the next [`next`](Self::next) waits for the reopened request.
    reconnecting: bool,
    cancelled: Arc<AtomicBool>,
    reopen: Mutex<Option<SummaryReopen>>,
}

struct SummaryReopen {
    task: JoinHandle<Result<OpenedSummary, Error>>,
    _admission: CancelWrite,
}

struct OpenedSummary {
    snapshots: AccountSummarySnapshots,
    mark: FarmMark,
}

impl<C> std::fmt::Debug for AccountSummarySnapshotsResubscribing<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountSummarySnapshotsResubscribing")
            .field("group", &self.group)
            .field("tags", &self.tags)
            .field("quiet", &self.quiet)
            .finish_non_exhaustive()
    }
}

impl<C: Deref<Target = Client>> AccountSummarySnapshotsResubscribing<C> {
    /// Requests the account summary through `client` and returns its snapshots, resubscribing
    /// after each transport reconnect or 1101, as
    /// [`Client::account_summary_snapshots_resubscribing`] does.
    ///
    /// Pass an `Arc<Client>` to get a stream that owns its client handle and can move into a
    /// spawned task.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// use ibapi::accounts::types::AccountGroup;
    /// use ibapi::accounts::{AccountSummarySnapshotsResubscribing, AccountSummaryTags};
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await.expect("connection failed"));
    ///
    ///     let group = AccountGroup("All".to_string());
    ///     let tags = &[AccountSummaryTags::NET_LIQUIDATION];
    ///     let mut snapshots = AccountSummarySnapshotsResubscribing::subscribe(Arc::clone(&client), &group, tags, Duration::from_secs(1))
    ///         .await
    ///         .expect("error requesting account summary");
    ///
    ///     // The first snapshot here, the rest in a task.
    ///     let first = snapshots.next().await;
    ///     println!("first: {first:?}");
    ///     tokio::spawn(async move {
    ///         while let Some(snapshot) = snapshots.next().await {
    ///             println!("{snapshot:?}");
    ///         }
    ///     });
    /// }
    /// ```
    // Four required arguments, the client method's three plus the handle; a builder would only
    // rename them.
    pub async fn subscribe(client: C, group: &AccountGroup, tags: &[&str], quiet: Duration) -> Result<Self, Error> {
        let mark = client.message_bus.farm_watch().mark();
        let snapshots = client.account_summary_snapshots(group, tags, quiet).await?;

        Ok(Self {
            client,
            group: group.clone(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            quiet,
            snapshots: Some(snapshots),
            mark,
            data_lost: None,
            invalidated: None,
            reconnecting: false,
            cancelled: Arc::default(),
            reopen: Mutex::default(),
        })
    }

    /// Cancels the underlying subscription. The next call to [`next`](Self::next) returns
    /// `Err(Error::Cancelled)`, then any rows not yet emitted, then `None`.
    /// Cancelling while waiting for a reconnect ends the stream. An unopened replacement is
    /// abandoned before its first byte. If its write has already started, it finishes in the
    /// background and cancels its subscription.
    pub async fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(snapshots) = &self.snapshots {
            snapshots.cancel().await;
        } else if self.reconnecting {
            let reopen = self.reopen.lock().unwrap().take();
            drop(reopen);
            if let Some(invalidated) = &self.invalidated {
                invalidated.cancel().await;
            }
        }
    }

    /// Returns the next snapshot, a notice after opening a replacement, or `None` once the stream has ended.
    ///
    /// Not cancel-safe while reading a snapshot, as [`AccountSummarySnapshots::next`]. A call
    /// dropped during a reconnect or reopen send (a losing `tokio::select!` branch, a
    /// `timeout`) preserves the reopen. The next call resumes waiting unless you call
    /// [`cancel`](Self::cancel).
    pub async fn next(&mut self) -> Option<Result<AccountSummarySnapshot, Error>> {
        if self.reconnecting && self.cancelled.load(Ordering::Relaxed) {
            self.reconnecting = false;
            self.invalidated = None;
            return Some(Err(Error::Cancelled));
        }
        if !self.reconnecting {
            let snapshots = self.snapshots.as_mut()?;
            let farms = self.client.message_bus.farm_watch();
            let watch = !self.cancelled.load(Ordering::Relaxed);
            let next = tokio::select! {
                biased;
                notice = farms.data_lost(self.mark), if watch => Err(notice),
                item = snapshots.next() => if watch {
                    farms.data_lost_since(self.mark).map_or(Ok(item), Err)
                } else {
                    Ok(item)
                },
            };
            let next = match next {
                Ok(next) => next,
                Err(notice) => {
                    self.data_lost = Some(notice);
                    self.invalidated = self.snapshots.take();
                    self.reconnecting = true;
                    return Some(Err(self.resubscribe().await));
                }
            };
            match next {
                Some(Err(e)) if e.is_connection_lost() => {
                    // Its request died with the old session; dropping it sends no cancel.
                    self.snapshots = None;
                    self.reconnecting = true;
                }
                Some(Err(Error::Cancelled)) => return Some(Err(Error::Cancelled)),
                item @ (Some(Err(_)) | None) => {
                    self.snapshots = None;
                    return item;
                }
                item => return item,
            }
        }
        Some(Err(self.resubscribe().await))
    }

    /// Waits for the reconnect and requests the summary again. Returns the gap notice, or the
    /// error that ends the stream.
    async fn resubscribe(&mut self) -> Error {
        let reopen = self.reopen.get_mut().unwrap();
        loop {
            if reopen.is_none() {
                if let Err(e) = self.client.wait_connected().await {
                    self.reconnecting = false;
                    return e;
                }
                let tags: Vec<&str> = self.tags.iter().map(String::as_str).collect();
                let (admission, cancel) = WriteAdmission::without_deadline();
                let request = match self.client.account_summary_send(&self.group, &tags, Some(admission)) {
                    Ok(request) => request,
                    Err(e) => {
                        self.reconnecting = false;
                        return e;
                    }
                };
                let cancelled = Arc::downgrade(&self.cancelled);
                let quiet = self.quiet;
                let message_bus = Arc::clone(&self.client.message_bus);
                let invalidated = self.invalidated.take();
                // The write may have reached TWS while its flush is still pending. If the
                // wrapper is dropped, finish that send so dropping its output can cancel it.
                let task = self.client.message_bus.runtime_handle().spawn(async move {
                    if let Some(invalidated) = invalidated {
                        invalidated.cancel().await;
                    }
                    if cancelled.upgrade().is_none_or(|cancelled| cancelled.load(Ordering::Relaxed)) {
                        return Err(Error::Cancelled);
                    }
                    let mark = message_bus.farm_watch().mark();
                    request.await.map(|subscription| OpenedSummary {
                        snapshots: AccountSummarySnapshots::new(subscription, quiet),
                        mark,
                    })
                });
                *reopen = Some(SummaryReopen { task, _admission: cancel });
            }
            let result = (&mut reopen.as_mut().unwrap().task).await;
            *reopen = None;
            let result = match result {
                Ok(result) => result,
                Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
                Err(_) => Err(Error::Shutdown),
            };
            match result {
                Ok(OpenedSummary { snapshots, mark }) => {
                    self.snapshots = Some(snapshots);
                    self.mark = mark;
                    self.reconnecting = false;
                    return Error::Notice(self.data_lost.take().unwrap_or_else(resubscribed_notice));
                }
                Err(e) if e.is_connection_lost() => {}
                Err(e) => {
                    self.reconnecting = false;
                    return e;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
