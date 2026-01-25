use std::fmt::{self, Debug, Display};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::{Error, ToField};

pub(crate) mod common;

#[cfg(feature = "sync")]
/// Synchronous historical market data API.
pub mod sync;

/// Async historical market data API.
#[cfg(feature = "async")]
pub mod r#async;

/// Errors surfaced while parsing historical market data parameters.
#[derive(Debug, Error, PartialEq)]
pub enum HistoricalParseError {
    /// Unsupported bar size string supplied by the caller.
    #[error("Invalid BarSize input '{0}'")]
    BarSize(String),
    /// Invalid duration string or unsupported unit.
    #[error("Invalid Duration input '{0}' {1}")]
    Duration(String, String),
    /// Unsupported `what_to_show` value.
    #[error("Invalid WhatToShow input '{0}'")]
    WhatToShow(String),
    /// Wrapper for integer parsing errors when reading duration values.
    #[error("ParseIntError '{0}' {1}")]
    ParseIntError(String, ParseIntError),
}

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
/// Request granularity for historical bars.
pub enum BarSize {
    /// One-second bars.
    Sec,
    /// Five-second bars.
    Sec5,
    /// Ten-second bars.
    Sec10,
    /// Fifteen-second bars.
    Sec15,
    /// Thirty-second bars.
    Sec30,
    /// One-minute bars.
    Min,
    /// Two-minute bars.
    Min2,
    /// Three-minute bars.
    Min3,
    /// Five-minute bars.
    Min5,
    /// Fifteen-minute bars.
    Min15,
    /// Twenty-minute bars.
    Min20,
    /// Thirty-minute bars.
    Min30,
    /// One-hour bars.
    Hour,
    /// Two-hour bars.
    Hour2,
    /// Three-hour bars.
    Hour3,
    /// Four-hour bars.
    Hour4,
    /// Eight-hour bars.
    Hour8,
    /// One-day bars.
    Day,
    /// One-week bars.
    Week,
    /// One-month bars.
    Month,
}

impl Display for BarSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Sec => write!(f, "1 secs"),
            Self::Sec5 => write!(f, "5 secs"),
            Self::Sec10 => write!(f, "10 secs"),
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

impl FromStr for BarSize {
    type Err = HistoricalParseError;

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
            _ => Err(HistoricalParseError::BarSize(s.to_string())),
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

/// Duration specifier used in historical data requests (e.g. `1 D`).
#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub struct Duration {
    value: i32,
    unit: char,
}

impl Duration {
    /// Convenience constant for a one-second duration.
    pub const SECOND: Self = Self::seconds(1);
    /// Convenience constant for a one-day duration.
    pub const DAY: Self = Self::days(1);
    /// Convenience constant for a one-week duration.
    pub const WEEK: Self = Self::weeks(1);
    /// Convenience constant for a one-month duration.
    pub const MONTH: Self = Self::months(1);
    /// Convenience constant for a one-year duration.
    pub const YEAR: Self = Self::years(1);

    /// Build a duration described in seconds (`S` unit).
    pub const fn seconds(seconds: i32) -> Self {
        Self { value: seconds, unit: 'S' }
    }

    /// Build a duration described in days (`D` unit).
    pub const fn days(days: i32) -> Self {
        Self { value: days, unit: 'D' }
    }

    /// Build a duration described in weeks (`W` unit).
    pub const fn weeks(weeks: i32) -> Self {
        Self { value: weeks, unit: 'W' }
    }

    /// Build a duration described in months (`M` unit).
    pub const fn months(months: i32) -> Self {
        Self { value: months, unit: 'M' }
    }

