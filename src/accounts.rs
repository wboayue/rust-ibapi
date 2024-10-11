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

use std::marker::PhantomData;

use crate::client::{SharesChannel, Subscribable, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::{server_versions, Client, Error};

mod decoders;
mod encoders;

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
        if let Some(request_id) = request_id {
            encoders::encode_cancel_pnl(request_id)
        } else {
            Err(Error::Simple("Request id request to encode cancel pnl single".into()))
        }
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
    pub unrealized_pnl: Option<f64>,
    /// Realized PnL for the position
    pub realized_pnl: Option<f64>,
    /// Current market value of the position
    pub value: f64,
}

impl Subscribable<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        if let Some(request_id) = request_id {
            encoders::encode_cancel_pnl_single(request_id)
        } else {
            Err(Error::Simple("Request id request to encode cancel pnl single".into()))
        }
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
pub enum PositionResponse {
    Position(Position),
    PositionEnd,
}

impl From<Position> for PositionResponse {
    fn from(val: Position) -> Self {
        PositionResponse::Position(val)
    }
}

impl Subscribable<PositionResponse> for PositionResponse {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionResponse::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionResponse::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>) -> Result<RequestMessage, Error> {
        Ok(encoders::encode_cancel_positions()?)
    }
}

#[derive(Debug, Default)]
pub struct FamilyCode {
    /// Account ID
    pub account_id: String,
    /// Family code
    pub family_code: String,
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub(crate) fn positions(client: &Client) -> Result<Subscription<PositionResponse>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let message = encoders::encode_request_positions()?;

    let responses = client.send_shared_request(OutgoingMessages::RequestPositions, message)?;

    Ok(Subscription {
        client,
        request_id: None,
        responses,
        phantom: PhantomData,
    })
}

impl SharesChannel for Subscription<'_, PositionResponse> {}

// Determine whether an account exists under an account family and find the account family code.
pub(crate) fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    client.check_server_version(server_versions::REQ_FAMILY_CODES, "It does not support family codes requests.")?;

    let message = encoders::encode_request_family_codes()?;

    let mut messages = client.send_shared_request(OutgoingMessages::RequestFamilyCodes, message)?;

    if let Some(mut message) = messages.next() {
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
    let responses = client.send_request(request_id, request)?;

    Ok(Subscription {
        client,
        request_id: None,
        responses,
        phantom: PhantomData,
    })
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
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();

    let request = encoders::encode_request_pnl_single(request_id, account, contract_id, model_code)?;
    let responses = client.send_request(request_id, request)?;

    Ok(Subscription {
        client,
        request_id: None,
        responses,
        phantom: PhantomData,
    })
}

#[cfg(test)]
mod tests;
