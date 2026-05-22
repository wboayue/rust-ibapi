use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::market_data::historical::{BarSize, Duration, HistoricalBarUpdate, HistoricalData, WhatToShow};
use crate::market_data::TradingHours;
use crate::Error;

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;

/// Builder for historical bar data requests.
///
/// Required: one of [`duration`](Self::duration) (with optional [`ending`](Self::ending))
/// or [`between`](Self::between) to specify the time range. Mixing the two styles
/// errors at the terminal.
#[must_use = "HistoricalDataBuilder does nothing until you call .fetch() or .stream()"]
pub struct HistoricalDataBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    bar_size: BarSize,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    duration: Option<Duration>,
    ending: Option<OffsetDateTime>,
    between: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl<'a, C> HistoricalDataBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, bar_size: BarSize) -> Self {
        Self {
            client,
            contract,
            bar_size,
            what_to_show: WhatToShow::Trades,
            trading_hours: TradingHours::Regular,
            duration: None,
            ending: None,
            between: None,
        }
    }

    /// Override the data type to retrieve (defaults to [`WhatToShow::Trades`]).
    pub fn what_to_show(mut self, what_to_show: WhatToShow) -> Self {
        self.what_to_show = what_to_show;
        self
    }

    /// Override regular- vs extended-hours (defaults to [`TradingHours::Regular`]).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }

    /// Amount of data going back from the end date (now if [`ending`](Self::ending) is unset).
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Anchor the query at a specific end date (defaults to now).
    pub fn ending(mut self, end_date: OffsetDateTime) -> Self {
        self.ending = Some(end_date);
        self
    }

    /// Convenience: specify an explicit date range (computes duration internally).
    pub fn between(mut self, start: OffsetDateTime, end: OffsetDateTime) -> Self {
        self.between = Some((start, end));
        self
    }

    /// Resolve the builder's date spec into (end_date, duration). Errors if the
    /// user mixed `.between` with `.duration`/`.ending`, or set neither.
    fn resolve_date_spec(&self) -> Result<(Option<OffsetDateTime>, Duration), Error> {
        match (self.between, self.duration, self.ending) {
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(Error::InvalidArgument(
                "historical_data: cannot mix .between(...) with .duration()/.ending()".to_owned(),
            )),
            (Some((start, end)), None, None) => {
                if end <= start {
                    return Err(Error::InvalidArgument(
                        "historical_data: .between(start, end) requires end > start".to_owned(),
                    ));
                }
                let seconds = (end - start).whole_seconds();
                if seconds > i32::MAX as i64 {
                    return Err(Error::InvalidArgument(
                        "historical_data: .between(start, end) range exceeds i32::MAX seconds".to_owned(),
                    ));
                }
                Ok((Some(end), Duration::seconds(seconds as i32)))
            }
            (None, Some(duration), ending) => Ok((ending, duration)),
            (None, None, _) => Err(Error::InvalidArgument(
                "historical_data: must set .duration() or .between(...)".to_owned(),
            )),
        }
    }

    /// Resolve the builder for a streaming request (`keep_up_to_date = true`).
    /// IBKR requires `end_date = None` for streaming, so `.ending()` / `.between()`
    /// are rejected.
    fn resolve_for_stream(&self) -> Result<Duration, Error> {
        if self.ending.is_some() || self.between.is_some() {
            return Err(Error::InvalidArgument(
                "historical_data().stream(): IBKR requires end_date = None for streaming; drop .ending() / .between()".to_owned(),
            ));
        }
        self.duration
            .ok_or_else(|| Error::InvalidArgument("historical_data().stream(): must set .duration()".to_owned()))
    }
}

#[cfg(feature = "sync")]
impl<'a> HistoricalDataBuilder<'a, crate::client::sync::Client> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration};
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// // 7 days of hourly bars, ending now:
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .duration(7.days())
    ///     .fetch()
    ///     .expect("historical data request failed");
    ///
    /// // Equivalent via explicit date range:
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical data request failed");
    /// # let _ = bars;
    /// ```
    pub fn fetch(self) -> Result<HistoricalData, Error> {
        let (end_date, duration) = self.resolve_date_spec()?;
        crate::market_data::historical::sync::historical_data(
            self.client,
            self.contract,
            end_date,
            duration,
            self.bar_size,
            self.what_to_show,
            self.trading_hours,
        )
    }

    /// Submit a streaming request (`keep_up_to_date = true`) and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalBarUpdate`].
    /// IBKR sends initial bars, then per-bar updates as they form.
    ///
    /// Rejects builders that called [`ending`](Self::ending) or [`between`](Self::between) —
    /// IBKR requires `end_date = None` for streaming.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("SPY").build();
    ///
    /// let subscription = client
    ///     .historical_data(&contract, BarSize::Min15)
    ///     .duration(1.days())
    ///     .stream()
    ///     .expect("streaming request failed");
    /// # drop(subscription);
    /// ```
    pub fn stream(self) -> Result<crate::subscriptions::sync::Subscription<HistoricalBarUpdate>, Error> {
        let duration = self.resolve_for_stream()?;
        crate::market_data::historical::sync::historical_data_stream(
            self.client,
            self.contract,
            duration,
            self.bar_size,
            self.what_to_show,
            self.trading_hours,
        )
    }
}

#[cfg(feature = "async")]
impl<'a> HistoricalDataBuilder<'a, crate::client::r#async::Client> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     // 7 days of hourly bars, ending now:
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .duration(7.days())
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///
    ///     // Equivalent via explicit date range:
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///     let _ = bars;
    /// }
    /// ```
    pub async fn fetch(self) -> Result<HistoricalData, Error> {
        let (end_date, duration) = self.resolve_date_spec()?;
        crate::market_data::historical::r#async::historical_data(
            self.client,
            self.contract,
            end_date,
            duration,
            self.bar_size,
            self.what_to_show,
            self.trading_hours,
        )
        .await
    }

    /// Submit a streaming request (`keep_up_to_date = true`) and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalBarUpdate`].
    ///
    /// Rejects builders that called [`ending`](Self::ending) or [`between`](Self::between) —
    /// IBKR requires `end_date = None` for streaming.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("SPY").build();
    ///
    ///     let subscription = client
    ///         .historical_data(&contract, HistoricalBarSize::Min15)
    ///         .duration(1.days())
    ///         .stream()
    ///         .await
    ///         .expect("streaming request failed");
    ///     drop(subscription);
    /// }
    /// ```
    pub async fn stream(self) -> Result<crate::subscriptions::Subscription<HistoricalBarUpdate>, Error> {
        let duration = self.resolve_for_stream()?;
        crate::market_data::historical::r#async::historical_data_stream(
            self.client,
            self.contract,
            duration,
            self.bar_size,
            self.what_to_show,
            self.trading_hours,
        )
        .await
    }
}