    /// Build a duration described in years (`Y` unit).
    pub const fn years(years: i32) -> Self {
        Self { value: years, unit: 'Y' }
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

impl FromStr for Duration {
    type Err = HistoricalParseError;
    fn from_str(val: &str) -> Result<Self, HistoricalParseError> {
        if val.is_empty() {
            return Err(HistoricalParseError::Duration(val.to_string(), "Empty string".to_string()));
        }
        match val.to_uppercase().rsplit_once(' ') {
            Some((value_part, unit_part)) => {
                let value = value_part
                    .parse::<i32>()
                    .map_err(|e| HistoricalParseError::ParseIntError(value_part.to_string(), e))?;
                match unit_part {
                    "S" => Ok(Self::seconds(value)),
                    "D" => Ok(Self::days(value)),
                    "W" => Ok(Self::weeks(value)),
                    "M" => Ok(Self::months(value)),
                    "Y" => Ok(Self::years(value)),
                    _ => Err(HistoricalParseError::Duration(val.to_string(), "Unsupported unit".to_string())),
                }
            }
            None => Err(HistoricalParseError::Duration(val.to_string(), "Missing delimiter".to_string())),
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

/// Helper trait to convert integer counts into [`Duration`]s.
pub trait ToDuration {
    /// Convert the value into seconds.
    fn seconds(&self) -> Duration;
    /// Convert the value into days.
    fn days(&self) -> Duration;
    /// Convert the value into weeks.
    fn weeks(&self) -> Duration;
    /// Convert the value into months.
    fn months(&self) -> Duration;
    /// Convert the value into years.
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

/// Histogram bucket entry returned from `reqHistogramData`.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct HistogramEntry {
    /// Price level represented by the bucket.
    pub price: f64,
    /// Total size accumulated at this price level.
    pub size: i32,
}

/// Container for historical bar responses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoricalData {
    /// Start timestamp of the requested window.
    pub start: OffsetDateTime,
    /// End timestamp of the requested window.
    pub end: OffsetDateTime,
    /// Bar data returned by the request.
    pub bars: Vec<Bar>,
}

/// Trading schedule describing sessions for a contract.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Overall start timestamp for the schedule.
    pub start: OffsetDateTime,
    /// Overall end timestamp for the schedule.
    pub end: OffsetDateTime,
    /// Time zone identifier.
    pub time_zone: String,
    /// Individual trade sessions.
    pub sessions: Vec<Session>,
}

/// Individual regular or special session entry.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Session {
    /// Calendar date for the session.
    pub reference: Date,
    /// Session start time.
    pub start: OffsetDateTime,
    /// Session end time.
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

/// Tick attributes accompanying bid/ask historical ticks.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct TickAttributeBidAsk {
    /// Indicates whether the bid is past the lower price band.
    pub bid_past_low: bool,
    /// Indicates whether the ask is past the upper price band.
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

/// Tick attributes accompanying trade historical ticks.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct TickAttributeLast {
    /// `true` if the trade occurred outside exchange limits.
    pub past_limit: bool,
    /// `true` if the trade is from an unreported trade source.
    pub unreported: bool,
}

/// Enumerates the data payload returned when requesting historical data.
#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub enum WhatToShow {
    /// Trade data including OHLC and volume.
    Trades,
    /// Mid-point prices (average of bid and ask).
    MidPoint,
    /// Bid quotes only.
    Bid,
    /// Ask quotes only.
    Ask,
    /// Bid/ask quote pairs.
    BidAsk,
    /// Historical volatility computed by IB.
    HistoricalVolatility,
    /// Option implied volatility.
    OptionImpliedVolatility,
    /// Exchange fee rates.
    FeeRate,
    /// Exchange trading schedule metadata.
    Schedule,
    /// Split-adjusted last price series.
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

/// Error returned when parsing an invalid `WhatToShow` value.
#[derive(Debug)]
pub struct WhatToShowParseError;

impl fmt::Display for WhatToShowParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid WhatToShow string")
    }
}

impl FromStr for WhatToShow {
    type Err = HistoricalParseError;

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
            _ => Err(HistoricalParseError::WhatToShow(s.to_string())),
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
/// Blocking historical market data helpers powered by the synchronous transport.
pub mod blocking {
    pub(crate) use super::sync::*;
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync::*;

#[cfg(feature = "async")]
pub use r#async::*;

/// Trait implemented by historical tick types that can decode IB messages.
pub trait TickDecoder<T> {
    /// Message discriminator emitted by TWS for this tick type.
    const MESSAGE_TYPE: IncomingMessages;
    /// Decode a batch of ticks, returning the payload and an end-of-stream flag.
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
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{TickSubscription, TickSubscriptionIter, TickSubscriptionOwnedIter, TickSubscriptionTimeoutIter, TickSubscriptionTryIter};

#[cfg(feature = "async")]
pub use r#async::TickSubscription;

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(
            "".parse::<Duration>(),
            Err(HistoricalParseError::Duration("".to_string(), "Empty string".to_string()))
        );
        assert_eq!(
            "1S".parse::<Duration>(),
            Err(HistoricalParseError::Duration("1S".to_string(), "Missing delimiter".to_string()))
        );
        assert_eq!(
            "1 X".parse::<Duration>(),
            Err(HistoricalParseError::Duration("1 X".to_string(), "Unsupported unit".to_string()))
        );

        let expected_int_error = "abc ".parse::<i32>().unwrap_err();
        assert_eq!(
            "abc ".parse::<Duration>(),
            Err(HistoricalParseError::ParseIntError("ABC".to_string(), expected_int_error))
        );

        assert_eq!(Duration::seconds(1), Duration::from("1 S"));
        assert_eq!(Duration::seconds(1), Duration::from(String::from("1 S")));
    }

    #[test]
    fn test_historical_parse_error_display() {
        let expected_int_error = "abc".parse::<i32>().unwrap_err();

        let cases = vec![
            (
                HistoricalParseError::BarSize("invalid".to_string()),
                "Invalid BarSize input 'invalid'".to_string(),
            ),
            (
                HistoricalParseError::Duration("invalid".to_string(), "Empty string".to_string()),
                "Invalid Duration input 'invalid' Empty string".to_string(),
            ),
            (
                HistoricalParseError::Duration("1S".to_string(), "Missing delimiter".to_string()),
                "Invalid Duration input '1S' Missing delimiter".to_string(),
            ),
            (
                HistoricalParseError::Duration("1 X".to_string(), "Unsupported unit".to_string()),
                "Invalid Duration input '1 X' Unsupported unit".to_string(),
            ),
            (
                HistoricalParseError::ParseIntError("abc ".to_string(), expected_int_error),
                "ParseIntError 'abc ' invalid digit found in string".to_string(),
            ),
            (
                HistoricalParseError::WhatToShow("invalid".to_string()),
                "Invalid WhatToShow input 'invalid'".to_string(),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
