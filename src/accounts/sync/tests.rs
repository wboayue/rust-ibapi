use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
use crate::accounts::{AccountSummaryTags, AccountUpdateMulti};
use crate::common::test_utils::helpers::*;
use crate::messages::OutgoingMessages;
use crate::testdata::builders::accounts::{
    account_download_end, account_summary, account_summary_end, account_update_multi, account_update_multi_end, account_value,
    cancel_account_summary, cancel_account_updates, cancel_account_updates_multi, cancel_pnl, cancel_pnl_single, current_time, family_codes,
    managed_accounts, request_account_summary, request_account_updates, request_account_updates_multi, request_current_time, request_family_codes,
    request_managed_accounts, request_pnl, request_pnl_single,
};
use crate::testdata::builders::positions::{
    cancel_positions, cancel_positions_multi, position, position_end, position_multi, position_multi_end, request_positions, request_positions_multi,
};
use crate::testdata::builders::ResponseEncoder;
use crate::{client::blocking::Client, server_versions, stubs::MessageBusStub, Error};
use std::sync::{Arc, RwLock};

#[test]
fn test_pnl() {
    let (client, message_bus) = create_blocking_test_client();

    let account = AccountId(TEST_ACCOUNT.to_string());
    let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));
    let _ = client.pnl(&account, model_code.as_ref()).expect("request pnl failed");
    let _ = client.pnl(&account, None).expect("request pnl failed");

    assert_eq!(request_message_count(&message_bus), 4);
    assert_request(&message_bus, 0, &request_pnl().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 1, &cancel_pnl().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 2, &request_pnl().request_id(TEST_REQ_ID_FIRST + 1).no_model_code());
    assert_request(&message_bus, 3, &cancel_pnl().request_id(TEST_REQ_ID_FIRST + 1));
}

#[test]
fn test_pnl_single() {
    let (client, message_bus) = create_blocking_test_client();

    let account = AccountId(TEST_ACCOUNT.to_string());
    let contract_id = ContractId(TEST_CONTRACT_ID);
    let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));
    let _ = client.pnl_single(&account, contract_id, model_code.as_ref()).expect("request pnl failed");
    let _ = client.pnl_single(&account, contract_id, None).expect("request pnl failed");

    assert_eq!(request_message_count(&message_bus), 4);
    assert_request(&message_bus, 0, &request_pnl_single().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 1, &cancel_pnl_single().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 2, &request_pnl_single().request_id(TEST_REQ_ID_FIRST + 1).no_model_code());
    assert_request(&message_bus, 3, &cancel_pnl_single().request_id(TEST_REQ_ID_FIRST + 1));
}

#[test]
fn test_positions() {
    use crate::accounts::PositionUpdate;

    let (client, message_bus) = create_blocking_test_client_with_responses(vec![position().encode_pipe(), position_end().encode_pipe()]);

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

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 0, &request_positions());
    assert_request(&message_bus, 1, &cancel_positions());
}

