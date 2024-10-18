use log::error;
use time::OffsetDateTime;

use crate::client::{ResponseContext, Subscribable, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::orders::TagValue;
use crate::server_versions;
use crate::transport::{InternalSubscription, Response};
use crate::ToField;
use crate::{Client, Error};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

// === Models ===

#[derive(Clone, Debug, Copy)]
pub enum BarSize {
    // Sec,
    Sec5,
    // Sec15,
    // Sec30,
    // Min,
    // Min2,
    // Min3,
    // Min5,
    // Min15,
    // Min30,
    // Hour,
    // Day,
}

#[derive(Debug)]
pub struct BidAsk {
    /// The spread's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// tick-by-tick real-time tick bid price
    pub bid_price: f64,
    /// tick-by-tick real-time tick ask price
    pub ask_price: f64,
    /// tick-by-tick real-time tick bid size
    pub bid_size: i64,
    /// tick-by-tick real-time tick ask size
    pub ask_size: i64,
    /// tick-by-tick real-time bid/ask tick attribs (bit 0 - bid past low, bit 1 - ask past high)
    pub bid_ask_attribute: BidAskAttribute,
}

#[derive(Debug)]
pub struct BidAskAttribute {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

#[derive(Debug)]
pub struct MidPoint {
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// mid point
    pub mid_point: f64,
}

impl Subscribable<MidPoint> for MidPoint {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::mid_point_tick(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        if let Some(request_id) = request_id {
            encoders::cancel_tick_by_tick(request_id)
        } else {
            Err(Error::Simple("Request ID required to encode cancel mid point ticks".into()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bar {
    pub date: OffsetDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub wap: f64,
    pub count: i32,
}

impl Subscribable<Bar> for Bar {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::RealTimeBars];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_realtime_bar(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        if let Some(request_id) = request_id {
            encoders::encode_cancel_realtime_bars(request_id)
        } else {
            Err(Error::Simple("Request ID required to encode cancel realtime bars".into()))
        }
    }
}

#[derive(Debug)]
pub struct Trade {
    /// Tick type: "Last" or "AllLast"
    pub tick_type: String,
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// Tick last price
    pub price: f64,
    /// Tick last size
    pub size: i64,
    /// Tick attribs (bit 0 - past limit, bit 1 - unreported)
    pub trade_attribute: TradeAttribute,
    /// Tick exchange
    pub exchange: String,
    /// Tick special conditions
    pub special_conditions: String,
}

impl Subscribable<Trade> for Trade {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_trade_tick(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>) -> Result<RequestMessage, Error> {
        if let Some(request_id) = request_id {
            encoders::cancel_tick_by_tick(request_id)
        } else {
            Err(Error::Simple("Request ID required to encode cancel realtime bars".into()))
        }
    }
}

#[derive(Debug)]
pub struct TradeAttribute {
    pub past_limit: bool,
    pub unreported: bool,
}

#[derive(Clone, Debug, Copy)]
pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask,
}

impl std::fmt::Display for WhatToShow {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Trades => write!(f, "TRADES"),
            Self::MidPoint => write!(f, "MIDPOINT"),
            Self::Bid => write!(f, "BID"),
            Self::Ask => write!(f, "ASK"),
        }
    }
}

impl ToField for WhatToShow {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

// === Implementation ===

// Requests realtime bars.
pub(crate) fn realtime_bars<'a>(
    client: &'a Client,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<Subscription<'a, Bar>, Error> {
    client.check_server_version(server_versions::REAL_TIME_BARS, "It does not support real time bars.")?;

    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support ConId nor TradingClass parameters in reqRealTimeBars.",
        )?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_realtime_bars(client.server_version(), request_id, contract, bar_size, what_to_show, use_rth, options)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(
        client,
        ResponseContext {
            subscription,
            ..Default::default()
        },
    ))
}

// Requests tick by tick AllLast ticks.
pub(crate) fn tick_by_tick_all_last<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<impl Iterator<Item = Trade> + 'a, Error> {
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

// Validates that server supports the given request.
fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    client.check_server_version(server_versions::TICK_BY_TICK, "It does not support tick-by-tick requests.")?;

    if number_of_ticks != 0 || ignore_size {
        client.check_server_version(
            server_versions::TICK_BY_TICK_IGNORE_SIZE,
            "It does not support ignoreSize and numberOfTicks parameters in tick-by-tick requests.",
        )?;
    }

    Ok(())
}

// Requests tick by tick Last ticks.
pub(crate) fn tick_by_tick_last<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<TradeIterator<'a>, Error> {
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

// Requests tick by tick BidAsk ticks.
pub(crate) fn tick_by_tick_bid_ask<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<BidAskIterator<'a>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "BidAsk", number_of_ticks, ignore_size)?;
    let responses = client.send_request(request_id, message)?;

    Ok(BidAskIterator {
        client,
        request_id,
        responses,
    })
}

// Requests tick by tick MidPoint ticks.
pub(crate) fn tick_by_tick_midpoint<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, MidPoint>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let message = encoders::tick_by_tick(server_version, request_id, contract, "MidPoint", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, message)?;

    Ok(Subscription::new(
        client,
        ResponseContext {
            subscription,
            ..Default::default()
        },
    ))
}

// Iterators

/// TradeIterator supports iteration over [Trade] ticks.
pub(crate) struct TradeIterator<'a> {
    client: &'a Client,
    request_id: i32,
    responses: InternalSubscription,
}

impl<'a> Drop for TradeIterator<'a> {
    // Ensures tick by tick request is cancelled
    fn drop(&mut self) {
        cancel_tick_by_tick(self.client, self.request_id);
    }
}

impl<'a> Iterator for TradeIterator<'a> {
    type Item = Trade;

    /// Advances the iterator and returns the next value.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.responses.next() {
                Some(Response::Message(mut message)) => match message.message_type() {
                    IncomingMessages::TickByTick => match decoders::decode_trade_tick(&mut message) {
                        Ok(tick) => return Some(tick),
                        Err(e) => error!("unexpected message {message:?}: {e:?}"),
                    },
                    _ => error!("unexpected message {message:?}"),
                },
                // TODO enumerate
                _ => return None,
            }
        }
    }
}

/// BidAskIterator supports iteration over [BidAsk] ticks.
pub(crate) struct BidAskIterator<'a> {
    client: &'a Client,
    request_id: i32,
    responses: InternalSubscription,
}

/// Cancels the tick by tick request
fn cancel_tick_by_tick(client: &Client, request_id: i32) {
    if client.server_version() >= server_versions::TICK_BY_TICK {
        let message = encoders::cancel_tick_by_tick(request_id).unwrap();
        client.message_bus.cancel_subscription(request_id, &message).unwrap();
    }
}

impl<'a> Drop for BidAskIterator<'a> {
    // Ensures tick by tick request is cancelled
    fn drop(&mut self) {
        cancel_tick_by_tick(self.client, self.request_id);
    }
}

impl<'a> Iterator for BidAskIterator<'a> {
    type Item = BidAsk;

    /// Advances the iterator and returns the next value.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.responses.next() {
                Some(Response::Message(mut message)) => match message.message_type() {
                    IncomingMessages::TickByTick => match decoders::bid_ask_tick(&mut message) {
                        Ok(tick) => return Some(tick),
                        Err(e) => error!("unexpected message {message:?}: {e:?}"),
                    },
                    _ => error!("unexpected message {message:?}"),
                },
                // TODO enumerate
                _ => return None,
            }
        }
    }
}
