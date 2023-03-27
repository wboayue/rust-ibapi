use log::error;

use crate::client::transport::GlobalResponseIterator;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::{server_versions, Client, Error};

mod decoders;
mod encoders;

#[derive(Debug, Default)]
pub struct Position {
    pub account: String,
    pub contract: Contract,
    pub position: f64,
    pub average_cost: f64,
}

// Subscribes to position updates for all accessible accounts.
// All positions sent initially, and then only updates as positions change.
pub(crate) fn positions(client: &Client) -> Result<impl Iterator<Item = Position>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let message = encoders::request_positions()?;

    let messages = client.request_order_data(message)?;

    Ok(PositionIterator {
        server_version: client.server_version(),
        messages,
    })
}

/// Supports iteration over [Position].
pub struct PositionIterator {
    server_version: i32,
    messages: GlobalResponseIterator,
}

impl Iterator for PositionIterator {
    type Item = Position;

    /// Returns the next [Position]. Waits up to x seconds for next [OrderDataResult].
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mut message) = self.messages.next() {
                match message.message_type() {
                    IncomingMessages::Position => match decoders::position(self.server_version, &mut message) {
                        Ok(val) => return Some(val),
                        Err(err) => {
                            error!("error decoding execution data: {err}");
                        }
                    },
                    IncomingMessages::PositionEnd => {
                        // cancel positions
                        return None;
                    }
                    message => {
                        error!("order data iterator unexpected messsage: {message:?}");
                    }
                }
            } else {
                return None;
            }
        }
    }
}

// cancelPositions, EWrapper::position, EWrapper::positionEnd
