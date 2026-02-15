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

/// Subscribe to streaming position updates for all accessible accounts.
///
/// The stream first replays the full position list and then sends incremental updates.
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    crate::common::request_helpers::blocking::shared_subscription(
        client,
        Features::POSITIONS,
        OutgoingMessages::RequestPositions,
        encoders::encode_request_positions,
    )
}

/// Subscribe to streaming position updates scoped by account and model code.
///
/// Requires [Features::MODELS_SUPPORT] to be available on the connected gateway.
pub fn positions_multi(
    client: &Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<PositionUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_positions_multi(builder.request_id(), account, model_code)?;

    builder.send(request)
}

/// Fetch the account family codes registered with the broker.
pub fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    crate::common::request_helpers::blocking::one_shot_request(
        client,
        Features::FAMILY_CODES,
        OutgoingMessages::RequestFamilyCodes,
        encoders::encode_request_family_codes,
        decoders::decode_family_codes,
        Vec::default,
    )
}

/// Subscribe to real-time daily and unrealized PnL updates for an account.
///
/// Optionally filter by model code to scope the updates.
pub fn pnl(client: &Client, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Subscription<PnL>, Error> {
    crate::common::request_helpers::blocking::request_with_id(client, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code))
}

/// Subscribe to real-time daily PnL updates for a single contract.
///
/// The stream includes realized and unrealized PnL information for the requested position.
pub fn pnl_single(
    client: &Client,
    account: &AccountId,
    contract_id: ContractId,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<PnLSingle>, Error> {
    crate::common::request_helpers::blocking::request_with_id(client, Features::REALIZED_PNL, |id| {
        encoders::encode_request_pnl_single(id, account, contract_id, model_code)
    })
}

/// Subscribe to account summary updates for a group of accounts.
pub fn account_summary(client: &Client, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<AccountSummaryResult>, Error> {
    crate::common::request_helpers::blocking::request_with_id(client, Features::ACCOUNT_SUMMARY, |id| {
        encoders::encode_request_account_summary(id, group, tags)
    })
}

/// Subscribe to detailed account updates for a specific account.
pub fn account_updates(client: &Client, account: &AccountId) -> Result<Subscription<AccountUpdate>, Error> {
    crate::common::request_helpers::blocking::shared_request(client, OutgoingMessages::RequestAccountData, || {
        encoders::encode_request_account_updates(client.server_version(), account)
    })
}

/// Subscribe to account updates scoped by account and model code.
///
/// Requires [Features::MODELS_SUPPORT] to be available on the connected gateway.
pub fn account_updates_multi(
    client: &Client,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Subscription<AccountUpdateMulti>, Error> {
    check_version(client.server_version(), Features::MODELS_SUPPORT)?;

    let builder = client.request();
    let request = encoders::encode_request_account_updates_multi(builder.request_id(), account, model_code)?;

    builder.send(request)
}

/// Fetch the list of accounts accessible to the current user.
pub fn managed_accounts(client: &Client) -> Result<Vec<String>, Error> {
    crate::common::request_helpers::blocking::one_shot_with_retry(
        client,
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
}

/// Query the current server time reported by TWS or IB Gateway.
pub fn server_time(client: &Client) -> Result<OffsetDateTime, Error> {
    crate::common::request_helpers::blocking::one_shot_with_retry(
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

/// Query the current server time in milliseconds reported by TWS or IB Gateway.
pub fn server_time_millis(client: &Client) -> Result<OffsetDateTime, Error> {
    check_version(client.server_version, Features::CURRENT_TIME_IN_MILLIS)?;

    crate::common::request_helpers::blocking::one_shot_with_retry(
        client,
        OutgoingMessages::RequestCurrentTimeInMillis,
        encoders::encode_request_server_time_millis,
        decoders::decode_server_time_millis,
        || Err(Error::Simple("No response from server".to_string())),
    )
}

#[cfg(test)]
mod tests {
    use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
    use crate::accounts::{AccountSummaryTags, AccountUpdateMulti};
    use crate::testdata::responses;
    use crate::{client::blocking::Client, server_versions, stubs::MessageBusStub, Error};
    use std::sync::{Arc, RwLock};

    use crate::common::test_utils::helpers::*;

    #[test]
    fn test_pnl() {
        let (client, message_bus) = create_blocking_test_client();

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
        let (client, message_bus) = create_blocking_test_client();

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

        let (client, message_bus) = create_blocking_test_client_with_responses(vec![responses::POSITION.into(), responses::POSITION_END.into()]);

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

        let (client, message_bus) =
            create_blocking_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

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
        let (client, _) = create_blocking_test_client_with_responses(vec![responses::MANAGED_ACCOUNT.into()]);
        let accounts = client.managed_accounts().expect("request managed accounts failed for valid response");
        assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");

        // Scenario: Empty response string
        let (client_empty, _) = create_blocking_test_client_with_responses(vec!["17|1||".to_string()]); // Empty accounts string
        let accounts_empty = client_empty
            .managed_accounts()
            .expect("request managed accounts failed for empty response");
        assert!(accounts_empty.is_empty(), "Empty accounts list should result in empty vec");

        // Scenario: No message (subscription.next() returns None)
        let (client_no_msg, _) = create_blocking_test_client();
        let accounts_no_msg = client_no_msg.managed_accounts().expect("request managed accounts failed for no message");
        assert!(accounts_no_msg.is_empty(), "Accounts list should be empty when no message is received");
    }

    #[test]
    fn test_managed_accounts_retry() {
        // Test that managed_accounts retries on connection reset
        // Since our stub doesn't simulate actual connection resets, we'll test with valid responses
        let (client, message_bus) = create_blocking_test_client_with_responses(vec![
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
        let (client, message_bus) = create_blocking_test_client_with_responses(vec![
            format!("49|1|{}|", valid_timestamp_str), // IncomingMessages::CurrentTime
        ]);

        let result = client.server_time();
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");
        assert_request_messages(&message_bus, &["49|1|"]);

        // Scenario 2: No response (returns default)
        let (client_no_resp, message_bus_no_resp) = create_blocking_test_client();
        let result_no_resp = client_no_resp.server_time();
        assert!(result_no_resp.is_err(), "Expected Err for no response");
        match result_no_resp.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "No response from server"),
            other => panic!("Unexpected error type: {other:?}"),
        }
        assert_request_messages(&message_bus_no_resp, &["49|1|"]);

        // Scenario 3: Invalid timestamp format
        let (client_invalid, message_bus_invalid) = create_blocking_test_client_with_responses(vec!["49|1|not_a_timestamp|".into()]);
        let result_invalid = client_invalid.server_time();
        assert!(result_invalid.is_err(), "Expected Err for invalid timestamp");
        assert_request_messages(&message_bus_invalid, &["49|1|"]);
    }

    #[test]
    fn test_account_updates() {
        use crate::accounts::AccountUpdate;

        let account_name = AccountId(TEST_ACCOUNT.to_string());

        // Create client with account update responses
        let (client, message_bus) = create_blocking_test_client_with_responses(vec![
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
        let (client, message_bus) = create_blocking_test_client_with_responses(vec!["78|2|ACC1|FC1|ACC2|FC2|".into()]);

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
        let (client_no_msg, message_bus_no_msg) = create_blocking_test_client();
        let result_no_msg = client_no_msg.family_codes();
        assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
        assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_no_msg, &["80|1|"]);

        // Scenario 3: Empty family codes list
        let (client_empty, message_bus_empty) = create_blocking_test_client_with_responses(vec![
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
        use super::common::test_tables::VERSION_TEST_CASES;

        let account = AccountId(TEST_ACCOUNT.to_string());
        let group = AccountGroup("All".to_string());

        for test_case in VERSION_TEST_CASES {
            let (client, _) = create_blocking_test_client_with_version(test_case.required_version - 1);

            let result = match test_case.function_name {
                "PnL" => client.pnl(&account, None).map(|_| ()),
                "PnL Single" => client.pnl_single(&account, ContractId(1001), None).map(|_| ()),
                "Account Summary" => client.account_summary(&group, &[AccountSummaryTags::ACCOUNT_TYPE]).map(|_| ()),
                "Positions Multi" => client.positions_multi(Some(&account), None).map(|_| ()),
                "Account Updates Multi" => client.account_updates_multi(Some(&account), None).map(|_| ()),
                "Family Codes" => client.family_codes().map(|_| ()),
                "Positions" => client.positions().map(|_| ()),
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

    #[test]
    fn test_managed_accounts_additional_scenarios() {
        use super::common::test_tables::managed_accounts_test_cases;

        for test_case in managed_accounts_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_blocking_test_client()
            } else {
                create_blocking_test_client_with_responses(test_case.responses)
            };

            let accounts = client
                .managed_accounts()
                .unwrap_or_else(|_| panic!("managed_accounts failed for {}", test_case.scenario));
            assert_eq!(accounts, test_case.expected, "{}: {}", test_case.scenario, test_case.description);
            assert_request_messages(&message_bus, &["17|1|"]);
        }
    }

    #[test]
    fn test_server_time_comprehensive() {
        use super::common::test_tables::server_time_test_cases;

        for test_case in server_time_test_cases() {
            let (client, message_bus) = if test_case.responses.is_empty() {
                create_blocking_test_client()
            } else {
                create_blocking_test_client_with_responses(test_case.responses)
            };

            let result = client.server_time();

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

    #[test]
    fn test_account_summary_comprehensive() {
        use super::common::test_tables::account_summary_tag_test_cases;
        use crate::accounts::AccountSummaryResult;

        let test_cases = account_summary_tag_test_cases();

        for test_case in test_cases {
            let group = AccountGroup(test_case.group.clone());

            if test_case.expect_responses {
                // Create client with mock responses for tests that expect data
                let (client, message_bus) =
                    create_blocking_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

                let subscription = client
                    .account_summary(&group, &test_case.tags)
                    .unwrap_or_else(|_| panic!("account_summary failed for {}", test_case.description));

                // Should get at least one summary
                let first_update = subscription.next();
                assert!(
                    matches!(first_update, Some(AccountSummaryResult::Summary(_))),
                    "Expected summary for {}",
                    test_case.description
                );

                // Should get end marker
                let second_update = subscription.next();
                assert!(
                    matches!(second_update, Some(AccountSummaryResult::End)),
                    "Expected end marker for {}",
                    test_case.description
                );

                drop(subscription);

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
                // For tests that don't expect responses (like empty tags)
                let (client, _) = create_blocking_test_client();
                let result = client.account_summary(&group, &test_case.tags);

                if test_case.should_succeed {
                    assert!(result.is_ok(), "Expected success for {}, got: {:?}", test_case.description, result.err());
                } else {
                    assert!(result.is_err(), "Expected failure for {}", test_case.description);
                }
            }
        }
    }

    #[test]
    fn test_pnl_comprehensive() {
        use super::common::test_tables::pnl_parameter_test_cases;

        let test_cases = pnl_parameter_test_cases();
        let (client, message_bus) = create_blocking_test_client();
        let account = AccountId(TEST_ACCOUNT.to_string());
        let mut subscriptions = Vec::new();

        // Create all subscriptions
        for test_case in &test_cases {
            let model_code = test_case.model_code.as_ref().map(|s| ModelCode(s.clone()));
            let sub = client
                .pnl(&account, model_code.as_ref())
                .unwrap_or_else(|_| panic!("PnL request failed for {}", test_case.description));
            subscriptions.push(sub);
        }

        // Drop subscriptions to trigger cancellation messages in sync mode
        drop(subscriptions);

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= test_cases.len() * 2,
            "Expected at least {} messages (subscribe + cancel for each), got {}",
            test_cases.len() * 2,
            request_messages.len()
        );

        // Verify model codes are encoded correctly
        let pnl_requests: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("92|")).collect();

        assert_eq!(
            pnl_requests.len(),
            test_cases.len(),
            "Expected {} PnL subscription messages",
            test_cases.len()
        );

        for (i, test_case) in test_cases.iter().enumerate() {
            if test_case.expected_pattern == "||" {
                assert!(
                    pnl_requests[i].ends_with("||"),
                    "Request {} should end with empty model code for {}",
                    i,
                    test_case.description
                );
            } else {
                assert!(
                    pnl_requests[i].contains(test_case.expected_pattern),
                    "Request {} should contain {} for {}",
                    i,
                    test_case.expected_pattern,
                    test_case.description
                );
            }
        }
    }

    #[test]
    fn test_pnl_single_edge_cases() {
        use super::common::test_tables::contract_id_test_cases;

        let test_cases = contract_id_test_cases();
        let (client, message_bus) = create_blocking_test_client();
        let account = AccountId(TEST_ACCOUNT.to_string());
        let mut subscriptions = Vec::new();

        // Create all subscriptions
        for test_case in &test_cases {
            let sub = client
                .pnl_single(&account, test_case.contract_id, None)
                .unwrap_or_else(|_| panic!("PnL single failed for {}", test_case.description));
            subscriptions.push(sub);
        }

        // Drop all subscriptions to trigger cancellation
        drop(subscriptions);

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

    #[test]
    fn test_positions_multi_parameter_combinations() {
        use super::common::test_tables::positions_multi_parameter_test_cases;

        let test_cases = positions_multi_parameter_test_cases();
        let (client, message_bus) =
            create_blocking_test_client_with_responses(vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()]);
        let mut subscriptions = Vec::new();

        // Create all subscriptions
        for test_case in &test_cases {
            let account = test_case.account.as_ref().map(|s| AccountId(s.clone()));
            let model_code = test_case.model_code.as_ref().map(|s| ModelCode(s.clone()));

            let sub = client
                .positions_multi(account.as_ref(), model_code.as_ref())
                .unwrap_or_else(|_| panic!("positions_multi failed for {}", test_case.description));
            subscriptions.push(sub);
        }

        // Drop subscriptions to trigger cancellation messages in sync mode
        drop(subscriptions);

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= test_cases.len() * 2,
            "Expected at least {} messages (subscribe + cancel for each)",
            test_cases.len() * 2
        );

        // Verify subscription messages are correct
        let subscription_messages: Vec<_> = request_messages.iter().filter(|msg| msg.starts_with("74|")).collect();

        assert_eq!(
            subscription_messages.len(),
            test_cases.len(),
            "Expected {} subscription messages",
            test_cases.len()
        );

        // In sync mode, we can verify exact message patterns since request IDs are predictable
        for (i, test_case) in test_cases.iter().enumerate() {
            let message = subscription_messages[i];

            // Verify message starts with positions_multi opcode
            assert!(message.starts_with("74|"), "Message should start with positions_multi opcode");

            // Check account parameter presence
            if let Some(expected_account) = &test_case.account {
                assert!(
                    message.contains(expected_account),
                    "Message should contain account {} for {}",
                    expected_account,
                    test_case.description
                );
            }

            // Check model code parameter presence
            if let Some(expected_model) = &test_case.model_code {
                assert!(
                    message.contains(expected_model),
                    "Message should contain model code {} for {}",
                    expected_model,
                    test_case.description
                );
            }
        }
    }

    #[test]
    fn test_subscription_lifecycle() {
        use super::common::test_tables::{subscription_lifecycle_test_cases, SubscriptionType};

        let test_cases = subscription_lifecycle_test_cases();
        let (client, message_bus) = create_blocking_test_client();

        // Test each subscription type individually to avoid lifetime issues
        for test_case in &test_cases {
            match &test_case.subscription_type {
                SubscriptionType::PnL { account, model_code } => {
                    let account_id = AccountId(account.clone());
                    let model = model_code.as_ref().map(|s| ModelCode(s.clone()));
                    let sub = client
                        .pnl(&account_id, model.as_ref())
                        .unwrap_or_else(|_| panic!("PnL subscription failed for {}", test_case.description));
                    drop(sub); // Trigger cancellation immediately
                }
                SubscriptionType::Positions => {
                    let sub = client
                        .positions()
                        .unwrap_or_else(|_| panic!("Positions subscription failed for {}", test_case.description));
                    drop(sub); // Trigger cancellation immediately
                }
                SubscriptionType::AccountSummary { group, tags } => {
                    let group_id = AccountGroup(group.clone());
                    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                    let sub = client
                        .account_summary(&group_id, &tag_refs)
                        .unwrap_or_else(|_| panic!("Account Summary subscription failed for {}", test_case.description));
                    drop(sub); // Trigger cancellation immediately
                }
                SubscriptionType::PositionsMulti { account, model_code } => {
                    let account_id = account.as_ref().map(|s| AccountId(s.clone()));
                    let model = model_code.as_ref().map(|s| ModelCode(s.clone()));
                    let sub = client
                        .positions_multi(account_id.as_ref(), model.as_ref())
                        .unwrap_or_else(|_| panic!("Positions Multi subscription failed for {}", test_case.description));
                    drop(sub); // Trigger cancellation immediately
                }
                SubscriptionType::PnLSingle {
                    account,
                    contract_id,
                    model_code,
                } => {
                    let account_id = AccountId(account.clone());
                    let contract = ContractId(*contract_id);
                    let model = model_code.as_ref().map(|s| ModelCode(s.clone()));
                    let sub = client
                        .pnl_single(&account_id, contract, model.as_ref())
                        .unwrap_or_else(|_| panic!("PnL Single subscription failed for {}", test_case.description));
                    drop(sub); // Trigger cancellation immediately
                }
            }
        }

        let request_messages = get_request_messages(&message_bus);
        assert!(
            request_messages.len() >= test_cases.len() * 2,
            "Expected subscribe and cancel messages for {} subscriptions, got {} messages",
            test_cases.len(),
            request_messages.len()
        );

        // Verify subscription and cancellation patterns
        for test_case in &test_cases {
            let subscribe_count = request_messages
                .iter()
                .filter(|msg| msg.starts_with(test_case.expected_subscribe_pattern))
                .count();

            let cancel_count = request_messages
                .iter()
                .filter(|msg| msg.starts_with(test_case.expected_cancel_pattern))
                .count();

            assert!(
                subscribe_count >= 1,
                "Expected at least 1 subscribe message with pattern '{}' for {}",
                test_case.expected_subscribe_pattern,
                test_case.description
            );

            assert!(
                cancel_count >= 1,
                "Expected at least 1 cancel message with pattern '{}' for {}",
                test_case.expected_cancel_pattern,
                test_case.description
            );
        }
    }

    #[test]
    fn test_account_updates_stream_handling() {
        use crate::accounts::AccountUpdate;

        // Test continuous account updates stream
        let (client, message_bus) = create_blocking_test_client_with_responses(vec![
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
        assert!(!request_messages.is_empty(), "Expected at least subscribe message");
        assert!(request_messages[0].starts_with("6|"), "First message should be RequestAccountData");
    }

    #[test]
    fn test_error_propagation() {
        // Test that encoding errors are properly propagated
        let (_client, _) = create_blocking_test_client();

        // These tests verify that version checks catch incompatible requests
        // The actual encoding shouldn't fail in normal circumstances since
        // we're using well-formed domain types, but version checks will prevent
        // sending requests to incompatible servers

        let account = AccountId(TEST_ACCOUNT.to_string());
        let old_version_client = create_blocking_test_client_with_version(50).0; // Very old version

        // All modern features should fail on old servers
        assert!(old_version_client.pnl(&account, None).is_err());
        assert!(old_version_client.positions().is_err());
        assert!(old_version_client.account_summary(&AccountGroup("All".to_string()), &[]).is_err());
    }
}
