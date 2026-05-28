//! Synchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::blocking::{ClientRequestBuilders, SharesChannel, Subscription};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::{client::sync::Client, Error};

use super::common::{decoders, encoders};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

// Implement SharesChannel for PositionUpdate subscription
impl SharesChannel for Subscription<PositionUpdate> {}

impl Client {
    /// TWS's current time. TWS is synchronized with the server (not local computer) using NTP and this function will receive the current time in TWS.
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
        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::RequestCurrentTime,
            encoders::encode_request_server_time,
            decoders::decode_server_time,
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Requests the current server time with millisecond precision.
    pub fn server_time_millis(&self) -> Result<OffsetDateTime, Error> {
        check_version(self.server_version, Features::CURRENT_TIME_IN_MILLIS)?;

        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::RequestCurrentTimeInMillis,
            encoders::encode_request_server_time_millis,
            decoders::decode_server_time_millis,
            || Err(Error::UnexpectedEndOfStream),
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
        crate::common::request_helpers::blocking::shared_subscription(
            self,
            Features::POSITIONS,
            OutgoingMessages::RequestPositions,
            encoders::encode_request_positions,
        )
    }

    /// Subscribes to [PositionUpdateMulti] updates for account and/or model.
    /// Initially all positions are returned, and then updates are returned for any position changes in real time.
    ///
    /// # Arguments
    /// * `account`    - If an account Id is provided, only the account's positions belonging to the specified model will be delivered.
    /// * `model_code` - The code of the model's positions we are interested in.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// use ibapi::accounts::types::AccountId;
    ///
    /// let account = AccountId("U1234567".to_string());
    /// let subscription = client.positions_multi(Some(&account), None).expect("error requesting positions by model");
    /// for position in subscription.iter() {
    ///     println!("{position:?}")
    /// }
    /// ```
    pub fn positions_multi(&self, account: Option<&AccountId>, model_code: Option<&ModelCode>) -> Result<Subscription<PositionUpdateMulti>, Error> {
        check_version(self.server_version(), Features::MODELS_SUPPORT)?;

        let builder = self.request();
        let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

        builder.send(request)
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// use ibapi::accounts::types::AccountId;
    ///
    /// let account = AccountId("account id".to_string());
    /// let subscription = client.pnl(&account, None).expect("error requesting pnl");
    /// for pnl in subscription.iter() {
    ///     println!("{pnl:?}")
    /// }
    /// ```
    pub fn pnl(&self, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Subscription<PnL>, Error> {
        crate::common::request_helpers::blocking::request_with_id(self, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code))
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// use ibapi::accounts::types::{AccountId, ContractId};
    ///
    /// let account = AccountId("<account id>".to_string());
    /// let contract_id = ContractId(1001);
    ///
    /// let subscription = client.pnl_single(&account, contract_id, None).expect("error requesting pnl");
    /// for pnl in &subscription {
    ///     println!("{pnl:?}")
    /// }
    /// ```
    pub fn pnl_single(&self, account: &AccountId, contract_id: ContractId, model_code: Option<&ModelCode>) -> Result<Subscription<PnLSingle>, Error> {
        crate::common::request_helpers::blocking::request_with_id(self, Features::REALIZED_PNL, |id| {
            encoders::encode_request_pnl_single(id, account, contract_id, model_code)
        })
    }

    /// Requests a specific account's summary. Subscribes to the account summary as presented in the TWS' Account Summary tab. Data received is specified by using a specific tags value.
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
    /// for summary in &subscription {
    ///     println!("{summary:?}")
    /// }
    /// ```
    pub fn account_summary(&self, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<AccountSummaryResult>, Error> {
        crate::common::request_helpers::blocking::request_with_id(self, Features::ACCOUNT_SUMMARY, |id| {
            encoders::encode_request_account_summary(id, group, tags)
        })
    }

    /// Subscribes to a specific account's information and portfolio.
    ///
    /// All account values and positions will be returned initially, and then there will only be updates when there is a change in a position, or to an account value every 3 minutes if it has changed. Only one account can be subscribed at a time.
    ///
    /// # Arguments
    /// * `account` - The account id (i.e. U1234567) for which the information is requested.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::AccountUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// use ibapi::accounts::types::AccountId;
    ///
    /// let account = AccountId("U1234567".to_string());
    ///
    /// let subscription = client.account_updates(&account).expect("error requesting account updates");
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
    pub fn account_updates(&self, account: &AccountId) -> Result<Subscription<AccountUpdate>, Error> {
        crate::common::request_helpers::blocking::shared_request(self, OutgoingMessages::RequestAccountData, || {
            encoders::encode_request_account_updates(true, account)
        })
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::accounts::AccountUpdateMulti;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// use ibapi::accounts::types::AccountId;
    ///
    /// let account = AccountId("U1234567".to_string());
    ///
    /// let subscription = client.account_updates_multi(Some(&account), None).expect("error requesting account updates multi");
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
    pub fn account_updates_multi(
        &self,
        account: Option<&AccountId>,
        model_code: Option<&ModelCode>,
    ) -> Result<Subscription<AccountUpdateMulti>, Error> {
        check_version(self.server_version(), Features::MODELS_SUPPORT)?;

        let builder = self.request();
        let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code)?;

        builder.send(request)
    }

    /// Requests the accounts to which the logged user has access to.
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
        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::RequestManagedAccounts,
            encoders::encode_request_managed_accounts,
            decoders::decode_managed_accounts,
            || Ok(Vec::default()),
        )
    }

    /// Get current [FamilyCode]s for all accessible accounts.
    pub fn family_codes(&self) -> Result<Vec<FamilyCode>, Error> {
        crate::common::request_helpers::blocking::one_shot_request(
            self,
            Features::FAMILY_CODES,
            OutgoingMessages::RequestFamilyCodes,
            encoders::encode_request_family_codes,
            decoders::decode_family_codes,
            Vec::default,
        )
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
        check_version(self.server_version, Features::SOFT_DOLLAR_TIER)?;

        crate::common::request_helpers::blocking::one_shot_request_with_retry(
            self,
            encoders::encode_request_soft_dollar_tiers,
            decoders::decode_soft_dollar_tiers_message,
            || Err(Error::UnexpectedEndOfStream),
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
        check_version(self.server_version, Features::USER_INFO)?;

        crate::common::request_helpers::blocking::one_shot_request_with_retry(
            self,
            encoders::encode_request_user_info,
            decoders::decode_user_info_message,
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Request the current Financial Advisor configuration as an XML string.
    ///
    /// # Arguments
    /// * `fa_data_type` - which FA dataset to fetch.
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
        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::RequestFA,
            || encoders::encode_request_fa(fa_data_type as i32),
            decoders::decode_receive_fa,
            || Err(Error::UnexpectedEndOfStream),
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
        check_version(self.server_version, Features::REPLACE_FA_END)?;

        crate::common::request_helpers::blocking::one_shot_request_with_retry(
            self,
            |request_id| encoders::encode_replace_fa(request_id, fa_data_type as i32, xml),
            decoders::decode_replace_fa_end_message,
            || Err(Error::UnexpectedEndOfStream),
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
        let message = encoders::encode_set_server_log_level(log_level as i32)?;
        self.send_message(message)?;
        Ok(())
    }

    /// Initiate a TWS extension verification handshake.
    ///
    /// Most users will not call this directly; it is part of the IB Linking flow.
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
        check_version(self.server_version, Features::LINKING)?;

        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::VerifyRequest,
            || encoders::encode_verify_request(api_name, api_version),
            decoders::decode_verify_message_api,
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Continue a TWS extension verification handshake by sending the API response data.
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
        check_version(self.server_version, Features::LINKING)?;

        crate::common::request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::VerifyMessage,
            || encoders::encode_verify_message(api_data),
            decoders::decode_verify_completed,
            || Err(Error::UnexpectedEndOfStream),
        )
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
