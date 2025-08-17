use std::fmt::{self, Debug, Display};
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::{Error, ToField};

pub(crate) mod common;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

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

#[derive(Debug)]
pub struct BarSizeParseError;

impl Display for BarSizeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid BarSize string")
    }
}

impl FromStr for BarSize {
    type Err = BarSizeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SEC" => Ok(Self::Sec),
            "SEC5" => Ok(Self::Sec5),
            "SEC15" => Ok(Self::Sec15),
            "SEC30" => Ok(Self::Sec30),
            "MIN" => Ok(Self::Min),
            "MIN2" => Ok(Self::Min2),
            "MIN3" => Ok(Self::Min3),
            "MIN5" => Ok(Self::Min5),
            "MIN15" => Ok(Self::Min15),
            "MIN20" => Ok(Self::Min20),
            "MIN30" => Ok(Self::Min30),
            "HOUR" => Ok(Self::Hour),
            "HOUR2" => Ok(Self::Hour2),
            "HOUR3" => Ok(Self::Hour3),
            "HOUR4" => Ok(Self::Hour4),
            "HOUR8" => Ok(Self::Hour8),
            "DAY" => Ok(Self::Day),
            "WEEK" => Ok(Self::Week),
            "MONTH" => Ok(Self::Month),
            _ => Err(BarSizeParseError),
        }
    }
}

impl From<&str> for BarSize {
    fn from(val: &str) -> Self {
        Self::from_str(val).unwrap()
    }
}
impl From<String> for BarSize {
    fn from(val: String) -> Self {
        Self::from(val.as_str())
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
    fn from(val: &str) -> Self {
        Self::from_str(val).unwrap()
    }
}
impl From<String> for Duration {
    fn from(val: String) -> Self {
        Self::from(val.as_str())
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

#[derive(Debug)]
pub struct WhatToShowParseError;

impl fmt::Display for WhatToShowParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid WhatToShow string")
    }
}

impl FromStr for WhatToShow {
    type Err = WhatToShowParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRADES" => Ok(Self::Trades),
            "MIDPOINT" => Ok(Self::MidPoint),
            "BID" => Ok(Self::Bid),
            "ASK" => Ok(Self::Ask),
            "BID_ASK" => Ok(Self::BidAsk),
            "HISTORICAL_VOLATILITY" => Ok(Self::HistoricalVolatility),
            "OPTION_IMPLIED_VOLATILITY" => Ok(Self::OptionImpliedVolatility),
            "FEE_RATE" => Ok(Self::FeeRate),
            "SCHEDULE" => Ok(Self::Schedule),
            "ADJUSTED_LAST" => Ok(Self::AdjustedLast),
            _ => Err(WhatToShowParseError),
        }
    }
}

impl From<&str> for WhatToShow {
    fn from(val: &str) -> Self {
        Self::from_str(val).unwrap()
    }
}

