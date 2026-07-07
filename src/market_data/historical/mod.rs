//! Historical market data API.
//!
//! ## Canonical paths
//!
//! Historical-data methods are inherent methods on `ibapi::Client` — call
//! `client.historical_data(...)`, `client.historical_ticks_mid_point(...)`,
//! etc. The public types (`Bar`, `BarSize`, `WhatToShow`, `TickSubscription`,
//! and the per-tick payload types) live at
//! `ibapi::market_data::historical::*` and via `ibapi::prelude::*`.
//!
//! The `historical::sync` and `historical::r#async` submodules where the impls
//! live are `#[doc(hidden)]`: still reachable as paths for crate-internal use,
//! but intentionally absent from the docs.rs navigation. Prefer the canonical
//! `Client` method calls and the `market_data::historical::*` type spellings.
//! Raw-identifier syntax (`market_data::historical::r#async::...`) is the
//! giveaway that the spelling is non-canonical.

use std::fmt::{self, Debug, Display};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::{Error, ToField};

pub(crate) mod common;

mod builder;
pub use builder::{HistoricalDataBuilder, HistoricalScheduleBuilder, HistoricalTicksBuilder};

#[doc(hidden)]
#[cfg(feature = "sync")]
pub mod sync;

#[doc(hidden)]
#[cfg(feature = "async")]
pub mod r#async;

/// Errors surfaced while parsing historical market data parameters.
#[derive(Clone, Debug, Error, PartialEq)]
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

/// Timestamp of a historical bar.
///
/// Daily (and longer) bars carry only a calendar date (`YYYYMMDD` on the
/// wire); intraday bars carry a full UTC datetime (unix seconds on the wire).
/// `BarTimestamp` preserves that distinction instead of coercing daily bars to
/// midnight UTC.
///
/// # Examples
///
/// ```no_run
/// use ibapi::market_data::historical::BarTimestamp;
///
/// fn format_bar_time(ts: &BarTimestamp) -> String {
///     match ts {
///         BarTimestamp::Date(d) => format!("{d}"),
///         BarTimestamp::DateTime(dt) => {
///             format!("{:02}:{:02}", dt.hour(), dt.minute())
///         }
///     }
/// }
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum BarTimestamp {
    /// Daily / weekly / monthly bars — only the trading day is meaningful.
    Date(Date),
    /// Intraday bars — full point-in-time timestamp.
    DateTime(OffsetDateTime),
}

impl PartialOrd for BarTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BarTimestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Date(a), Self::Date(b)) => a.cmp(b),
            (Self::DateTime(a), Self::DateTime(b)) => a.cmp(b),
            (Self::Date(d), Self::DateTime(dt)) => d.midnight().assume_utc().cmp(dt),
            (Self::DateTime(dt), Self::Date(d)) => dt.cmp(&d.midnight().assume_utc()),
        }
    }
}

impl Display for BarTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => {
                let fmt = format_description!("[year][month][day]");
                write!(f, "{}", d.format(&fmt).unwrap_or_default())
            }
            Self::DateTime(dt) => write!(f, "{}", dt.unix_timestamp()),
        }
    }
}

impl FromStr for BarTimestamp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()) {
            let fmt = format_description!("[year][month][day]");
            let d = Date::parse(s, fmt).map_err(|e| Error::parse_field(s, e.to_string()))?;
            return Ok(Self::Date(d));
        }
        let secs: i64 = s.parse().map_err(|e: std::num::ParseIntError| Error::parse_field(s, e.to_string()))?;
        OffsetDateTime::from_unix_timestamp(secs)
            .map(Self::DateTime)
            .map_err(|e| Error::parse_field(s, e.to_string()))
    }
}

impl BarTimestamp {
    /// Returns `true` for the [`Date`](Self::Date) variant.
    pub fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    /// Returns `true` for the [`DateTime`](Self::DateTime) variant.
    pub fn is_date_time(&self) -> bool {
        matches!(self, Self::DateTime(_))
    }
}

impl From<Date> for BarTimestamp {
    fn from(d: Date) -> Self {
        Self::Date(d)
    }
}

impl From<OffsetDateTime> for BarTimestamp {
    fn from(dt: OffsetDateTime) -> Self {
        Self::DateTime(dt)
    }
}

/// Bar describes the historical data bar.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct Bar {
    /// The bar's timestamp — either a calendar date (daily+ bars) or a full
    /// datetime (intraday bars). See [`BarTimestamp`] for details.
    pub date: BarTimestamp,
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

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
    /// Ten-minute bars.
    Min10,
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
            Self::Min10 => write!(f, "10 mins"),
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
            "SEC10" => Ok(Self::Sec10),
            "SEC15" => Ok(Self::Sec15),
            "SEC30" => Ok(Self::Sec30),
            "MIN" => Ok(Self::Min),
            "MIN2" => Ok(Self::Min2),
            "MIN3" => Ok(Self::Min3),
            "MIN5" => Ok(Self::Min5),
            "MIN10" => Ok(Self::Min10),
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct HistogramEntry {
    /// Price level represented by the bucket.
    pub price: f64,
    /// Total size accumulated at this price level.
    pub size: i32,
}

