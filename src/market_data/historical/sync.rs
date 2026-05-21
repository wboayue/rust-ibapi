use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{debug, error, warn};
use time::OffsetDateTime;

use crate::client::blocking::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::sync::Subscription;
use crate::transport::{InternalSubscription, MessageBus, Response};
use crate::{client::sync::Client, Error, MAX_RETRIES};

use super::common::{self, decoders, encoders};
use super::{BarSize, Duration, HistogramEntry, HistoricalBarUpdate, HistoricalData, Schedule, TickDecoder, WhatToShow};
use crate::market_data::TradingHours;

impl Client {
    /// Returns the timestamp of earliest available historical data for a contract and data type.
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
        check_version(self.server_version(), Features::HEAD_TIMESTAMP)?;

        let builder = self.request();
        let request = encoders::encode_request_head_timestamp(builder.request_id(), contract, what_to_show, trading_hours.use_rth())?;
        let subscription = builder.send_raw(request)?;

        match subscription.next() {
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HeadTimestamp => {
                Ok(decoders::decode_head_timestamp(&mut message, self.time_zone())?)
            }
            Some(Ok(message)) => Err(Error::unexpected_response(&message)),
            Some(Err(Error::ConnectionReset)) => self.head_timestamp(contract, what_to_show, trading_hours),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Requests interval of historical data ending at specified time for [Contract].
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [HistoricalData] for.
    /// * `end_date`     - optional end of the interval. If `None`, current time or last trading of contract is implied.
    /// * `duration`     - duration of interval to retrieve [HistoricalData] for.
    /// * `bar_size`     - [BarSize] to return.
    /// * `what_to_show` - requested bar type: [WhatToShow].
    /// * `trading_hours` - Use [TradingHours::Regular] for data generated only during regular trading hours, or [TradingHours::Extended] to include data from outside regular trading hours.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::historical::{BarSize, ToDuration, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let historical_data = client
    ///     .historical_data(&contract, Some(datetime!(2023-04-15 0:00 UTC)), 7.days(), BarSize::Day, WhatToShow::Trades, TradingHours::Regular)
    ///     .expect("historical data request failed");
    ///
    /// println!("start_date: {}, end_date: {}", historical_data.start, historical_data.end);
    ///
    /// for bar in &historical_data.bars {
    ///     println!("{bar:?}");
    /// }
    /// ```
    pub fn historical_data(
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

            let subscription = builder.send_raw(request)?;

            match subscription.next() {
                Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalData => {
                    let mut data = decoders::decode_historical_data(self.server_version, time_zone(self), &mut message)?;

                    if self.server_version >= crate::server_versions::HISTORICAL_DATA_END {
                        if let Some(Ok(mut end_msg)) = subscription.next() {
                            let (start, end) = decoders::decode_historical_data_end(self.server_version, time_zone(self), &mut end_msg)?;
                            data.start = start;
                            data.end = end;
                        }
                    }

                    return Ok(data);
                }
                Some(Ok(message)) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
                Some(Ok(message)) => return Err(Error::unexpected_response(&message)),
                Some(Err(Error::ConnectionReset)) => {}
                Some(Err(e)) => return Err(e),
                None => return Err(Error::UnexpectedEndOfStream),
            }
        }

        Err(Error::ConnectionReset)
    }

