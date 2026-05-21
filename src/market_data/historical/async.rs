use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, error, warn};
use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::r#async::Subscription;
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};
use crate::{Client, Error, MAX_RETRIES};

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
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HeadTimestamp => {
                Ok(decoders::decode_head_timestamp(&mut message, self.time_zone())?)
            }
            Some(Ok(message)) => Err(Error::unexpected_response(&message)),
            Some(Err(e)) => Err(e),
            None => {
                // Connection might have been reset, retry
                Box::pin(self.head_timestamp(contract, what_to_show, trading_hours)).await
            }
        }
    }

    /// Requests interval of historical data ending at specified time for [Contract].
    ///
    /// # Arguments
    /// * `contract`      - [Contract] to retrieve [HistoricalData] for.
    /// * `end_date`      - optional end of the interval. If `None`, current time or last trading of contract is implied.
    /// * `duration`      - duration of interval to retrieve [HistoricalData] for.
    /// * `bar_size`      - [BarSize] to return.
    /// * `what_to_show`  - requested bar type: [WhatToShow].
    /// * `trading_hours` - Use [TradingHours::Regular] for data generated only during regular trading hours, or [TradingHours::Extended] to include data from outside regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("TSLA").build();
    ///     let historical_data = client
    ///         .historical_data(
    ///             &contract,
    ///             Some(datetime!(2023-04-15 0:00 UTC)),
    ///             7.days(),
    ///             BarSize::Day,
    ///             WhatToShow::Trades,
    ///             TradingHours::Regular,
    ///         )
    ///         .await
    ///         .expect("historical data request failed");
    ///
    ///     println!("start_date: {}, end_date: {}", historical_data.start, historical_data.end);
    ///
    ///     for bar in &historical_data.bars {
    ///         println!("{bar:?}");
    ///     }
    /// }
    /// ```
    pub async fn historical_data(
        &self,
        contract: &Contract,
        end_date: Option<OffsetDateTime>,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: WhatToShow,
        trading_hours: TradingHours,
    ) -> Result<HistoricalData, Error> {
        common::validate_historical_data(self.server_version(), contract, end_date, Some(what_to_show))?;

        for _ in 0..MAX_RETRIES {
            let builder = self.request();
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
                Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalData => {
                    let mut data = decoders::decode_historical_data(self.server_version(), time_zone(self), &mut message)?;

                    if self.server_version() >= crate::server_versions::HISTORICAL_DATA_END {
                        if let Some(Ok(mut end_msg)) = subscription.next().await {
                            let (start, end) = decoders::decode_historical_data_end(self.server_version(), time_zone(self), &mut end_msg)?;
                            data.start = start;
                            data.end = end;
                        }
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
    /// use ibapi::market_data::historical::IgnoreSize;
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
                Some(Ok(mut message)) => return decoders::decode_histogram_data(&mut message),
                Some(Err(e)) => return Err(e),
                None => continue, // Connection reset, retry
            }
        }
    }

    /// Requests historical data with optional streaming updates.
    ///
    /// This method returns a subscription that first yields the initial historical bars.
    /// When `keep_up_to_date` is `true`, it continues to yield streaming updates for
    /// the current bar as it builds. IBKR sends updated bars every ~4-6 seconds until
    /// the bar completes.
    ///
    /// # Arguments
    /// * `contract`         - Contract object that is subject of query
    /// * `duration`         - The amount of time for which the data needs to be retrieved
    /// * `bar_size`         - The bar size (resolution)
    /// * `what_to_show`     - The type of data to retrieve (Trades, MidPoint, etc.)
    /// * `trading_hours`    - Regular trading hours only, or include extended hours
    /// * `keep_up_to_date`  - If true, continue receiving streaming updates after initial data
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{HistoricalBarUpdate, ToDuration};
    /// use ibapi::prelude::{HistoricalBarSize, HistoricalWhatToShow, TradingHours};
    /// use ibapi::subscriptions::SubscriptionItemStreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("SPY").build();
    ///
    ///     let subscription = client
    ///         .historical_data_streaming(
    ///             &contract, 3.days(), HistoricalBarSize::Min15,
    ///             HistoricalWhatToShow::Trades, TradingHours::Extended, true,
    ///         )
    ///         .await
    ///         .expect("streaming request failed");
    ///
    ///     let mut data = subscription.filter_data();
    ///     while let Some(update) = data.next().await {
    ///         match update? {
    ///             HistoricalBarUpdate::Historical(data) => println!("Initial bars: {}", data.bars.len()),
    ///             HistoricalBarUpdate::Update(bar) => println!("Streaming update: {bar:?}"),
    ///             HistoricalBarUpdate::End { start, end } => println!("Stream ended: {start} - {end}"),
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn historical_data_streaming(
        &self,
        contract: &Contract,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: WhatToShow,
        trading_hours: TradingHours,
        keep_up_to_date: bool,
    ) -> Result<Subscription<HistoricalBarUpdate>, Error> {
        if !contract.trading_class.is_empty() || contract.contract_id > 0 {
            check_version(self.server_version(), Features::TRADING_CLASS)?;
        }

        let builder = self.request();
        let request = encoders::encode_request_historical_data(
            builder.request_id(),
            contract,
            None, // end_date must be None when keepUpToDate=true (IBKR requirement)
            duration,
            bar_size,
            Some(what_to_show),
            trading_hours.use_rth(),
            keep_up_to_date,
            &Vec::<crate::contracts::TagValue>::default(),
        )?;

        builder.send::<HistoricalBarUpdate>(request).await
    }
}

