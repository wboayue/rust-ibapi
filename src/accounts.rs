//! # Account Management
//!
//! This module provides functionality for managing positions and profit and loss (PnL)
//! information in a trading system. It includes structures and implementations for:
//!
//! - Position tracking
//! - Daily, unrealized, and realized PnL calculations
//! - Family code management
//! - Real-time PnL updates for individual positions
//!

use crate::client::{SharesChannel, Subscribable, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::transport::Response;
use crate::{server_versions, Client, Error};

mod decoders;
mod encoders;

#[derive(Debug, Default)]
/// Account information as it appears in the TWS’ Account Summary Window
pub struct AccountSummary {
    /// The account identifier.
    pub account: String,
    /// The account’s attribute.
    pub tag: String,
    /// The account’s attribute’s value.
    pub value: String,
    /// The currency in which the value is expressed.
    pub currency: String,
}

pub struct AccountSummaryTags {}

impl AccountSummaryTags {
    pub const ACCOUNT_TYPE: &str = "AccountType";
    pub const NET_LIQUIDATION: &str = "NetLiquidation";
    pub const TOTAL_CASH_VALUE: &str = "TotalCashValue";
    pub const SETTLED_CASH: &str = "SettledCash";
    pub const ACCRUED_CASH: &str = "AccruedCash";
    pub const BUYING_POWER: &str = "BuyingPower";
    pub const EQUITY_WITH_LOAN_VALUE: &str = "EquityWithLoanValue";
    pub const PREVIOUS_DAY_EQUITY_WITH_LOAN_VALUE: &str = "PreviousDayEquityWithLoanValue";
    pub const GROSS_POSITION_VALUE: &str = "GrossPositionValue";
    pub const REQ_T_EQUITY: &str = "ReqTEquity";
    pub const REQ_T_MARGIN: &str = "ReqTMargin";
    pub const SMA: &str = "SMA";
    pub const INIT_MARGIN_REQ: &str = "InitMarginReq";
    pub const MAINT_MARGIN_REQ: &str = "MaintMarginReq";
    pub const AVAILABLE_FUNDS: &str = "AvailableFunds";
    pub const EXCESS_LIQUIDITY: &str = "ExcessLiquidity";
    pub const CUSHION: &str = "Cushion";
    pub const FULL_INIT_MARGIN_REQ: &str = "FullInitMarginReq";
    pub const FULL_MAINT_MARGIN_REQ: &str = "FullMaintMarginReq";
    pub const FULL_AVAILABLE_FUNDS: &str = "FullAvailableFunds";
    pub const FULL_EXCESS_LIQUIDITY: &str = "FullExcessLiquidity";
    pub const LOOK_AHEAD_NEXT_CHANGE: &str = "LookAheadNextChange";
    pub const LOOK_AHEAD_INIT_MARGIN_REQ: &str = "LookAheadInitMarginReq";
    pub const LOOK_AHEAD_MAINT_MARGIN_REQ: &str = "LookAheadMaintMarginReq";
    pub const LOOK_AHEAD_AVAILABLE_FUNDS: &str = "LookAheadAvailableFunds";
    pub const LOOK_AHEAD_EXCESS_LIQUIDITY: &str = "LookAheadExcessLiquidity";
    pub const HIGHEST_SEVERITY: &str = "HighestSeverity";
    pub const DAY_TRADES_REMAINING: &str = "DayTradesRemaining";
    pub const LEVERAGE: &str = "Leverage";

    pub const ALL: &[&str] = &[
        Self::ACCOUNT_TYPE,
        Self::NET_LIQUIDATION,
        Self::TOTAL_CASH_VALUE,
        Self::SETTLED_CASH,
        Self::ACCRUED_CASH,
        Self::BUYING_POWER,
        Self::EQUITY_WITH_LOAN_VALUE,
        Self::PREVIOUS_DAY_EQUITY_WITH_LOAN_VALUE,
        Self::GROSS_POSITION_VALUE,
        Self::REQ_T_EQUITY,
        Self::REQ_T_MARGIN,
        Self::SMA,
        Self::INIT_MARGIN_REQ,
        Self::MAINT_MARGIN_REQ,
        Self::AVAILABLE_FUNDS,
        Self::EXCESS_LIQUIDITY,
        Self::CUSHION,
        Self::FULL_INIT_MARGIN_REQ,
        Self::FULL_MAINT_MARGIN_REQ,
        Self::FULL_AVAILABLE_FUNDS,
        Self::FULL_EXCESS_LIQUIDITY,
        Self::LOOK_AHEAD_NEXT_CHANGE,
        Self::LOOK_AHEAD_INIT_MARGIN_REQ,
        Self::LOOK_AHEAD_MAINT_MARGIN_REQ,
        Self::LOOK_AHEAD_AVAILABLE_FUNDS,
        Self::LOOK_AHEAD_EXCESS_LIQUIDITY,
        Self::HIGHEST_SEVERITY,
        Self::DAY_TRADES_REMAINING,
        Self::LEVERAGE,
    ];
}

#[derive(Debug)]
pub enum AccountSummaries {
    Summary(AccountSummary),
    End,
}

impl From<AccountSummary> for AccountSummaries {
    fn from(val: AccountSummary) -> Self {
        AccountSummaries::Summary(val)
    }
}

impl Subscribable<AccountSummaries> for AccountSummaries {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaries::Summary(decoders::decode_account_summary(server_version, message)?)),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaries::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

// Realtime PnL update for account.
#[derive(Debug, Default)]
pub struct PnL {
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL total unrealized PnL for the position (since inception) updating in real time.
    pub unrealized_pnl: Option<f64>,
    /// Realized PnL for the position
    pub realized_pnl: Option<f64>,
}

impl Subscribable<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel pnl");
        encoders::encode_cancel_pnl(request_id)
    }
}

