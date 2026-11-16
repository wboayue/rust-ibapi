use time::OffsetDateTime;

use crate::client::blocking::ClientRequestBuilders;
use crate::client::ids::RequestId;
use crate::common::request_helpers::{self, empty_on_end_of_stream};
use crate::common::retry::blocking::retry_on_connection_reset;
use crate::contracts::Contract;
use crate::subscriptions::sync::Subscription;
use crate::{client::sync::Client, Error};

use super::common::{self, encoders};
use super::{
    BarRequest, BarSize, Duration, HeadTimestampRequest, HistogramEntry, HistogramRequest, HistoricalData, HistoricalDataFrame, Schedule,
    ScheduleRequest, TickDecoder, TickFrame, WhatToShow,
};
use crate::market_data::TradingHours;

impl Client {
    /// Returns the timestamp of earliest available historical data for a contract and data type.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{self, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("MSFT").build();
    /// let what_to_show = WhatToShow::Trades;
    /// let trading_hours = TradingHours::Regular;
    ///
    /// let result = client.head_timestamp(&contract, what_to_show, trading_hours).expect("head timestamp failed");
    ///
    /// print!("head_timestamp: {result:?}");
    /// ```
    pub fn head_timestamp(&self, contract: &Contract, what_to_show: WhatToShow, trading_hours: TradingHours) -> Result<OffsetDateTime, Error> {
        request_helpers::blocking::one_shot_cancelling::<HeadTimestampRequest>(self, |request_id| {
            encoders::encode_request_head_timestamp(request_id, contract, what_to_show, trading_hours.use_rth())
        })
    }

    /// Build a request for historical bar data.
    ///
    /// Required: a date spec via either [`HistoricalDataBuilder::duration`](super::HistoricalDataBuilder::duration)
    /// (with optional [`HistoricalDataBuilder::ending`](super::HistoricalDataBuilder::ending)) or
    /// [`HistoricalDataBuilder::between`](super::HistoricalDataBuilder::between). Terminals:
    /// [`HistoricalDataBuilder::fetch`](super::HistoricalDataBuilder::fetch) for a one-shot
    /// [`HistoricalData`] result; [`HistoricalDataBuilder::subscribe`](super::HistoricalDataBuilder::subscribe)
    /// and [`HistoricalDataBuilder::subscribe_with_updates`](super::HistoricalDataBuilder::subscribe_with_updates)
    /// for a `Subscription<HistoricalDataFrame>` that yields frames as they arrive.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `bar_size` - Bar size (resolution)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// // IBKR-native: amount of data ending at a specific time (or now if `.ending` is unset)
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .what_to_show(WhatToShow::Trades)
    ///     .duration(7.days())
    ///     .fetch()
    ///     .expect("historical data request failed");
    ///
    /// // Convenience: explicit date range; bars outside it are dropped
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical data request failed");
    /// # let _ = bars;
    /// ```
    pub fn historical_data<'a>(&'a self, contract: &'a Contract, bar_size: BarSize) -> super::HistoricalDataBuilder<'a, Self> {
        super::HistoricalDataBuilder::new(self, contract, bar_size, self.mint_request_id())
    }

    /// Build a request for [`Schedule`] data over the given duration.
    ///
    /// Defaults to regular trading hours, anchored at the current time. Use
    /// [`HistoricalScheduleBuilder::trading_hours`](super::HistoricalScheduleBuilder::trading_hours)
    /// to include extended-hours sessions, and
    /// [`HistoricalScheduleBuilder::ending`](super::HistoricalScheduleBuilder::ending)
    /// to anchor at a specific end date.
    ///
    /// # Arguments
    /// * `contract` - [Contract] to retrieve [Schedule] for.
    /// * `duration` - [Duration] of the interval to retrieve.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::ToDuration;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("GM").build();
    ///
    /// // Anchored to a specific end date:
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .ending(datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical schedule request failed");
    ///
    /// // Extended-hours sessions, ending now:
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .trading_hours(TradingHours::Extended)
    ///     .fetch()
    ///     .expect("historical schedule request failed");
    ///
    /// for session in &schedule.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn historical_schedules<'a>(&'a self, contract: &'a Contract, duration: Duration) -> super::HistoricalScheduleBuilder<'a, Self> {
        super::HistoricalScheduleBuilder::new(self, contract, duration)
    }

    /// Build a request for historical time & sales data (tick-by-tick).
    ///
    /// The terminal method selects the tick type:
    /// [`HistoricalTicksBuilder::last`](super::HistoricalTicksBuilder::last) /
    /// `.mid_point()` / `.bid_ask(IgnoreSize)`. Use
    /// [`HistoricalTicksBuilder::starting`](super::HistoricalTicksBuilder::starting) /
    /// `.ending()` to anchor the query (at least one is required per IBKR). The
    /// terminal sends the request and returns the complete reply as a `Vec`.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::IgnoreSize;
    /// use ibapi::market_data::TradingHours;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// // Trade ticks anchored at a start date:
    /// let trades = client
    ///     .historical_ticks(&contract, 100)
    ///     .starting(datetime!(2023-04-15 0:00 UTC))
    ///     .trading_hours(TradingHours::Regular)
    ///     .last()
    ///     .expect("historical ticks request failed");
    ///
    /// // Bid/ask ticks anchored at an end date, skipping size-only updates:
    /// let quotes = client
    ///     .historical_ticks(&contract, 100)
    ///     .ending(datetime!(2023-04-15 0:00 UTC))
    ///     .bid_ask(IgnoreSize::Yes)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in trades {
    ///     println!("{tick:?}");
    /// }
    /// # let _ = quotes;
    /// ```
    pub fn historical_ticks<'a>(&'a self, contract: &'a Contract, number_of_ticks: i32) -> super::HistoricalTicksBuilder<'a, Self> {
        super::HistoricalTicksBuilder::new(self, contract, number_of_ticks)
    }

