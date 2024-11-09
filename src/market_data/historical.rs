use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::contracts::Contract;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::transport::InternalSubscription;
use crate::{server_versions, Client, Error, ToField};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

/// Bar describes the historical data bar.
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct Bar {
    /// The bar's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    // pub time: OffsetDateTime,
    pub date: OffsetDateTime,
    /// The bar's open price.
    pub open: f64,
    /// The bar's high price.
    pub high: f64,
    /// The bar's low price.
    pub low: f64,
    /// The bar's close price.
    pub close: f64,
    /// The bar's traded volume if available (only available for TRADES)
    pub volume: f64,
    /// The bar's Weighted Average Price (only available for TRADES)
    pub wap: f64,
    /// The number of trades during the bar's timespan (only available for TRADES)
    pub count: i32,
}

#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub enum BarSize {
    Sec,
    Sec5,
    Sec15,
    Sec30,
    Min,
    Min2,
    Min3,
    Min5,
    Min15,
    Min20,
    Min30,
    Hour,
    Hour2,
    Hour3,
    Hour4,
    Hour8,
    Day,
    Week,
    Month,
}

impl Display for BarSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Sec => write!(f, "1 sec"),
            Self::Sec5 => write!(f, "5 secs"),
            Self::Sec15 => write!(f, "15 secs"),
            Self::Sec30 => write!(f, "30 secs"),
            Self::Min => write!(f, "1 min"),
            Self::Min2 => write!(f, "2 mins"),
            Self::Min3 => write!(f, "3 mins"),
            Self::Min5 => write!(f, "5 mins"),
            Self::Min15 => write!(f, "15 mins"),
            Self::Min20 => write!(f, "20 mins"),
            Self::Min30 => write!(f, "30 mins"),
            Self::Hour => write!(f, "1 hour"),
            Self::Hour2 => write!(f, "2 hours"),
            Self::Hour3 => write!(f, "3 hours"),
            Self::Hour4 => write!(f, "4 hours"),
            Self::Hour8 => write!(f, "8 hours"),
            Self::Day => write!(f, "1 day"),
            Self::Week => write!(f, "1 week"),
            Self::Month => write!(f, "1 month"),
        }
    }
}

impl ToField for BarSize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub struct Duration {
    value: i32,
    unit: char,
}

impl Duration {
    pub const SECOND: Self = Self::seconds(1);
    pub const DAY: Self = Self::days(1);
    pub const WEEK: Self = Self::weeks(1);
    pub const MONTH: Self = Self::months(1);
    pub const YEAR: Self = Self::years(1);

    pub const fn seconds(seconds: i32) -> Self {
        Self { value: seconds, unit: 'S' }
    }

    pub const fn days(days: i32) -> Self {
        Self { value: days, unit: 'D' }
    }

    pub const fn weeks(weeks: i32) -> Self {
        Self { value: weeks, unit: 'W' }
    }

    pub const fn months(months: i32) -> Self {
        Self { value: months, unit: 'M' }
    }

