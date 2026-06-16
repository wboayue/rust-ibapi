use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{error, warn};
use time::OffsetDateTime;

use crate::client::blocking::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::common::{RoutedItem, SubscriptionItem};
use crate::subscriptions::sync::{FilterData, Subscription, SubscriptionItemIterExt};
use crate::transport::{InternalSubscription, MessageBus};
use crate::{client::sync::Client, Error, MAX_RETRIES};

use super::common::tick::{classify, TickAction};
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
            Some(Ok(message)) if message.message_type() == IncomingMessages::HeadTimestamp => Ok(decoders::decode_head_timestamp(&message)?),
            Some(Ok(message)) => Err(Error::unexpected_response(&message)),
            Some(Err(Error::ConnectionReset)) => self.head_timestamp(contract, what_to_show, trading_hours),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
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
    /// // Convenience: explicit date range (computes duration internally)
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical data request failed");
    /// # let _ = bars;
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
                Some(Ok(message)) => return decoders::decode_histogram_data(&message),
                Some(Err(Error::ConnectionReset)) => continue,
                Some(Err(e)) => return Err(e),
                None => return Ok(Vec::new()),
            }
        }
    }
}

pub(crate) fn historical_data(
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

        let subscription = builder.send_raw(request)?;

        match subscription.next() {
            Some(Ok(message)) if message.message_type() == IncomingMessages::HistoricalData => {
                let mut data = decoders::decode_historical_data(&message)?;

                if let Some(Ok(end_msg)) = subscription.next() {
                    let (start, end) = decoders::decode_historical_data_end(&end_msg)?;
                    data.start = start;
                    data.end = end;
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

pub(crate) fn historical_data_stream(
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

    builder.send::<HistoricalBarUpdate>(request)
}

// pub(crate) internal plumbing called from `HistoricalTicksBuilder`; the
// public API is already a builder, so flat args here are the deliberate
// seam between the typed builder and the wire encoder (rule 19 canary
// acceptable for builder-fed helpers).
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
            Some(Ok(message)) if message.message_type() == IncomingMessages::HistoricalSchedule => {
                return decoders::decode_historical_schedule(&message)
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
///
/// Each [`next`](Self::next), [`try_next`](Self::try_next), or
/// [`next_timeout`](Self::next_timeout) returns
/// `Option<Result<SubscriptionItem<T>, Error>>`, the same shape as
/// [`Subscription`](crate::subscriptions::Subscription):
///
/// * `None` — the stream has ended.
/// * `Some(Ok(SubscriptionItem::Data(tick)))` — a decoded tick.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice bound to
///   this request; the stream stays open.
/// * `Some(Err(e))` — terminal error (decode or transport); subsequent calls
///   return `None`.
///
/// When you only care about ticks, use [`iter_data`](Self::iter_data) (or
/// [`next_data`](Self::next_data)), which filter notices and yield
/// `Result<T, Error>`.
#[must_use = "TickSubscription must be polled (.next() or .iter_data()) to receive ticks; dropping it cancels the request"]
pub struct TickSubscription<T: TickDecoder<T>> {
    done: AtomicBool,
    stream_ended: AtomicBool,
    messages: InternalSubscription,
    buffer: Mutex<VecDeque<T>>,
    request_id: i32,
    message_bus: Arc<dyn MessageBus>,
    cancelled: AtomicBool,
}

impl<T: TickDecoder<T>> TickSubscription<T> {
    fn new(messages: InternalSubscription, request_id: i32, message_bus: Arc<dyn MessageBus>) -> Self {
        Self {
            done: false.into(),
            stream_ended: AtomicBool::new(false),
            messages,
            buffer: Mutex::new(VecDeque::new()),
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// subscription.cancel();
    /// ```
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

    /// Blocking iterator yielding `Result<SubscriptionItem<T>, Error>` — both
    /// `Data` and `Notice` arms surface to the caller. Use
    /// [`iter_data`](Self::iter_data) when you only want ticks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// for item in subscription.iter() {
    ///     match item {
    ///         Ok(SubscriptionItem::Data(tick))   => println!("tick: {tick:?}"),
    ///         Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
    ///         Err(e)                             => { eprintln!("error: {e}"); break; }
    ///     }
    /// }
    /// ```
    pub fn iter(&self) -> TickSubscriptionIter<'_, T> {
        TickSubscriptionIter { subscription: self }
    }

    /// Non-blocking iterator. Same `SubscriptionItem<T>` shape as [`iter`](Self::iter);
    /// see [`try_iter_data`](Self::try_iter_data) for the data-only variant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// for tick in subscription.try_iter_data() {
    ///     println!("tick: {:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn try_iter(&self) -> TickSubscriptionTryIter<'_, T> {
        TickSubscriptionTryIter { subscription: self }
    }

    /// Iterator that waits up to `duration` for each item. Same `SubscriptionItem<T>`
    /// shape as [`iter`](Self::iter); see [`timeout_iter_data`](Self::timeout_iter_data)
    /// for the data-only variant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// for tick in subscription.timeout_iter_data(Duration::from_secs(5)) {
    ///     println!("tick: {:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn timeout_iter(&self, duration: std::time::Duration) -> TickSubscriptionTimeoutIter<'_, T> {
        TickSubscriptionTimeoutIter {
            subscription: self,
            timeout: duration,
        }
    }

    /// Blocking data iterator that filters notices and yields `Result<T, Error>`.
    /// Notices are logged at `warn!` level.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// for tick in subscription.iter_data() {
    ///     println!("tick: {:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn iter_data(&self) -> FilterData<TickSubscriptionIter<'_, T>> {
        self.iter().filter_data()
    }

    /// Non-blocking data iterator (notices filtered).
    pub fn try_iter_data(&self) -> FilterData<TickSubscriptionTryIter<'_, T>> {
        self.try_iter().filter_data()
    }

    /// Timeout-bounded data iterator (notices filtered).
    pub fn timeout_iter_data(&self, duration: std::time::Duration) -> FilterData<TickSubscriptionTimeoutIter<'_, T>> {
        self.timeout_iter(duration).filter_data()
    }

    /// Block until the next item is available.
    ///
    /// Returns the `SubscriptionItem<T>` envelope; use [`next_data`](Self::next_data)
    /// when you only want ticks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// while let Some(item) = subscription.next() {
    ///     match item {
    ///         Ok(SubscriptionItem::Data(tick))   => println!("tick: {tick:?}"),
    ///         Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
    ///         Err(e)                             => { eprintln!("error: {e}"); break; }
    ///     }
    /// }
    /// ```
    pub fn next(&self) -> Option<Result<SubscriptionItem<T>, Error>> {
        self.next_helper(|| self.messages.next_routed())
    }

    /// Attempt to fetch the next item without blocking.
    ///
    /// Same `SubscriptionItem<T>` shape as [`next`](Self::next); returns `None`
    /// if nothing is queued right now.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// if let Some(Ok(item)) = subscription.try_next() {
    ///     println!("item: {item:?}");
    /// }
    /// ```
    pub fn try_next(&self) -> Option<Result<SubscriptionItem<T>, Error>> {
        self.next_helper(|| self.messages.try_next_routed())
    }

    /// Wait up to `duration` for the next item to arrive.
    ///
    /// Same `SubscriptionItem<T>` shape as [`next`](Self::next).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// if let Some(Ok(item)) = subscription.next_timeout(Duration::from_secs(5)) {
    ///     println!("item: {item:?}");
    /// }
    /// ```
    pub fn next_timeout(&self, duration: std::time::Duration) -> Option<Result<SubscriptionItem<T>, Error>> {
        self.next_helper(|| self.messages.next_timeout_routed(duration))
    }

    /// Convenience: blocking `next` that filters out notices and yields just a tick.
    /// Equivalent to `iter_data().next()`. Filtered notices are logged at `warn!`.
    /// Use [`next`](Self::next) instead if you want to observe `Notice` items.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("MSFT").build();
    /// let subscription = client.historical_ticks(&contract, 100).trade().expect("request failed");
    /// while let Some(tick) = subscription.next_data() {
    ///     println!("tick: {:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn next_data(&self) -> Option<Result<T, Error>> {
        self.iter_data().next()
    }

    fn next_helper<F>(&self, next_routed: F) -> Option<Result<SubscriptionItem<T>, Error>>
    where
        F: Fn() -> Option<RoutedItem>,
    {
        loop {
            if let Some(tick) = self.next_buffered() {
                return Some(Ok(SubscriptionItem::Data(tick)));
            }

            // `done` (decoder said this was the last batch) and `stream_ended`
            // (terminal error/EndOfStream) are distinct: the former is graceful
            // completion, the latter is termination. Either ends the stream once
            // the buffer is drained.
            if self.done.load(Ordering::Relaxed) || self.stream_ended.load(Ordering::Relaxed) {
                return None;
            }

            let item = next_routed()?;

            match classify::<T>(item) {
                TickAction::Batch(ticks, done) => {
                    self.buffer.lock().unwrap().extend(ticks);
                    self.done.store(done, Ordering::Relaxed);
                }
                TickAction::Skip => {}
                TickAction::Notice(notice) => return Some(Ok(SubscriptionItem::Notice(notice))),
                TickAction::EndOfStream => {
                    self.stream_ended.store(true, Ordering::Relaxed);
                    return None;
                }
                TickAction::Error(e) => {
                    self.stream_ended.store(true, Ordering::Relaxed);
                    return Some(Err(e));
                }
            }
        }
    }

    fn next_buffered(&self) -> Option<T> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.pop_front()
    }
}