#[test]
fn test_positions_multi() {
    use crate::accounts::PositionUpdateMulti;

    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![position_multi().encode_pipe(), position_multi_end().encode_pipe()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let account = Some(AccountId(TEST_ACCOUNT.to_string()));
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

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(
        &message_bus,
        0,
        &request_positions_multi()
            .request_id(TEST_REQ_ID_FIRST)
            .account(TEST_ACCOUNT)
            .model_code("TARGET2024"),
    );
    assert_request(&message_bus, 1, &cancel_positions_multi().request_id(TEST_REQ_ID_FIRST));
}

#[test]
fn test_account_summary() {
    use crate::accounts::AccountSummaryResult;

    let (client, message_bus) =
        create_blocking_test_client_with_responses(vec![account_summary().encode_pipe(), account_summary_end().encode_pipe()]);

    let group = AccountGroup("All".to_string());
    let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

    let subscription = client.account_summary(&group, tags).expect("request account_summary failed");

    let first_update = subscription.next();
    match first_update {
        Some(AccountSummaryResult::Summary(summary_data)) => {
            assert_eq!(summary_data.account, TEST_ACCOUNT);
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

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(
        &message_bus,
        0,
        &request_account_summary()
            .request_id(TEST_REQ_ID_FIRST)
            .group("All")
            .tags([AccountSummaryTags::ACCOUNT_TYPE]),
    );
    assert_request(&message_bus, 1, &cancel_account_summary().request_id(TEST_REQ_ID_FIRST));
}

#[test]
fn test_managed_accounts() {
    // Scenario: Valid response
    let (client, message_bus) =
        create_blocking_test_client_with_responses(vec![managed_accounts().accounts([TEST_ACCOUNT, TEST_ACCOUNT_2]).encode_pipe()]);
    let accounts = client.managed_accounts().expect("request managed accounts failed for valid response");
    assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");
    assert_request(&message_bus, 0, &request_managed_accounts());

    // Scenario: Empty response string
    let (client_empty, _) = create_blocking_test_client_with_responses(vec![managed_accounts().accounts(Vec::<String>::new()).encode_pipe()]);
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
    // Test that managed_accounts retries on connection reset.
    // Since our stub doesn't simulate actual connection resets, we exercise the happy path.
    let (client, message_bus) =
        create_blocking_test_client_with_responses(vec![managed_accounts().accounts([TEST_ACCOUNT, TEST_ACCOUNT_2]).encode_pipe()]);

    let accounts = client.managed_accounts().expect("managed_accounts failed");
    assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Accounts list mismatch");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_managed_accounts());
}

#[test]
fn test_server_time() {
    use time::macros::datetime;

    // Scenario 1: Success
    let expected_datetime = datetime!(2023-03-15 14:20:00 UTC);
    let (client, message_bus) = create_blocking_test_client_with_responses(vec![current_time().encode_pipe()]);

    let result = client.server_time();
    assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_current_time());

    // Scenario 2: No response (returns default)
    let (client_no_resp, message_bus_no_resp) = create_blocking_test_client();
    let result_no_resp = client_no_resp.server_time();
    assert!(result_no_resp.is_err(), "Expected Err for no response");
    match result_no_resp.err().unwrap() {
        Error::Simple(msg) => assert_eq!(msg, "No response from server"),
        other => panic!("Unexpected error type: {other:?}"),
    }
    assert_eq!(request_message_count(&message_bus_no_resp), 1);
    assert_request(&message_bus_no_resp, 0, &request_current_time());

    // Scenario 3: Invalid timestamp format
    let (client_invalid, message_bus_invalid) = create_blocking_test_client_with_responses(vec!["49|1|not_a_timestamp|".into()]);
    let result_invalid = client_invalid.server_time();
    assert!(result_invalid.is_err(), "Expected Err for invalid timestamp");
    assert_eq!(request_message_count(&message_bus_invalid), 1);
    assert_request(&message_bus_invalid, 0, &request_current_time());
}

#[test]
fn test_account_updates() {
    use crate::accounts::AccountUpdate;

    let account_name = AccountId(TEST_ACCOUNT.to_string());

    let (client, message_bus) = create_blocking_test_client_with_responses(vec![account_value().encode_pipe(), account_download_end().encode_pipe()]);

    let subscription = client.account_updates(&account_name).expect("subscribe failed");

    let first_update = subscription.next();
    match first_update {
        Some(AccountUpdate::AccountValue(av)) => {
            assert_eq!(av.key, "CashBalance");
            assert_eq!(av.value, "1000.00");
            assert_eq!(av.currency, "USD");
        }
        other => panic!("First update was not AccountValue: {other:?}"),
    }

    let second_update = subscription.next();
    assert!(
        matches!(second_update, Some(AccountUpdate::End)),
        "Expected AccountUpdate::End, got {:?}",
        second_update
    );

    drop(subscription); // Trigger cancellation

    assert!(request_message_count(&message_bus) >= 2, "Expected subscribe and cancel messages");
    assert_request(&message_bus, 0, &request_account_updates().account(TEST_ACCOUNT));
    assert_request(&message_bus, 1, &cancel_account_updates());
}

#[test]
fn test_family_codes() {
    use crate::accounts::FamilyCode;

    // Scenario 1: Success with multiple codes
    let (client, message_bus) =
        create_blocking_test_client_with_responses(vec![family_codes().push("ACC1", "FC1").push("ACC2", "FC2").encode_pipe()]);

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
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_family_codes());

    // Scenario 2: No message received (returns empty vector)
    let (client_no_msg, message_bus_no_msg) = create_blocking_test_client();
    let result_no_msg = client_no_msg.family_codes();
    assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
    assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
    assert_eq!(request_message_count(&message_bus_no_msg), 1);
    assert_request(&message_bus_no_msg, 0, &request_family_codes());

    // Scenario 3: Empty family codes list
    let (client_empty, message_bus_empty) = create_blocking_test_client_with_responses(vec![family_codes().encode_pipe()]);
    let result_empty = client_empty.family_codes();
    assert!(result_empty.is_ok(), "Expected Ok for empty list");
    assert!(result_empty.unwrap().is_empty(), "Expected empty vector");
    assert_eq!(request_message_count(&message_bus_empty), 1);
    assert_request(&message_bus_empty, 0, &request_family_codes());
}

#[test]
fn test_account_updates_multi() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            account_update_multi().key("CashBalance").value("94629.71").currency("USD").encode_pipe(),
            account_update_multi().key("Currency").value("USD").currency("USD").encode_pipe(),
            account_update_multi()
                .key("StockMarketValue")
                .value("0.00")
                .currency("BASE")
                .encode_pipe(),
            account_update_multi_end().encode_pipe(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let account = Some(AccountId(TEST_ACCOUNT.to_string()));
    let subscription = client
        .account_updates_multi(account.as_ref(), None)
        .expect("request account updates multi failed");

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

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(
        &message_bus,
        0,
        &request_account_updates_multi()
            .request_id(TEST_REQ_ID_FIRST)
            .account(TEST_ACCOUNT)
            .ledger_and_nlv(true),
    );
    assert_request(&message_bus, 1, &cancel_account_updates_multi().request_id(TEST_REQ_ID_FIRST));
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
        assert_eq!(request_message_count(&message_bus), 1);
        assert_request_msg_id(&message_bus, 0, OutgoingMessages::RequestManagedAccounts);
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

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request_msg_id(&message_bus, 0, OutgoingMessages::RequestCurrentTime);
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
            let (client, message_bus) =
                create_blocking_test_client_with_responses(vec![account_summary().encode_pipe(), account_summary_end().encode_pipe()]);

            let subscription = client
                .account_summary(&group, &test_case.tags)
                .unwrap_or_else(|_| panic!("account_summary failed for {}", test_case.description));

            let first_update = subscription.next();
            assert!(
                matches!(first_update, Some(AccountSummaryResult::Summary(_))),
                "Expected summary for {}",
                test_case.description
            );

            let second_update = subscription.next();
            assert!(
                matches!(second_update, Some(AccountSummaryResult::End)),
                "Expected end marker for {}",
                test_case.description
            );

            drop(subscription);

            assert!(
                request_message_count(&message_bus) >= 1,
                "Expected request messages for {}",
                test_case.description
            );
            assert_request(
                &message_bus,
                0,
                &request_account_summary()
                    .request_id(TEST_REQ_ID_FIRST)
                    .group(test_case.group.clone())
                    .tags(test_case.tags.clone()),
            );
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

    let count = request_message_count(&message_bus);
    assert!(
        count >= test_cases.len() * 2,
        "Expected at least {} messages (subscribe + cancel for each), got {}",
        test_cases.len() * 2,
        count
    );
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

    let count = request_message_count(&message_bus);
    assert!(
        count >= test_cases.len() * 2,
        "Expected at least {} messages (subscribe + cancel for each)",
        test_cases.len() * 2
    );
}

#[test]
fn test_positions_multi_parameter_combinations() {
    use super::common::test_tables::positions_multi_parameter_test_cases;

    let test_cases = positions_multi_parameter_test_cases();
    let (client, message_bus) = create_blocking_test_client_with_responses(vec![position_multi().encode_pipe(), position_multi_end().encode_pipe()]);
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

    let count = request_message_count(&message_bus);
    assert!(
        count >= test_cases.len() * 2,
        "Expected at least {} messages (subscribe + cancel for each)",
        test_cases.len() * 2
    );
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
                drop(sub);
            }
            SubscriptionType::Positions => {
                let sub = client
                    .positions()
                    .unwrap_or_else(|_| panic!("Positions subscription failed for {}", test_case.description));
                drop(sub);
            }
            SubscriptionType::AccountSummary { group, tags } => {
                let group_id = AccountGroup(group.clone());
                let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                let sub = client
                    .account_summary(&group_id, &tag_refs)
                    .unwrap_or_else(|_| panic!("Account Summary subscription failed for {}", test_case.description));
                drop(sub);
            }
            SubscriptionType::PositionsMulti { account, model_code } => {
                let account_id = account.as_ref().map(|s| AccountId(s.clone()));
                let model = model_code.as_ref().map(|s| ModelCode(s.clone()));
                let sub = client
                    .positions_multi(account_id.as_ref(), model.as_ref())
                    .unwrap_or_else(|_| panic!("Positions Multi subscription failed for {}", test_case.description));
                drop(sub);
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
                drop(sub);
            }
        }
    }

    let count = request_message_count(&message_bus);
    assert!(
        count >= test_cases.len() * 2,
        "Expected subscribe and cancel messages for {} subscriptions, got {} messages",
        test_cases.len(),
        count
    );
}

#[test]
fn test_account_updates_stream_handling() {
    use crate::accounts::AccountUpdate;

    let (client, message_bus) = create_blocking_test_client_with_responses(vec![
        account_value().encode_pipe(),
        account_value().encode_pipe(),
        account_value().encode_pipe(),
        account_download_end().encode_pipe(),
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

    assert!(request_message_count(&message_bus) >= 1, "Expected at least subscribe message");
    assert_request(&message_bus, 0, &request_account_updates().account(TEST_ACCOUNT));
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
