use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use log::{error, warn};
use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::common::SubscriptionItem;
use crate::subscriptions::r#async::Subscription;
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};
use crate::{Client, Error, MAX_RETRIES};

use super::common::tick::{classify, TickAction};
use super::common::{self, decoders, encoders};
use super::{BarSize, Duration, HistogramEntry, HistoricalBarUpdate, HistoricalData, Schedule, TickDecoder, WhatToShow};
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
        check_version(self.server_version(), Features::HEAD_TIMESTAMP)?;

        let builder = self.request();
        let request = encoders::encode_request_head_timestamp(builder.request_id(), contract, what_to_show, trading_hours.use_rth())?;
        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(message)) if message.message_type() == IncomingMessages::HeadTimestamp => Ok(decoders::decode_head_timestamp(&message)?),
            Some(Ok(message)) => Err(Error::unexpected_response(&message)),
            Some(Err(e)) => Err(e),
            None => {
                // Connection might have been reset, retry
                Box::pin(self.head_timestamp(contract, what_to_show, trading_hours)).await
            }
        }
    }

    /// Build a request for historical bar data.
    ///
    /// Required: a date spec via either [`HistoricalDataBuilder::duration`](super::HistoricalDataBuilder::duration)
    /// (with optional [`HistoricalDataBuilder::ending`](super::HistoricalDataBuilder::ending)) or
    /// [`HistoricalDataBuilder::between`](super::HistoricalDataBuilder::between). Terminals:
    /// [`HistoricalDataBuilder::fetch`](super::HistoricalDataBuilder::fetch) for a one-shot
    /// [`HistoricalData`] result; [`HistoricalDataBuilder::stream`](super::HistoricalDataBuilder::stream)
    /// for a `Subscription<HistoricalBarUpdate>` that yields bars as they arrive.
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
    ///     // Convenience: explicit date range (computes duration internally)
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
        super::HistoricalDataBuilder::new(self, contract, bar_size)
    }

    /// Build a request for [`Schedule`] data over the given duration.
    ///
    /// Defaults to anchoring at the current time. Use [`HistoricalScheduleBuilder::ending`](super::HistoricalScheduleBuilder::ending)
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
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("GM").build();
    ///
    ///     // Ending now:
    ///     let schedule = client
    ///         .historical_schedules(&contract, 30.days())
    ///         .fetch()
    ///         .await
    ///         .expect("historical schedule request failed");
    ///
    ///     // Anchored to a specific end date:
    ///     let schedule = client
    ///         .historical_schedules(&contract, 30.days())
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
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
    /// [`HistoricalTicksBuilder::trade`](super::HistoricalTicksBuilder::trade) /
    /// `.mid_point()` / `.bid_ask(IgnoreSize)`. Use
    /// [`HistoricalTicksBuilder::starting`](super::HistoricalTicksBuilder::starting) /
    /// `.ending()` to anchor the query (at least one is required per IBKR).
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
    ///     let mut trades = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .trade()
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     while let Some(tick) = trades.next().await {
    ///         println!("{tick:?}");
    ///     }
    ///
    ///     // Bid/ask ticks anchored at an end date, ignoring tick sizes:
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

    /// Cancels an in-flight historical ticks request.
    ///
    /// # Arguments
    /// * `request_id` - The request ID of the historical ticks subscription to cancel.
    pub async fn cancel_historical_ticks(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version(), Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_historical_ticks(request_id)?;
        self.send_message(message).await?;
        Ok(())
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
        check_version(self.server_version(), Features::HISTOGRAM)?;

        loop {
            let builder = self.request();
            let request = encoders::encode_request_histogram_data(builder.request_id(), contract, trading_hours.use_rth(), period)?;
            let mut subscription = builder.send_raw(request).await?;

            match subscription.next().await {
                Some(Ok(message)) => return decoders::decode_histogram_data(&message),
                Some(Err(e)) => return Err(e),
                None => continue, // Connection reset, retry
            }
        }
    }
}

pub(crate) async fn historical_data(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
) -> Result<HistoricalData, Error> {
    common::validate_historical_data(client.server_version(), contract, end_date, Some(what_to_show))?;

    for _ in 0..MAX_RETRIES {
        let builder = client.request();
        let request = encoders::encode_request_historical_data(
            builder.request_id(),
            contract,
            end_date,
            duration,
            bar_size,
            Some(what_to_show),
            trading_hours.use_rth(),
            false,
            &Vec::<crate::contracts::TagValue>::default(),
        )?;

        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(message)) if message.message_type() == IncomingMessages::HistoricalData => {
                let mut data = decoders::decode_historical_data(&message)?;

                if let Some(Ok(end_msg)) = subscription.next().await {
                    let (start, end) = decoders::decode_historical_data_end(&end_msg)?;
                    data.start = start;
                    data.end = end;
                }

                return Ok(data);
            }
            Some(Ok(message)) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
            Some(Ok(message)) => return Err(Error::unexpected_response(&message)),
            Some(Err(e)) => return Err(e),
            None => continue, // Connection reset, retry
        }
    }

    Err(Error::ConnectionReset)
}

pub(crate) async fn historical_data_stream(
    client: &Client,
    contract: &Contract,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
) -> Result<Subscription<HistoricalBarUpdate>, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_historical_data(
        builder.request_id(),
        contract,
        None, // IBKR requires end_date=None when keep_up_to_date=true
        duration,
        bar_size,
        Some(what_to_show),
        trading_hours.use_rth(),
        true, // keep_up_to_date — the whole point of .stream()
        &Vec::<crate::contracts::TagValue>::default(),
    )?;

    builder.send::<HistoricalBarUpdate>(request).await
}

