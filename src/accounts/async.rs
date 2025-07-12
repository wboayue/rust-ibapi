//! Asynchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{AsyncDataStream, Subscription};
use crate::{Client, Error};

use super::common::{decoders, encoders, errors, helpers::async_helpers};
use super::*;

// Implement AsyncDataStream traits for the account types
impl AsyncDataStream<AccountSummaries> for AccountSummaries {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaries::Summary(decoders::decode_account_summary(
                client.server_version(),
                message,
            )?)),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaries::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = errors::require_request_id(request_id)?;
        encoders::encode_cancel_account_summary(request_id)
    }
}

impl AsyncDataStream<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(client.server_version(), message)
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = errors::require_request_id(request_id)?;
        encoders::encode_cancel_pnl(request_id)
    }
}

impl AsyncDataStream<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(client.server_version(), message)
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = errors::require_request_id(request_id)?;
        encoders::encode_cancel_pnl_single(request_id)
    }
}

impl AsyncDataStream<PositionUpdate> for PositionUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionUpdate::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionUpdate::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        _server_version: i32,
        _request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

impl AsyncDataStream<PositionUpdateMulti> for PositionUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::PositionMulti => Ok(PositionUpdateMulti::Position(decoders::decode_position_multi(message)?)),
            IncomingMessages::PositionMultiEnd => Ok(PositionUpdateMulti::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = errors::require_request_id(request_id)?;
        encoders::encode_cancel_positions_multi(request_id)
    }
}

impl AsyncDataStream<AccountUpdate> for AccountUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::AccountValue,
        IncomingMessages::PortfolioValue,
        IncomingMessages::AccountUpdateTime,
        IncomingMessages::AccountDownloadEnd,
    ];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountValue => Ok(AccountUpdate::AccountValue(decoders::decode_account_value(message)?)),
            IncomingMessages::PortfolioValue => Ok(AccountUpdate::PortfolioValue(decoders::decode_account_portfolio_value(
                client.server_version(),
                message,
            )?)),
            IncomingMessages::AccountUpdateTime => Ok(AccountUpdate::UpdateTime(decoders::decode_account_update_time(message)?)),
            IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        server_version: i32,
        _request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        encoders::encode_cancel_account_updates(server_version)
    }
}

impl AsyncDataStream<AccountUpdateMulti> for AccountUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountUpdateMulti, IncomingMessages::AccountUpdateMultiEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountUpdateMulti => Ok(AccountUpdateMulti::AccountMultiValue(decoders::decode_account_multi_value(message)?)),
            IncomingMessages::AccountUpdateMultiEnd => Ok(AccountUpdateMulti::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel account updates multi")?;
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}

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

