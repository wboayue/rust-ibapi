use time::OffsetDateTime;

use crate::client::{ResponseContext, Subscribable, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::orders::TagValue;
use crate::server_versions;
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

// Some(Ok(mut message)) => match message.message_type() {
//     IncomingMessages::TickByTick => match decoders::bid_ask_tick(&mut message) {

impl Subscribable<BidAsk> for BidAsk {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_bid_ask_tick(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        encoders::encode_cancel_tick_by_tick(request_id)
    }
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

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel mid point ticks");
        encoders::encode_cancel_tick_by_tick(request_id)
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

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        encoders::encode_cancel_realtime_bars(request_id)
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

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        encoders::encode_cancel_tick_by_tick(request_id)
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

#[derive(Debug)]
pub enum MarketDepths {
    MarketDepth(MarketDepth),
    MarketDepthL2(MarketDepthL2),
}

#[derive(Debug, Default)]
pub struct MarketDepth {}
#[derive(Debug, Default)]
pub struct MarketDepthL2 {}

impl Subscribable<MarketDepths> for MarketDepths {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::MarketDepth, IncomingMessages::MarketDepthL2];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::MarketDepth => Ok(MarketDepths::MarketDepth(decoders::decode_market_depth(message)?)),
            IncomingMessages::MarketDepthL2 => Ok(MarketDepths::MarketDepthL2(decoders::decode_market_depth_l2(message)?)),
            e => Err(Error::NotImplemented),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        encoders::encode_cancel_tick_by_tick(request_id)
    }
}


/// Stores depth market data description.
#[derive(Debug, Default)]
pub struct DepthMarketDataDescription{
    /// The exchange name
    pub exchange_name: String,
    /// The security type
    pub security_type: String,
    /// The listing exchange name
    pub listing_exchange: String,
    /// The service data type
    pub service_data_type: String,
    /// The aggregated group
    pub aggregated_group: String,
}

pub enum TickTypes {
    Price(TickPrice),
    Size(TickSize),
    String(TickString),
    EFP(TickEFP),
    Generic(TickGeneric),
    OptionComputation(TickOptionComputation),
    SnapshotEnd,
}

pub struct TickPrice {}

pub struct TickSize {}

pub struct TickString {}

pub struct TickEFP {}

pub struct TickGeneric {}

pub struct TickOptionComputation {}

//        * @sa cancelMktData, EWrapper::tickPrice, EWrapper::tickSize, EWrapper::tickString,
// EWrapper::tickEFP, EWrapper::tickGeneric, EWrapper::tickOptionComputation, EWrapper::tickSnapshotEnd

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

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests tick by tick AllLast ticks.
pub(crate) fn tick_by_tick_all_last<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "AllLast", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Validates that server supports the given request.
fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    client.check_server_version(server_versions::TICK_BY_TICK, "It does not support tick-by-tick requests.")?;

    if number_of_ticks != 0 || ignore_size {
        client.check_server_version(
            server_versions::TICK_BY_TICK_IGNORE_SIZE,
            "It does not support ignore_size and number_of_ticks parameters in tick-by-tick requests.",
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
) -> Result<Subscription<'a, Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "Last", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests tick by tick BidAsk ticks.
pub(crate) fn tick_by_tick_bid_ask<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, BidAsk>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "BidAsk", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
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

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "MidPoint", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub(crate) fn market_depth<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_rows: i32,
    is_smart_depth: bool,
) -> Result<Subscription<'a, MarketDepths>, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support contract_id nor trading_class parameters in request_market_depth.",
        )?;
    }
    if is_smart_depth {
        client.check_server_version(server_versions::SMART_DEPTH, "It does not support SMART depth request.")?;
    }
    if !contract.primary_exchange.is_empty() {
        client.check_server_version(
            server_versions::MKT_DEPTH_PRIM_EXCHANGE,
            "It does not support primary_exchange parameter in request_market_depth",
        )?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_market_depth(client.server_version, request_id, contract, number_of_rows, is_smart_depth)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests venues for which market data is returned to market_depth (those with market makers)
pub fn market_depth_exchanges(client: &Client) -> Result<Vec<DepthMarketDataDescription>, Error> {
    Ok(Vec::new())
}

// Requests real time market data.
pub fn market_data(client: &Client, contract: &Contract, generic_ticks: &[&str], snapshot: bool, regulatory_snapshot: bool) {
//        * @sa cancelMktData, EWrapper::tickPrice, EWrapper::tickSize, EWrapper::tickString, EWrapper::tickEFP, EWrapper::tickGeneric, EWrapper::tickOptionComputation, EWrapper::tickSnapshotEnd
}

