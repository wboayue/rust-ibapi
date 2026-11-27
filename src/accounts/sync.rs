//! Synchronous implementation of account management functionality

use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;

use crate::client::blocking::{ClientRequestBuilders, Subscription, SubscriptionBuilderExt};
use crate::common::request_helpers::{self, empty_on_end_of_stream, expect_proto};
use crate::messages::OutgoingMessages;
use crate::{client::sync::Client, Error};

use super::common::snapshots::SnapshotBuilder;
use super::common::{decoders, encoders};
use super::types::{AccountGroup, AccountId};
use super::*;
use crate::contracts::ContractId;

impl Client {
    /// TWS's current time. TWS is synchronized with the server (not local computer) using NTP and this function will receive the current time in TWS.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let server_time = client.server_time().expect("error requesting server time");
    /// println!("server time: {server_time:?}");
    /// ```
    pub fn server_time(&self) -> Result<OffsetDateTime, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestCurrentTime,
            encoders::encode_request_server_time,
            expect_proto(decoders::decode_server_time_proto),
        )
    }

    /// Requests the current server time with millisecond precision.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let server_time = client.server_time_millis().expect("error requesting server time");
    /// println!("server time to the millisecond: {server_time:?}");
    /// ```
    pub fn server_time_millis(&self) -> Result<OffsetDateTime, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestCurrentTimeInMillis,
            encoders::encode_request_server_time_millis,
            expect_proto(decoders::decode_server_time_millis_proto),
        )
    }

    /// Subscribes to [PositionUpdate]s for all accessible accounts.
    /// All positions sent initially, and then only updates as positions change.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::PositionUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.positions().expect("error requesting positions");
    /// for position_response in subscription.iter_data() {
    ///     match position_response? {
    ///         PositionUpdate::Position(position) => println!("{position:?}"),
    ///         PositionUpdate::PositionEnd => println!("initial set of positions received"),
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        let request = encoders::encode_request_positions()?;
        self.subscription().send_shared(OutgoingMessages::RequestPositions, request)
    }

    /// Subscribes to [PositionUpdateMulti]s, optionally narrowed to one account or model.
    /// TWS sends the current positions, then updates as positions change.
    ///
    /// Returns a builder: set [`account`](PositionsMultiBuilder::account) and
    /// [`model_code`](PositionsMultiBuilder::model_code), then call
    /// [`subscribe`](PositionsMultiBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.positions_multi().account("DU1234567").model_code("TARGET2024").subscribe()?;
    /// for position in subscription.iter_data() {
    ///     println!("{:?}", position?);
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn positions_multi(&self) -> PositionsMultiBuilder<'_, Self> {
        PositionsMultiBuilder::new(self)
    }

    /// Subscribes to real time daily PnL and unrealized PnL updates for `account`.
    ///
    /// Returns a builder: narrow to one model with [`model_code`](PnLBuilder::model_code), then
    /// call [`subscribe`](PnLBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.pnl("DU1234567").subscribe()?;
    /// for pnl in subscription.iter_data() {
    ///     println!("{:?}", pnl?);
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pnl(&self, account: impl Into<AccountId>) -> PnLBuilder<'_, Self> {
        PnLBuilder::new(self, account)
    }

    /// Subscribes to real time daily PnL for one position: `contract_id` held in `account`.
    ///
    /// Returns a builder: name the model the position is held in with
    /// [`model_code`](PnLSingleBuilder::model_code), then call
    /// [`subscribe`](PnLSingleBuilder::subscribe).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.pnl_single("DU1234567", 265598).subscribe()?; // AAPL
    /// for pnl in subscription.iter_data() {
    ///     println!("{:?}", pnl?);
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn pnl_single(&self, account: impl Into<AccountId>, contract_id: impl Into<ContractId>) -> PnLSingleBuilder<'_, Self> {
        PnLSingleBuilder::new(self, account, contract_id)
    }

    /// Requests a specific account's summary. Subscribes to the account summary as presented in the TWS' Account Summary tab. Data received is specified by using a specific tags value.
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
    /// * `group` - Set to "All" to return account summary data for all accounts, or set to a specific Advisor Account Group name that has already been created in TWS Global Configuration.
    /// * `tags`  - List of the desired tags.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::AccountSummaryTags;
    /// use ibapi::accounts::types::AccountGroup;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let group = AccountGroup("All".to_string());
    ///
    /// let subscription = client.account_summary(&group, &[AccountSummaryTags::ACCOUNT_TYPE]).expect("error requesting account summary");
    /// for summary in subscription.iter_data() {
    ///     match summary {
    ///         Ok(summary) => println!("{summary:?}"),
    ///         Err(e) => eprintln!("error: {e}"),
    ///     }
    /// }
    /// ```
    pub fn account_summary(&self, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<AccountSummaryResult>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_account_summary(builder.request_id(), group, tags)?;
        builder.send(request)
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
    /// Dropping the returned iterator cancels the subscription.
    ///
    /// # Arguments
    /// * `group` - Set to "All" to return account summary data for all accounts, or set to a specific Advisor Account Group name that has already been created in TWS Global Configuration.
    /// * `tags`  - List of the desired tags.
    /// * `quiet` - How long without a row completes a pushed update.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::AccountSummaryTags;
    /// use ibapi::accounts::types::AccountGroup;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let group = AccountGroup("All".to_string());
    /// let tags = &[AccountSummaryTags::NET_LIQUIDATION];
    ///
    /// let snapshots = client
    ///     .account_summary_snapshots(&group, tags, Duration::from_secs(1))
    ///     .expect("error requesting account summary");
    /// for snapshot in snapshots {
    ///     let snapshot = snapshot.expect("account summary error");
    ///     println!("{} rows", snapshot.len());
    /// }
    /// ```
    pub fn account_summary_snapshots(&self, group: &AccountGroup, tags: &[&str], quiet: Duration) -> Result<AccountSummarySnapshots, Error> {
        let subscription = self.account_summary(group, tags)?;

        Ok(AccountSummarySnapshots {
            subscription,
            builder: SnapshotBuilder::default(),
            quiet,
        })
    }

    /// Subscribes to a specific account's information and portfolio.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change in a position, or to an account value every 3 minutes if it has changed.
    ///
    /// TWS streams one account at a time per connection. Subscribing to the same
    /// account again shares the stream; another account is refused until every
    /// subscription to the first has been cancelled or dropped. To stream several
    /// accounts at once, use [`account_updates_multi`](Client::account_updates_multi).
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::AccountUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.account_updates("U1234567").expect("error requesting account updates");
    /// for update in subscription.iter_data() {
    ///     let update = update?;
    ///     println!("{update:?}");
    ///
    ///     // stop after full initial update
    ///     if let AccountUpdate::End = update {
    ///         subscription.cancel();
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn account_updates(&self, account: impl Into<AccountId>) -> Result<Subscription<AccountUpdate>, Error> {
        let account = &account.into();
        let request = encoders::encode_request_account_updates(true, account)?;
        let subscription = self.message_bus.send_account_updates_request(account, &request)?;
        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests account updates for account and/or model.
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let subscription = client.account_updates_multi(LedgerOnly::No).account("DU1234567").subscribe()?;
    /// for update in subscription.iter_data() {
    ///     let update = update?;
    ///     println!("{update:?}");
    ///
    ///     // stop after full initial update
    ///     if let AccountUpdateMulti::End = update {
    ///         subscription.cancel();
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
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
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let accounts = client.managed_accounts().expect("error requesting managed accounts");
    /// println!("managed accounts: {accounts:?}")
    /// ```
    pub fn managed_accounts(&self) -> Result<Vec<String>, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestManagedAccounts,
            encoders::encode_request_managed_accounts,
            expect_proto(decoders::decode_managed_accounts_proto),
        )
        .or_else(empty_on_end_of_stream)
    }

    /// Get current [FamilyCode]s for all accessible accounts.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let codes = client.family_codes().expect("error requesting family codes");
    /// println!("family codes: {codes:?}")
    /// ```
    pub fn family_codes(&self) -> Result<Vec<FamilyCode>, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestFamilyCodes,
            encoders::encode_request_family_codes,
            expect_proto(decoders::decode_family_codes_proto),
        )
        .or_else(empty_on_end_of_stream)
    }

    /// Request the configured soft dollar tiers available to the account.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let tiers = client.soft_dollar_tiers().expect("request failed");
    /// for tier in &tiers {
    ///     println!("{}: {}", tier.name, tier.display_name);
    /// }
    /// ```
    pub fn soft_dollar_tiers(&self) -> Result<Vec<crate::orders::SoftDollarTier>, Error> {
        request_helpers::blocking::one_shot_by_request_id(
            self,
            encoders::encode_request_soft_dollar_tiers,
            expect_proto(decoders::decode_soft_dollar_tiers_proto),
        )
    }

    /// Request white-branding identity information for the logged-in user.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let info = client.user_info().expect("request failed");
    /// println!("white branding id: {}", info.white_branding_id);
    /// ```
    pub fn user_info(&self) -> Result<UserInfo, Error> {
        request_helpers::blocking::one_shot_by_request_id(self, encoders::encode_request_user_info, expect_proto(decoders::decode_user_info_proto))
    }

    /// Request the current Financial Advisor configuration as an XML string.
    ///
    /// # Arguments
    /// * `fa_data_type` - which FA dataset to fetch.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::FaDataType;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let cfg = client.request_fa(FaDataType::Groups).expect("request failed");
    /// println!("{}", cfg.xml);
    /// ```
    pub fn request_fa(&self, fa_data_type: FaDataType) -> Result<FaConfig, Error> {
        request_helpers::blocking::one_shot_shared_matching(
            self,
            OutgoingMessages::RequestFA,
            || encoders::encode_request_fa(fa_data_type),
            decoders::receive_fa_for(fa_data_type),
        )
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::FaDataType;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let result = client.replace_fa(FaDataType::Groups, "<xml/>").expect("request failed");
    /// println!("{}", result.text);
    /// ```
    pub fn replace_fa(&self, fa_data_type: FaDataType, xml: &str) -> Result<ReplaceFaResult, Error> {
        request_helpers::blocking::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_replace_fa(request_id, fa_data_type, xml),
            expect_proto(decoders::decode_replace_fa_end_proto),
        )
    }

    /// Set the verbosity level for server-side TWS API diagnostics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::ServerLogLevel;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// client.set_server_log_level(ServerLogLevel::Detail).expect("request failed");
    /// ```
    pub fn set_server_log_level(&self, log_level: ServerLogLevel) -> Result<(), Error> {
        let message = encoders::encode_set_server_log_level(log_level)?;
        self.send_message(message)?;
        Ok(())
    }

    /// Initiate a TWS extension verification handshake.
    ///
    /// Most users will not call this directly; it is part of the IB Linking flow.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let challenge = client.verify_request("MyApp", "1.0").expect("request failed");
    /// println!("{}", challenge.api_data);
    /// ```
    pub fn verify_request(&self, api_name: &str, api_version: &str) -> Result<VerificationChallenge, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::VerifyRequest,
            || encoders::encode_verify_request(api_name, api_version),
            expect_proto(decoders::decode_verify_message_api_proto),
        )
    }

    /// Continue a TWS extension verification handshake by sending the API response data.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let result = client.verify_message("signed-challenge").expect("request failed");
    /// if result.is_successful {
    ///     println!("verified");
    /// } else {
    ///     eprintln!("{}", result.error_text);
    /// }
    /// ```
    pub fn verify_message(&self, api_data: &str) -> Result<VerificationResult, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::VerifyMessage,
            || encoders::encode_verify_message(api_data),
            expect_proto(decoders::decode_verify_completed_proto),
        )
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
    /// Cancels the underlying subscription. Iteration then ends, after a final snapshot of any rows
    /// not yet emitted.
    pub fn cancel(&self) {
        self.subscription.cancel();
    }
}

impl Iterator for AccountSummarySnapshots {
    type Item = Result<AccountSummarySnapshot, Error>;

    /// Returns the next snapshot, or `None` once the subscription has ended.
    ///
    /// A snapshot completes at an `End` marker or after the `quiet` period without a row. Rows
    /// received before the subscription ends are returned as a final snapshot.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.subscription.next_batch(self.quiet) {
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
#[path = "sync_tests.rs"]
mod tests;
