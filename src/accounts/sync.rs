//! Synchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::{ClientRequestBuilders, SharesChannel, Subscription};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::{Client, Error};

use super::common::{decoders, encoders, helpers};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

// Implement SharesChannel for PositionUpdate subscription
impl SharesChannel for Subscription<'_, PositionUpdate> {}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    helpers::shared_subscription(
        client,
        Features::POSITIONS,
        OutgoingMessages::RequestPositions,
        encoders::encode_request_positions,
    )
}

pub fn positions_multi<'a>(
    client: &'a Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<'a, PositionUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

    builder.send(request)
}

// Determine whether an account exists under an account family and find the account family code.
pub fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    helpers::one_shot_request(
        client,
        Features::FAMILY_CODES,
        OutgoingMessages::RequestFamilyCodes,
        encoders::encode_request_family_codes,
        decoders::decode_family_codes,
        Vec::default,
    )
}

// Creates subscription for real time daily PnL and unrealized PnL updates
//
// # Arguments
// * `client`     - client
// * `account`    - account for which to receive PnL updates
// * `model_code` - specify to request PnL updates for a specific model
pub fn pnl<'a>(client: &'a Client, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Subscription<'a, PnL>, Error> {
    helpers::request_with_id(client, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code))
}

// Requests real time updates for daily PnL of individual positions.
//
// # Arguments
// * `client` - Client
// * `account` - Account in which position exists
// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// * `model_code` - Model in which position exists
pub fn pnl_single<'a>(
    client: &'a Client,
    account: &AccountId,
    contract_id: ContractId,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<'a, PnLSingle>, Error> {
    helpers::request_with_id(client, Features::REALIZED_PNL, |id| {
        encoders::encode_request_pnl_single(id, account, contract_id, model_code)
    })
}

pub fn account_summary<'a>(client: &'a Client, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<'a, AccountSummaryResult>, Error> {
    helpers::request_with_id(client, Features::ACCOUNT_SUMMARY, |id| {
        encoders::encode_request_account_summary(id, group, tags)
    })
}

pub fn account_updates<'a>(client: &'a Client, account: &AccountId) -> Result<Subscription<'a, AccountUpdate>, Error> {
    helpers::shared_request(client, OutgoingMessages::RequestAccountData, || {
        encoders::encode_request_account_updates(client.server_version(), account)
    })
}

pub fn account_updates_multi<'a>(
    client: &'a Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<'a, AccountUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code)?;

    builder.send(request)
}