    /// Requests historical data with optional streaming updates.
    ///
    /// This method returns a subscription that first yields the initial historical bars.
    /// When `keep_up_to_date` is `true`, it continues to yield streaming updates for
    /// the current bar as it builds. IBKR sends updated bars every ~4-6 seconds until
    /// the bar completes.
    ///
    /// # Arguments
    /// * `contract` - Contract object that is subject of query
    /// * `duration` - The amount of time for which the data needs to be retrieved
    /// * `bar_size` - The bar size (resolution)
    /// * `what_to_show` - The type of data to retrieve (Trades, MidPoint, etc.)
    /// * `trading_hours` - Regular trading hours only, or include extended hours
    /// * `keep_up_to_date` - If true, continue receiving streaming updates after initial data
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::historical::{ToDuration, HistoricalBarUpdate};
    /// use ibapi::prelude::{HistoricalBarSize, HistoricalWhatToShow, TradingHours};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("SPY").build();
    ///
    /// let subscription = client
    ///     .historical_data_streaming(
    ///         &contract, 3.days(), HistoricalBarSize::Min15,
    ///         HistoricalWhatToShow::Trades, TradingHours::Extended, true
    ///     )
    ///     .expect("streaming request failed");
    ///
    /// while let Some(update) = subscription.next_data() {
    ///     match update? {
    ///         HistoricalBarUpdate::Historical(data) => println!("Initial bars: {}", data.bars.len()),
    ///         HistoricalBarUpdate::Update(bar) => println!("Streaming update: {:?}", bar),
    ///         HistoricalBarUpdate::End { start, end } => println!("Stream ended: {} - {}", start, end),
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn historical_data_streaming(
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

        builder.send::<HistoricalBarUpdate>(request)
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
    /// use time::macros::datetime;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("GM").build();
    ///
    /// // Ending now:
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .fetch()
    ///     .expect("historical schedule request failed");
    ///
    /// // Anchored to a specific end date:
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .ending(datetime!(2023-04-15 0:00 UTC))
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::IgnoreSize;
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
    ///     .trade()
    ///     .expect("historical ticks request failed");
    ///
    /// // Bid/ask ticks anchored at an end date, ignoring tick sizes:
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

    /// Cancels an in-flight historical ticks request.
    ///
    /// # Arguments
    /// * `request_id` - The request ID of the historical ticks subscription to cancel.
    pub fn cancel_historical_ticks(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version(), Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_historical_ticks(request_id)?;
        self.send_message(message)?;
        Ok(())
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
    //
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
        check_version(self.server_version(), Features::HISTOGRAM)?;

        loop {
            let builder = self.request();
            let request = encoders::encode_request_histogram_data(builder.request_id(), contract, trading_hours.use_rth(), period)?;
            let subscription = builder.send_raw(request)?;

            match subscription.next() {
                Some(Ok(mut message)) => return decoders::decode_histogram_data(&mut message),
                Some(Err(Error::ConnectionReset)) => continue,
                Some(Err(e)) => return Err(e),
                None => return Ok(Vec::new()),
            }
        }
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
pub(crate) fn historical_ticks<T: TickDecoder<T>>(
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
    let subscription = builder.send_raw(request)?;

    Ok(TickSubscription::new(subscription, request_id, Arc::clone(&client.message_bus)))
}

pub(crate) fn historical_schedule(
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

        let subscription = builder.send_raw(request)?;

        match subscription.next() {
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalSchedule => {
                return decoders::decode_historical_schedule(&mut message)
            }
            Some(Ok(message)) => return Err(Error::unexpected_response(&message)),
            Some(Err(Error::ConnectionReset)) => {}
            Some(Err(e)) => return Err(e),
            None => return Err(Error::UnexpectedEndOfStream),
        }
    }
}

// TickSubscription and related types

/// Shared subscription handle that decodes historical tick batches as they arrive.
#[must_use = "TickSubscription must be polled (.next() or .iter()) to receive ticks; dropping it cancels the request"]
pub struct TickSubscription<T: TickDecoder<T>> {
    done: AtomicBool,
    messages: InternalSubscription,
    buffer: Mutex<VecDeque<T>>,
    error: Mutex<Option<Error>>,
    request_id: i32,
    message_bus: Arc<dyn MessageBus>,
    cancelled: AtomicBool,
}

impl<T: TickDecoder<T>> TickSubscription<T> {
    fn new(messages: InternalSubscription, request_id: i32, message_bus: Arc<dyn MessageBus>) -> Self {
        Self {
            done: false.into(),
            messages,
            buffer: Mutex::new(VecDeque::new()),
            error: Mutex::new(None),
            request_id,
            message_bus,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Cancel the historical-ticks request. Safe to call after completion (no-op).
    /// Also fired automatically on `Drop` for unfinished subscriptions; explicit calls are idempotent.
    pub fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        match encoders::encode_cancel_historical_ticks(self.request_id) {
            Ok(message) => {
                if let Err(e) = self.message_bus.cancel_subscription(self.request_id, &message) {
                    warn!("error cancelling historical ticks subscription: {e}");
                }
                self.messages.cancel();
            }
            Err(e) => error!("error encoding cancel historical ticks: {e}"),
        }
    }

    /// Return an iterator that blocks until each tick batch becomes available.
    pub fn iter(&self) -> TickSubscriptionIter<'_, T> {
        TickSubscriptionIter { subscription: self }
    }

    /// Return a non-blocking iterator that yields immediately with cached ticks.
    pub fn try_iter(&self) -> TickSubscriptionTryIter<'_, T> {
        TickSubscriptionTryIter { subscription: self }
    }

    /// Return an iterator that waits up to `duration` for each tick batch.
    pub fn timeout_iter(&self, duration: std::time::Duration) -> TickSubscriptionTimeoutIter<'_, T> {
        TickSubscriptionTimeoutIter {
            subscription: self,
            timeout: duration,
        }
    }