/// Container for historical bar responses.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoricalData {
    /// Start timestamp of the requested window.
    pub start: OffsetDateTime,
    /// End timestamp of the requested window.
    pub end: OffsetDateTime,
    /// Bar data returned by the request.
    pub bars: Vec<Bar>,
}

/// Update from historical data streaming with keepUpToDate=true.
///
/// When requesting historical data with `keepUpToDate=true`, IBKR first sends
/// the initial historical bars as a `Historical` variant, then continues
/// streaming real-time updates for the current bar as `Update` variants.
/// The current bar is updated approximately every 4-6 seconds until a new
/// bar begins.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HistoricalBarUpdate {
    /// Initial batch of historical bars. Always received first.
    Historical(HistoricalData),
    /// Real-time update of the current (incomplete) bar.
    /// Multiple updates with the same timestamp will be sent as the bar builds.
    Update(Bar),
    /// End of the streaming subscription.
    End {
        /// Start date of the historical data range.
        start: OffsetDateTime,
        /// End date of the historical data range.
        end: OffsetDateTime,
    },
}

impl StreamDecoder<HistoricalBarUpdate> for HistoricalBarUpdate {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[
        IncomingMessages::HistoricalData,
        IncomingMessages::HistoricalDataUpdate,
        IncomingMessages::HistoricalDataEnd,
        IncomingMessages::Error,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalData => Ok(Self::Historical(common::decoders::decode_historical_data(message)?)),
            IncomingMessages::HistoricalDataUpdate => Ok(Self::Update(common::decoders::decode_historical_data_update(message)?)),
            IncomingMessages::HistoricalDataEnd => {
                let (start, end) = common::decoders::decode_historical_data_end(message)?;
                Ok(Self::End { start, end })
            }
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel historical data");
        common::encoders::encode_cancel_historical_data(request_id)
    }
}

/// Trading schedule describing sessions for a contract.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct TickAttributeBidAsk {
    /// Indicates whether the bid is past the lower price band.
    pub bid_past_low: bool,
    /// Indicates whether the ask is past the upper price band.
    pub ask_past_high: bool,
}

/// The historical last tick's description. Used when requesting historical tick data with whatToShow = TRADES.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct TickAttributeLast {
    /// `true` if the trade occurred outside exchange limits.
    pub past_limit: bool,
    /// `true` if the trade is from an unreported trade source.
    pub unreported: bool,
}

/// Enumerates the data payload returned when requesting historical data.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
    /// Aggregated trade data (wire value `AGGTRADES`).
    ///
    /// Required for crypto contracts: TWS rejects `TRADES` for crypto with
    /// error 10299, so crypto trade-price series must be requested as `AGGTRADES`.
    AggTrades,
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
            Self::AggTrades => write!(f, "AGGTRADES"),
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
            "AGGTRADES" => Ok(Self::AggTrades),
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

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync::*;

// Async API methods are now on Client directly via historical/async.rs
// Re-export non-function items
#[cfg(feature = "async")]
pub use r#async::TickSubscription;

/// Trait implemented by historical tick types that can decode IB messages.
///
/// External users can name the trait as a bound on [`TickSubscription<T>`], but
/// cannot call [`decode`](Self::decode) themselves — the argument
/// `&mut ResponseMessage` is crate-private. Implementations are restricted to
/// the three built-in tick types ([`TickBidAsk`], [`TickLast`],
/// [`TickMidpoint`]); custom implementations are not supported.
#[allow(private_interfaces)]
pub trait TickDecoder<T> {
    /// Message discriminator emitted by TWS for this tick type.
    const MESSAGE_TYPE: IncomingMessages;
    /// Decode a batch of ticks, returning the payload and an end-of-stream flag.
    fn decode(message: &ResponseMessage) -> Result<(Vec<T>, bool), Error>;
}

#[allow(private_interfaces)]
impl TickDecoder<TickBidAsk> for TickBidAsk {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTickBidAsk;

    fn decode(message: &ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
        common::decoders::decode_historical_ticks_bid_ask(message)
    }
}

#[allow(private_interfaces)]
impl TickDecoder<TickLast> for TickLast {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTickLast;

    fn decode(message: &ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
        common::decoders::decode_historical_ticks_last(message)
    }
}

#[allow(private_interfaces)]
impl TickDecoder<TickMidpoint> for TickMidpoint {
    const MESSAGE_TYPE: IncomingMessages = IncomingMessages::HistoricalTick;

    fn decode(message: &ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
        common::decoders::decode_historical_ticks_mid_point(message)
    }
}

// Re-export TickSubscription and iterator types based on active feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{TickSubscription, TickSubscriptionIter, TickSubscriptionOwnedIter, TickSubscriptionTimeoutIter, TickSubscriptionTryIter};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
