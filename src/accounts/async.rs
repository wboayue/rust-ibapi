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
    ///     while let Some(update_result) = subscription.next().await {
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
            encoders::encode_request_account_updates(self.server_version(), account)
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
    ///     while let Some(update_result) = subscription.next().await {
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
            |message| {
                message.skip(); // message type
                message.skip(); // message version
                let timestamp = message.next_long()?;
                match OffsetDateTime::from_unix_timestamp(timestamp) {
                    Ok(date) => Ok(date),
                    Err(e) => Err(Error::Simple(format!("Error parsing date: {e}"))),
                }
            },
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
mod tests {
    use super::*;
    use crate::testdata::responses;

    use crate::common::test_utils::helpers::*;

    #[tokio::test]
    async fn test_positions() {
        let (client, message_bus) = create_test_client_with_responses(vec![responses::POSITION.into(), responses::POSITION_END.into()]);

        let mut subscription = client.positions().await.expect("request positions failed");

        // First update should be a position
        let first_update = subscription.next().await;
        assert!(
            matches!(first_update, Some(Ok(PositionUpdate::Position(_)))),
            "Expected PositionUpdate::Position, got {:?}",
            first_update
        );

        // Second update should be position end
        let second_update = subscription.next().await;
        assert!(
            matches!(second_update, Some(Ok(PositionUpdate::PositionEnd))),
            "Expected PositionUpdate::PositionEnd, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check both subscribe and cancel messages
        assert_request_messages(
            &message_bus,
            &[
                "61|1|", // Subscribe
                "64|1|", // CancelPositions
            ],
        );
    }

    #[tokio::test]
    async fn test_positions_multi() {
        let (client, message_bus) = create_test_client_with_responses(vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()]);

        let account = Some(AccountId(TEST_ACCOUNT.to_string()));
        let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));

        let mut subscription = client
            .positions_multi(account.as_ref(), model_code.as_ref())
            .await
            .expect("request positions_multi failed");

        // First update should be a position
        let first_update = subscription.next().await;
        assert!(
            matches!(first_update, Some(Ok(PositionUpdateMulti::Position(_)))),
            "Expected PositionUpdateMulti::Position"
        );

        // Second update should be position end
        let second_update = subscription.next().await;
        assert!(
            matches!(second_update, Some(Ok(PositionUpdateMulti::PositionEnd))),
            "Expected PositionUpdateMulti::PositionEnd"
        );

        drop(subscription); // Trigger cancellation

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check both subscribe and cancel messages
        let request_messages = get_request_messages(&message_bus);
        assert_eq!(request_messages.len(), 2, "Expected subscribe and cancel messages");
        assert_eq!(request_messages[0], "74|1|9000|DU1234567|TARGET2024|");
        assert_eq!(request_messages[1], "75|1|9000|"); // Cancel request
    }

    #[tokio::test]
    async fn test_account_summary() {
        let (client, message_bus) = create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

        let group = AccountGroup("All".to_string());
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let mut subscription = client.account_summary(&group, tags).await.expect("request account_summary failed");

        // First update should be a summary
        let first_update = subscription.next().await;
        match first_update {
            Some(Ok(AccountSummaryResult::Summary(summary))) => {
                assert_eq!(summary.account, TEST_ACCOUNT);
                assert_eq!(summary.tag, AccountSummaryTags::ACCOUNT_TYPE);
                assert_eq!(summary.value, "FA");
            }
            _ => panic!("Expected AccountSummaryResult::Summary, got {first_update:?}"),
        }

        // Second update should be end
        let second_update = subscription.next().await;
        assert!(
            matches!(second_update, Some(Ok(AccountSummaryResult::End))),
            "Expected AccountSummaryResult::End, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check both subscribe and cancel messages
        assert_request_messages(
            &message_bus,
            &[
                "62|1|9000|All|AccountType|",
                "63|1|9000|", // CancelAccountSummary
            ],
        );
    }

    #[tokio::test]
    async fn test_pnl() {
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));

        let subscription1 = client.pnl(&account, model_code.as_ref()).await.expect("request pnl failed");
        drop(subscription1);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let subscription2 = client.pnl(&account, None).await.expect("request pnl failed");
        drop(subscription2);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_request_messages(
            &message_bus,
            &["92|9000|DU1234567|TARGET2024|", "93|9000|", "92|9001|DU1234567||", "93|9001|"],
        );
    }

    #[tokio::test]
    async fn test_pnl_single() {
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let contract_id = ContractId(TEST_CONTRACT_ID);
        let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));

        let subscription1 = client
            .pnl_single(&account, contract_id, model_code.as_ref())
            .await
            .expect("request pnl_single failed");
        drop(subscription1);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let subscription2 = client.pnl_single(&account, contract_id, None).await.expect("request pnl_single failed");
        drop(subscription2);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_request_messages(
            &message_bus,
            &["94|9000|DU1234567|TARGET2024|1001|", "95|9000|", "94|9001|DU1234567||1001|", "95|9001|"],
        );
    }

    #[tokio::test]
    async fn test_managed_accounts() {
        let (client, message_bus) = create_test_client_with_responses(vec![responses::MANAGED_ACCOUNT.into()]);

        let accounts = client.managed_accounts().await.expect("request managed accounts failed");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");

        // Check request message
        assert_request_messages(&message_bus, &["17|1|"]);
    }

    #[tokio::test]
    async fn test_managed_accounts_retry() {
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::MANAGED_ACCOUNT.into(), // Successful response
        ]);

        let accounts = client.managed_accounts().await.expect("managed_accounts failed");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Accounts list mismatch");

        // Verify request was sent
        assert_request_messages(&message_bus, &["17|1|"]);
    }

    #[tokio::test]
    async fn test_server_time() {
        use time::macros::datetime;

        let valid_timestamp_str = "1678890000"; // 2023-03-15 14:20:00 UTC
        let expected_datetime = datetime!(2023-03-15 14:20:00 UTC);

        let (client, message_bus) = create_test_client_with_responses(vec![format!("49|1|{}|", valid_timestamp_str)]);

        let result = client.server_time().await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");

        // Check request message
        assert_request_messages(&message_bus, &["49|1|"]);
    }

    #[tokio::test]
    async fn test_family_codes() {
        use crate::accounts::FamilyCode;

        // Scenario 1: Success with multiple codes
        let (client, message_bus) = create_test_client_with_responses(vec!["78|2|ACC1|FC1|ACC2|FC2|".into()]);

        let result = client.family_codes().await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let codes = result.unwrap();
        assert_eq!(codes.len(), 2, "Expected 2 family codes");
        assert_eq!(
            codes[0],
            FamilyCode {
                account_id: "ACC1".to_string(),
                family_code: "FC1".to_string()
            }
        );
        assert_eq!(
            codes[1],
            FamilyCode {
                account_id: "ACC2".to_string(),
                family_code: "FC2".to_string()
            }
        );
        assert_request_messages(&message_bus, &["80|1|"]);

        // Scenario 2: No message received (returns empty vector)
        let (client_no_msg, message_bus_no_msg) = create_test_client();
        let result_no_msg = client_no_msg.family_codes().await;
        assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
        assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_no_msg, &["80|1|"]);

        // Scenario 3: Empty family codes list
        let (client_empty, message_bus_empty) = create_test_client_with_responses(vec![
            "78|0|".into(), // Zero family codes
        ]);
        let result_empty = client_empty.family_codes().await;
        assert!(result_empty.is_ok(), "Expected Ok for empty list");
        assert!(result_empty.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_empty, &["80|1|"]);
    }

    #[tokio::test]
    async fn test_account_updates() {
        use crate::accounts::AccountUpdate;

        let account_name = AccountId(TEST_ACCOUNT.to_string());

        // Create client with account update responses
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("{}|", responses::ACCOUNT_VALUE), // AccountValue with trailing delimiter
            format!("54|1|{}|", TEST_ACCOUNT),        // AccountDownloadEnd
        ]);

        // Subscribe to account updates
        let mut subscription = client.account_updates(&account_name).await.expect("subscribe failed");

        // First update should be AccountValue
        let first_update = subscription.next().await;
        match first_update {
            Some(Ok(AccountUpdate::AccountValue(av))) => {
                assert_eq!(av.key, "CashBalance");
                assert_eq!(av.value, "1000.00");
                assert_eq!(av.currency, "USD");
            }
            other => panic!("First update was not AccountValue: {other:?}"),
        }

        // Second update should be End
        let second_update = subscription.next().await;
        assert!(
            matches!(second_update, Some(Ok(AccountUpdate::End))),
            "Expected AccountUpdate::End, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify request messages - subscribe and cancel
        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 2, "Expected subscribe and cancel messages");

        // First message should be subscribe (RequestAccountData = 6)
        assert!(request_messages[0].starts_with("6|"), "First message should be RequestAccountData");

        // Last message should be cancel
        let last_msg = &request_messages[request_messages.len() - 1];
        assert!(last_msg.starts_with("6|"), "Last message should be RequestAccountData (cancel)");
    }

    #[tokio::test]
    async fn test_account_updates_multi() {
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::ACCOUNT_UPDATE_MULTI_CASH_BALANCE.into(),
            responses::ACCOUNT_UPDATE_MULTI_CURRENCY.into(),
            responses::ACCOUNT_UPDATE_MULTI_STOCK_MARKET_VALUE.into(),
            responses::ACCOUNT_UPDATE_MULTI_END.into(),
        ]);

        let account = Some(AccountId(TEST_ACCOUNT.to_string()));
        let mut subscription = client
            .account_updates_multi(account.as_ref(), None)
            .await
            .expect("request account_updates_multi failed");

        let expected_keys = &["CashBalance", "Currency", "StockMarketValue"];

        for key in expected_keys {
            let update = subscription.next().await.unwrap().unwrap();
            match update {
                AccountUpdateMulti::AccountMultiValue(value) => {
                    assert_eq!(value.key, *key);
                }
                AccountUpdateMulti::End => {
                    panic!("value expected")
                }
            }
        }

        let end = subscription.next().await.unwrap().unwrap();
        assert_eq!(end, AccountUpdateMulti::End);

        subscription.cancel().await;

        // Check both subscribe and cancel messages
        assert_request_messages(
            &message_bus,
            &[
                "76|1|9000|DU1234567||1|",
                "77|1|9000|", // Cancel request
            ],
        );
    }

    // Additional comprehensive tests

    #[tokio::test]
    async fn test_server_version_errors() {
        use super::common::test_tables::VERSION_TEST_CASES;

        let account = AccountId(TEST_ACCOUNT.to_string());
        let group = AccountGroup("All".to_string());

        for test_case in VERSION_TEST_CASES {
            let (client, _) = create_test_client_with_version(test_case.required_version - 1);

            let result = match test_case.function_name {
                "PnL" => client.pnl(&account, None).await.map(|_| ()),
                "PnL Single" => client.pnl_single(&account, ContractId(1001), None).await.map(|_| ()),
                "Account Summary" => client.account_summary(&group, &[AccountSummaryTags::ACCOUNT_TYPE]).await.map(|_| ()),
                "Positions Multi" => client.positions_multi(Some(&account), None).await.map(|_| ()),
                "Account Updates Multi" => client.account_updates_multi(Some(&account), None).await.map(|_| ()),
                "Family Codes" => client.family_codes().await.map(|_| ()),
                "Positions" => client.positions().await.map(|_| ()),
                _ => panic!("Unknown function: {}", test_case.function_name),
            };

            assert!(result.is_err(), "Expected version error for {}", test_case.function_name);
            if let Err(error) = result {
                assert!(
                    matches!(error, Error::ServerVersion(_, _, _)),
                    "Expected ServerVersion error for {}, got: {:?}",
                    test_case.function_name,
                    error
                );
            }
        }
    }

    #[tokio::test]
    async fn test_managed_accounts_scenarios() {
        use super::common::test_tables::managed_accounts_test_cases;

        for test_case in managed_accounts_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_test_client()
            } else {
                create_test_client_with_responses(test_case.responses)
            };

            let accounts = client
                .managed_accounts()
                .await
                .unwrap_or_else(|_| panic!("managed_accounts failed for {}", test_case.scenario));
            assert_eq!(accounts, test_case.expected, "{}: {}", test_case.scenario, test_case.description);
            assert_request_messages(&message_bus, &["17|1|"]);
        }
    }

    #[tokio::test]
    async fn test_server_time_scenarios() {
        use super::common::test_tables::server_time_test_cases;

        for test_case in server_time_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_test_client()
            } else {
                create_test_client_with_responses(test_case.responses)
            };

            let result = client.server_time().await;

            match test_case.expected_result {
                Ok(expected_time) => {
                    assert!(result.is_ok(), "Expected Ok for {}, got: {:?}", test_case.scenario, result.err());
                    assert_eq!(result.unwrap(), expected_time, "Timestamp mismatch for {}", test_case.scenario);
                }
                Err("No response from server") => {
                    assert!(result.is_err(), "Expected error for {}", test_case.scenario);
                    if let Err(Error::Simple(msg)) = result {
                        assert_eq!(msg, "No response from server", "Error message mismatch for {}", test_case.scenario);
                    } else {
                        panic!("Expected Simple error with 'No response from server' for {}", test_case.scenario);
                    }
                }
                Err(_) => {
                    assert!(result.is_err(), "Expected error for {}", test_case.scenario);
                    // Accept Parse, ParseInt, or Simple errors for invalid timestamps
                    match result.unwrap_err() {
                        Error::Parse(_, _, _) | Error::ParseInt(_) | Error::Simple(_) => {}
                        other => panic!("Expected Parse, ParseInt, or Simple error for {}, got: {:?}", test_case.scenario, other),
                    }
                }
            }

            assert_request_messages(&message_bus, &[test_case.expected_request]);
        }
    }

    #[tokio::test]
    async fn test_concurrent_subscriptions() {
        // Test multiple concurrent subscriptions
        let (client, message_bus) = create_test_client();

        let account1 = AccountId("ACCOUNT1".to_string());
        let account2 = AccountId("ACCOUNT2".to_string());

        // Create multiple concurrent subscriptions
        let sub1_future = client.pnl(&account1, None);
        let sub2_future = client.pnl(&account2, None);
        let sub3_future = client.positions();

        let (sub1, sub2, sub3) = tokio::join!(sub1_future, sub2_future, sub3_future);

        assert!(sub1.is_ok(), "First PnL subscription should succeed");
        assert!(sub2.is_ok(), "Second PnL subscription should succeed");
        assert!(sub3.is_ok(), "Positions subscription should succeed");

        drop(sub1.unwrap());
        drop(sub2.unwrap());
        drop(sub3.unwrap());

        // Allow time for async cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify all requests were sent
        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 6, "Expected at least 6 messages (3 subscribe + 3 cancel)");
    }

    #[tokio::test]
    async fn test_account_summary_multiple_tags() {
        use super::common::test_tables::account_summary_tag_test_cases;

        let test_cases = account_summary_tag_test_cases();

        for test_case in test_cases {
            let group = AccountGroup(test_case.group.clone());

            if test_case.expect_responses {
                // Create client with mock responses for tests that expect data
                let (client, message_bus) =
                    create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

                let mut subscription = client
                    .account_summary(&group, &test_case.tags)
                    .await
                    .unwrap_or_else(|_| panic!("account_summary failed for {}", test_case.description));

                // Should get at least one summary
                let first_update = subscription.next().await;
                assert!(
                    matches!(first_update, Some(Ok(AccountSummaryResult::Summary(_)))),
                    "Expected summary for {}",
                    test_case.description
                );

                // Should get end marker
                let second_update = subscription.next().await;
                assert!(
                    matches!(second_update, Some(Ok(AccountSummaryResult::End))),
                    "Expected end marker for {}",
                    test_case.description
                );

                drop(subscription);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // Verify the encoded tags are sent correctly
                if let Some(expected_encoding) = test_case.expected_tag_encoding {
                    let request_messages = get_request_messages(&message_bus);
                    assert!(!request_messages.is_empty(), "Expected request messages for {}", test_case.description);

                    if !expected_encoding.is_empty() {
                        assert!(
                            request_messages[0].contains(expected_encoding),
                            "Request should contain '{}' for {}, got: {}",
                            expected_encoding,
                            test_case.description,
                            request_messages[0]
                        );
                    }
                }
            } else {
                // Create client without specific responses
                let (client, _) = create_test_client();

                let result = client.account_summary(&group, &test_case.tags).await;
                assert!(result.is_ok(), "account_summary should succeed for {}", test_case.description);
            }
        }
    }
}