// pub(crate) internal plumbing called from `HistoricalTicksBuilder`; the
// public API is already a builder, so flat args here are the deliberate
// seam between the typed builder and the wire encoder (rule 19 canary
// acceptable for builder-fed helpers).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn historical_ticks<T: TickDecoder<T> + Send>(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    ignore_size: bool,
) -> Result<TickSubscription<T>, Error> {
    check_version(client.server_version(), Features::HISTORICAL_TICKS)?;

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
    let request_id = builder.request_id();
    let subscription = builder.send_raw(request).await?;

    Ok(TickSubscription::new(subscription, request_id, Arc::clone(&client.message_bus)))
}

pub(crate) async fn historical_schedule(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
) -> Result<Schedule, Error> {
    common::validate_historical_data(client.server_version(), contract, end_date, Some(WhatToShow::Schedule))?;

    loop {
        let builder = client.request();
        let request = encoders::encode_request_historical_data(
            builder.request_id(),
            contract,
            end_date,
            duration,
            BarSize::Day,
            Some(WhatToShow::Schedule),
            true,
            false,
            &Vec::<crate::contracts::TagValue>::default(),
        )?;

        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(message)) if message.message_type() == IncomingMessages::HistoricalSchedule => {
                return decoders::decode_historical_schedule(&message)
            }
            Some(Ok(message)) => return Err(Error::unexpected_response(&message)),
            Some(Err(e)) => return Err(e),
            None => continue, // Connection reset, retry
        }
    }
}

// === TickSubscription and related types ===

/// Async subscription handle that decodes historical tick batches as they arrive.
///
/// `TickSubscription<T>` is a [`Stream`] of `Result<SubscriptionItem<T>, Error>`,
/// the same shape as the async [`Subscription`](crate::subscriptions::Subscription):
///
/// * `Some(Ok(SubscriptionItem::Data(tick)))` — a decoded tick.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice bound to
///   this request; the stream stays open.
/// * `Some(Err(e))` — terminal error (decode or transport); the stream is over.
/// * `None` — the stream has ended.
///
/// Bring [`StreamExt`](futures::StreamExt) into scope for `.next().await`, and
/// [`SubscriptionItemStreamExt`](crate::subscriptions::SubscriptionItemStreamExt)
/// for `.filter_data()` when you only want ticks. Both are in
/// [`ibapi::prelude`](crate::prelude).
#[must_use = "TickSubscription must be polled (.next().await or .filter_data()) to receive ticks; dropping it cancels the request"]
pub struct TickSubscription<T: TickDecoder<T> + Send> {
    done: bool,
    stream_ended: bool,
    messages: AsyncInternalSubscription,
    buffer: VecDeque<T>,
    request_id: i32,
    message_bus: Arc<dyn AsyncMessageBus>,
    cancelled: AtomicBool,
}

impl<T: TickDecoder<T> + Send> TickSubscription<T> {
    fn new(messages: AsyncInternalSubscription, request_id: i32, message_bus: Arc<dyn AsyncMessageBus>) -> Self {
        Self {
            done: false,
            stream_ended: false,
            messages,
            buffer: VecDeque::new(),
            request_id,
            message_bus,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Cancel the historical-ticks request. Safe to call after completion (no-op).
    /// Also fired automatically on `Drop` for unfinished subscriptions; explicit calls are idempotent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect("127.0.0.1:4002", 100).await?;
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().await?;
    /// subscription.cancel().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        match encoders::encode_cancel_historical_ticks(self.request_id) {
            Ok(message) => {
                if let Err(e) = self.message_bus.cancel_subscription(self.request_id, message).await {
                    warn!("error cancelling historical ticks subscription: {e}");
                }
            }
            Err(e) => error!("error encoding cancel historical ticks: {e}"),
        }
    }
}

impl<T: TickDecoder<T> + Send + Unpin> Stream for TickSubscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // `T: Unpin` (all tick types are plain data) makes `TickSubscription<T>`
        // Unpin — the only `T`-bearing field is `VecDeque<T>` — so we can project
        // to `&mut Self`. Every other field (BroadcastStream, bool, Arc) is Unpin.
        let this = self.get_mut();

        loop {
            if let Some(tick) = this.buffer.pop_front() {
                return Poll::Ready(Some(Ok(SubscriptionItem::Data(tick))));
            }

            // `done` (decoder's last-batch flag) and `stream_ended` (terminal
            // error / EndOfStream) are distinct reasons the stream is over;
            // either ends it once the buffer is drained.
            if this.done || this.stream_ended {
                return Poll::Ready(None);
            }

            let routed = match Pin::new(&mut this.messages.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(item))) => item,
                Poll::Ready(Some(Err(_lagged))) => continue, // skip BroadcastStream lag
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            match classify::<T>(routed) {
                TickAction::Batch(ticks, done) => {
                    this.buffer.extend(ticks);
                    this.done = done;
                }
                TickAction::Skip => {}
                TickAction::Notice(notice) => return Poll::Ready(Some(Ok(SubscriptionItem::Notice(notice)))),
                TickAction::EndOfStream => {
                    this.stream_ended = true;
                    return Poll::Ready(None);
                }
                TickAction::Error(e) => {
                    this.stream_ended = true;
                    return Poll::Ready(Some(Err(e)));
                }
            }
        }
    }
}

impl<T: TickDecoder<T> + Send> Drop for TickSubscription<T> {
    fn drop(&mut self) {
        if self.done || self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }
        let request_id = self.request_id;
        let message_bus = self.message_bus.clone();
        if let Ok(message) = encoders::encode_cancel_historical_ticks(request_id) {
            tokio::spawn(async move {
                if let Err(e) = message_bus.cancel_subscription(request_id, message).await {
                    warn!("error sending cancel historical ticks in drop: {e}");
                }
            });
        }
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
