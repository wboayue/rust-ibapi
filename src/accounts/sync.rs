//! Synchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::{DataStream, ResponseContext, SharesChannel, Subscription};
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::{server_versions, Client, Error};

use super::decoders;
use super::encoders;
use super::types::*;

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
        if let Some(request_id) = request_id {
            encoders::encode_cancel_account_summary(request_id)
        } else {
            Err(Error::Simple("Request ID required to encode cancel account summary".to_string()))
        }
    }
}

impl DataStream<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(client.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel pnl");
        encoders::encode_cancel_pnl(request_id)
    }
}

impl DataStream<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(client.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel pnl single");
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
        let request_id = request_id.expect("Request ID required to encode cancel positions multi");
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
        let request_id = request_id.expect("Request ID required to encode cancel account updates multi");
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    use crate::client::subscription_builder::SubscriptionBuilderExt;

    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let request = encoders::encode_request_positions()?;

    client
        .subscription::<PositionUpdate>()
        .send_shared(OutgoingMessages::RequestPositions, request)
}

pub fn positions_multi<'a>(
    client: &'a Client,
    account: Option<&str>,
    model_code: Option<&str>,
) -> Result<Subscription<'a, PositionUpdateMulti>, Error> {
    use crate::client::subscription_builder::SubscriptionBuilderExt;

    client.check_server_version(server_versions::MODELS_SUPPORT, "It does not support positions multi requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_positions_multi(request_id, account, model_code)?;

    client.subscription::<PositionUpdateMulti>().send_with_request_id(request_id, request)
}

// Determine whether an account exists under an account family and find the account family code.
pub fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    client.check_server_version(server_versions::REQ_FAMILY_CODES, "It does not support family codes requests.")?;

    let request = encoders::encode_request_family_codes()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestFamilyCodes, request)?;

    // TODO: enumerate
    if let Some(Ok(mut message)) = subscription.next() {
        decoders::decode_family_codes(&mut message)
    } else {
        Ok(Vec::default())
    }
}