pub async fn positions_multi(client: &Client, account: Option<&str>, model_code: Option<&str>) -> Result<Subscription<PositionUpdateMulti>, Error> {
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
pub async fn pnl(client: &Client, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
    async_helpers::request_with_id(client, Features::PNL, |id| encoders::encode_request_pnl(id, account, model_code)).await
}

// Requests real time updates for daily PnL of individual positions.
//
// # Arguments
// * `client` - Client
// * `account` - Account in which position exists
// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// * `model_code` - Model in which position exists
pub async fn pnl_single(client: &Client, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<Subscription<PnLSingle>, Error> {
    async_helpers::request_with_id(client, Features::REALIZED_PNL, |id| {
        encoders::encode_request_pnl_single(id, account, contract_id, model_code)
    })
    .await
}

pub async fn account_summary(client: &Client, group: &str, tags: &[&str]) -> Result<Subscription<AccountSummaries>, Error> {
    async_helpers::request_with_id(client, Features::ACCOUNT_SUMMARY, |id| {
        encoders::encode_request_account_summary(id, group, tags)
    })
    .await
}

pub async fn account_updates(client: &Client, account: &str) -> Result<Subscription<AccountUpdate>, Error> {
    async_helpers::shared_request(client, OutgoingMessages::RequestAccountData, || {
        encoders::encode_request_account_updates(client.server_version(), account)
    })
    .await
}

pub async fn account_updates_multi(
    client: &Client,
    account: Option<&str>,
    model_code: Option<&str>,
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
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::POSITION.into(), 
            responses::POSITION_END.into()
        ]);

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
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::POSITION_MULTI.into(), 
            responses::POSITION_MULTI_END.into()
        ]);

        let account = Some(TEST_ACCOUNT);
        let model_code = Some(TEST_MODEL_CODE);

        let mut subscription = positions_multi(&client, account, model_code)
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
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::ACCOUNT_SUMMARY.into(), 
            responses::ACCOUNT_SUMMARY_END.into()
        ]);

        let group = "All";
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let mut subscription = account_summary(&client, group, tags).await.expect("request account_summary failed");

        // First update should be a summary
        let first_update = subscription.next().await;
        match first_update {
            Some(Ok(AccountSummaries::Summary(summary))) => {
                assert_eq!(summary.account, TEST_ACCOUNT);
                assert_eq!(summary.tag, AccountSummaryTags::ACCOUNT_TYPE);
                assert_eq!(summary.value, "FA");
            }
            _ => panic!("Expected AccountSummaries::Summary, got {first_update:?}"),
        }

        // Second update should be end
        let second_update = subscription.next().await;
        assert!(
            matches!(second_update, Some(Ok(AccountSummaries::End))),
            "Expected AccountSummaries::End, got {:?}",
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

        let account = TEST_ACCOUNT;
        let model_code = Some(TEST_MODEL_CODE);

        let subscription1 = pnl(&client, account, model_code).await.expect("request pnl failed");
        drop(subscription1);
        
        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let subscription2 = pnl(&client, account, None).await.expect("request pnl failed");
        drop(subscription2);
        
        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_request_messages(&message_bus, &[
            "92|9000|DU1234567|TARGET2024|", 
            "93|9000|",
            "92|9001|DU1234567||", 
            "93|9001|"
        ]);
    }

    #[tokio::test]
    async fn test_pnl_single() {
        let (client, message_bus) = create_test_client();

        let account = TEST_ACCOUNT;
        let contract_id = TEST_CONTRACT_ID;
        let model_code = Some(TEST_MODEL_CODE);

        let subscription1 = pnl_single(&client, account, contract_id, model_code)
            .await
            .expect("request pnl_single failed");
        drop(subscription1);
        
        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let subscription2 = pnl_single(&client, account, contract_id, None).await.expect("request pnl_single failed");
        drop(subscription2);
        
        // Allow time for async cancellation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_request_messages(&message_bus, &[
            "94|9000|DU1234567|TARGET2024|1001|",
            "95|9000|",
            "94|9001|DU1234567||1001|",
            "95|9001|"
        ]);
    }

    #[tokio::test]
    async fn test_managed_accounts() {
        let (client, message_bus) = create_test_client_with_responses(vec![
            responses::MANAGED_ACCOUNT.into()
        ]);

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

        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("49|1|{}|", valid_timestamp_str)
        ]);

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
            "78|0|".into() // Zero family codes
        ]);
        let result_empty = family_codes(&client_empty).await;
        assert!(result_empty.is_ok(), "Expected Ok for empty list");
        assert!(result_empty.unwrap().is_empty(), "Expected empty vector");
        assert_request_messages(&message_bus_empty, &["80|1|"]);
    }

    #[tokio::test]
    async fn test_account_updates() {
        use crate::accounts::AccountUpdate;

        let account_name = TEST_ACCOUNT;

        // Create client with account update responses
        let (client, message_bus) = create_test_client_with_responses(vec![
            format!("{}|", responses::ACCOUNT_VALUE), // AccountValue with trailing delimiter
            format!("54|1|{}|", account_name),        // AccountDownloadEnd
        ]);

        // Subscribe to account updates
        let mut subscription = account_updates(&client, account_name).await.expect("subscribe failed");
        
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

        let account = Some(TEST_ACCOUNT);
        let mut subscription = account_updates_multi(&client, account, None)
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
        assert_request_messages(&message_bus, &[
            "76|1|9000|DU1234567||1|",
            "77|1|9000|" // Cancel request
        ]);
    }
}
