//! Asynchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::{decoders, encoders, helpers::async_helpers};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

// DataStream implementations are now in common/stream_decoders.rs

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub async fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    async_helpers::shared_subscription(
        client,
        Features::POSITIONS,
        OutgoingMessages::RequestPositions,
        encoders::encode_request_positions,
    )
    .await
}

pub async fn positions_multi(
    client: &Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<PositionUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

    builder.send::<PositionUpdateMulti>(request).await
}

// Determine whether an account exists under an account family and find the account family code.
pub async fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    async_helpers::one_shot_request(
        client,
        Features::FAMILY_CODES,
        OutgoingMessages::RequestFamilyCodes,
        encoders::encode_request_family_codes,
        decoders::decode_family_codes,
        Vec::default,
    )
    .await
}

// Creates subscription for real time daily PnL and unrealized PnL updates
//
// # Arguments
// * `client`     - client
// * `account`    - account for which to receive PnL updates
// * `model_code` - specify to request PnL updates for a specific model
pub async fn pnl(client: &Client, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Subscription<PnL>, Error> {
    async_helpers::request_with_id(client, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code)).await
}

// Requests real time updates for daily PnL of individual positions.
//
// # Arguments
// * `client` - Client
// * `account` - Account in which position exists
// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// * `model_code` - Model in which position exists
pub async fn pnl_single(
    client: &Client,
    account: &AccountId,
    contract_id: ContractId,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<PnLSingle>, Error> {
    async_helpers::request_with_id(client, Features::REALIZED_PNL, |id| {
        encoders::encode_request_pnl_single(id, account, contract_id, model_code)
    })
    .await
}

pub async fn account_summary(client: &Client, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<AccountSummaryResult>, Error> {
    async_helpers::request_with_id(client, Features::ACCOUNT_SUMMARY, |id| {
        encoders::encode_request_account_summary(id, group, tags)
    })
    .await
}

pub async fn account_updates(client: &Client, account: &AccountId) -> Result<Subscription<AccountUpdate>, Error> {
    async_helpers::shared_request(client, OutgoingMessages::RequestAccountData, || {
        encoders::encode_request_account_updates(client.server_version(), account)
    })
    .await
}

pub async fn account_updates_multi(
    client: &Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<AccountUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code)?;

    builder.send::<AccountUpdateMulti>(request).await
}

pub async fn managed_accounts(client: &Client) -> Result<Vec<String>, Error> {
    async_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::RequestManagedAccounts,
        encoders::encode_request_managed_accounts,
        |message| {
            message.skip(); // message type
            message.skip(); // message version
            let accounts = message.next_string()?;
            Ok(accounts.split(",").map(String::from).collect())
        },
        || Ok(Vec::default()),
    )
    .await
}