    /// Requests data histogram of specified contract.
    ///
    /// # Arguments
    /// * `contract`  - [Contract] to retrieve [Histogram Entries](HistogramEntry) for.
    /// * `trading_hours` - Regular trading hours only, or include extended hours.
    /// * `period`    - The time period of each histogram bar (e.g., `BarSize::Day`, `BarSize::Week`, `BarSize::Month`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::historical::BarSize;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM").build();
    ///
    /// let histogram = client
    ///     .histogram_data(&contract, TradingHours::Regular, BarSize::Week)
    ///     .expect("histogram request failed");
    ///
    /// for item in &histogram {
    ///     println!("{item:?}");
    /// }
    /// ```
    pub fn histogram_data(&self, contract: &Contract, trading_hours: TradingHours, period: BarSize) -> Result<Vec<HistogramEntry>, Error> {
        request_helpers::blocking::one_shot_cancelling::<HistogramRequest>(self, |request_id| {
            encoders::encode_request_histogram_data(request_id, contract, trading_hours.use_rth(), period)
        })
        .or_else(empty_on_end_of_stream)
    }
}

pub(crate) fn historical_data(client: &Client, request: BarRequest, request_id: RequestId) -> Result<HistoricalData, Error> {
    common::validate_historical_data(request.end_date, request.what_to_show)?;
    // One deadline for both reads and every resend.
    let deadline = request_helpers::blocking::one_shot_deadline(client);
    let mut first_id = Some(request_id);

    retry_on_connection_reset(client, deadline, || {
        request_helpers::blocking::check_deadline(deadline)?;
        // The first attempt uses the builder's id; a resend takes a new one.
        let builder = match first_id.take() {
            Some(id) => client.request_with_id(id),
            None => client.request(),
        };
        let message = request.encode(builder.request_id(), false)?;

        // The window frame is a second read, which is why this is not a
        // one-shot and cannot use the helpers in `common::request_helpers`.
        // Dropping the subscription before it, on any error below, sends
        // TWS's cancel; after it, nothing is sent.
        let subscription = builder.send::<HistoricalDataFrame>(message)?;

        let bars = request_helpers::blocking::next_frame(&subscription, deadline)?.into_bars()?;
        // Only the window frame proves the download is complete, so a close
        // before it is an error.
        let (start, end) = request_helpers::blocking::next_frame(&subscription, deadline)?.into_window()?;

        Ok(HistoricalData { start, end, bars })
    })
}

/// `request.end_date` is `None`: IBKR requires it with `keep_up_to_date`.
/// The context flag keeps the mid-stream `End` from skipping drop's cancel.
pub(crate) fn historical_data_updates(
    client: &Client,
    request: BarRequest,
    request_id: RequestId,
    buffer_limit: Option<usize>,
) -> Result<Subscription<HistoricalDataFrame>, Error> {
    let message = request.encode(request_id.raw(), true)?;
    let context = client.decoder_context().with_keep_up_to_date(true);
    client
        .request_with_id(request_id)
        .send_capped_with_context(message, buffer_limit, context)
}

/// Sent once, never resent: the caller owns the request. Reached through
/// [`HistoricalDataBuilder::subscribe`](super::HistoricalDataBuilder::subscribe).
pub(crate) fn historical_data_subscription(
    client: &Client,
    request: BarRequest,
    request_id: RequestId,
    buffer_limit: Option<usize>,
) -> Result<Subscription<HistoricalDataFrame>, Error> {
    common::validate_historical_data(request.end_date, request.what_to_show)?;
    let message = request.encode(request_id.raw(), false)?;
    client.request_with_id(request_id).send_capped(message, buffer_limit)
}

// pub(crate) internal plumbing called from `HistoricalTicksBuilder`; the
// public API is already a builder, so flat args here are the deliberate
// seam between the typed builder and the wire encoder. Internal, so the param
// budget doesn't apply; the allow is for clippy's 8-arg lint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn historical_ticks<T: TickDecoder<T>>(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    ignore_size: bool,
) -> Result<Vec<T>, Error> {
    let deadline = request_helpers::blocking::one_shot_deadline(client);

    retry_on_connection_reset(client, deadline, || {
        request_helpers::blocking::check_deadline(deadline)?;
        let builder = client.request();
        let request = encoders::encode_request_historical_ticks(
            builder.request_id(),
            contract,
            start,
            end,
            number_of_ticks,
            what_to_show,
            trading_hours.use_rth(),
            ignore_size,
        )?;

        // Dropping the subscription before the `done` frame, on any error
        // below, sends TWS's cancel; after it, nothing is sent.
        let subscription = builder.send::<TickFrame<T>>(request)?;

        let mut ticks = Vec::new();
        loop {
            let frame = request_helpers::blocking::next_frame(&subscription, deadline)?;
            ticks.extend(frame.ticks);
            if frame.done {
                return Ok(ticks);
            }
        }
    })
}

pub(crate) fn historical_schedule(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    trading_hours: TradingHours,
) -> Result<Schedule, Error> {
    request_helpers::blocking::one_shot_cancelling::<ScheduleRequest>(client, |request_id| {
        encoders::encode_request_historical_data(
            request_id,
            contract,
            end_date,
            duration,
            BarSize::Day,
            WhatToShow::Schedule,
            trading_hours.use_rth(),
            false,
        )
    })
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
