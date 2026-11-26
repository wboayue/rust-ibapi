use time::OffsetDateTime;

use crate::client::ids::RequestId;
use crate::client::ClientRequestBuilders;
use crate::common::request_helpers::{self, empty_on_end_of_stream};
use crate::common::retry::retry_on_connection_reset;
use crate::contracts::Contract;
use crate::subscriptions::r#async::Subscription;
use crate::{Client, Error};

use super::common::{self, encoders};
use super::{
    BarRequest, BarSize, Duration, HeadTimestampRequest, HistogramEntry, HistogramRequest, HistoricalData, HistoricalDataFrame, Schedule,
    ScheduleRequest, TickDecoder, TickFrame, WhatToShow,
};
use crate::market_data::TradingHours;

// === Public API Functions ===

impl Client {
    /// Returns the timestamp of earliest available historical data for a contract and data type.
    ///
    /// # Arguments
    /// * `contract` - [Contract] to retrieve the head timestamp for.
    /// * `what_to_show` - requested bar type: [WhatToShow].
    /// * `trading_hours` - Use [TradingHours::Regular] for data generated only during regular trading hours, or [TradingHours::Extended] to include data from outside regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::WhatToShow;
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("MSFT").build();
    ///     let result = client
    ///         .head_timestamp(&contract, WhatToShow::Trades, TradingHours::Regular)
    ///         .await
    ///         .expect("head timestamp failed");
    ///
    ///     println!("head_timestamp: {result:?}");
    /// }
    /// ```
    pub async fn head_timestamp(&self, contract: &Contract, what_to_show: WhatToShow, trading_hours: TradingHours) -> Result<OffsetDateTime, Error> {
        request_helpers::one_shot_cancelling::<HeadTimestampRequest>(self, |request_id| {
            encoders::encode_request_head_timestamp(request_id, contract, what_to_show, trading_hours.use_rth())
        })
        .await
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
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     // IBKR-native: amount of data ending at a specific time (or now)
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .what_to_show(HistoricalWhatToShow::Trades)
    ///         .duration(7.days())
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///
    ///     // Convenience: explicit date range; bars outside it are dropped
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///     let _ = bars;
    /// }
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
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::TradingHours;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("GM").build();
    ///
    ///     // Anchored to a specific end date:
    ///     let schedule = client
    ///         .historical_schedules(&contract, 30.days())
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
    ///         .fetch()
    ///         .await
    ///         .expect("historical schedule request failed");
    ///
    ///     // Extended-hours sessions, ending now:
    ///     let schedule = client
    ///         .historical_schedules(&contract, 30.days())
    ///         .trading_hours(TradingHours::Extended)
    ///         .fetch()
    ///         .await
    ///         .expect("historical schedule request failed");
    ///
    ///     for session in &schedule.sessions {
    ///         println!("{session:?}");
    ///     }
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
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::IgnoreSize;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     // Trade ticks anchored at a start date:
    ///     let trades = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .last()
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     for tick in &trades {
    ///         println!("{tick:?}");
    ///     }
    ///
    ///     // Bid/ask ticks anchored at an end date, skipping size-only updates:
    ///     let _quotes = client
    ///         .historical_ticks(&contract, 100)
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
    ///         .bid_ask(IgnoreSize::Yes)
    ///         .await
    ///         .expect("historical ticks request failed");
    /// }
    /// ```
    pub fn historical_ticks<'a>(&'a self, contract: &'a Contract, number_of_ticks: i32) -> super::HistoricalTicksBuilder<'a, Self> {
        super::HistoricalTicksBuilder::new(self, contract, number_of_ticks)
    }

    /// Requests data histogram of specified contract.
    ///
    /// # Arguments
    /// * `contract`      - [Contract] to retrieve [HistogramEntry] data for.
    /// * `trading_hours` - Regular trading hours only, or include extended hours.
    /// * `period`        - The time period of each histogram bar (e.g., `BarSize::Day`, `BarSize::Week`, `BarSize::Month`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::BarSize;
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("GM").build();
    ///     let histogram = client
    ///         .histogram_data(&contract, TradingHours::Regular, BarSize::Week)
    ///         .await
    ///         .expect("histogram request failed");
    ///
    ///     for item in &histogram {
    ///         println!("{item:?}");
    ///     }
    /// }
    /// ```
    pub async fn histogram_data(&self, contract: &Contract, trading_hours: TradingHours, period: BarSize) -> Result<Vec<HistogramEntry>, Error> {
        request_helpers::one_shot_cancelling::<HistogramRequest>(self, |request_id| {
            encoders::encode_request_histogram_data(request_id, contract, trading_hours.use_rth(), period)
        })
        .await
        .or_else(empty_on_end_of_stream)
    }
}

pub(crate) async fn historical_data(client: &Client, request: BarRequest<'_>, request_id: RequestId) -> Result<HistoricalData, Error> {
    common::validate_historical_data(request.end_date, request.what_to_show)?;
    // One deadline for both reads and every resend.
    let deadline = request_helpers::one_shot_deadline(client);
    let mut first_id = Some(request_id);

    retry_on_connection_reset(client, deadline, || {
        // The first attempt uses the builder's id; a resend takes a new one.
        let first_id = first_id.take();
        async move {
            request_helpers::check_deadline(deadline)?;
            let builder = match first_id {
                Some(id) => client.request_with_id(id),
                None => client.request(),
            };
            let message = request.encode(builder.request_id(), false)?;

            // The window frame is a second read, which is why this is not a
            // one-shot and cannot use the helpers in `common::request_helpers`.
            // Dropping the subscription before it, on any error below or with
            // the future, sends TWS's cancel; after it, nothing is sent.
            let mut subscription = builder.send::<HistoricalDataFrame>(message).await?;

            let bars = request_helpers::next_frame(&mut subscription, deadline).await?.into_bars()?;
            // Only the window frame proves the download is complete, so a close
            // before it is an error.
            let (start, end) = request_helpers::next_frame(&mut subscription, deadline).await?.into_window()?;

            Ok(HistoricalData { start, end, bars })
        }
    })
    .await
}

/// `request.end_date` is `None`: IBKR requires it with `keep_up_to_date`.
/// The context flag keeps the mid-stream `End` from skipping drop's cancel.
pub(crate) async fn historical_data_updates(
    client: &Client,
    request: BarRequest<'_>,
    request_id: RequestId,
    buffer_limit: Option<usize>,
) -> Result<Subscription<HistoricalDataFrame>, Error> {
    let message = request.encode(request_id.raw(), true)?;
    let context = client.decoder_context().with_keep_up_to_date(true);
    client
        .request_with_id(request_id)
        .send_capped_with_context(message, buffer_limit, context)
        .await
}

/// Sent once, never resent: the caller owns the request. Reached through
/// [`HistoricalDataBuilder::subscribe`](super::HistoricalDataBuilder::subscribe).
pub(crate) async fn historical_data_subscription(
    client: &Client,
    request: BarRequest<'_>,
    request_id: RequestId,
    buffer_limit: Option<usize>,
) -> Result<Subscription<HistoricalDataFrame>, Error> {
    common::validate_historical_data(request.end_date, request.what_to_show)?;
    let message = request.encode(request_id.raw(), false)?;
    client.request_with_id(request_id).send_capped(message, buffer_limit).await
}

// pub(crate) internal plumbing called from `HistoricalTicksBuilder`; the
// public API is already a builder, so flat args here are the deliberate
// seam between the typed builder and the wire encoder. Internal, so the param
// budget doesn't apply; the allow is for clippy's 8-arg lint.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn historical_ticks<T: TickDecoder<T> + Send + 'static>(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    ignore_size: bool,
) -> Result<Vec<T>, Error> {
    let deadline = request_helpers::one_shot_deadline(client);

    retry_on_connection_reset(client, deadline, || async move {
        request_helpers::check_deadline(deadline)?;
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
        // below or with the future, sends TWS's cancel; after it, nothing is
        // sent.
        let mut subscription = builder.send::<TickFrame<T>>(request).await?;

        let mut ticks = Vec::new();
        loop {
            let frame = request_helpers::next_frame(&mut subscription, deadline).await?;
            ticks.extend(frame.ticks);
            if frame.done {
                return Ok(ticks);
            }
        }
    })
    .await
}

pub(crate) async fn historical_schedule(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    trading_hours: TradingHours,
) -> Result<Schedule, Error> {
    request_helpers::one_shot_cancelling::<ScheduleRequest>(client, |request_id| {
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
    .await
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