// Realtime PnL update for a position in account.
#[derive(Debug, Default)]
pub struct PnLSingle {
    // Current size of the position
    pub position: f64,
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL total unrealized PnL for the position (since inception) updating in real time.
    pub unrealized_pnl: f64,
    /// Realized PnL for the position
    pub realized_pnl: f64,
    /// Current market value of the position
    pub value: f64,
}

impl Subscribable<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel pnl single");
        encoders::encode_cancel_pnl_single(request_id)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Position {
    /// Account holding position
    pub account: String,
    /// Contract
    pub contract: Contract,
    /// Size of position
    pub position: f64,
    /// Average cost of position
    pub average_cost: f64,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum PositionUpdate {
    Position(Position),
    PositionEnd,
}

impl From<Position> for PositionUpdate {
    fn from(val: Position) -> Self {
        PositionUpdate::Position(val)
    }
}

impl Subscribable<PositionUpdate> for PositionUpdate {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionUpdate::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionUpdate::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum PositionUpdateMulti {
    Position(PositionMulti),
    PositionEnd,
}

impl From<PositionMulti> for PositionUpdateMulti {
    fn from(val: PositionMulti) -> Self {
        PositionUpdateMulti::Position(val)
    }
}

/// Portfolio's open positions.
#[derive(Debug, Clone, Default)]
pub struct PositionMulti {
    /// The account holding the position.
    pub account: String,
    /// The model code holding the position.
    pub model_code: String,
    /// The position's Contract
    pub contract: Contract,
    /// The number of positions held.
    pub position: f64,
    /// The average cost of the position.
    pub average_cost: f64,
}

impl Subscribable<PositionUpdateMulti> for PositionUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::PositionMulti => Ok(PositionUpdateMulti::Position(decoders::decode_position_multi(message)?)),
            IncomingMessages::PositionMultiEnd => Ok(PositionUpdateMulti::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel positions multi");
        encoders::encode_cancel_positions_multi(request_id)
    }
}

#[derive(Debug, Default)]
pub struct FamilyCode {
    /// Account ID
    pub account_id: String,
    /// Family code
    pub family_code: String,
}

/// Account's information, portfolio and last update time
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum AccountUpdates {
    /// Receives the subscribed account's information.
    AccountValue(AccountValue),
    /// Receives the subscribed account's portfolio.
    PortfolioValue(AccountPortfolioValue),
    /// Receives the last time on which the account was updated.
    UpdateTime(AccountUpdateTime),
    /// Notifies when all the account’s information has finished.
    End,
}

impl Subscribable<AccountUpdates> for AccountUpdates {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[
        IncomingMessages::AccountValue,
        IncomingMessages::PortfolioValue,
        IncomingMessages::AccountUpdateTime,
        IncomingMessages::AccountDownloadEnd,
    ];
    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountValue => Ok(AccountUpdates::AccountValue(decoders::decode_account_value(message)?)),
            IncomingMessages::PortfolioValue => Ok(AccountUpdates::PortfolioValue(decoders::decode_account_portfolio_value(
                server_version,
                message,
            )?)),
            IncomingMessages::AccountUpdateTime => Ok(AccountUpdates::UpdateTime(decoders::decode_account_update_time(message)?)),
            IncomingMessages::AccountDownloadEnd => Ok(AccountUpdates::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, _request_id: Option<i32>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_account_updates(server_version)
    }
}

/// A value of subscribed account's information.
#[derive(Debug, Default)]
pub struct AccountValue {
    /// The value being updated.
    pub key: String,
    /// Current value
    pub value: String,
    /// The currency inn which the value is expressed.
    pub currency: String,
    /// The account identifier.
    pub account: Option<String>,
}

/// Subscribed account's portfolio.
#[derive(Debug, Default)]
pub struct AccountPortfolioValue {
    /// The Contract for which a position is held.
    pub contract: Contract,
    /// The number of positions held.
    pub position: f64,
    /// The instrument's unitary price
    pub market_price: f64,
    /// Total market value of the instrument.
    pub market_value: f64,
    /// Average cost of the overall position.
    pub average_cost: f64,
    /// Daily unrealized profit and loss on the position.
    pub unrealized_pnl: f64,
    /// Daily realized profit and loss on the position.
    pub realized_pnl: f64,
    /// Account identifier for the update.
    pub account: Option<String>,
}

/// Last time at which the account was updated.
#[derive(Debug, Default)]
pub struct AccountUpdateTime {
    /// The last update system time.
    pub timestamp: String,
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub(crate) fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let request = encoders::encode_request_positions()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestPositions, request)?;

    Ok(Subscription::new(client, subscription))
}

