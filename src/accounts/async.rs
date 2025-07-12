//! Asynchronous implementation of account management functionality

use time::OffsetDateTime;

use crate::client::{ClientRequestBuilders, SubscriptionBuilderExt};
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{AsyncDataStream, Subscription};
use crate::{Client, Error};

use super::common::{decoders, encoders};
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
        if let Some(request_id) = request_id {
            encoders::encode_cancel_account_summary(request_id)
        } else {
            Err(Error::Simple("request_id required".into()))
        }
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
        encoders::encode_cancel_pnl(request_id.ok_or(Error::Simple("request_id required".into()))?)
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
        encoders::encode_cancel_pnl_single(request_id.ok_or(Error::Simple("request_id required".into()))?)
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
        encoders::encode_cancel_positions_multi(request_id.ok_or(Error::Simple("request_id required".into()))?)
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
        let request_id = request_id.expect("Request ID required to encode cancel account updates multi");
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub async fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    check_version(client.server_version(), Features::POSITIONS)?;

    let request = encoders::encode_request_positions()?;

    client
        .subscription::<PositionUpdate>()
        .send_shared::<PositionUpdate>(OutgoingMessages::RequestPositions, request)
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
    check_version(client.server_version(), Features::FAMILY_CODES)?;

    let request = encoders::encode_request_family_codes()?;
    let mut subscription = client.shared_request(OutgoingMessages::RequestFamilyCodes).send_raw(request).await?;

    // TODO: enumerate - for now just get the first message
    if let Some(mut message) = subscription.next().await {
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
pub async fn pnl(client: &Client, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
    check_version(client.server_version(), Features::PNL)?;

    let builder = client.request();
    let request = encoders::encode_request_pnl(builder.request_id(), account, model_code)?;

    builder.send::<PnL>(request).await
}

// Requests real time updates for daily PnL of individual positions.
//
// # Arguments
// * `client` - Client
// * `account` - Account in which position exists
// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// * `model_code` - Model in which position exists
pub async fn pnl_single(client: &Client, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<Subscription<PnLSingle>, Error> {
    check_version(client.server_version(), Features::REALIZED_PNL)?;

    let builder = client.request();
    let request = encoders::encode_request_pnl_single(builder.request_id(), account, contract_id, model_code)?;

    builder.send::<PnLSingle>(request).await
}

pub async fn account_summary(client: &Client, group: &str, tags: &[&str]) -> Result<Subscription<AccountSummaries>, Error> {
    check_version(client.server_version(), Features::ACCOUNT_SUMMARY)?;

    let builder = client.request();
    let request = encoders::encode_request_account_summary(builder.request_id(), group, tags)?;

    builder.send::<AccountSummaries>(request).await
}

pub async fn account_updates(client: &Client, account: &str) -> Result<Subscription<AccountUpdate>, Error> {
    let request = encoders::encode_request_account_updates(client.server_version(), account)?;

    client
        .shared_request(OutgoingMessages::RequestAccountData)
        .send::<AccountUpdate>(request)
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
    let request = encoders::encode_request_managed_accounts()?;
    let mut subscription = client.shared_request(OutgoingMessages::RequestManagedAccounts).send_raw(request).await?;

    match subscription.next().await {
        Some(mut message) => {
            message.skip(); // message type
            message.skip(); // message version

            let accounts = message.next_string()?;
            Ok(accounts.split(",").map(String::from).collect())
        }
        None => Ok(Vec::default()),
    }
}

pub async fn server_time(client: &Client) -> Result<OffsetDateTime, Error> {
    let request = encoders::encode_request_server_time()?;
    let mut subscription = client.shared_request(OutgoingMessages::RequestCurrentTime).send_raw(request).await?;

    match subscription.next().await {
        Some(mut message) => {
            message.skip(); // message type
            message.skip(); // message version

            let timestamp = message.next_long()?;
            match OffsetDateTime::from_unix_timestamp(timestamp) {
                Ok(date) => Ok(date),
                Err(e) => Err(Error::Simple(format!("Error parsing date: {e}"))),
            }
        }
        None => Err(Error::Simple("No response from server".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use crate::testdata::responses;
    use crate::{server_versions, Client};
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn test_positions() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::POSITION.into(), responses::POSITION_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

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

        // Check that the correct request was sent
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "61|1|");
    }

    #[tokio::test]
    async fn test_positions_multi() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::POSITION_MULTI.into(), responses::POSITION_MULTI_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = Some("DU1234567");
        let model_code = Some("TARGET2024");

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

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "74|1|9000|DU1234567|TARGET2024|");
    }

    #[tokio::test]
    async fn test_account_summary() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::ACCOUNT_SUMMARY.into(), responses::ACCOUNT_SUMMARY_END.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let group = "All";
        let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

        let mut subscription = account_summary(&client, group, tags).await.expect("request account_summary failed");

        // First update should be a summary
        let first_update = subscription.next().await;
        match first_update {
            Some(Ok(AccountSummaries::Summary(summary))) => {
                assert_eq!(summary.account, "DU1234567");
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

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "62|1|9000|All|AccountType|");
    }

    #[tokio::test]
    async fn test_pnl() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = "DU1234567";
        let model_code = Some("TARGET2024");

        let _ = pnl(&client, account, model_code).await.expect("request pnl failed");
        let _ = pnl(&client, account, None).await.expect("request pnl failed");

        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 2, "Expected two request messages");
        assert_eq!(request_messages[0].encode_simple(), "92|9000|DU1234567|TARGET2024|");
        assert_eq!(request_messages[1].encode_simple(), "92|9001|DU1234567||");
    }

    #[tokio::test]
    async fn test_pnl_single() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = "DU1234567";
        let contract_id = 1001;
        let model_code = Some("TARGET2024");

        let _ = pnl_single(&client, account, contract_id, model_code)
            .await
            .expect("request pnl_single failed");
        let _ = pnl_single(&client, account, contract_id, None).await.expect("request pnl_single failed");

        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 2, "Expected two request messages");
        assert_eq!(request_messages[0].encode_simple(), "94|9000|DU1234567|TARGET2024|1001|");
        assert_eq!(request_messages[1].encode_simple(), "94|9001|DU1234567||1001|");
    }

    #[tokio::test]
    async fn test_managed_accounts() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![responses::MANAGED_ACCOUNT.into()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let accounts = managed_accounts(&client).await.expect("request managed accounts failed");
        assert_eq!(accounts, &["DU1234567", "DU7654321"], "Valid accounts list mismatch");

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "17|1|");
    }

    #[tokio::test]
    async fn test_server_time() {
        use time::macros::datetime;

        let valid_timestamp_str = "1678886400"; // 2023-03-15 13:20:00 UTC
        let expected_datetime = datetime!(2023-03-15 13:20:00 UTC);

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![format!("49|1|{}|", valid_timestamp_str)],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let result = server_time(&client).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), expected_datetime, "DateTime mismatch");

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "49|1|");
    }

    #[tokio::test]
    async fn test_account_updates_multi() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                responses::ACCOUNT_UPDATE_MULTI_CASH_BALANCE.into(),
                responses::ACCOUNT_UPDATE_MULTI_CURRENCY.into(),
                responses::ACCOUNT_UPDATE_MULTI_STOCK_MARKET_VALUE.into(),
                responses::ACCOUNT_UPDATE_MULTI_END.into(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let account = Some("DU1234567");
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

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_eq!(request_messages[0].encode_simple(), "76|1|9000|DU1234567||1|");
    }
}
