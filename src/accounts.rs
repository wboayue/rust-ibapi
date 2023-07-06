use log::error;

use crate::client::transport::GlobalResponseIterator;
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, self};
use crate::{server_versions, Client, Error};

mod decoders;
mod encoders;

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
    /// Acount ID
    pub account_id: String,
    /// Family code
    pub family_code: String,
}

#[derive(Debug, Default)]
pub struct PositionMulti {
    /// Request ID
    pub req_id: i32,
    /// Account holding position
    pub account: String,
    /// Contract
    pub contract: Contract,
    /// Code of model's positions
    pub model_code: String,
    /// Size of position
    pub position: f64,
    /// Average cost of position
    pub average_cost: f64,
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
pub(crate) fn family_codes(client: &Client) -> Result<impl Iterator<Item = FamilyCode> + '_, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support family code requests.")?;

    let message = encoders::request_family_codes()?;

    let messages = client.request_family_codes(message)?;
   
    Ok(FamilyCodeIterator { client, messages})
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
                    IncomingMessages::Position => match decoders::position(&mut message) {
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


// Iteration over [FamilyCode].
pub(crate) struct FamilyCodeIterator<'a> {
    client: &'a Client,
    messages: GlobalResponseIterator,
}

impl<'a> Iterator for FamilyCodeIterator<'a> {
    type Item = FamilyCode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mut message) = self.messages.next() {
                match message.message_type() {
                    IncomingMessages::FamilyCode => match decoders::family_code(&mut message) {
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