    /// Block until the next tick batch is available.
    pub fn next(&self) -> Option<T> {
        self.next_helper(|| self.messages.next())
    }

    /// Attempt to fetch the next tick batch without blocking.
    pub fn try_next(&self) -> Option<T> {
        self.next_helper(|| self.messages.try_next())
    }

    /// Wait up to `duration` for the next tick batch to arrive.
    pub fn next_timeout(&self, duration: std::time::Duration) -> Option<T> {
        self.next_helper(|| self.messages.next_timeout(duration))
    }

    fn next_helper<F>(&self, next_response: F) -> Option<T>
    where
        F: Fn() -> Option<Response>,
    {
        self.clear_error();

        loop {
            if let Some(message) = self.next_buffered() {
                return Some(message);
            }

            if self.done.load(Ordering::Relaxed) {
                return None;
            }

            match self.fill_buffer(next_response()) {
                Ok(()) => {}
                Err(()) => return None,
            }
        }
    }

    fn fill_buffer(&self, response: Option<Response>) -> Result<(), ()> {
        match response {
            Some(Ok(mut message)) if message.message_type() == T::MESSAGE_TYPE => {
                let mut buffer = self.buffer.lock().unwrap();

                let (ticks, done) = T::decode(&mut message).unwrap();

                buffer.append(&mut ticks.into());
                self.done.store(done, Ordering::Relaxed);

                Ok(())
            }
            Some(Ok(message)) => {
                debug!("unexpected message: {message:?}");
                Ok(())
            }
            Some(Err(e)) => {
                self.set_error(e);
                Err(())
            }
            None => Err(()),
        }
    }

    fn next_buffered(&self) -> Option<T> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.pop_front()
    }

    fn set_error(&self, e: Error) {
        let mut error = self.error.lock().unwrap();
        *error = Some(e);
    }

    fn clear_error(&self) {
        let mut error = self.error.lock().unwrap();
        *error = None;
    }
}

impl<T: TickDecoder<T>> Drop for TickSubscription<T> {
    fn drop(&mut self) {
        if !self.done.load(Ordering::Relaxed) {
            self.cancel();
        }
    }
}

/// An iterator that yields items as they become available, blocking if necessary.
pub struct TickSubscriptionIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: TickDecoder<T>> IntoIterator for &'a TickSubscription<T> {
    type Item = T;
    type IntoIter = TickSubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator that yields items as they become available, blocking if necessary.
pub struct TickSubscriptionOwnedIter<T: TickDecoder<T>> {
    subscription: TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionOwnedIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<T: TickDecoder<T>> IntoIterator for TickSubscription<T> {
    type Item = T;
    type IntoIter = TickSubscriptionOwnedIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        TickSubscriptionOwnedIter { subscription: self }
    }
}

/// An iterator that yields items if they are available, without waiting.
pub struct TickSubscriptionTryIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionTryIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// An iterator that waits for the specified timeout duration for available data.
pub struct TickSubscriptionTimeoutIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
    timeout: std::time::Duration,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionTimeoutIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
