use super::*;
use crate::common::test_utils::helpers::*;
use crate::testdata::builders::accounts::{
    account_download_end, account_summary, account_summary_end, account_update_multi, account_update_multi_end, account_value,
    cancel_account_summary, cancel_account_updates_multi, cancel_pnl, cancel_pnl_single, current_time, family_codes, managed_accounts,
    request_account_summary, request_account_updates, request_account_updates_multi, request_current_time, request_family_codes,
    request_managed_accounts, request_pnl, request_pnl_single,
};
use crate::testdata::builders::positions::{
    cancel_positions, cancel_positions_multi, position, position_end, position_multi, position_multi_end, request_positions, request_positions_multi,
};
use crate::testdata::builders::ResponseEncoder;

#[tokio::test]
async fn test_positions() {
    let (client, message_bus) = create_test_client_with_responses(vec![position().encode_pipe(), position_end().encode_pipe()]);

    let mut subscription = client.positions().await.expect("request positions failed");

    let first_update = subscription.next().await;
    assert!(
        matches!(first_update, Some(Ok(PositionUpdate::Position(_)))),
        "Expected PositionUpdate::Position, got {:?}",
        first_update
    );

    let second_update = subscription.next().await;
    assert!(
        matches!(second_update, Some(Ok(PositionUpdate::PositionEnd))),
        "Expected PositionUpdate::PositionEnd, got {:?}",
        second_update
    );

    drop(subscription); // Trigger cancellation
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 0, &request_positions());
    assert_request(&message_bus, 1, &cancel_positions());
}

#[tokio::test]
async fn test_positions_multi() {
    let (client, message_bus) = create_test_client_with_responses(vec![position_multi().encode_pipe(), position_multi_end().encode_pipe()]);

    let account = Some(AccountId(TEST_ACCOUNT.to_string()));
    let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));

    let mut subscription = client
        .positions_multi(account.as_ref(), model_code.as_ref())
        .await
        .expect("request positions_multi failed");

    let first_update = subscription.next().await;
    assert!(
        matches!(first_update, Some(Ok(PositionUpdateMulti::Position(_)))),
        "Expected PositionUpdateMulti::Position"
    );

    let second_update = subscription.next().await;
    assert!(
        matches!(second_update, Some(Ok(PositionUpdateMulti::PositionEnd))),
        "Expected PositionUpdateMulti::PositionEnd"
    );

    drop(subscription); // Trigger cancellation
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(
        &message_bus,
        0,
        &request_positions_multi()
            .request_id(TEST_REQ_ID_FIRST)
            .account(TEST_ACCOUNT)
            .model_code(TEST_MODEL_CODE),
    );
    assert_request(&message_bus, 1, &cancel_positions_multi().request_id(TEST_REQ_ID_FIRST));
}

