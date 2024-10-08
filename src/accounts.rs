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

use log::error;

use crate::client::{Subscribable, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::transport::BusSubscription;
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
    const INCOMING_MESSAGE_ID: IncomingMessages = IncomingMessages::PnL;

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(server_version, message)
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
    const INCOMING_MESSAGE_ID: IncomingMessages = IncomingMessages::PnLSingle;

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(server_version, message)
    }
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
pub struct FamilyCode {
    /// Account ID
    pub account_id: String,
    /// Family code
    pub family_code: String,
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub(crate) fn positions(client: &Client) -> Result<impl Iterator<Item = Position> + '_, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let message = encoders::request_positions()?;

    let messages = client.send_shared_request(OutgoingMessages::RequestPositions, message)?;

    Ok(PositionIterator { client, messages })
}

pub(crate) fn cancel_positions(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position cancellation.")?;

    let message = encoders::cancel_positions()?;

    client.send_shared_request(OutgoingMessages::CancelPositions, message)?;

    Ok(())
}

// Determine whether an account exists under an account family and find the account family code.
pub(crate) fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    client.check_server_version(server_versions::REQ_FAMILY_CODES, "It does not support family codes requests.")?;

    let message = encoders::request_family_codes()?;

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
        responses,
        phantom: PhantomData,
    })
}

// Supports iteration over [Position].
pub(crate) struct PositionIterator<'a> {
    client: &'a Client,
    messages: BusSubscription,
}

impl<'a> Iterator for PositionIterator<'a> {
    type Item = Position;

    // Returns the next [Position]. Waits up to x seconds for next [OrderDataResult].
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mut message) = self.messages.next() {
                match message.message_type() {
                    IncomingMessages::Position => match decoders::decode_position(&mut message) {
                        Ok(val) => return Some(val),
                        Err(err) => {
                            error!("error decoding execution data: {err}");
                        }
                    },
                    IncomingMessages::PositionEnd => {
                        if let Err(e) = cancel_positions(self.client) {
                            error!("error cancelling positions: {e}")
                        }
                        return None;
                    }
                    message => {
                        error!("order data iterator unexpected message: {message:?}");
                    }
                }
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
