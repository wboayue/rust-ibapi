use std::{fmt::Debug, marker::PhantomData, num};

use anyhow::Result;
use log::error;

use crate::client::Client;
use crate::client::{transport::ResponsePacketPromise, ResponseMessage};
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::orders::TagValue;
use crate::server_versions;

use super::{BarSize, RealTimeBar, Trade, WhatToShow};

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
/// use ibapi::market_data::{realtime, BarSize, WhatToShow};
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = IBClient::connect("localhost:4002")?;
///
///     let contract = Contract::stock("TSLA");
///     let bars = realtime::realtime_bars(&mut client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;
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

/// Requests tick by tick AllLast ticks.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
/// * `number_of_ticks` - number of ticks.
/// * `ignore_size` - ignore size flag.
pub fn tick_by_tick_all_last<'a, C: Client + Debug>(
    client: &'a mut C,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> anyhow::Result<TradeIterator<'a, C>> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "AllLast", number_of_ticks, ignore_size)?;

    let responses = client.send_request(request_id, message)?;

    Ok(TradeIterator {
        client,
        request_id,
        responses,
    })
}

fn validate_tick_by_tick_request<C: Client + Debug>(client: &C, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> anyhow::Result<()> {
    client.check_server_version(server_versions::TICK_BY_TICK, "It does not support tick-by-tick requests.")?;

    if number_of_ticks != 0 || ignore_size {
        client.check_server_version(
            server_versions::TICK_BY_TICK_IGNORE_SIZE,
            "It does not support ignoreSize and numberOfTicks parameters in tick-by-tick requests.",
        )?;
    }

    Ok(())
}

pub struct TradeIterator<'a, C: Client> {
    client: &'a mut C,
    request_id: i32,
    responses: ResponsePacketPromise,
}

impl<'a, C: Client> TradeIterator<'a, C> {
    /// Cancels request to stream [Trade] ticks
    fn cancel(&mut self) {
        let message = encoders::cancel_realtime_bars(self.request_id).unwrap();

        self.client.send_message(message).unwrap();
    }
}

impl<'a, C: Client> Drop for TradeIterator<'a, C> {
    fn drop(&mut self) {
        self.cancel()
    }
}

impl<'a, C: Client> Iterator for TradeIterator<'a, C> {
    type Item = Trade;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut message) = self.responses.next() {
            match message.message_type() {
                IncomingMessages::TickByTick => match decoders::trade_tick(&mut message) {
                    Ok(tick) => Some(tick),
                    Err(e) => {
                        error!("unexpected message: {e:?}");
                        None
                    }
                },
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

/// Requests tick by tick Last ticks.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
/// * `number_of_ticks` - number of ticks.
/// * `ignore_size` - ignore size flag.
pub fn tick_by_tick_last<'a, C: Client + Debug>(
    client: &'a mut C,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> anyhow::Result<TradeIterator<'a, C>> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "Last", number_of_ticks, ignore_size)?;
    let responses = client.send_request(request_id, message)?;

    Ok(TradeIterator {
        client,
        request_id,
        responses,
    })
}

/// Requests tick by tick BidAsk ticks.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
/// * `number_of_ticks` - number of ticks.
/// * `ignore_size` - ignore size flag.
pub fn tick_by_tick_bid_ask<C: Client + Debug>(client: &mut C, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> anyhow::Result<()> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "BidAsk", number_of_ticks, ignore_size)?;

    Ok(())
}

/// Requests tick by tick MidPoint ticks.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
/// * `number_of_ticks` - number of ticks.
/// * `ignore_size` - ignore size flag.
pub fn tick_by_tick_midpoint<C: Client + Debug>(client: &mut C, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> anyhow::Result<()> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "MidPoint", number_of_ticks, ignore_size)?;

    Ok(())
}