impl SharesChannel for Subscription<'_, PositionUpdate> {}

pub(crate) fn positions_multi<'a>(
    client: &'a Client,
    account: Option<&str>,
    model_code: Option<&str>,
) -> Result<Subscription<'a, PositionUpdateMulti>, Error> {
    client.check_server_version(server_versions::MODELS_SUPPORT, "It does not support positions multi requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_positions_multi(request_id, account, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription))
}

// Determine whether an account exists under an account family and find the account family code.
pub(crate) fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    client.check_server_version(server_versions::REQ_FAMILY_CODES, "It does not support family codes requests.")?;

    let request = encoders::encode_request_family_codes()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestFamilyCodes, request)?;

    // TODO: enumerate
    if let Some(Response::Message(mut message)) = subscription.next() {
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
pub(crate) fn pnl<'a>(client: &'a Client, account: &str, model_code: Option<&str>) -> Result<Subscription<'a, PnL>, Error> {
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl(request_id, account, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription))
}

// Requests real time updates for daily PnL of individual positions.
//
// # Arguments
// * `client` - Client
// * `account` - Account in which position exists
// * `contract_id` - Contract ID of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// * `model_code` - Model in which position exists
pub(crate) fn pnl_single<'a>(
    client: &'a Client,
    account: &str,
    contract_id: i32,
    model_code: Option<&str>,
) -> Result<Subscription<'a, PnLSingle>, Error> {
    client.check_server_version(server_versions::REALIZED_PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl_single(request_id, account, contract_id, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription))
}

pub fn account_summary<'a>(client: &'a Client, group: &str, tags: &[&str]) -> Result<Subscription<'a, AccountSummaries>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support account summary requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_account_summary(request_id, group, tags)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription))
}

pub fn account_updates<'a>(client: &'a Client, account: &str) -> Result<Subscription<'a, AccountUpdates>, Error> {
    let request = encoders::encode_request_account_updates(client.server_version(), account)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAccountData, request)?;

    Ok(Subscription::new(client, subscription))
}

pub fn managed_accounts(client: &Client) -> Result<Vec<String>, Error> {
    let request = encoders::encode_request_managed_accounts()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestManagedAccounts, request)?;

    match subscription.next() {
        Some(Response::Message(mut message)) => {
            message.skip(); // message type
            message.skip(); // message version

            let accounts = message.next_string()?;
            Ok(accounts.split(",").map(String::from).collect())
        }
        Some(Response::Cancelled) => Err(Error::Cancelled),
        Some(Response::Disconnected) => Err(Error::ConnectionFailed),
        None => Ok(Vec::default()),
    }
}

#[cfg(test)]
mod tests;