// Creates subscription for real time daily PnL and unrealized PnL updates
//
// # Arguments
// * `client`     - client
// * `account`    - account for which to receive PnL updates
// * `model_code` - specify to request PnL updates for a specific model
pub fn pnl<'a>(client: &'a Client, account: &str, model_code: Option<&str>) -> Result<Subscription<'a, PnL>, Error> {
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl(request_id, account, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
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
    account: &str,
    contract_id: i32,
    model_code: Option<&str>,
) -> Result<Subscription<'a, PnLSingle>, Error> {
    client.check_server_version(server_versions::REALIZED_PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl_single(request_id, account, contract_id, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub fn account_summary<'a>(client: &'a Client, group: &str, tags: &[&str]) -> Result<Subscription<'a, AccountSummaries>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support account summary requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_account_summary(request_id, group, tags)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub fn account_updates<'a>(client: &'a Client, account: &str) -> Result<Subscription<'a, AccountUpdate>, Error> {
    let request = encoders::encode_request_account_updates(client.server_version(), account)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAccountData, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub fn account_updates_multi<'a>(
    client: &'a Client,
    account: Option<&str>,
    model_code: Option<&str>,
) -> Result<Subscription<'a, AccountUpdateMulti>, Error> {
    client.check_server_version(server_versions::MODELS_SUPPORT, "It does not support account updates multi requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_account_updates_multi(request_id, account, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub fn managed_accounts(client: &Client) -> Result<Vec<String>, Error> {
    let request = encoders::encode_request_managed_accounts()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestManagedAccounts, request)?;

    match subscription.next() {
        Some(Ok(mut message)) => {
            message.skip(); // message type
            message.skip(); // message version

            let accounts = message.next_string()?;
            Ok(accounts.split(",").map(String::from).collect())
        }
        Some(Err(Error::ConnectionReset)) => managed_accounts(client),
        Some(Err(e)) => Err(e),
        None => Ok(Vec::default()),
    }
}

pub fn server_time(client: &Client) -> Result<OffsetDateTime, Error> {
    let request = encoders::encode_request_server_time()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestCurrentTime, request)?;

    match subscription.next() {
        Some(Ok(mut message)) => {
            message.skip(); // message type
            message.skip(); // message version

            let timestamp = message.next_long()?;
            match OffsetDateTime::from_unix_timestamp(timestamp) {
                Ok(date) => Ok(date),
                Err(e) => Err(Error::Simple(format!("Error parsing date: {e}"))),
            }
        }
        Some(Err(Error::ConnectionReset)) => server_time(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::Simple("No response from server".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use crate::accounts::AccountUpdateMulti;
    use crate::testdata::responses;
    use crate::{accounts::AccountSummaryTags, server_versions, stubs::MessageBusStub, Client};
    use crate::{Error, ToField};

    #[test]
    fn test_pnl() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let account = "DU1234567";
        let model_code = Some("TARGET2024");

        let _ = client.pnl(account, model_code).expect("request pnl failed");
        let _ = client.pnl(account, None).expect("request pnl failed");

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "92|9000|DU1234567|TARGET2024|");
        assert_eq!(request_messages[1].encode_simple(), "93|9000|");

        assert_eq!(request_messages[2].encode_simple(), "92|9001|DU1234567||");
        assert_eq!(request_messages[3].encode_simple(), "93|9001|");
    }

    #[test]
    fn test_pnl_single() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let account = "DU1234567";
        let contract_id = 1001;
        let model_code = Some("TARGET2024");

        let _ = client.pnl_single(account, contract_id, model_code).expect("request pnl failed");
        let _ = client.pnl_single(account, contract_id, None).expect("request pnl failed");

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "94|9000|DU1234567|TARGET2024|1001|");
        assert_eq!(request_messages[1].encode_simple(), "95|9000|");

        assert_eq!(request_messages[2].encode_simple(), "94|9001|DU1234567||1001|");
        assert_eq!(request_messages[3].encode_simple(), "95|9001|");
    }

    #[test]
    fn test_positions() {
        use crate::accounts::PositionUpdate;

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::POSITION.into(), responses::POSITION_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

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

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages.len(), 2, "Expected subscribe and cancel messages for positions");
        assert_eq!(request_messages[0].encode_simple(), "61|1|"); // Subscribe
                                                                  // For `positions()`, the cancel message is `OutgoingMessages::CancelPositions` (type 62)
                                                                  // The `RequestMessage` for this is built by `encoders::encode_cancel_positions()`.
                                                                  // It sends: type (62), version (1).
                                                                  // The `Subscription` cancel logic for `shared_request` (which `positions` uses)
                                                                  // will call `T::cancel_message`. `PositionUpdate::cancel_message` calls `encode_cancel_positions`.
        assert_eq!(request_messages[1].encode_simple(), "64|1|"); // Verifying CancelPositions
    }

    #[test]
    fn test_positions_multi() {
        use crate::accounts::PositionUpdateMulti;

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = Some("DU1234567");
        let model_code = Some("TARGET2024");

        let subscription = client.positions_multi(account, model_code).expect("request positions_multi failed");

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

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let group = "All";
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let subscription = client.account_summary(group, tags).expect("request account_summary failed");

        let first_update = subscription.next();
        match first_update {
            Some(AccountSummaries::Summary(summary_data)) => {
                assert_eq!(summary_data.account, "DU1234567"); // From responses::ACCOUNT_SUMMARY
                assert_eq!(summary_data.tag, AccountSummaryTags::ACCOUNT_TYPE);
                assert_eq!(summary_data.value, "FA");
            }
            _ => panic!("Expected AccountSummaries::Summary, got {:?}", first_update),
        }

        let second_update = subscription.next();
        assert!(
            matches!(second_update, Some(AccountSummaries::End)),
            "Expected AccountSummaries::End, got {:?}",
            second_update
        );

        drop(subscription); // Trigger cancellation

        let request_messages = message_bus.request_messages.read().unwrap();

        assert_eq!(request_messages.len(), 2, "Expected subscribe and cancel messages for account_summary");
        assert_eq!(request_messages[0].encode_simple(), "62|1|9000|All|AccountType|");
        assert_eq!(request_messages[1].encode_simple(), "63|1|9000|"); // Verifying CancelAccountSummary
    }

    #[test]
    #[ignore] // refactor for new stub
    fn test_managed_accounts() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::MANAGED_ACCOUNT.into()],
        });
        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let accounts = client.managed_accounts().expect("request managed accounts failed for valid response");
        assert_eq!(accounts, &["DU1234567", "DU7654321"], "Valid accounts list mismatch");

        // Scenario: Empty response string
        let message_bus_empty = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["7|1|0|0".to_string()], // Message Type 7, Version 1, Empty accounts string
        });
        let client_empty = Client::stubbed(message_bus_empty, server_versions::SIZE_RULES);
        let accounts_empty = client_empty
            .managed_accounts()
            .expect("request managed accounts failed for empty response");
        // Based on String::split behavior, an empty string results in a vec with one empty string.
        // If an empty Vec is desired, the parsing logic in `decode_managed_accounts` would need adjustment.
        assert_eq!(
            accounts_empty,
            vec![""],
            "Empty accounts list should result in a vec with one empty string"
        );

        // Scenario: No message (subscription.next() returns None)
        let message_bus_no_msg = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client_no_msg = Client::stubbed(message_bus_no_msg, server_versions::SIZE_RULES);
        let accounts_no_msg = client_no_msg.managed_accounts().expect("request managed accounts failed for no message");
        assert!(accounts_no_msg.is_empty(), "Accounts list should be empty when no message is received");

        // Scenario: Error response
        let message_bus_err = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["Test Managed Account Error".into()],
        });
        let client_err = Client::stubbed(message_bus_err, server_versions::SIZE_RULES);
        let result_err = client_err.managed_accounts();
        assert!(result_err.is_err(), "Expected error for error response scenario");
        match result_err.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "Test Managed Account Error", "Error message mismatch for managed accounts"),
            other_err => panic!("Unexpected error type for managed accounts: {:?}", other_err),
        }
    }

    #[test]
    #[ignore] // refactor for new stub
    fn test_managed_accounts_retry_connection_reset() {
        use crate::messages::OutgoingMessages;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let message_bus_retry = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "Test reset".to_string(),               // First attempt
                responses::MANAGED_ACCOUNT.to_string(), // Second attempt
            ],
        });

        let client_retry = Client::stubbed(message_bus_retry.clone(), server_versions::SIZE_RULES);
        let accounts_retry = client_retry.managed_accounts().expect("managed_accounts failed after retry");

        assert_eq!(accounts_retry, &["DU1234567", "DU7654321"], "Accounts list mismatch after retry");
        assert_eq!(call_count.load(Ordering::SeqCst), 2, "Expected two calls to the message bus for retry");

        // Verify that the request was sent twice (though RequestMessage doesn't implement Clone for direct comparison easily)
        // We can check the count of sent messages of the correct type.
        let sent_requests = message_bus_retry.request_messages.read().unwrap();
        let managed_account_req_count = sent_requests
            .iter()
            .filter(|req_msg| req_msg[0] == OutgoingMessages::RequestManagedAccounts.to_field())
            .count();
        assert_eq!(managed_account_req_count, 2, "RequestManagedAccounts should have been sent twice");
    }

    #[test]
    #[ignore] // refactor for new stub
    fn test_server_time_integration() {
        use crate::Error;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use time::macros::datetime;

        // Scenario 1: Success
        let valid_timestamp_str = "1678886400"; // 2023-03-15 12:00:00 UTC
        let expected_datetime = datetime!(2023-03-15 12:00:00 UTC);
        let message_bus_s1 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![format!("4\x001\x00{}\x00", valid_timestamp_str).into()],
        });
        let client_s1 = Client::stubbed(message_bus_s1, server_versions::SIZE_RULES);
        let result_s1 = client_s1.server_time();
        assert!(result_s1.is_ok(), "S1: Expected Ok, got Err: {:?}", result_s1.err());
        assert_eq!(result_s1.unwrap(), expected_datetime, "S1: DateTime mismatch");

        // Scenario 2: No response
        let message_bus_s2 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![], // No message
        });
        let client_s2 = Client::stubbed(message_bus_s2, server_versions::SIZE_RULES);
        let result_s2 = client_s2.server_time();
        assert!(result_s2.is_err(), "S2: Expected Err, got Ok: {:?}", result_s2.ok());
        match result_s2.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "No response from server", "S2: Error message mismatch"),
            other => panic!("S2: Unexpected error type: {:?}", other),
        }

        // Scenario 3: Error response from TWS
        let message_bus_s3 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["Test TWS Error".into()],
        });
        let client_s3 = Client::stubbed(message_bus_s3, server_versions::SIZE_RULES);
        let result_s3 = client_s3.server_time();
        assert!(result_s3.is_err(), "S3: Expected Err, got Ok: {:?}", result_s3.ok());
        match result_s3.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "Test TWS Error", "S3: Error message mismatch"),
            other => panic!("S3: Unexpected error type: {:?}", other),
        }

        // Scenario 4: Retry on ConnectionReset
        let call_count_s4 = Arc::new(AtomicUsize::new(0));
        let message_bus_s4 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "Simulated reset".into(), // Simulate connection reset
                format!("4\x001\x00{}\x00", valid_timestamp_str),
            ],
        });
        let client_s4 = Client::stubbed(message_bus_s4.clone(), server_versions::SIZE_RULES);
        let result_s4 = client_s4.server_time();
        assert!(result_s4.is_ok(), "S4: Expected Ok after retry, got Err: {:?}", result_s4.err());
        assert_eq!(result_s4.unwrap(), expected_datetime, "S4: DateTime mismatch after retry");
        assert_eq!(call_count_s4.load(Ordering::SeqCst), 2, "S4: Expected 2 calls for retry");

        // Scenario 5: Invalid timestamp (unparsable long)
        let message_bus_s5_unparsable = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["4\x001\x00not_a_long\x00".into()], // Invalid timestamp
        });
        let client_s5_unparsable = Client::stubbed(message_bus_s5_unparsable, server_versions::SIZE_RULES);
        let result_s5_unparsable = client_s5_unparsable.server_time();
        assert!(
            result_s5_unparsable.is_err(),
            "S5 Unparsable: Expected Err, got Ok: {:?}",
            result_s5_unparsable.ok()
        );
        // Depending on how decode_server_time handles parsing errors, the exact error type might vary.
        // For now, let's check if it's an Error::Decode.
        match result_s5_unparsable.err().unwrap() {
            Error::Simple(field) => assert_eq!(field, "server_time", "S5 Unparsable: Error field mismatch"),
            other => panic!("S5 Unparsable: Unexpected error type: {:?}", other),
        }

        // Scenario 5b: Invalid timestamp (out of range for OffsetDateTime, e.g., year 10000)
        // OffsetDateTime::from_unix_timestamp can fail for out-of-range values.
        // A very large number that's a valid i64 but out of typical date range.
        let out_of_range_timestamp_str = "253402300800"; // Year 10000, should be out of range for OffsetDateTime typically
        let message_bus_s5_range = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![format!("4\x001\x00{}\x00", out_of_range_timestamp_str)],
        });
        let client_s5_range = Client::stubbed(message_bus_s5_range, server_versions::SIZE_RULES);
        let result_s5_range = client_s5_range.server_time();
        assert!(result_s5_range.is_err(), "S5 Range: Expected Err, got Ok: {:?}", result_s5_range.ok());
        match result_s5_range.err().unwrap() {
            Error::Simple(field) => assert_eq!(field, "server_time", "S5 Range: Error field mismatch (likely time conversion)"),
            other => panic!("S5 Range: Unexpected error type: {:?}", other),
        }
    }

    #[test]
    #[ignore] // refactor for new stub
    fn test_account_updates_flow() {
        use crate::accounts::AccountUpdate;

        let account_name_to_subscribe = "TestAccount123";

        // Assemble
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // 1. Valid AccountValue
                responses::ACCOUNT_VALUE.to_string(),
                // 2. Valid PortfolioValue - Assuming responses::PORTFOLIO_VALUE is a complete message string
                // responses::PORTFOLIO_VALUE.to_string(),
                // 3. Valid AccountUpdateTime
                "8|1|10:20:30|".into(), // Type 8, Ver 1, Time "10:20:30"
                // 4. AccountDownloadEnd
                format!("16|1|{}|", account_name_to_subscribe), // Type 16, Ver 1, AccountName
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::ACCOUNT_SUMMARY); // Use a server version that supports account in cancel

        // Act
        let subscription = client.account_updates(account_name_to_subscribe).expect("subscribe failed");
        let mut updates_received = Vec::new();
        for update_result in &subscription {
            // Iterating over &subscription
            updates_received.push(update_result);
        }

        // Assert
        assert_eq!(updates_received.len(), 4, "Expected four updates to be received");

        // 1. AccountValue
        match &updates_received[0] {
            AccountUpdate::AccountValue(av) => {
                assert_eq!(av.key, "TotalCashBalance", "AccountValue.key");
                assert_eq!(av.value, "1000.00", "AccountValue.value");
                assert_eq!(av.currency, "USD", "AccountValue.currency");
                assert_eq!(av.account.as_deref(), Some("TestAccount"), "AccountValue.account_name");
            }
            other => panic!("First update was not AccountValue: {:?}", other),
        }

        // 2. PortfolioValue
        match &updates_received[1] {
            AccountUpdate::PortfolioValue(pv) => {
                assert_eq!(pv.contract.symbol, "AAPL", "PortfolioValue.contract.symbol");
                assert_eq!(pv.position, 100.0, "PortfolioValue.position");
            }
            other => panic!("Second update was not PortfolioValue: {:?}", other),
        }

        // 3. UpdateTime
        match &updates_received[2] {
            AccountUpdate::UpdateTime(ut) => {
                assert_eq!(ut.timestamp, "10:20:30", "UpdateTime.timestamp");
            }
            other => panic!("Third update was not UpdateTime: {:?}", other),
        }

        // 4. End
        match &updates_received[3] {
            AccountUpdate::End => { /* Correct */ }
            other => panic!("Fourth update was not End: {:?}", other),
        }

        // Verify cancellation message
        // Drop the subscription to trigger the cancel message
        drop(subscription);

        let request_messages = message_bus.request_messages.read().unwrap();
        // The first message is the subscription request, the second should be the cancel request.
        assert!(request_messages.len() >= 2, "Expected at least two messages (subscribe and cancel)");

        let cancel_message_found = request_messages.iter().any(|req_msg| {
            if req_msg[0] == crate::messages::OutgoingMessages::RequestAccountData.to_field() {
                // Version for cancel is 1 if server_version < ACCOUNT_SUMMARY (10)
                // Version for cancel is 2 if server_version >= ACCOUNT_SUMMARY (10)
                // Subscribe field (index 2) should be false ("0")
                // Account field (index 3, only if server_version >= 10) should be account_name_to_subscribe

                let version_field = req_msg[1].to_string();
                let subscribe_field = req_msg[2].to_string();
                let account_field_for_cancel = req_msg[3].to_string();

                let expected_version_for_cancel = if client.server_version() < server_versions::ACCOUNT_SUMMARY {
                    "1"
                } else {
                    "2"
                };
                let correct_version = version_field == expected_version_for_cancel.to_string();
                let correct_subscribe_flag = subscribe_field == "0";

                let correct_account_field = if client.server_version() >= server_versions::ACCOUNT_SUMMARY {
                    account_field_for_cancel == account_name_to_subscribe
                } else {
                    account_field_for_cancel == "".to_string() // No account field for older server versions on cancel
                };

                correct_version && correct_subscribe_flag && correct_account_field
            } else {
                false
            }
        });
        assert!(cancel_message_found, "Cancel account updates message not found or incorrect");
    }

    #[test]
    #[ignore] // refactor for new stub
    fn test_family_codes_integration() {
        use crate::accounts::FamilyCode;
        use crate::Error;

        // Scenario 1: Success with multiple codes
        let message_bus_s1 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["78|2|ACC1|FC1|ACC2|FC2|".into()],
        });
        let client_s1 = Client::stubbed(message_bus_s1, server_versions::SIZE_RULES);
        let result_s1 = client_s1.family_codes();
        assert!(result_s1.is_ok(), "Scenario 1: Expected Ok, got Err: {:?}", result_s1.err());
        let codes_s1 = result_s1.unwrap();
        assert_eq!(codes_s1.len(), 2, "Scenario 1: Expected 2 family codes");
        assert_eq!(
            codes_s1[0],
            FamilyCode {
                account_id: "ACC1".to_string(),
                family_code: "FC1".to_string()
            }
        );
        assert_eq!(
            codes_s1[1],
            FamilyCode {
                account_id: "ACC2".to_string(),
                family_code: "FC2".to_string()
            }
        );

        // Scenario 2: No message received
        let message_bus_s2 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![], // No message will lead to subscription.next() returning None
        });
        let client_s2 = Client::stubbed(message_bus_s2, server_versions::SIZE_RULES);
        let result_s2 = client_s2.family_codes();
        assert!(result_s2.is_ok(), "Scenario 2: Expected Ok, got Err: {:?}", result_s2.err());
        assert!(result_s2.unwrap().is_empty(), "Scenario 2: Expected empty vector");

        // Scenario 3: Error response
        let message_bus_s3 = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["Test Error".into()],
        });
        let client_s3 = Client::stubbed(message_bus_s3, server_versions::SIZE_RULES);
        let result_s3 = client_s3.family_codes();
        assert!(result_s3.is_err(), "Scenario 3: Expected Err, got Ok: {:?}", result_s3.ok());
        match result_s3.err().unwrap() {
            Error::Simple(msg) => assert_eq!(msg, "Test Error", "Scenario 3: Error message mismatch"),
            _ => panic!("Scenario 3: Unexpected error type"),
        }
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

        let account = Some("DU1234567");
        let subscription = client.account_updates_multi(account, None).expect("request managed accounts failed");

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