use std::fmt::{Debug, Display};
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::{Error, ToField};

pub(crate) mod common;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub mod sync;

#[cfg(all(test, feature = "sync", not(feature = "async")))]
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
            Self::Sec => write!(f, "1 secs"),
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

impl From<&str> for BarSize {
    fn from(val: &str) -> Self {
        match val.to_uppercase().as_str() {
            "SEC" => Self::Sec,
            "SEC5" => Self::Sec5,
            "SEC15" => Self::Sec15,
            "SEC30" => Self::Sec30,
            "MIN" => Self::Min,
            "MIN2" => Self::Min2,
            "MIN3" => Self::Min3,
            "MIN5" => Self::Min5,
            "MIN15" => Self::Min15,
            "MIN20" => Self::Min20,
            "MIN30" => Self::Min30,
            "HOUR" => Self::Hour,
            "HOUR2" => Self::Hour2,
            "HOUR3" => Self::Hour3,
            "HOUR4" => Self::Hour4,
            "HOUR8" => Self::Hour8,
            "DAY" => Self::Day,
            "WEEK" => Self::Week,
            "MONTH" => Self::Month,
            _ => panic!("unsupported value: {val}"),
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
#[derive(Debug, PartialEq)]
pub enum DurationParseError {
    EmptyString,
    MissingDelimiter(String),
    ParseIntError(ParseIntError),
    UnsupportedUnit(String),
}
impl From<ParseIntError> for DurationParseError {
    fn from(err: ParseIntError) -> Self {
        DurationParseError::ParseIntError(err)
    }
}
impl Display for DurationParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DurationParseError::EmptyString => write!(f, "Empty duration string"),
            DurationParseError::ParseIntError(err) => write!(f, "Parse integer error: {err}"),
            DurationParseError::MissingDelimiter(msg) => write!(f, "Missing delimiter: {msg}"),
            DurationParseError::UnsupportedUnit(unit) => write!(f, "Unsupported duration unit: {unit}"),
        }
    }
}
impl std::error::Error for DurationParseError {}

impl FromStr for Duration {
    type Err = DurationParseError;
    fn from_str(val: &str) -> Result<Self, DurationParseError> {
        if val.is_empty() {
            return Err(DurationParseError::EmptyString);
        }
        match val.to_uppercase().rsplit_once(' ') {
            Some((value_part, unit_part)) => {
                let value = value_part.parse::<i32>().map_err(DurationParseError::from)?;
                match unit_part {
                    "S" => Ok(Self::seconds(value)),
                    "D" => Ok(Self::days(value)),
                    "W" => Ok(Self::weeks(value)),
                    "M" => Ok(Self::months(value)),
                    "Y" => Ok(Self::years(value)),
                    _ => Err(DurationParseError::UnsupportedUnit(unit_part.to_string())),
                }
            }
            None => Err(DurationParseError::MissingDelimiter(val.to_string())),
        }
    }
}

impl From<&str> for Duration {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}
impl From<String> for Duration {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
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
    AdjustedLast,
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
            Self::AdjustedLast => write!(f, "ADJUSTED_LAST"),
        }
    }
}

impl From<&str> for WhatToShow {
    fn from(val: &str) -> Self {
        match val.to_uppercase().as_str() {
            "TRADES" => Self::Trades,
            "MIDPOINT" => Self::MidPoint,
            "BID" => Self::Bid,
            "ASK" => Self::Ask,
            "BID_ASK" => Self::BidAsk,
            "HISTORICAL_VOLATILITY" => Self::HistoricalVolatility,
            "OPTION_IMPLIED_VOLATILITY" => Self::OptionImpliedVolatility,
            "FEE_RATE" => Self::FeeRate,
            "SCHEDULE" => Self::Schedule,
            "ADJUSTED_LAST" => Self::AdjustedLast,
            _ => panic!("unsupported value: {val}"),
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

// Re-export sync functions when sync feature is enabled
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) use sync::*;

pub trait TickDecoder<T> {
    const MESSAGE_TYPE: IncomingMessages;
    fn decode(message: &mut ResponseMessage) -> Result<(Vec<T>, bool), Error>;
}

impl TickDecoder<TickBidAsk> for TickBidAsk {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTickBidAsk;

    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
        common::decoders::decode_historical_ticks_bid_ask(message)
    }
}

impl TickDecoder<TickLast> for TickLast {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTickLast;

    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
        common::decoders::decode_historical_ticks_last(message)
    }
}

impl TickDecoder<TickMidpoint> for TickMidpoint {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTick;

    fn decode(message: &mut ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
        common::decoders::decode_historical_ticks_mid_point(message)
    }
}

// Re-export TickSubscription and iterator types from sync module when sync feature is enabled
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{TickSubscription, TickSubscriptionIter, TickSubscriptionOwnedIter, TickSubscriptionTimeoutIter, TickSubscriptionTryIter};