    pub const fn years(years: i32) -> Self {
        Self { value: years, unit: 'Y' }
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

impl ToField for Duration {
    fn to_field(&self) -> String {
        self.to_string()
    }
}
pub trait ToDuration {
    fn seconds(&self) -> Duration;
    fn days(&self) -> Duration;
    fn weeks(&self) -> Duration;
    fn months(&self) -> Duration;
    fn years(&self) -> Duration;
}

impl ToDuration for i32 {
    fn seconds(&self) -> Duration {
        Duration::seconds(*self)
    }

    fn days(&self) -> Duration {
        Duration::days(*self)
    }

    fn weeks(&self) -> Duration {
        Duration::weeks(*self)
    }

    fn months(&self) -> Duration {
        Duration::months(*self)
    }

    fn years(&self) -> Duration {
        Duration::years(*self)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct HistogramEntry {
    pub price: f64,
    pub size: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoricalData {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub bars: Vec<Bar>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub time_zone: String,
    pub sessions: Vec<Session>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Session {
    pub reference: Date,
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

/// The historical tick's description. Used when requesting historical tick data with whatToShow = MIDPOINT
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct TickMidpoint {
    /// timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// historical tick price.
    pub price: f64,
    /// historical tick size
    pub size: i32,
}

/// The historical tick's description. Used when requesting historical tick data with whatToShow = BID_ASK.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct TickBidAsk {
    /// Timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// Tick attributes of historical bid/ask tick.
    pub tick_attribute_bid_ask: TickAttributeBidAsk,
    /// Bid price of the historical tick.
    pub price_bid: f64,
    /// Ask price of the historical tick.
    pub price_ask: f64,
    /// Bid size of the historical tick
    pub size_bid: i32,
    /// ask size of the historical tick
    pub size_ask: i32,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct TickAttributeBidAsk {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

/// The historical last tick's description. Used when requesting historical tick data with whatToShow = TRADES.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TickLast {
    /// Timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// Tick attributes of historical bid/ask tick.
    pub tick_attribute_last: TickAttributeLast,
    /// Last price of the historical tick.
    pub price: f64,
    /// Last size of the historical tick.
    pub size: i32,
    /// Source exchange of the historical tick.
    pub exchange: String,
    /// Conditions of the historical tick. Refer to Trade Conditions page for more details: <https://www.interactivebrokers.com/en/index.php?f=7235>.
    pub special_conditions: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct TickAttributeLast {
    pub past_limit: bool,
    pub unreported: bool,
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask,
    BidAsk,
    HistoricalVolatility,
    OptionImpliedVolatility,
    FeeRate,
    Schedule,
}

impl std::fmt::Display for WhatToShow {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Trades => write!(f, "TRADES"),
            Self::MidPoint => write!(f, "MIDPOINT"),
            Self::Bid => write!(f, "BID"),
            Self::Ask => write!(f, "ASK"),
            Self::BidAsk => write!(f, "BID_ASK"),
            Self::HistoricalVolatility => write!(f, "HISTORICAL_VOLATILITY"),
            Self::OptionImpliedVolatility => write!(f, "OPTION_IMPLIED_VOLATILITY"),
            Self::FeeRate => write!(f, "FEE_RATE"),
            Self::Schedule => write!(f, "SCHEDULE"),
        }
    }
}

impl ToField for WhatToShow {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<WhatToShow> {
    fn to_field(&self) -> String {
        match self {
            Some(what_to_show) => what_to_show.to_string(),
            None => "".into(),
        }
    }
}

// Returns the timestamp of earliest available historical data for a contract and data type.
pub(crate) fn head_timestamp(client: &Client, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<OffsetDateTime, Error> {
    client.check_server_version(server_versions::REQ_HEAD_TIMESTAMP, "It does not support head time stamp requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_head_timestamp(request_id, contract, what_to_show, use_rth)?;
    let subscription = client.send_request(request_id, request)?;

    match subscription.next() {
        Some(Ok(mut message)) if message.message_type() == IncomingMessages::HeadTimestamp => Ok(decoders::decode_head_timestamp(&mut message)?),
        Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
        Some(Err(Error::ConnectionReset)) => head_timestamp(client, contract, what_to_show, use_rth),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

// https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
pub(crate) fn historical_data(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
) -> Result<HistoricalData, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support contract_id nor trading class parameters when requesting historical data.",
        )?;
    }

    if what_to_show == Some(WhatToShow::Schedule) {
        client.check_server_version(
            server_versions::HISTORICAL_SCHEDULE,
            "It does not support requesting of historical schedule.",
        )?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_historical_data(
        client.server_version(),
        request_id,
        contract,
        end_date,
        duration,
        bar_size,
        what_to_show,
        use_rth,
        false,
        Vec::<crate::contracts::TagValue>::default(),
    )?;

    let subscription = client.send_request(request_id, request)?;

    match subscription.next() {
        Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalData => {
            Ok(decoders::decode_historical_data(client.server_version, time_zone(client), &mut message)?)
        }
        Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
        Some(Err(Error::ConnectionReset)) => historical_data(client, contract, end_date, duration, bar_size, what_to_show, use_rth),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

fn time_zone(client: &Client) -> &time_tz::Tz {
    if let Some(tz) = client.time_zone {
        tz
    } else {
        warn!("server timezone unknown. assuming UTC, but that may be incorrect!");
        time_tz::timezones::db::UTC
    }
}

pub(crate) fn historical_schedule(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
) -> Result<Schedule, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support contract_id nor trading class parameters when requesting historical data.",
        )?;
    }

    client.check_server_version(
        server_versions::HISTORICAL_SCHEDULE,
        "It does not support requesting of historical schedule.",
    )?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_historical_data(
        client.server_version(),
        request_id,
        contract,
        end_date,
        duration,
        BarSize::Day,
        Some(WhatToShow::Schedule),
        true,
        false,
        Vec::<crate::contracts::TagValue>::default(),
    )?;

    let subscription = client.send_request(request_id, request)?;

    match subscription.next() {
        Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalSchedule => {
            Ok(decoders::decode_historical_schedule(&mut message)?)
        }
        Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
        Some(Err(Error::ConnectionReset)) => historical_schedule(client, contract, end_date, duration),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub(crate) fn historical_ticks_bid_ask(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
    ignore_size: bool,
) -> Result<TickSubscription<TickBidAsk>, Error> {
    client.check_server_version(server_versions::HISTORICAL_TICKS, "It does not support historical ticks request.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_request_historical_ticks(
        request_id,
        contract,
        start,
        end,
        number_of_ticks,
        WhatToShow::BidAsk,
        use_rth,
        ignore_size,
    )?;

    let messages = client.send_request(request_id, message)?;

    Ok(TickSubscription::new(messages))
}

pub(crate) fn historical_ticks_mid_point(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickSubscription<TickMidpoint>, Error> {
    client.check_server_version(server_versions::HISTORICAL_TICKS, "It does not support historical ticks request.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_request_historical_ticks(request_id, contract, start, end, number_of_ticks, WhatToShow::MidPoint, use_rth, false)?;

    let messages = client.send_request(request_id, message)?;

    Ok(TickSubscription::new(messages))
}

pub(crate) fn historical_ticks_trade(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickSubscription<TickLast>, Error> {
    client.check_server_version(server_versions::HISTORICAL_TICKS, "It does not support historical ticks request.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_request_historical_ticks(request_id, contract, start, end, number_of_ticks, WhatToShow::Trades, use_rth, false)?;

    let messages = client.send_request(request_id, message)?;

    Ok(TickSubscription::new(messages))
}

pub(crate) fn histogram_data(client: &Client, contract: &Contract, use_rth: bool, period: BarSize) -> Result<Vec<HistogramEntry>, Error> {
    client.check_server_version(server_versions::REQ_HISTOGRAM, "It does not support histogram data requests.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_request_histogram_data(request_id, contract, use_rth, period)?;

    let subscription = client.send_request(request_id, message)?;

    match subscription.next() {
        Some(Ok(mut message)) => decoders::decode_histogram_data(&mut message),
        Some(Err(e)) => Err(e),
        None => Ok(Vec::new()),
    }
}

pub trait TickDecoder<T> {
    fn decode(message: &mut ResponseMessage) -> Result<(Vec<T>, bool), Error>;
    fn message_type() -> IncomingMessages;
}

impl TickDecoder<TickBidAsk> for TickBidAsk {
    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
        decoders::decode_historical_ticks_bid_ask(message)
    }
    fn message_type() -> IncomingMessages {
        IncomingMessages::HistoricalTickBidAsk
    }
}

impl TickDecoder<TickLast> for TickLast {
    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
        decoders::decode_historical_ticks_last(message)
    }
    fn message_type() -> IncomingMessages {
        IncomingMessages::HistoricalTickLast
    }
}

impl TickDecoder<TickMidpoint> for TickMidpoint {
    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
        decoders::decode_historical_ticks_mid_point(message)
    }
    fn message_type() -> IncomingMessages {
        IncomingMessages::HistoricalTick
    }
}

pub struct TickSubscription<T: TickDecoder<T>> {
    done: AtomicBool,
    messages: InternalSubscription,
    buffer: Mutex<VecDeque<T>>,
    error: Mutex<Option<Error>>,
}

impl<T: TickDecoder<T>> TickSubscription<T> {
    fn new(messages: InternalSubscription) -> Self {
        Self {
            done: false.into(),
            messages,
            buffer: Mutex::new(VecDeque::new()),
            error: Mutex::new(None),
        }
    }

    pub fn next(&self) -> Option<T> {
        self.clear_error();

        if let Some(message) = self.next_buffered() {
            return Some(message);
        }

        if self.done.load(Ordering::Relaxed) {
            return None;
        }

        match self.messages.next() {
            Some(Ok(message)) if message.message_type() == T::message_type() => {
                self.fill_buffer(message);
                self.next()
            }
            Some(Ok(message)) => {
                debug!("unexpected message: {:?}", message);
                self.next()
            }
            Some(Err(e)) => {
                self.set_error(e);
                None
            }
            None => None,
        }
    }

    pub fn try_next(&self) -> Option<T> {
        self.clear_error();

        if let Some(message) = self.next_buffered() {
            return Some(message);
        }

        if self.done.load(Ordering::Relaxed) {
            return None;
        }

        match self.messages.try_next() {
            Some(Ok(message)) if message.message_type() == T::message_type() => {
                self.fill_buffer(message);
                self.try_next()
            }
            Some(Ok(message)) => {
                debug!("unexpected message: {:?}", message);
                self.try_next()
            }
            Some(Err(e)) => {
                self.set_error(e);
                None
            }
            None => None,
        }
    }

    pub fn next_timeout(&self, duration: std::time::Duration) -> Option<T> {
        self.clear_error();

        if let Some(message) = self.next_buffered() {
            return Some(message);
        }

        if self.done.load(Ordering::Relaxed) {
            return None;
        }

        match self.messages.next_timeout(duration) {
            Some(Ok(message)) if message.message_type() == T::message_type() => {
                self.fill_buffer(message);
                self.next_timeout(duration)
            }
            Some(Ok(message)) => {
                debug!("unexpected message: {:?}", message);
                self.next_timeout(duration)
            }
            Some(Err(e)) => {
                self.set_error(e);
                None
            }
            None => None,
        }
    }

    fn next_buffered(&self) -> Option<T> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.pop_front()
    }

    fn set_error(&self, e: Error) {
        let mut error = self.error.lock().unwrap();
        *error = Some(e);
    }

    fn clear_error(&self) {
        let mut error = self.error.lock().unwrap();
        *error = None;
    }

    fn fill_buffer(&self, mut message: ResponseMessage) {
        let mut buffer = self.buffer.lock().unwrap();

        let (ticks, done) = T::decode(&mut message).unwrap();

        buffer.append(&mut ticks.into());
        self.done.store(done, Ordering::Relaxed);
    }
}

impl<T: TickDecoder<T> + Debug> Iterator for TickSubscription<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        {
            let mut buffer = self.buffer.lock().unwrap();
            if !buffer.is_empty() {
                return buffer.pop_front();
            }
        }

        if self.done.load(Ordering::Relaxed) {
            return None;
        }

        loop {
            match self.messages.next() {
                Some(Ok(mut message)) => {
                    if message.message_type() == Self::Item::message_type() {
                        let mut buffer = self.buffer.lock().unwrap();

                        let (ticks, done) = Self::Item::decode(&mut message).unwrap();

                        buffer.append(&mut ticks.into());
                        self.done.store(done, Ordering::Relaxed);

                        if buffer.is_empty() && self.done.load(Ordering::Relaxed) {
                            return None;
                        }

                        if !buffer.is_empty() {
                            return buffer.pop_front();
                        }
                    } else if message.message_type() == IncomingMessages::Error {
                        error!("error reading ticks: {:?}", message.peek_string(4));
                        return None;
                    } else {
                        error!("unexpected message: {:?}", message)
                    }
                }
                // TODO enumerate
                _ => return None,
            }
        }
    }
}

/// An iterator that yields items as they become available, blocking if necessary.
pub struct TickSubscriptionIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<'a, T: TickDecoder<T>> Iterator for TickSubscriptionIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

/// An iterator that yields items if they are available, without waiting.
pub struct TickSubscriptionTryIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<'a, T: TickDecoder<T>> Iterator for TickSubscriptionTryIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// An iterator that waits for the specified timeout duration for available data.
pub struct TickSubscriptionTimeoutIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
    timeout: std::time::Duration,
}

impl<'a, T: TickDecoder<T>> Iterator for TickSubscriptionTimeoutIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}