pub(crate) fn time_zone(client: &Client) -> &time_tz::Tz {
    if let Some(tz) = client.time_zone {
        tz
    } else {
        warn!("server timezone unknown. assuming UTC, but that may be incorrect!");
        time_tz::timezones::db::UTC
    }
}

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
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalSchedule => {
                return decoders::decode_historical_schedule(&mut message)
            }
            Some(Ok(message)) => return Err(Error::unexpected_response(&message)),
            Some(Err(e)) => return Err(e),
            None => continue, // Connection reset, retry
        }
    }
}

// === TickSubscription and related types ===

/// Async subscription handle that decodes historical tick batches as they arrive.
#[must_use = "TickSubscription must be polled (.next().await) to receive ticks; dropping it cancels the request"]
pub struct TickSubscription<T: TickDecoder<T> + Send> {
    done: bool,
    messages: AsyncInternalSubscription,
    buffer: VecDeque<T>,
    error: Option<Error>,
    request_id: i32,
    message_bus: Arc<dyn AsyncMessageBus>,
    cancelled: AtomicBool,
}

impl<T: TickDecoder<T> + Send> TickSubscription<T> {
    fn new(messages: AsyncInternalSubscription, request_id: i32, message_bus: Arc<dyn AsyncMessageBus>) -> Self {
        Self {
            done: false,
            messages,
            buffer: VecDeque::new(),
            error: None,
            request_id,
            message_bus,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Cancel the historical-ticks request. Safe to call after completion (no-op).
    /// Also fired automatically on `Drop` for unfinished subscriptions; explicit calls are idempotent.
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

    /// Block until the next tick batch is available.
    pub async fn next(&mut self) -> Option<T> {
        self.clear_error();

        loop {
            if let Some(tick) = self.next_buffered() {
                return Some(tick);
            }

            if self.done {
                return None;
            }

            match self.fill_buffer().await {
                Ok(()) => continue,
                Err(()) => return None,
            }
        }
    }

    async fn fill_buffer(&mut self) -> Result<(), ()> {
        match self.messages.next().await {
            Some(Ok(mut message)) if message.message_type() == T::MESSAGE_TYPE => {
                let (ticks, done) = T::decode(&mut message).unwrap();
                self.buffer.extend(ticks);
                self.done = done;
                Ok(())
            }
            Some(Ok(message)) => {
                debug!("unexpected message: {message:?}");
                Ok(())
            }
            Some(Err(_)) => Err(()),
            None => Err(()),
        }
    }

    fn next_buffered(&mut self) -> Option<T> {
        self.buffer.pop_front()
    }

    #[allow(dead_code)]
    fn set_error(&mut self, e: Error) {
        self.error = Some(e);
    }

    fn clear_error(&mut self) {
        self.error = None;
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
