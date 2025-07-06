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