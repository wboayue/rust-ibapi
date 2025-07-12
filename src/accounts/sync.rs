//! Synchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::{ClientRequestBuilders, DataStream, ResponseContext, SharesChannel, Subscription};
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::{Client, Error};

use super::common::{decoders, encoders, errors, helpers};
use super::types::{AccountGroup, AccountId, ContractId, ModelCode};
use super::*;

// Implement SharesChannel for PositionUpdate subscription
impl SharesChannel for Subscription<'_, PositionUpdate> {}

// Implement DataStream traits for the account types
impl DataStream<AccountSummaries> for AccountSummaries {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaries::Summary(decoders::decode_account_summary(
                client.server_version,
                message,
            )?)),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaries::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel account summary")?;
        encoders::encode_cancel_account_summary(request_id)
    }
}

impl DataStream<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(client.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel pnl")?;
        encoders::encode_cancel_pnl(request_id)
    }
}

impl DataStream<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(client.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel pnl single")?;
        encoders::encode_cancel_pnl_single(request_id)
    }
}

impl DataStream<PositionUpdate> for PositionUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionUpdate::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionUpdate::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

impl DataStream<PositionUpdateMulti> for PositionUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::PositionMulti => Ok(PositionUpdateMulti::Position(decoders::decode_position_multi(message)?)),
            IncomingMessages::PositionMultiEnd => Ok(PositionUpdateMulti::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel positions multi")?;
        encoders::encode_cancel_positions_multi(request_id)
    }
}

impl DataStream<AccountUpdate> for AccountUpdate {
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
                client.server_version,
                message,
            )?)),
            IncomingMessages::AccountUpdateTime => Ok(AccountUpdate::UpdateTime(decoders::decode_account_update_time(message)?)),
            IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, _request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_account_updates(server_version)
    }
}

impl DataStream<AccountUpdateMulti> for AccountUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountUpdateMulti, IncomingMessages::AccountUpdateMultiEnd];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountUpdateMulti => Ok(AccountUpdateMulti::AccountMultiValue(decoders::decode_account_multi_value(message)?)),
            IncomingMessages::AccountUpdateMultiEnd => Ok(AccountUpdateMulti::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = errors::require_request_id_for(request_id, "encode cancel account updates multi")?;
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}

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

pub fn account_summary<'a>(client: &'a Client, group: &AccountGroup, tags: &[&str]) -> Result<Subscription<'a, AccountSummaries>, Error> {
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
        use crate::accounts::AccountSummaries;

        let (client, message_bus) = create_test_client_with_responses(vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()]);

        let group = AccountGroup("All".to_string());
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let subscription = client.account_summary(&group, tags).expect("request account_summary failed");

        let first_update = subscription.next();
        match first_update {
            Some(AccountSummaries::Summary(summary_data)) => {
                assert_eq!(summary_data.account, TEST_ACCOUNT); // From responses::ACCOUNT_SUMMARY
                assert_eq!(summary_data.tag, AccountSummaryTags::ACCOUNT_TYPE);
                assert_eq!(summary_data.value, "FA");
            }
            _ => panic!("Expected AccountSummaries::Summary, got {first_update:?}"),
        }

        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(AccountSummaries::End)),
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
}