#[tokio::test]
async fn test_account_summary() {
    let (client, message_bus) = create_test_client_with_responses(vec![account_summary().encode_pipe(), account_summary_end().encode_pipe()]);

    let group = AccountGroup("All".to_string());
    let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

    let mut subscription = client.account_summary(&group, tags).await.expect("request account_summary failed");

    let first_update = subscription.next().await;
    match first_update {
        Some(Ok(AccountSummaryResult::Summary(summary))) => {
            assert_eq!(summary.account, TEST_ACCOUNT);
            assert_eq!(summary.tag, AccountSummaryTags::ACCOUNT_TYPE);
            assert_eq!(summary.value, "FA");
        }
        _ => panic!("Expected AccountSummaryResult::Summary, got {first_update:?}"),
    }

    let second_update = subscription.next().await;
    assert!(
        matches!(second_update, Some(Ok(AccountSummaryResult::End))),
        "Expected AccountSummaryResult::End, got {:?}",
        second_update
    );

    drop(subscription); // Trigger cancellation
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

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

#[tokio::test]
async fn test_pnl() {
    let (client, message_bus) = create_test_client();

    let account = AccountId(TEST_ACCOUNT.to_string());
    let model_code = Some(ModelCode(TEST_MODEL_CODE.to_string()));

    let subscription1 = client.pnl(&account, model_code.as_ref()).await.expect("request pnl failed");
    drop(subscription1);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let subscription2 = client.pnl(&account, None).await.expect("request pnl failed");
    drop(subscription2);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert_eq!(request_message_count(&message_bus), 4);
    assert_request(&message_bus, 0, &request_pnl().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 1, &cancel_pnl().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 2, &request_pnl().request_id(TEST_REQ_ID_FIRST + 1).no_model_code());
    assert_request(&message_bus, 3, &cancel_pnl().request_id(TEST_REQ_ID_FIRST + 1));
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
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let subscription2 = client.pnl_single(&account, contract_id, None).await.expect("request pnl_single failed");
    drop(subscription2);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert_eq!(request_message_count(&message_bus), 4);
    assert_request(&message_bus, 0, &request_pnl_single().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 1, &cancel_pnl_single().request_id(TEST_REQ_ID_FIRST));
    assert_request(&message_bus, 2, &request_pnl_single().request_id(TEST_REQ_ID_FIRST + 1).no_model_code());
    assert_request(&message_bus, 3, &cancel_pnl_single().request_id(TEST_REQ_ID_FIRST + 1));
}

#[tokio::test]
async fn test_managed_accounts() {
    let (client, message_bus) = create_test_client_with_responses(vec![managed_accounts().accounts([TEST_ACCOUNT, TEST_ACCOUNT_2]).encode_pipe()]);

    let accounts = client.managed_accounts().await.expect("request managed accounts failed");
    assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Valid accounts list mismatch");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_managed_accounts());
}

#[tokio::test]
async fn test_managed_accounts_retry() {
    let (client, message_bus) = create_test_client_with_responses(vec![managed_accounts().accounts([TEST_ACCOUNT, TEST_ACCOUNT_2]).encode_pipe()]);

    let accounts = client.managed_accounts().await.expect("managed_accounts failed");
    assert_eq!(accounts, &[TEST_ACCOUNT, TEST_ACCOUNT_2], "Accounts list mismatch");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_managed_accounts());
}

#[tokio::test]
async fn test_server_time() {
    use time::macros::datetime;

    let expected_datetime = datetime!(2023-03-15 14:20:00 UTC);

    let (client, message_bus) = create_test_client_with_responses(vec![current_time().encode_pipe()]);

    let result = client.server_time().await;
    assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_current_time());
}

#[tokio::test]
async fn test_family_codes() {
    use crate::accounts::FamilyCode;

    // Scenario 1: Success with multiple codes
    let (client, message_bus) = create_test_client_with_responses(vec![family_codes().push("ACC1", "FC1").push("ACC2", "FC2").encode_pipe()]);

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
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &request_family_codes());

    // Scenario 2: No message received (returns empty vector)
    let (client_no_msg, message_bus_no_msg) = create_test_client();
    let result_no_msg = client_no_msg.family_codes().await;
    assert!(result_no_msg.is_ok(), "Expected Ok, got Err: {:?}", result_no_msg.err());
    assert!(result_no_msg.unwrap().is_empty(), "Expected empty vector");
    assert_eq!(request_message_count(&message_bus_no_msg), 1);
    assert_request(&message_bus_no_msg, 0, &request_family_codes());

    // Scenario 3: Empty family codes list
    let (client_empty, message_bus_empty) = create_test_client_with_responses(vec![family_codes().encode_pipe()]);
    let result_empty = client_empty.family_codes().await;
    assert!(result_empty.is_ok(), "Expected Ok for empty list");
    assert!(result_empty.unwrap().is_empty(), "Expected empty vector");
    assert_eq!(request_message_count(&message_bus_empty), 1);
    assert_request(&message_bus_empty, 0, &request_family_codes());
}

