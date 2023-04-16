use time::{Date, OffsetDateTime};

use crate::contracts::Contract;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::{server_versions, Client, Error, ToField};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

/// Bar describes the historical data bar.
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Copy)]
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

impl ToString for BarSize {
    fn to_string(&self) -> String {
        match self {
            Self::Sec => "1 sec".into(),
            Self::Sec5 => "5 secs".into(),
            Self::Sec15 => "15 secs".into(),
            Self::Sec30 => "30 secs".into(),
            Self::Min => "1 min".into(),
            Self::Min2 => "2 mins".into(),
            Self::Min3 => "3 mins".into(),
            Self::Min5 => "5 mins".into(),
            Self::Min15 => "15 mins".into(),
            Self::Min20 => "20 mins".into(),
            Self::Min30 => "30 mins".into(),
            Self::Hour => "1 hour".into(),
            Self::Hour2 => "2 hours".into(),
            Self::Hour3 => "3 hours".into(),
            Self::Hour4 => "4 hours".into(),
            Self::Hour8 => "8 hours".into(),
            Self::Day => "1 day".into(),
            Self::Week => "1 week".into(),
            Self::Month => "1 month".into(),
        }
    }
}

impl ToField for BarSize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug, Copy)]
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

impl ToString for Duration {
    fn to_string(&self) -> String {
        format!("{} {}", self.value, self.unit)
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

#[derive(Debug)]
struct HistogramData {
    pub price: f64,
    pub count: i32,
}

#[derive(Clone, Debug)]
pub struct HistoricalData {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub bars: Vec<Bar>,
}

#[derive(Debug)]
pub struct Schedule {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub time_zone: String,
    pub sessions: Vec<Session>,
}

#[derive(Debug)]
pub struct Session {
    pub reference: Date,
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

/// The historical tick's description. Used when requesting historical tick data with whatToShow = MIDPOINT
pub struct TickMidpoint {
    /// timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// historical tick price.
    pub price: f64,
    /// historical tick size
    pub size: i32,
}

/// The historical tick's description. Used when requesting historical tick data with whatToShow = BID_ASK.
pub struct TickBidAsk {
    /// Timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// Tick attributes of historical bid/ask tick.
    pub tick_attrib_bid_ask: TickAttribBidAsk,
    /// Bid price of the historical tick.
    pub price_bid: f64,
    /// Ask price of the historical tick.
    pub price_ask: f64,
    /// Bid size of the historical tick
    pub size_bid: i32,
    /// ask size of the historical tick
    pub size_ask: i32,
}

pub struct TickAttribBidAsk {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

/// The historical last tick's description. Used when requesting historical tick data with whatToShow = TRADES.
pub struct TickLast {
    /// Timestamp of the historical tick.
    pub timestamp: OffsetDateTime,
    /// Tick attributes of historical bid/ask tick.
    pub tick_attrib_last: TickAttribLast,
    /// Last price of the historical tick.
    pub price: f64,
    /// Last size of the historical tick.
    pub size: i32,
    /// Source exchange of the historical tick.
    pub exchange: String,
    /// Conditions of the historical tick. Refer to Trade Conditions page for more details: <https://www.interactivebrokers.com/en/index.php?f=7235>.
    pub special_conditions: String,
}

pub struct TickAttribLast {
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

impl ToString for WhatToShow {
    fn to_string(&self) -> String {
        match self {
            Self::Trades => "TRADES".to_string(),
            Self::MidPoint => "MIDPOINT".to_string(),
            Self::Bid => "BID".to_string(),
            Self::Ask => "ASK".to_string(),
            Self::BidAsk => "BID_ASK".to_string(),
            Self::HistoricalVolatility => "HISTORICAL_VOLATILITY".to_string(),
            Self::OptionImpliedVolatility => "OPTION_IMPLIED_VOLATILITY".to_string(),
            Self::FeeRate => "FEE_RATE".to_string(),
            Self::Schedule => "SCHEDULE".to_string(),
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
    let request = encoders::encode_head_timestamp(request_id, contract, what_to_show, use_rth)?;

    let mut messages = client.send_request(request_id, request)?;

    if let Some(mut message) = messages.next() {
        decoders::decode_head_timestamp(&mut message)
    } else {
        Err(Error::Simple("did not receive head timestamp message".into()))
    }
}

/// Returns data histogram of specified contract
fn histogram_data(client: &Client, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator, Error> {
    // " S (seconds) - " D (days)
    // " W (weeks) - " M (months)
    // " Y (years)
    print!("{client:?} {contract:?} {use_rth:?} {period:?}");
    Err(Error::NotImplemented)
}

//     // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
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

    let mut messages = client.send_request(request_id, request)?;

    if let Some(mut message) = messages.next() {
        match message.message_type() {
            IncomingMessages::HistoricalData => decoders::decode_historical_data(client.server_version, client.time_zone, &mut message),
            IncomingMessages::Error => Err(Error::Simple(message.peek_string(4))),
            _ => Err(Error::Simple(format!("unexpected message: {:?}", message.message_type()))),
        }
    } else {
        Err(Error::Simple("did not receive historical data response".into()))
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

    let mut messages = client.send_request(request_id, request)?;

    if let Some(mut message) = messages.next() {
        match message.message_type() {
            IncomingMessages::HistoricalSchedule => decoders::decode_historical_schedule(&mut message),
            IncomingMessages::Error => Err(Error::Simple(message.peek_string(4))),
            _ => Err(Error::Simple(format!("unexpected message: {:?}", message.message_type()))),
        }
    } else {
        Err(Error::Simple("did not receive historical schedule response".into()))
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
) -> Result<TickBidAskIterator, Error> {
    print!("{client:?} {contract:?} {start:?} {end:?} {number_of_ticks:?} {use_rth:?} {ignore_size:?}");
    Err(Error::NotImplemented)
}


pub(crate) fn historical_ticks_mid_point(
    client: &Client,
    contract: &Contract,
    start_date: Option<OffsetDateTime>,
    end_date: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickMidPointIterator, Error> {
    print!("{client:?} {contract:?} {start_date:?} {end_date:?} {number_of_ticks:?} {use_rth:?}");
    Err(Error::NotImplemented)
}

pub(crate) fn historical_ticks_trade(
    client: &Client,
    contract: &Contract,
    start_date: Option<OffsetDateTime>,
    end_date: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickLastIterator, Error> {
    print!("{client:?} {contract:?} {start_date:?} {end_date:?} {number_of_ticks:?} {use_rth:?}");
    Err(Error::NotImplemented)
}

#[derive(Default)]
pub(crate) struct TickMidPointIterator {}

impl TickMidPointIterator {
    pub fn new() -> TickMidPointIterator {
        TickMidPointIterator {}
    }
}

impl Iterator for TickMidPointIterator {
    // we will be counting with usize
    type Item = TickMidpoint;

    // next() is the only required method
    fn next(&mut self) -> Option<TickMidpoint> {
        None
    }
}

pub(crate) struct TickBidAskIterator {}

impl Iterator for TickBidAskIterator {
    type Item = TickBidAsk;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub(crate) struct TickLastIterator {}

impl Iterator for TickLastIterator {
    type Item = TickLast;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

struct HistogramDataIterator {}