pub async fn server_time(client: &Client) -> Result<OffsetDateTime, Error> {
    async_helpers::one_shot_with_retry(
        client,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::responses;

    use super::super::common::test_utils::helpers::*;

    #[tokio::test]
    async fn test_positions() {
        let (client, message_bus) = create_test_client_with_responses(vec![responses::POSITION.into(), responses::POSITION_END.into()]);

        let mut subscription = positions(&client).await.expect("request positions failed");

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

        let mut subscription = positions_multi(&client, account.as_ref(), model_code.as_ref())
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

        let mut subscription = account_summary(&client, &group, tags).await.expect("request account_summary failed");

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

        let subscription1 = pnl(&client, &account, model_code.as_ref()).await.expect("request pnl failed");
        drop(subscription1);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let subscription2 = pnl(&client, &account, None).await.expect("request pnl failed");
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

        let subscription1 = pnl_single(&client, &account, contract_id, model_code.as_ref())
            .await
            .expect("request pnl_single failed");
        drop(subscription1);

        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let subscription2 = pnl_single(&client, &account, contract_id, None).await.expect("request pnl_single failed");
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

        let accounts = managed_accounts(&client).await.expect("request managed accounts failed");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");

        // Check request message
        assert_request_messages(&message_bus, &["17|1|"]);
    }

    #[tokio::test]
    async fn test_managed_accounts_retry() {
        // Test that managed_accounts handles retry scenarios
        // Since our stub doesn't simulate actual connection resets, we'll test with valid responses
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::MANAGED_ACCOUNT.into(), // Successful response
        ]);

        let accounts = managed_accounts(&client).await.expect("managed_accounts failed");
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

        let result = server_time(&client).await;
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

        let result = family_codes(&client).await;
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
        let result_no_msg = family_codes(&client_no_msg).await;
        assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
        assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_no_msg, &["80|1|"]);

        // Scenario 3: Empty family codes list
        let (client_empty, message_bus_empty) = create_test_client_with_responses(vec![
            "78|0|".into(), // Zero family codes
        ]);
        let result_empty = family_codes(&client_empty).await;
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
        let mut subscription = account_updates(&client, &account_name).await.expect("subscribe failed");

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
        let mut subscription = account_updates_multi(&client, account.as_ref(), None)
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
        use super::common::test_data::tables::VERSION_TEST_CASES;

        let account = AccountId(TEST_ACCOUNT.to_string());
        let group = AccountGroup("All".to_string());

        for test_case in VERSION_TEST_CASES {
            let (client, _) = create_test_client_with_version(test_case.required_version - 1);

            let result = match test_case.function_name {
                "PnL" => pnl(&client, &account, None).await.map(|_| ()),
                "PnL Single" => pnl_single(&client, &account, ContractId(1001), None).await.map(|_| ()),
                "Account Summary" => account_summary(&client, &group, &[AccountSummaryTags::ACCOUNT_TYPE]).await.map(|_| ()),
                "Positions Multi" => positions_multi(&client, Some(&account), None).await.map(|_| ()),
                "Account Updates Multi" => account_updates_multi(&client, Some(&account), None).await.map(|_| ()),
                "Family Codes" => family_codes(&client).await.map(|_| ()),
                "Positions" => positions(&client).await.map(|_| ()),
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
        use super::common::test_data::tables::managed_accounts_test_cases;

        for test_case in managed_accounts_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_test_client()
            } else {
                create_test_client_with_responses(test_case.responses)
            };

            let accounts = managed_accounts(&client)
                .await
                .expect(&format!("managed_accounts failed for {}", test_case.scenario));
            assert_eq!(accounts, test_case.expected, "{}: {}", test_case.scenario, test_case.description);
            assert_request_messages(&message_bus, &["17|1|"]);
        }
    }

    #[tokio::test]
    async fn test_server_time_scenarios() {
        use super::common::test_data::tables::server_time_test_cases;

        for test_case in server_time_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_test_client()
            } else {
                create_test_client_with_responses(test_case.responses)
            };

            let result = server_time(&client).await;

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
        let sub1_future = pnl(&client, &account1, None);
        let sub2_future = pnl(&client, &account2, None);
        let sub3_future = positions(&client);

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
        // Test account summary with multiple tags
        let (client, message_bus) = create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

        let group = AccountGroup("All".to_string());
        let tags = &[
            AccountSummaryTags::ACCOUNT_TYPE,
            AccountSummaryTags::NET_LIQUIDATION,
            AccountSummaryTags::TOTAL_CASH_VALUE,
        ];

        let mut subscription = account_summary(&client, &group, tags).await.expect("account_summary failed");

        // Should get at least one summary
        let first_update = subscription.next().await;
        assert!(matches!(first_update, Some(Ok(AccountSummaryResult::Summary(_)))));

        // Should get end marker
        let second_update = subscription.next().await;
        assert!(matches!(second_update, Some(Ok(AccountSummaryResult::End))));

        drop(subscription);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify the encoded tags are sent correctly
        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages[0].contains("AccountType,NetLiquidation,TotalCashValue"),
            "Request should contain all tags"
        );
    }

    #[tokio::test]
    async fn test_account_summary_empty_tags() {
        // Test account summary with empty tags list
        let (client, _) = create_test_client();
        let group = AccountGroup("All".to_string());
        let tags: &[&str] = &[];

        let result = account_summary(&client, &group, tags).await;
        assert!(result.is_ok(), "Empty tags should be allowed");
    }

    #[tokio::test]
    async fn test_pnl_different_model_codes() {
        // Test PnL requests with different model codes
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model1 = ModelCode("MODEL1".to_string());
        let model2 = ModelCode("MODEL2".to_string());

        // Request PnL with different model codes
        let sub1 = pnl(&client, &account, Some(&model1)).await.expect("PnL request 1 failed");
        let sub2 = pnl(&client, &account, Some(&model2)).await.expect("PnL request 2 failed");
        let sub3 = pnl(&client, &account, None).await.expect("PnL request 3 failed");

        drop(sub1);
        drop(sub2);
        drop(sub3);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 6, "Expected at least 6 messages");

        // Verify model codes are encoded correctly - filter for subscription messages only
        let pnl_requests: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("92|")).collect();
        assert!(pnl_requests.len() >= 3, "Expected at least 3 PnL subscription messages");
        assert!(pnl_requests[0].contains("MODEL1"), "First request should contain MODEL1");
        assert!(pnl_requests[1].contains("MODEL2"), "Second request should contain MODEL2");
        assert!(pnl_requests[2].ends_with("||"), "Third request should have empty model code");
    }

    #[tokio::test]
    async fn test_positions_multi_with_various_parameters() {
        // Test positions_multi with different parameter combinations
        let (client, message_bus) = create_test_client_with_responses(vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()]);

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model = ModelCode(TEST_MODEL_CODE.to_string());

        // Test all parameter combinations
        let sub1 = positions_multi(&client, Some(&account), Some(&model))
            .await
            .expect("positions_multi with both params failed");
        drop(sub1);

        let sub2 = positions_multi(&client, Some(&account), None)
            .await
            .expect("positions_multi with account only failed");
        drop(sub2);

        let sub3 = positions_multi(&client, None, Some(&model))
            .await
            .expect("positions_multi with model only failed");
        drop(sub3);

        let sub4 = positions_multi(&client, None, None).await.expect("positions_multi with no params failed");
        drop(sub4);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 8, "Expected at least 8 messages (4 subscribe + 4 cancel)");
    }

    #[tokio::test]
    async fn test_subscription_cleanup_on_drop() {
        // Test that subscriptions are properly cleaned up when dropped
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());

        // Create and immediately drop subscriptions
        {
            let _sub1 = pnl(&client, &account, None).await.expect("PnL subscription failed");
            let _sub2 = positions(&client).await.expect("Positions subscription failed");
            // Subscriptions dropped here
        }

        // Allow time for async cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= 4,
            "Expected subscribe and cancel messages for both subscriptions"
        );

        // Should have cancel messages
        let cancel_count = request_messages
            .iter()
            .filter(|msg| msg.starts_with("93|") || msg.starts_with("64|"))
            .count();
        assert!(cancel_count >= 2, "Expected at least 2 cancel messages");
    }

    #[tokio::test]
    async fn test_edge_case_contract_ids() {
        use super::common::test_data::tables::contract_id_test_cases;

        let test_cases = contract_id_test_cases();
        let (client, message_bus) = create_test_client();
        let account = AccountId(TEST_ACCOUNT.to_string());
        let mut subscriptions = Vec::new();

        // Create all subscriptions
        for test_case in &test_cases {
            let sub = pnl_single(&client, &account, test_case.contract_id, None)
                .await
                .expect(&format!("PnL single failed for {}", test_case.description));
            subscriptions.push(sub);
        }

        // Drop all subscriptions to trigger cancellation
        drop(subscriptions);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= test_cases.len() * 2,
            "Expected at least {} messages (subscribe + cancel for each)",
            test_cases.len() * 2
        );

        // Verify contract IDs are encoded correctly
        let subscription_messages: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("94|")).collect();

        assert_eq!(
            subscription_messages.len(),
            test_cases.len(),
            "Expected {} subscription messages",
            test_cases.len()
        );

        for (i, test_case) in test_cases.iter().enumerate() {
            assert!(
                subscription_messages[i].contains(&test_case.expected_pattern),
                "Request {} should contain {} for {}",
                i,
                test_case.expected_pattern,
                test_case.description
            );
        }
    }

    #[tokio::test]
    async fn test_account_updates_stream_handling() {
        // Test continuous account updates stream
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("54|1|{}|", TEST_ACCOUNT), // End marker
        ]);

        let account = AccountId(TEST_ACCOUNT.to_string());
        let mut subscription = account_updates(&client, &account).await.expect("account_updates failed");

        let mut update_count = 0;
        while let Some(update_result) = subscription.next().await {
            match update_result.expect("Update should not error") {
                AccountUpdate::AccountValue(_) => {
                    update_count += 1;
                }
                AccountUpdate::End => {
                    break;
                }
                _ => {} // Ignore other update types
            }
        }

        assert_eq!(update_count, 3, "Expected 3 account value updates");
        assert_request_messages(&message_bus, &[&format!("6|2|1|{}|", TEST_ACCOUNT)]);
    }
}