impl From<String> for WhatToShow {
    fn from(val: String) -> Self {
        Self::from(val.as_str())
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

// Re-export functions based on active feature
#[cfg(feature = "sync")]
pub use sync::*;

#[cfg(feature = "async")]
pub use r#async::*;

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

// Re-export TickSubscription and iterator types based on active feature
#[cfg(feature = "sync")]
pub use sync::{TickSubscription, TickSubscriptionIter, TickSubscriptionOwnedIter, TickSubscriptionTimeoutIter, TickSubscriptionTryIter};

#[cfg(feature = "async")]
pub use r#async::TickSubscription;

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_bar_size_to_string() {
        assert_eq!("1 secs", BarSize::Sec.to_string());
        assert_eq!("5 secs", BarSize::Sec5.to_string());
        assert_eq!("15 secs", BarSize::Sec15.to_string());
        assert_eq!("30 secs", BarSize::Sec30.to_string());
        assert_eq!("1 min", BarSize::Min.to_string());
        assert_eq!("2 mins", BarSize::Min2.to_string());
        assert_eq!("3 mins", BarSize::Min3.to_string());
        assert_eq!("5 mins", BarSize::Min5.to_string());
        assert_eq!("15 mins", BarSize::Min15.to_string());
        assert_eq!("20 mins", BarSize::Min20.to_string());
        assert_eq!("30 mins", BarSize::Min30.to_string());
        assert_eq!("1 hour", BarSize::Hour.to_string());
        assert_eq!("2 hours", BarSize::Hour2.to_string());
        assert_eq!("3 hours", BarSize::Hour3.to_string());
        assert_eq!("4 hours", BarSize::Hour4.to_string());
        assert_eq!("8 hours", BarSize::Hour8.to_string());
        assert_eq!("1 day", BarSize::Day.to_string());
        assert_eq!("1 week", BarSize::Week.to_string());
        assert_eq!("1 month", BarSize::Month.to_string());
    }

    #[test]
    fn test_bar_size_from_string() {
        assert_eq!(BarSize::Sec, BarSize::from("SEC"));
        assert_eq!(BarSize::Sec5, BarSize::from("SEC5"));
        assert_eq!(BarSize::Sec15, BarSize::from("SEC15"));
        assert_eq!(BarSize::Sec30, BarSize::from("SEC30"));
        assert_eq!(BarSize::Min, BarSize::from("MIN"));
        assert_eq!(BarSize::Min2, BarSize::from("MIN2"));
        assert_eq!(BarSize::Min3, BarSize::from("MIN3"));
        assert_eq!(BarSize::Min5, BarSize::from("MIN5"));
        assert_eq!(BarSize::Min15, BarSize::from("MIN15"));
        assert_eq!(BarSize::Min20, BarSize::from("MIN20"));
        assert_eq!(BarSize::Min30, BarSize::from("MIN30"));
        assert_eq!(BarSize::Hour, BarSize::from("HOUR"));
        assert_eq!(BarSize::Hour2, BarSize::from("HOUR2"));
        assert_eq!(BarSize::Hour3, BarSize::from("HOUR3"));
        assert_eq!(BarSize::Hour4, BarSize::from("HOUR4"));
        assert_eq!(BarSize::Hour8, BarSize::from("HOUR8"));
        assert_eq!(BarSize::Day, BarSize::from("DAY"));
        assert_eq!(BarSize::Week, BarSize::from("WEEK"));
        assert_eq!(BarSize::Month, BarSize::from("MONTH"));
    }

    #[test]
    fn test_what_to_show_to_string() {
        assert_eq!("TRADES", WhatToShow::Trades.to_string());
        assert_eq!("MIDPOINT", WhatToShow::MidPoint.to_string());
        assert_eq!("BID", WhatToShow::Bid.to_string());
        assert_eq!("ASK", WhatToShow::Ask.to_string());
        assert_eq!("BID_ASK", WhatToShow::BidAsk.to_string());
        assert_eq!("HISTORICAL_VOLATILITY", WhatToShow::HistoricalVolatility.to_string());
        assert_eq!("OPTION_IMPLIED_VOLATILITY", WhatToShow::OptionImpliedVolatility.to_string());
        assert_eq!("FEE_RATE", WhatToShow::FeeRate.to_string());
        assert_eq!("SCHEDULE", WhatToShow::Schedule.to_string());
        assert_eq!("ADJUSTED_LAST", WhatToShow::AdjustedLast.to_string());
    }

    #[test]
    fn test_what_to_show_from_string() {
        assert_eq!(WhatToShow::Trades, WhatToShow::from("TRADES"));
        assert_eq!(WhatToShow::MidPoint, WhatToShow::from("MIDPOINT"));
        assert_eq!(WhatToShow::Bid, WhatToShow::from("BID"));
        assert_eq!(WhatToShow::Ask, WhatToShow::from("ASK"));
        assert_eq!(WhatToShow::BidAsk, WhatToShow::from("BID_ASK"));
        assert_eq!(WhatToShow::HistoricalVolatility, WhatToShow::from("HISTORICAL_VOLATILITY"));
        assert_eq!(WhatToShow::OptionImpliedVolatility, WhatToShow::from("OPTION_IMPLIED_VOLATILITY"));
        assert_eq!(WhatToShow::FeeRate, WhatToShow::from("FEE_RATE"));
        assert_eq!(WhatToShow::Schedule, WhatToShow::from("SCHEDULE"));
        assert_eq!(WhatToShow::AdjustedLast, WhatToShow::from("ADJUSTED_LAST"));
    }

    #[test]
    fn test_duration() {
        assert_eq!(Duration::SECOND.to_field(), "1 S");
        assert_eq!(Duration::DAY.to_field(), "1 D");
        assert_eq!(Duration::WEEK.to_field(), "1 W");
        assert_eq!(Duration::MONTH.to_field(), "1 M");
        assert_eq!(Duration::YEAR.to_field(), "1 Y");

        assert_eq!(2.seconds().to_field(), "2 S");
        assert_eq!(3.days().to_field(), "3 D");
        assert_eq!(4.weeks().to_field(), "4 W");
        assert_eq!(5.months().to_field(), "5 M");
        assert_eq!(6.years().to_field(), "6 Y");
    }

    #[test]
    fn test_duration_parse() {
        assert_eq!("1 S".parse(), Ok(Duration::seconds(1)));
        assert_eq!("2 D".parse(), Ok(Duration::days(2)));
        assert_eq!("3 W".parse(), Ok(Duration::weeks(3)));
        assert_eq!("4 M".parse(), Ok(Duration::months(4)));
        assert_eq!("5 Y".parse(), Ok(Duration::years(5)));

        assert_eq!("".parse::<Duration>(), Err(DurationParseError::EmptyString));
        assert_eq!("1S".parse::<Duration>(), Err(DurationParseError::MissingDelimiter("1S".to_string())));
        assert!("abc S".parse::<Duration>().unwrap_err().to_string().contains("Parse integer error"));
        assert_eq!("1 X".parse::<Duration>(), Err(DurationParseError::UnsupportedUnit("X".to_string())));

        assert_eq!(DurationParseError::EmptyString.to_string(), "Empty duration string");
        assert_eq!(
            DurationParseError::MissingDelimiter("1S".to_string()).to_string(),
            "Missing delimiter: 1S"
        );
        assert_eq!(
            DurationParseError::UnsupportedUnit("X".to_string()).to_string(),
            "Unsupported duration unit: X"
        );

        if let Err(err) = i32::from_str("abc") {
            assert_eq!(
                DurationParseError::ParseIntError(err).to_string(),
                "Parse integer error: invalid digit found in string"
            );
        }

        assert_eq!(Duration::seconds(1), Duration::from("1 S"));
        assert_eq!(Duration::seconds(1), Duration::from(String::from("1 S")));
    }
}