#[tokio::test]
async fn test_account_updates() {
    use crate::accounts::AccountUpdate;

    let account_name = AccountId(TEST_ACCOUNT.to_string());

    let (client, message_bus) = create_test_client_with_responses(vec![account_value().encode_pipe(), account_download_end().encode_pipe()]);

    let mut subscription = client.account_updates(&account_name).await.expect("subscribe failed");

    let first_update = subscription.next().await;
    match first_update {
        Some(Ok(AccountUpdate::AccountValue(av))) => {
            assert_eq!(av.key, "CashBalance");
            assert_eq!(av.value, "1000.00");
            assert_eq!(av.currency, "USD");
        }
        other => panic!("First update was not AccountValue: {other:?}"),
    }

    let second_update = subscription.next().await;
    assert!(
        matches!(second_update, Some(Ok(AccountUpdate::End))),
        "Expected AccountUpdate::End, got {:?}",
        second_update
    );

    drop(subscription); // Trigger cancellation
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert!(request_message_count(&message_bus) >= 2, "Expected subscribe and cancel messages");
    assert_request(&message_bus, 0, &request_account_updates().account(TEST_ACCOUNT));
}

#[tokio::test]
async fn test_account_updates_multi() {
    let (client, message_bus) = create_test_client_with_responses(vec![
        account_update_multi().key("CashBalance").value("94629.71").currency("USD").encode_pipe(),
        account_update_multi().key("Currency").value("USD").currency("USD").encode_pipe(),
        account_update_multi()
            .key("StockMarketValue")
            .value("0.00")
            .currency("BASE")
            .encode_pipe(),
        account_update_multi_end().encode_pipe(),
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
        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(&message_bus, 0, &request_managed_accounts());
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
                match result.unwrap_err() {
                    Error::Parse(_, _, _) | Error::ParseInt(_) | Error::Simple(_) => {}
                    other => panic!("Expected Parse, ParseInt, or Simple error for {}, got: {:?}", test_case.scenario, other),
                }
            }
        }

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(&message_bus, 0, &request_current_time());
    }
}

#[tokio::test]
async fn test_concurrent_subscriptions() {
    let (client, message_bus) = create_test_client();

    let account1 = AccountId("ACCOUNT1".to_string());
    let account2 = AccountId("ACCOUNT2".to_string());

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

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert!(
        request_message_count(&message_bus) >= 6,
        "Expected at least 6 messages (3 subscribe + 3 cancel)"
    );
}

#[tokio::test]
async fn test_account_summary_multiple_tags() {
    use super::common::test_tables::account_summary_tag_test_cases;

    let test_cases = account_summary_tag_test_cases();

    for test_case in test_cases {
        let group = AccountGroup(test_case.group.clone());

        if test_case.expect_responses {
            let (client, message_bus) = create_test_client_with_responses(vec![account_summary().encode_pipe(), account_summary_end().encode_pipe()]);

            let mut subscription = client
                .account_summary(&group, &test_case.tags)
                .await
                .unwrap_or_else(|_| panic!("account_summary failed for {}", test_case.description));

            let first_update = subscription.next().await;
            assert!(
                matches!(first_update, Some(Ok(AccountSummaryResult::Summary(_)))),
                "Expected summary for {}",
                test_case.description
            );

            let second_update = subscription.next().await;
            assert!(
                matches!(second_update, Some(Ok(AccountSummaryResult::End))),
                "Expected end marker for {}",
                test_case.description
            );

            drop(subscription);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            if test_case.expected_tag_encoding.is_some() {
                assert!(
                    request_message_count(&message_bus) > 0,
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
            }
        } else {
            let (client, _) = create_test_client();

            let result = client.account_summary(&group, &test_case.tags).await;
            assert!(result.is_ok(), "account_summary should succeed for {}", test_case.description);
        }
    }
}
