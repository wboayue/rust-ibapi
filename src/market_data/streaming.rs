use std::fmt::Debug;

use anyhow::Result;
use log::error;

use crate::client::transport::ResponsePacketPromise;
use crate::client::Client;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::orders::TagValue;
use crate::server_versions;

use super::{BarSize, RealTimeBar, WhatToShow};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

/// Requests realtime bars.
///
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::IBClient;
/// use ibapi::contracts::{self, Contract};
/// use ibapi::market_data::{streaming, BarSize, WhatToShow};
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = IBClient::connect("localhost:4002")?;
///
///     let contract = Contract::stock("TSLA");
///     let bars = streaming::realtime_bars(&mut client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;
///
///     for (i, bar) in bars.enumerate() {
///         println!("bar[{i}]: {bar:?}");
///
///         if i > 60 {
///             break;
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub fn realtime_bars<'a, C: Client + Debug>(
    client: &'a mut C,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
) -> Result<RealTimeBarIterator<'a>> {
    realtime_bars_with_options(client, contract, bar_size, what_to_show, use_rth, Vec::default())
}

pub fn realtime_bars_with_options<'a>(
    client: &'a mut dyn Client,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<RealTimeBarIterator<'a>> {
    client.check_server_version(server_versions::REAL_TIME_BARS, "It does not support real time bars.")?;

    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support ConId nor TradingClass parameters in reqRealTimeBars.",
        )?;
    }

    let request_id = client.next_request_id();
    let packet = encoders::encode_request_realtime_bars(client.server_version(), request_id, contract, bar_size, what_to_show, use_rth, options)?;

    let responses = client.send_request(request_id, packet)?;

    Ok(RealTimeBarIterator::new(client, request_id, responses))
}

pub struct RealTimeBarIterator<'a> {
    client: &'a mut dyn Client,
    request_id: i32,
    responses: ResponsePacketPromise,
}

impl<'a> RealTimeBarIterator<'a> {
    fn new(client: &'a mut dyn Client, request_id: i32, responses: ResponsePacketPromise) -> RealTimeBarIterator<'a> {
        RealTimeBarIterator {
            client,
            request_id,
            responses,
        }
    }

    /// Cancels request to stream realtime bars
    fn cancel_realtime_bars(&mut self) {
        let message = encoders::cancel_realtime_bars(self.request_id).unwrap();

        self.client.send_message(message).unwrap();

        ()
    }
}

impl<'a> Iterator for RealTimeBarIterator<'a> {
    type Item = RealTimeBar;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut message) = self.responses.next() {
            match message.message_type() {
                IncomingMessages::RealTimeBars => {
                    let decoded = decoders::decode_realtime_bar(&mut message);

                    if let Ok(bar) = decoded {
                        return Some(bar);
                    }

                    error!("unexpected message: {:?}", decoded.err());
                    None
                }
                _ => {
                    error!("unexpected message: {message:?}");
                    None
                }
            }
        } else {
            None
        }
    }
}

impl<'a> Drop for RealTimeBarIterator<'a> {
    fn drop(&mut self) {
        self.cancel_realtime_bars()
    }
}