impl<T: TickDecoder<T>> Drop for TickSubscription<T> {
    fn drop(&mut self) {
        if !self.done.load(Ordering::Relaxed) {
            self.cancel();
        }
    }
}

/// A blocking iterator over `Result<SubscriptionItem<T>, Error>`.
pub struct TickSubscriptionIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: TickDecoder<T>> IntoIterator for &'a TickSubscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;
    type IntoIter = TickSubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An owned blocking iterator over `Result<SubscriptionItem<T>, Error>`.
pub struct TickSubscriptionOwnedIter<T: TickDecoder<T>> {
    subscription: TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionOwnedIter<T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<T: TickDecoder<T>> IntoIterator for TickSubscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;
    type IntoIter = TickSubscriptionOwnedIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        TickSubscriptionOwnedIter { subscription: self }
    }
}

/// A non-blocking iterator over `Result<SubscriptionItem<T>, Error>`.
pub struct TickSubscriptionTryIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionTryIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// An iterator that waits for the specified timeout duration for each item.
pub struct TickSubscriptionTimeoutIter<'a, T: TickDecoder<T>> {
    subscription: &'a TickSubscription<T>,
    timeout: std::time::Duration,
}

impl<T: TickDecoder<T>> Iterator for TickSubscriptionTimeoutIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