pub fn managed_accounts(client: &Client) -> Result<Vec<String>, Error> {
    helpers::one_shot_with_retry(
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
}

pub fn server_time(client: &Client) -> Result<OffsetDateTime, Error> {
    helpers::one_shot_with_retry(
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
}

#[cfg(test)]
mod tests {
    use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
    use crate::accounts::{AccountSummaryTags, AccountUpdateMulti};
    use crate::testdata::responses;
    use crate::{server_versions, stubs::MessageBusStub, Client, Error};
    use std::sync::{Arc, RwLock};

    use super::super::common::test_utils::helpers::*;

    #[test]
    fn test_pnl() {
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));
        let _ = client.pnl(&account, model_code.as_ref()).expect("request pnl failed");
        let _ = client.pnl(&account, None).expect("request pnl failed");

        assert_request_messages(
            &message_bus,
            &["92|9000|DU1234567|TARGET2024|", "93|9000|", "92|9001|DU1234567||", "93|9001|"],
        );
    }

    #[test]
    fn test_pnl_single() {
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let contract_id = ContractId(TEST_CONTRACT_ID);
        let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));
        let _ = client.pnl_single(&account, contract_id, model_code.as_ref()).expect("request pnl failed");
        let _ = client.pnl_single(&account, contract_id, None).expect("request pnl failed");

        assert_request_messages(
            &message_bus,
            &["94|9000|DU1234567|TARGET2024|1001|", "95|9000|", "94|9001|DU1234567||1001|", "95|9001|"],
        );
    }

    #[test]
    fn test_positions() {
        use crate::accounts::PositionUpdate;

        let (client, message_bus) = create_test_client_with_responses(vec![responses::POSITION.into(), responses::POSITION_END.into()]);

        let subscription = client.positions().expect("request positions failed");

        let first_update = subscription.next();
        assert!(
            matches!(first_update, Some(PositionUpdate::Position(_))),
            "Expected PositionUpdate::Position, got {:?}",
            first_update
        );

        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(PositionUpdate::PositionEnd)),
            "Expected PositionUpdate::PositionEnd, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        assert_request_messages(
            &message_bus,
            &[
                "61|1|", // Subscribe
                "64|1|", // CancelPositions
            ],
        );
    }

    #[test]
    fn test_positions_multi() {
        use crate::accounts::PositionUpdateMulti;

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = Some(AccountId("DU1234567".to_string()));
        let model_code = Some(ModelCode("TARGET2024".to_string()));

        let subscription = client
            .positions_multi(account.as_ref(), model_code.as_ref())
            .expect("request positions_multi failed");

        let first_update = subscription.next();
        assert!(
            matches!(first_update, Some(PositionUpdateMulti::Position(_))),
            "Expected PositionUpdateMulti::Position"
        );

        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(PositionUpdateMulti::PositionEnd)),
            "Expected PositionUpdateMulti::PositionEnd"
        );

        drop(subscription); // Trigger cancellation

        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 2, "Expected subscribe and cancel messages for positions_multi");
        assert_eq!(request_messages[0].encode_simple(), "74|1|9000|DU1234567|TARGET2024|");
        assert_eq!(request_messages[1].encode_simple(), "75|1|9000|"); // Cancel request for positions_multi
    }

    #[test]
    fn test_account_summary() {
        use crate::accounts::AccountSummaryResult;

        let (client, message_bus) = create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

        let group = AccountGroup("All".to_string());
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let subscription = client.account_summary(&group, tags).expect("request account_summary failed");

        let first_update = subscription.next();
        match first_update {
            Some(AccountSummaryResult::Summary(summary_data)) => {
                assert_eq!(summary_data.account, TEST_ACCOUNT); // From responses::ACCOUNT_SUMMARY
                assert_eq!(summary_data.tag, AccountSummaryTags::ACCOUNT_TYPE);
                assert_eq!(summary_data.value, "FA");
            }
            _ => panic!("Expected AccountSummaries::Summary, got {first_update:?}"),
        }

        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(AccountSummaryResult::End)),
            "Expected AccountSummaries::End, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        assert_request_messages(
            &message_bus,
            &[
                "62|1|9000|All|AccountType|",
                "63|1|9000|", // CancelAccountSummary
            ],
        );
    }

    #[test]
    fn test_managed_accounts() {
        // Scenario: Valid response
        let (client, _) = create_test_client_with_responses(vec![responses::MANAGED_ACCOUNT.into()]);
        let accounts = client.managed_accounts().expect("request managed accounts failed for valid response");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");

        // Scenario: Empty response string
        let (client_empty, _) = create_test_client_with_responses(vec!["17|1||".to_string()]); // Empty accounts string
        let accounts_empty = client_empty
            .managed_accounts()
            .expect("request managed accounts failed for empty response");
        assert_eq!(
            accounts_empty,
            vec![""],
            "Empty accounts list should result in a vec with one empty string"
        );

        // Scenario: No message (subscription.next() returns None)
        let (client_no_msg, _) = create_test_client();
        let accounts_no_msg = client_no_msg.managed_accounts().expect("request managed accounts failed for no message");
        assert!(accounts_no_msg.is_empty(), "Accounts list should be empty when no message is received");
    }

    #[test]
    fn test_managed_accounts_retry() {
        // Test that managed_accounts retries on connection reset
        // Since our stub doesn't simulate actual connection resets, we'll test with valid responses
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::MANAGED_ACCOUNT.into(), // Successful response
        ]);

        let accounts = client.managed_accounts().expect("managed_accounts failed");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Accounts list mismatch");

        // Verify request was sent
        assert_request_messages(&message_bus, &["17|1|"]);
    }

    #[test]
    fn test_server_time() {
        use time::macros::datetime;

        // Scenario 1: Success
        let valid_timestamp_str = "1678890000"; // 2023-03-15 14:20:00 UTC
        let expected_datetime = datetime!(2023-03-15 14:20:00 UTC);
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("49|1|{}|", valid_timestamp_str), // IncomingMessages::CurrentTime
        ]);

        let result = client.server_time();
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");
        assert_request_messages(&message_bus, &["49|1|"]);

        // Scenario 2: No response (returns default)
        let (client_no_resp, message_bus_no_resp) = create_test_client();
        let result_no_resp = client_no_resp.server_time();
        assert!(result_no_resp.is_err(), "Expected Err for no response");
        match result_no_resp.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "No response from server"),
            other => panic!("Unexpected error type: {other:?}"),
        }
        assert_request_messages(&message_bus_no_resp, &["49|1|"]);

        // Scenario 3: Invalid timestamp format
        let (client_invalid, message_bus_invalid) = create_test_client_with_responses(vec!["49|1|not_a_timestamp|".into()]);
        let result_invalid = client_invalid.server_time();
        assert!(result_invalid.is_err(), "Expected Err for invalid timestamp");
        assert_request_messages(&message_bus_invalid, &["49|1|"]);
    }

    #[test]
    fn test_account_updates() {
        use crate::accounts::AccountUpdate;

        let account_name = AccountId(TEST_ACCOUNT.to_string());

        // Create client with account update responses
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("{}|", responses::ACCOUNT_VALUE), // AccountValue with trailing delimiter
            format!("54|1|{}|", TEST_ACCOUNT),        // AccountDownloadEnd
        ]);

        // Subscribe to account updates
        let subscription = client.account_updates(&account_name).expect("subscribe failed");

        // First update should be AccountValue
        let first_update = subscription.next();
        match first_update {
            Some(AccountUpdate::AccountValue(av)) => {
                assert_eq!(av.key, "CashBalance");
                assert_eq!(av.value, "1000.00");
                assert_eq!(av.currency, "USD");
            }
            other => panic!("First update was not AccountValue: {other:?}"),
        }

        // Second update should be End
        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(AccountUpdate::End)),
            "Expected AccountUpdate::End, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        // Verify request messages - subscribe and cancel
        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 2, "Expected subscribe and cancel messages");

        // First message should be subscribe (RequestAccountData = 6)
        assert!(request_messages[0].starts_with("6|"), "First message should be RequestAccountData");

        // Last message should be cancel
        let last_msg = &request_messages[request_messages.len() - 1];
        assert!(last_msg.starts_with("6|"), "Last message should be RequestAccountData (cancel)");
    }

    #[test]
    fn test_family_codes() {
        use crate::accounts::FamilyCode;

        // Scenario 1: Success with multiple codes
        let (client, message_bus) = create_test_client_with_responses(vec!["78|2|ACC1|FC1|ACC2|FC2|".into()]);

        let result = client.family_codes();
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
        let result_no_msg = client_no_msg.family_codes();
        assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
        assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_no_msg, &["80|1|"]);

        // Scenario 3: Empty family codes list
        let (client_empty, message_bus_empty) = create_test_client_with_responses(vec![
            "78|0|".into(), // Zero family codes
        ]);
        let result_empty = client_empty.family_codes();
        assert!(result_empty.is_ok(), "Expected Ok for empty list");
        assert!(result_empty.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_empty, &["80|1|"]);
    }

    #[test]
    fn test_account_updates_multi() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                responses::ACCOUNT_UPDATE_MULTI_CASH_BALANCE.into(),
                responses::ACCOUNT_UPDATE_MULTI_CURRENCY.into(),
                responses::ACCOUNT_UPDATE_MULTI_STOCK_MARKET_VALUE.into(),
                responses::ACCOUNT_UPDATE_MULTI_END.into(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let account = Some(AccountId("DU1234567".to_string()));
        let subscription = client
            .account_updates_multi(account.as_ref(), None)
            .expect("request managed accounts failed");

        let expected_keys = &["CashBalance", "Currency", "StockMarketValue"];

        for key in expected_keys {
            let update = subscription.next().unwrap();
            match update {
                AccountUpdateMulti::AccountMultiValue(value) => {
                    assert_eq!(value.key, *key);
                }
                AccountUpdateMulti::End => {
                    panic!("value expected")
                }
            }
        }

        let end = subscription.next().unwrap();
        assert_eq!(end, AccountUpdateMulti::End);

        subscription.cancel();

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "76|1|9000|DU1234567||1|");
        assert_eq!(request_messages[1].encode_simple(), "77|1|9000|"); // Cancel request
    }

    // Additional comprehensive tests for sync module

    #[test]
    fn test_server_version_errors() {
        use crate::server_versions;

        // Test PnL version check
        let (client_old, _) = create_test_client_with_version(server_versions::PNL - 1);
        let account = AccountId(TEST_ACCOUNT.to_string());
        let result = client_old.pnl(&account, None);
        assert!(result.is_err(), "Expected version error for PnL");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test PnL Single version check
        let (client_pnl_single, _) = create_test_client_with_version(server_versions::REALIZED_PNL - 1);
        let result = client_pnl_single.pnl_single(&account, ContractId(1001), None);
        assert!(result.is_err(), "Expected version error for PnL Single");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test Account Summary version check
        let (client_summary, _) = create_test_client_with_version(server_versions::ACCOUNT_SUMMARY - 1);
        let group = AccountGroup("All".to_string());
        let result = client_summary.account_summary(&group, &[AccountSummaryTags::ACCOUNT_TYPE]);
        assert!(result.is_err(), "Expected version error for Account Summary");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test Positions Multi version check
        let (client_multi, _) = create_test_client_with_version(server_versions::MODELS_SUPPORT - 1);
        let result = client_multi.positions_multi(Some(&account), None);
        assert!(result.is_err(), "Expected version error for Positions Multi");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test Account Updates Multi version check
        let result = client_multi.account_updates_multi(Some(&account), None);
        assert!(result.is_err(), "Expected version error for Account Updates Multi");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test Family Codes version check
        let (client_family, _) = create_test_client_with_version(server_versions::REQ_FAMILY_CODES - 1);
        let result = client_family.family_codes();
        assert!(result.is_err(), "Expected version error for Family Codes");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }

        // Test Positions version check
        let (client_positions, _) = create_test_client_with_version(server_versions::POSITIONS - 1);
        let result = client_positions.positions();
        assert!(result.is_err(), "Expected version error for Positions");
        if let Err(error) = result {
            assert!(matches!(error, Error::ServerVersion(_, _, _)));
        }
    }

    #[test]
    fn test_managed_accounts_additional_scenarios() {
        // Test single account response
        let (client_single, message_bus) = create_test_client_with_responses(vec!["15|1|SINGLE_ACCOUNT|".into()]);
        let accounts = client_single.managed_accounts().expect("managed_accounts failed");
        assert_eq!(accounts, vec!["SINGLE_ACCOUNT"], "Single account mismatch");
        assert_request_messages(&message_bus, &["17|1|"]);

        // Test multiple accounts with extra commas
        let (client_extra, message_bus_extra) = create_test_client_with_responses(vec!["15|1|ACC1,ACC2,|".into()]);
        let accounts_extra = client_extra.managed_accounts().expect("managed_accounts failed");
        assert_eq!(accounts_extra, vec!["ACC1", "ACC2", ""], "Extra comma handling failed");
        assert_request_messages(&message_bus_extra, &["17|1|"]);
    }

    #[test]
    fn test_server_time_comprehensive() {
        use time::macros::datetime;

        // Test edge case timestamps
        let edge_cases = vec![
            ("0", datetime!(1970-01-01 0:00 UTC)),         // Unix epoch
            ("946684800", datetime!(2000-01-01 0:00 UTC)), // Y2K
        ];

        for (timestamp_str, expected) in edge_cases {
            let (client, message_bus) = create_test_client_with_responses(vec![format!("49|1|{}|", timestamp_str)]);

            let result = client.server_time();
            assert!(result.is_ok(), "Expected Ok for timestamp {}", timestamp_str);
            assert_eq!(result.unwrap(), expected, "Timestamp {} mismatch", timestamp_str);
            assert_request_messages(&message_bus, &["49|1|"]);
        }

        // Test overflow timestamp
        let (client_overflow, message_bus_overflow) = create_test_client_with_responses(vec!["49|1|99999999999999999999|".into()]);
        let result_overflow = client_overflow.server_time();
        assert!(result_overflow.is_err(), "Expected error for overflow timestamp");
        assert_request_messages(&message_bus_overflow, &["49|1|"]);
    }

    #[test]
    fn test_account_summary_comprehensive() {
        use crate::accounts::AccountSummaryResult;

        // Test multiple tags
        let (client, message_bus) = create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

        let group = AccountGroup("All".to_string());
        let tags = &[
            AccountSummaryTags::ACCOUNT_TYPE,
            AccountSummaryTags::NET_LIQUIDATION,
            AccountSummaryTags::TOTAL_CASH_VALUE,
        ];

        let subscription = client.account_summary(&group, tags).expect("account_summary failed");

        // Should get at least one summary
        let first_update = subscription.next();
        assert!(matches!(first_update, Some(AccountSummaryResult::Summary(_))));

        // Should get end marker
        let second_update = subscription.next();
        assert!(matches!(second_update, Some(AccountSummaryResult::End)));

        drop(subscription);

        // Verify the encoded tags are sent correctly
        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages[0].contains("AccountType,NetLiquidation,TotalCashValue"),
            "Request should contain all tags"
        );

        // Test empty tags
        let (client_empty, _) = create_test_client();
        let group_empty = AccountGroup("All".to_string());
        let tags_empty: &[&str] = &[];

        let result = client_empty.account_summary(&group_empty, tags_empty);
        assert!(result.is_ok(), "Empty tags should be allowed");
    }

    #[test]
    fn test_pnl_comprehensive() {
        // Test different model codes and parameter combinations
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model1 = ModelCode("MODEL1".to_string());
        let model2 = ModelCode("MODEL2".to_string());

        // Request PnL with different model codes
        let sub1 = client.pnl(&account, Some(&model1)).expect("PnL request 1 failed");
        let sub2 = client.pnl(&account, Some(&model2)).expect("PnL request 2 failed");
        let sub3 = client.pnl(&account, None).expect("PnL request 3 failed");

        // Drop subscriptions to trigger cancellation messages in sync mode
        drop(sub1);
        drop(sub2);
        drop(sub3);

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= 6,
            "Expected at least 6 messages, got {}",
            request_messages.len()
        );

        // Verify model codes are encoded correctly
        let pnl_requests: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("92|")).collect();
        assert!(pnl_requests.len() >= 3, "Expected at least 3 PnL subscription messages");
        assert!(pnl_requests[0].contains("MODEL1"), "First request should contain MODEL1");
        assert!(pnl_requests[1].contains("MODEL2"), "Second request should contain MODEL2");
        assert!(pnl_requests[2].ends_with("||"), "Third request should have empty model code");
    }

    #[test]
    fn test_pnl_single_edge_cases() {
        // Test edge case contract IDs
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());

        // Test with zero contract ID
        let sub1 = client
            .pnl_single(&account, ContractId(0), None)
            .expect("PnL single with contract ID 0 failed");

        // Test with very large contract ID
        let sub2 = client
            .pnl_single(&account, ContractId(i32::MAX), None)
            .expect("PnL single with large contract ID failed");

        // Drop subscriptions to trigger cancellation in sync mode
        drop(sub1);
        drop(sub2);

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= 4,
            "Expected subscribe and cancel messages, got {}",
            request_messages.len()
        );

        // Verify contract IDs are encoded correctly
        assert!(request_messages[0].contains("|0|"), "First request should contain contract ID 0");

        // Find the second subscription message (not cancel message)
        let subscription_messages: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("94|")).collect();
        assert!(subscription_messages.len() >= 2, "Expected at least 2 subscription messages");
        assert!(
            subscription_messages[1].contains(&format!("|{}|", i32::MAX)),
            "Second request should contain max contract ID"
        );
    }

    #[test]
    fn test_positions_multi_parameter_combinations() {
        // Test all parameter combinations
        let (client, _) = create_test_client_with_responses(vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()]);

        let account = AccountId(TEST_ACCOUNT.to_string());
        let model = ModelCode(TEST_MODEL_CODE.to_string());

        // Test all parameter combinations
        let _sub1 = client
            .positions_multi(Some(&account), Some(&model))
            .expect("positions_multi with both params failed");

        let _sub2 = client
            .positions_multi(Some(&account), None)
            .expect("positions_multi with account only failed");

        let _sub3 = client
            .positions_multi(None, Some(&model))
            .expect("positions_multi with model only failed");

        let _sub4 = client.positions_multi(None, None).expect("positions_multi with no params failed");
    }

    #[test]
    fn test_subscription_lifecycle() {
        // Test that subscriptions are properly cleaned up when dropped
        let (client, message_bus) = create_test_client();

        let account = AccountId(TEST_ACCOUNT.to_string());

        // Create and immediately drop subscriptions
        {
            let _sub1 = client.pnl(&account, None).expect("PnL subscription failed");
            let _sub2 = client.positions().expect("Positions subscription failed");
            // Subscriptions dropped here
        }

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

    #[test]
    fn test_account_updates_stream_handling() {
        use crate::accounts::AccountUpdate;

        // Test continuous account updates stream
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("{}|", responses::ACCOUNT_VALUE),
            format!("54|1|{}|", TEST_ACCOUNT), // End marker
        ]);

        let account = AccountId(TEST_ACCOUNT.to_string());
        let subscription = client.account_updates(&account).expect("account_updates failed");

        let mut update_count = 0;
        for update_result in subscription {
            match update_result {
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

        // In sync mode, account_updates sends subscribe and unsubscribe messages
        let request_messages = get_request_messages(&message_bus);
        assert!(request_messages.len() >= 1, "Expected at least subscribe message");
        assert!(request_messages[0].starts_with("6|"), "First message should be RequestAccountData");
    }

    #[test]
    fn test_error_propagation() {
        // Test that encoding errors are properly propagated
        let (_client, _) = create_test_client();

        // These tests verify that version checks catch incompatible requests
        // The actual encoding shouldn't fail in normal circumstances since
        // we're using well-formed domain types, but version checks will prevent
        // sending requests to incompatible servers

        let account = AccountId(TEST_ACCOUNT.to_string());
        let old_version_client = create_test_client_with_version(50).0; // Very old version

        // All modern features should fail on old servers
        assert!(old_version_client.pnl(&account, None).is_err());
        assert!(old_version_client.positions().is_err());
        assert!(old_version_client.account_summary(&AccountGroup("All".to_string()), &[]).is_err());
    }
}
