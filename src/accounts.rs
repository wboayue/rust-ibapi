use log::error;

use crate::client::transport::{GlobalResponseIterator, ResponseIterator};
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::{server_versions, Client, Error};

mod decoders;
mod encoders;

#[derive(Debug, Default)]
pub struct PnL {
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL total unrealized PnL for the position (since inception) updating in real time.
    pub unrealized_pnl: f64,
    /// Realized PnL for the position
    pub realized_pnl: f64,
}

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

    let messages = client.request_positions(message)?;

    Ok(PositionIterator { client, messages })
}

pub(crate) fn cancel_positions(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position cancellation.")?;

    let message = encoders::cancel_positions()?;

    client.request_positions(message)?;

    Ok(())
}

// Determine whether an account exists under an account family and find the account family code.
pub(crate) fn family_codes(client: &Client) -> Result<Vec<FamilyCode>, Error> {
    client.check_server_version(server_versions::REQ_FAMILY_CODES, "It does not support family codes requests.")?;

    let message = encoders::request_family_codes()?;

    let mut messages = client.request_family_codes(message)?;

    if let Some(mut message) = messages.next() {
        decoders::decode_family_codes(&mut message)
    } else {
        Ok(Vec::default())
    }
}

/**
 * @brief Creates subscription for real time daily PnL and unrealized PnL updates
 * @param account account for which to receive PnL updates
 * @param modelCode specify to request PnL updates for a specific model
 */

// https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EDecoder.cs#L674
// https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EClient.cs#L2744
// Creates subscription for real time daily PnL and unrealized PnL updates.
// Parameters
// account	account for which to receive PnL updates
// modelCode	specify to request PnL updates for a specific model
pub(crate) fn pnl<'a>(client: &'a Client, account: &str, model_code: Option<&str>) -> Result<impl Iterator<Item = PnL> + 'a, Error> {
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl(request_id, account, model_code)?;
    let responses = client.send_durable_request(request_id, request)?;

    Ok(PnlIterator { client, responses })
}

/// Requests real time updates for daily PnL of individual positions.
// account in which position exists
// modelCode	model in which position exists
// conId	contract ID (conId) of contract to receive daily PnL updates for. Note: does not return message if invalid conId is entered
// https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EClient.cs#L2794
pub(crate) fn pnl_single<'a>(
    client: &'a Client,
    account: &str,
    contract_id: &str,
    model_code: Option<&str>,
) -> Result<impl Iterator<Item = PnLSingle> + 'a, Error> {
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_request_pnl_single(request_id, account, contract_id, model_code)?;

    let messages = client.send_durable_request(request_id, message)?;

    Ok(PnlSingleIterator { client, messages })
}

// Supports iteration over [Position].
pub(crate) struct PositionIterator<'a> {
    client: &'a Client,
    messages: GlobalResponseIterator,
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

// Supports iteration over [Pnl].
pub(crate) struct PnlIterator<'a> {
    client: &'a Client,
    responses: ResponseIterator,
}

impl<'a> Iterator for PnlIterator<'a> {
    type Item = PnL;

    // Returns the next [Position]. Waits up to x seconds for next [OrderDataResult].
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mut message) = self.responses.next() {
                match message.message_type() {
                    IncomingMessages::PnL => match decoders::decode_pnl(&mut message) {
                        Ok(val) => return Some(val),
                        Err(err) => {
                            error!("error decoding execution data: {err}");
                        }
                    },
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

// Supports iteration over [Pnl].
pub(crate) struct PnlSingleIterator<'a> {
    client: &'a Client,
    messages: ResponseIterator,
}

impl<'a> Iterator for PnlSingleIterator<'a> {
    type Item = PnLSingle;

    // Returns the next [Position]. Waits up to x seconds for next [OrderDataResult].
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mut message) = self.messages.next() {
                match message.message_type() {
                    IncomingMessages::PnL => match decoders::decode_pnl_single(&mut message) {
                        Ok(val) => return Some(val),
                        Err(err) => {
                            error!("error decoding execution data: {err}");
                        }
                    },
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
