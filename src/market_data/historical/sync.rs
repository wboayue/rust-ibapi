use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{debug, warn};
use time::OffsetDateTime;

use crate::client::blocking::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::transport::{InternalSubscription, MessageBus, Response};
use crate::{client::sync::Client, Error, MAX_RETRIES};

use time_tz::Tz;

use super::common::{self, decoders, encoders};
use super::{
    BarSize, Duration, HistogramEntry, HistoricalBarUpdate, HistoricalData, Schedule, TickBidAsk, TickDecoder, TickLast, TickMidpoint, WhatToShow,
};
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
            Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
            Some(Err(Error::ConnectionReset)) => self.head_timestamp(contract, what_to_show, trading_hours),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Requests interval of historical data ending at specified time for [Contract].
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [HistoricalData] for.
    /// * `interval_end` - optional end date of interval to retrieve [HistoricalData] for. If `None` current time or last trading of contract is implied.
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
        interval_end: Option<OffsetDateTime>,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: WhatToShow,
        trading_hours: TradingHours,
    ) -> Result<HistoricalData, Error> {
        common::validate_historical_data(self.server_version(), contract, interval_end, Some(what_to_show))?;

        for _ in 0..MAX_RETRIES {
            let builder = self.request();
            let request = encoders::encode_request_historical_data(
                builder.request_id(),
                contract,
                interval_end,
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
                Some(Ok(message)) => return Err(Error::UnexpectedResponse(message)),
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
    ///         Some(HistoricalWhatToShow::Trades), TradingHours::Extended, true
    ///     )
    ///     .expect("streaming request failed");
    ///
    /// while let Some(update) = subscription.next() {
    ///     match update {
    ///         HistoricalBarUpdate::Historical(data) => println!("Initial bars: {}", data.bars.len()),
    ///         HistoricalBarUpdate::Update(bar) => println!("Streaming update: {:?}", bar),
    ///         HistoricalBarUpdate::End { start, end } => println!("Stream ended: {} - {}", start, end),
    ///     }
    /// }
    /// ```
    pub fn historical_data_streaming(
        &self,
        contract: &Contract,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: Option<WhatToShow>,
        trading_hours: TradingHours,
        keep_up_to_date: bool,
    ) -> Result<HistoricalDataStreamingSubscription, Error> {
        if !contract.trading_class.is_empty() || contract.contract_id > 0 {
            check_version(self.server_version(), Features::TRADING_CLASS)?;
        }

        // Note: end_date must be None when keepUpToDate=true (IBKR requirement)
        let builder = self.request();
        let request = encoders::encode_request_historical_data(
            builder.request_id(),
            contract,
            None, // end_date must be None for keepUpToDate
            duration,
            bar_size,
            what_to_show,
            trading_hours.use_rth(),
            keep_up_to_date,
            &Vec::<crate::contracts::TagValue>::default(),
        )?;

        let request_id = builder.request_id();
        let subscription = builder.send_raw(request)?;

        // Get the timezone directly
        let tz: &'static Tz = self.time_zone.unwrap_or_else(|| {
            warn!("server timezone unknown. assuming UTC, but that may be incorrect!");
            time_tz::timezones::db::UTC
        });

        Ok(HistoricalDataStreamingSubscription::new(
            subscription,
            self.server_version,
            tz,
            request_id,
            self.message_bus.clone(),
        ))
    }

    /// Requests [Schedule] for an interval of given duration
    /// ending at specified date.
    ///
    /// # Arguments
    /// * `contract`     - [Contract] to retrieve [Schedule] for.
    /// * `interval_end` - end date of interval to retrieve [Schedule] for.
    /// * `duration`     - duration of interval to retrieve [Schedule] for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM").build();
    ///
    /// let historical_data = client
    ///     .historical_schedules(&contract, datetime!(2023-04-15 0:00 UTC), 30.days())
    ///     .expect("historical schedule request failed");
    ///
    /// println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);
    ///
    /// for session in &historical_data.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn historical_schedules(&self, contract: &Contract, interval_end: OffsetDateTime, duration: Duration) -> Result<Schedule, Error> {
        historical_schedule(self, contract, Some(interval_end), duration)
    }

    /// Requests [Schedule] for interval ending at current time.
    ///
    /// # Arguments
    /// * `contract` - [Contract] to retrieve [Schedule] for.
    /// * `duration` - [Duration] for interval to retrieve [Schedule] for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::historical::ToDuration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("GM").build();
    ///
    /// let historical_data = client
    ///     .historical_schedules_ending_now(&contract, 30.days())
    ///     .expect("historical schedule request failed");
    ///
    /// println!("start: {:?}, end: {:?}", historical_data.start, historical_data.end);
    ///
    /// for session in &historical_data.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn historical_schedules_ending_now(&self, contract: &Contract, duration: Duration) -> Result<Schedule, Error> {
        historical_schedule(self, contract, None, duration)
    }

    /// Requests historical time & sales data (Bid/Ask) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `trading_hours`   - Regular trading hours only, or include extended hours
    /// * `ignore_size`     - A filter only used when the source price is Bid_Ask
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks_bid_ask(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, TradingHours::Regular, false)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_bid_ask(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        trading_hours: TradingHours,
        ignore_size: bool,
    ) -> Result<TickSubscription<TickBidAsk>, Error> {
        check_version(self.server_version(), Features::HISTORICAL_TICKS)?;

        let builder = self.request();
        let request = encoders::encode_request_historical_ticks(
            builder.request_id(),
            contract,
            start,
            end,
            number_of_ticks,
            WhatToShow::BidAsk,
            trading_hours.use_rth(),
            ignore_size,
        )?;
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription))
    }

    /// Requests historical time & sales data (Midpoint) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `trading_hours`   - Regular trading hours only, or include extended hours
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks_mid_point(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, TradingHours::Regular)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_mid_point(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        trading_hours: TradingHours,
    ) -> Result<TickSubscription<TickMidpoint>, Error> {
        check_version(self.server_version(), Features::HISTORICAL_TICKS)?;

        let builder = self.request();
        let request = encoders::encode_request_historical_ticks(
            builder.request_id(),
            contract,
            start,
            end,
            number_of_ticks,
            WhatToShow::MidPoint,
            trading_hours.use_rth(),
            false,
        )?;
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription))
    }

    /// Requests historical time & sales data (Trades) for an instrument.
    ///
    /// # Arguments
    /// * `contract` - [Contract] object that is subject of query
    /// * `start`    - Start time. Either start time or end time is specified.
    /// * `end`      - End time. Either start time or end time is specified.
    /// * `number_of_ticks` - Number of distinct data points. Max currently 1000 per request.
    /// * `trading_hours`   - Regular trading hours only, or include extended hours
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use time::macros::datetime;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks_trade(&contract, Some(datetime!(2023-04-15 0:00 UTC)), None, 100, TradingHours::Regular)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn historical_ticks_trade(
        &self,
        contract: &Contract,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
        number_of_ticks: i32,
        trading_hours: TradingHours,
    ) -> Result<TickSubscription<TickLast>, Error> {
        check_version(self.server_version(), Features::HISTORICAL_TICKS)?;

        let builder = self.request();
        let request = encoders::encode_request_historical_ticks(
            builder.request_id(),
            contract,
            start,
            end,
            number_of_ticks,
            WhatToShow::Trades,
            trading_hours.use_rth(),
            false,
        )?;
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription))
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

fn historical_schedule(client: &Client, contract: &Contract, end_date: Option<OffsetDateTime>, duration: Duration) -> Result<Schedule, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    check_version(client.server_version(), Features::HISTORICAL_SCHEDULE)?;

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
            Some(Ok(message)) => return Err(Error::UnexpectedResponse(message)),
            Some(Err(Error::ConnectionReset)) => {}
            Some(Err(e)) => return Err(e),
            None => return Err(Error::UnexpectedEndOfStream),
        }
    }
}

// === Historical Data Streaming with keepUpToDate ===

/// Blocking subscription for streaming historical data with keepUpToDate=true.
///
/// This subscription first yields the initial historical bars as a `Historical` variant,
/// then continues to yield streaming updates for the current bar as `Update` variants.
pub struct HistoricalDataStreamingSubscription {
    messages: InternalSubscription,
    server_version: i32,
    time_zone: &'static Tz,
    error: Mutex<Option<Error>>,
    request_id: i32,
    message_bus: Arc<dyn MessageBus>,
    cancelled: AtomicBool,
}

impl HistoricalDataStreamingSubscription {
    fn new(messages: InternalSubscription, server_version: i32, time_zone: &'static Tz, request_id: i32, message_bus: Arc<dyn MessageBus>) -> Self {
        Self {
            messages,
            server_version,
            time_zone,
            error: Mutex::new(None),
            request_id,
            message_bus,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Block until the next update is available.
    ///
    /// Returns:
    /// - `Some(HistoricalBarUpdate::Historical(data))` - Initial batch of historical bars (always first)
    /// - `Some(HistoricalBarUpdate::Update(bar))` - Streaming bar update
    /// - `None` - Subscription ended (connection closed or error)
    pub fn next(&self) -> Option<HistoricalBarUpdate> {
        self.next_helper(|| self.messages.next())
    }

    /// Attempt to fetch the next update without blocking.
    pub fn try_next(&self) -> Option<HistoricalBarUpdate> {
        self.next_helper(|| self.messages.try_next())
    }

    /// Wait up to `duration` for the next update to arrive.
    pub fn next_timeout(&self, duration: std::time::Duration) -> Option<HistoricalBarUpdate> {
        self.next_helper(|| self.messages.next_timeout(duration))
    }

    fn next_helper<F>(&self, next_response: F) -> Option<HistoricalBarUpdate>
    where
        F: Fn() -> Option<Response>,
    {
        self.clear_error();

        loop {
            match next_response() {
                Some(Ok(mut message)) => {
                    match message.message_type() {
                        IncomingMessages::HistoricalData => {
                            // Initial historical data batch
                            match decoders::decode_historical_data(self.server_version, self.time_zone, &mut message) {
                                Ok(data) => {
                                    return Some(HistoricalBarUpdate::Historical(data));
                                }
                                Err(e) => {
                                    self.set_error(e);
                                    return None;
                                }
                            }
                        }
                        IncomingMessages::HistoricalDataUpdate => {
                            // Streaming bar update
                            match decoders::decode_historical_data_update(self.time_zone, &mut message) {
                                Ok(bar) => {
                                    return Some(HistoricalBarUpdate::Update(bar));
                                }
                                Err(e) => {
                                    self.set_error(e);
                                    return None;
                                }
                            }
                        }
                        IncomingMessages::HistoricalDataEnd => {
                            match decoders::decode_historical_data_end(self.server_version, self.time_zone, &mut message) {
                                Ok((start, end)) => return Some(HistoricalBarUpdate::End { start, end }),
                                Err(e) => {
                                    self.set_error(e);
                                    return None;
                                }
                            }
                        }
                        IncomingMessages::Error => {
                            self.set_error(Error::from(message));
                            return None;
                        }
                        _ => {
                            // Skip unexpected messages
                            debug!("unexpected message in streaming subscription: {:?}", message.message_type());
                            continue;
                        }
                    }
                }
                Some(Err(e)) => {
                    self.set_error(e);
                    return None;
                }
                None => {
                    return None;
                }
            }
        }
    }

    /// Returns and clears the last error that occurred, if any.
    pub fn error(&self) -> Option<Error> {
        self.error.lock().unwrap().take()
    }

    fn set_error(&self, e: Error) {
        *self.error.lock().unwrap() = Some(e);
    }

    fn clear_error(&self) {
        *self.error.lock().unwrap() = None;
    }

    /// Cancel the subscription, sending CancelHistoricalData to the server.
    pub fn cancel(&self) {
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Ok(message) = encoders::encode_cancel_historical_data(self.request_id) {
            if let Err(e) = self.message_bus.cancel_subscription(self.request_id, &message) {
                warn!("error sending cancel historical data: {e}");
            }
        }
        self.messages.cancel();
    }
}

impl Drop for HistoricalDataStreamingSubscription {
    fn drop(&mut self) {
        self.cancel();
    }
}

// TickSubscription and related types

/// Shared subscription handle that decodes historical tick batches as they arrive.
pub struct TickSubscription<T: TickDecoder<T>> {
    done: AtomicBool,
    messages: InternalSubscription,
    buffer: Mutex<VecDeque<T>>,
    error: Mutex<Option<Error>>,
}

impl<T: TickDecoder<T>> TickSubscription<T> {
    fn new(messages: InternalSubscription) -> Self {
        Self {
            done: false.into(),
            messages,
            buffer: Mutex::new(VecDeque::new()),
            error: Mutex::new(None),
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
mod tests {
    use super::*;
    use crate::client::blocking::Client;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::Contract;
    use crate::market_data::historical::ToDuration;
    use crate::market_data::TradingHours;
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};
    use time::macros::{date, datetime};
    use time::OffsetDateTime;
    use time_tz::{self, PrimitiveDateTimeExt, Tz};

    #[test]
    fn test_head_timestamp() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["88|9000|1678323335|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract::stock("MSFT").build();
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let head_timestamp = client
            .head_timestamp(&contract, what_to_show, trading_hours)
            .expect("head timestamp request failed");

        assert_eq!(head_timestamp, OffsetDateTime::from_unix_timestamp(1678323335).unwrap(), "bar.date");

        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHeadTimestamp);
    }

    #[test]
    fn test_histogram_data() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["19|9000|3|125.50|1000|126.00|2000|126.50|3000|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract::stock("MSFT").build();
        let trading_hours = TradingHours::Regular;
        let period = BarSize::Day;

        let histogram_data = client
            .histogram_data(&contract, trading_hours, period)
            .expect("histogram data request failed");

        // Assert Response
        assert_eq!(histogram_data.len(), 3, "histogram_data.len()");

        assert_eq!(histogram_data[0].price, 125.50, "histogram_data[0].price");
        assert_eq!(histogram_data[0].size, 1000, "histogram_data[0].size");

        assert_eq!(histogram_data[1].price, 126.00, "histogram_data[1].price");
        assert_eq!(histogram_data[1].size, 2000, "histogram_data[1].size");

        assert_eq!(histogram_data[2].price, 126.50, "histogram_data[2].price");
        assert_eq!(histogram_data[2].size, 3000, "histogram_data[2].size");

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert!(!request_messages.is_empty(), "Should have sent a request message");
    }

    #[test]
    fn test_historical_data() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "17|9000|20230413  16:31:22|20230415  16:31:22|2|20230413|182.9400|186.5000|180.9400|185.9000|948837.22|184.869|324891|20230414|183.8800|186.2800|182.0100|185.0000|810998.27|183.9865|277547|".to_owned()
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract::stock("MSFT").build();
        let interval_end = datetime!(2023-04-15 16:31:22 UTC);
        let duration = 2.days();
        let bar_size = BarSize::Hour;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let historical_data = client
            .historical_data(&contract, Some(interval_end), duration, bar_size, what_to_show, trading_hours)
            .expect("historical data request failed");

        // Assert Response

        assert_eq!(historical_data.start, datetime!(2023-04-13 16:31:22 UTC), "historical_data.start");
        assert_eq!(historical_data.end, datetime!(2023-04-15 16:31:22 UTC), "historical_data.end");
        assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");

        assert_eq!(historical_data.bars[0].date, datetime!(2023-04-13 00:00:00 UTC), "bar.date");
        assert_eq!(historical_data.bars[0].open, 182.94, "bar.open");
        assert_eq!(historical_data.bars[0].high, 186.50, "bar.high");
        assert_eq!(historical_data.bars[0].low, 180.94, "bar.low");
        assert_eq!(historical_data.bars[0].close, 185.90, "bar.close");
        assert_eq!(historical_data.bars[0].volume, 948837.22, "bar.volume");
        assert_eq!(historical_data.bars[0].wap, 184.869, "bar.wap");
        assert_eq!(historical_data.bars[0].count, 324891, "bar.count");

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
    }

    #[test]
    fn test_historical_schedule() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_SCHEDULE);

        let contract = Contract::stock("MSFT").build();
        let end_date = datetime!(2023-04-15 16:31:22 UTC);
        let duration = 7.days();

        let schedule = client
            .historical_schedules(&contract, end_date, duration)
            .expect("historical schedule request failed");

        // Assert Response
        assert_eq!(schedule.time_zone, "US/Eastern", "schedule.time_zone");

        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;
        assert_eq!(
            schedule.start,
            datetime!(2023-04-14 09:30:00).assume_timezone(time_zone).unwrap(),
            "schedule.start"
        );
        assert_eq!(
            schedule.end,
            datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
            "schedule.end"
        );

        assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
        assert_eq!(schedule.sessions[0].reference, date!(2023 - 04 - 14), "schedule.sessions[0].reference");
        assert_eq!(
            schedule.sessions[0].start,
            datetime!(2023-04-14 09:30:00).assume_timezone(time_zone).unwrap(),
            "schedule.sessions[0].start"
        );
        assert_eq!(
            schedule.sessions[0].end,
            datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
            "schedule.sessions[0].end"
        );

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
    }

    #[test]
    fn test_historical_ticks_bid_ask() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let start = Some(datetime!(2023-04-01 09:30:00 UTC));
        let end = Some(datetime!(2023-04-01 16:00:00 UTC));
        let number_of_ticks = 10;
        let trading_hours = TradingHours::Regular;
        let ignore_size = true;

        // Just test that the function doesn't panic and returns a subscription
        let _tick_subscription = client
            .historical_ticks_bid_ask(&contract, start, end, number_of_ticks, trading_hours, ignore_size)
            .expect("historical ticks bid ask request failed");

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert!(!request_messages.is_empty(), "Should have sent a request message");
    }

    #[test]
    fn test_historical_ticks_mid_point() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let start = Some(datetime!(2023-04-01 09:30:00 UTC));
        let end = Some(datetime!(2023-04-01 16:00:00 UTC));
        let number_of_ticks = 10;
        let trading_hours = TradingHours::Regular;

        // Just test that the function doesn't panic and returns a subscription
        let _tick_subscription = client
            .historical_ticks_mid_point(&contract, start, end, number_of_ticks, trading_hours)
            .expect("historical ticks mid point request failed");

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert!(!request_messages.is_empty(), "Should have sent a request message");
    }

    #[test]
    fn test_historical_ticks_trade() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let start = Some(datetime!(2023-04-01 09:30:00 UTC));
        let end = Some(datetime!(2023-04-01 16:00:00 UTC));
        let number_of_ticks = 10;
        let trading_hours = TradingHours::Regular;

        // Just test that the function doesn't panic and returns a subscription
        let _tick_subscription = client
            .historical_ticks_trade(&contract, start, end, number_of_ticks, trading_hours)
            .expect("historical ticks trade request failed");

        // Assert Request
        let request_messages = client.message_bus.request_messages();
        assert!(!request_messages.is_empty(), "Should have sent a request message");
    }

    #[test]
    fn test_historical_data_version_check() {
        // Test with a server version that doesn't support trading class
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        // Use an older server version
        let client = Client::stubbed(message_bus, server_versions::TRADING_CLASS - 1);

        // Create a contract with trading_class set
        let mut contract = Contract::stock("MSFT").build();
        contract.trading_class = "CLASS".to_string();

        let end_date = datetime!(2023-04-15 16:31:22 UTC);
        let duration = 2.days();
        let bar_size = BarSize::Hour;
        let trading_hours = TradingHours::Regular;

        // This should return an error due to server version
        let result = client.historical_data(&contract, Some(end_date), duration, bar_size, WhatToShow::Trades, trading_hours);
        assert!(result.is_err(), "Expected error due to server version incompatibility");
    }

    #[test]
    fn test_historical_data_adjusted_last_validation() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("MSFT").build();
        let end_date = Some(datetime!(2023-04-15 16:31:22 UTC));
        let duration = 2.days();
        let bar_size = BarSize::Hour;
        let what_to_show = WhatToShow::AdjustedLast;
        let trading_hours = TradingHours::Regular;

        // This should return an error because AdjustedLast can't be used with end_date
        let result = client.historical_data(&contract, end_date, duration, bar_size, what_to_show, trading_hours);
        assert!(result.is_err(), "Expected error due to AdjustedLast with end_date");

        match result {
            Err(Error::InvalidArgument(_)) => {
                // This is the expected error type
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[test]
    fn test_historical_data_error_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Respond with an error message
                "3\09000\0200\0No security definition has been found for the request\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("MSFT").build();
        let duration = 2.days();
        let bar_size = BarSize::Hour;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        // This should return an error because the server sent an error response
        let result = client.historical_data(&contract, None, duration, bar_size, what_to_show, trading_hours);
        assert!(result.is_err(), "Expected error due to error response from server");
    }

    #[test]
    fn test_historical_data_unexpected_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Respond with an unexpected message type (using market data type message)
                "58\09000\02\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("MSFT").build();
        let duration = 2.days();
        let bar_size = BarSize::Hour;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        // This should return an error because the server sent an unexpected response
        let result = client.historical_data(&contract, None, duration, bar_size, what_to_show, trading_hours);
        assert!(result.is_err(), "Expected error due to unexpected response type");

        match result {
            Err(Error::UnexpectedResponse(_)) => {
                // This is the expected error type
            }
            _ => panic!("Expected UnexpectedResponse error"),
        }
    }

    #[test]
    fn test_tick_subscription_methods() {
        // For now, we'll use a minimal test to ensure the methods exist and are called correctly
        // Testing the subscription iterators fully would require more complex setup with mocked messages

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let number_of_ticks = 10;
        let trading_hours = TradingHours::Regular;

        let tick_subscription = client
            .historical_ticks_trade(&contract, None, None, number_of_ticks, trading_hours)
            .expect("historical ticks trade request failed");

        // Just test that these methods can be called without panicking
        let _iter = tick_subscription.iter();
        let _try_iter = tick_subscription.try_iter();
        let _timeout_iter = tick_subscription.timeout_iter(std::time::Duration::from_millis(100));

        // Test IntoIterator trait exists
        let _iter_ref: TickSubscriptionIter<TickLast> = (&tick_subscription).into_iter();
    }

    #[test]
    fn test_tick_subscription_buffer_and_iteration() {
        // Create a message bus with predetermined responses
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // First response has 3 ticks, done = false
                "98\09000\03\01681133400\00\011.63\024547\0ISLAND\0 O X\01681133401\00\011.64\0179\0FINRA\0\01681133402\00\011.65\0200\0NYSE\0\00\0"
                    .to_owned(),
                // Second response has 2 ticks, done = true
                "98\09000\02\01681133403\00\011.66\0100\0ARCA\0\01681133404\00\011.67\0300\0BATS\0\01\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let number_of_ticks = 10;
        let trading_hours = TradingHours::Regular;

        let tick_subscription = client
            .historical_ticks_trade(&contract, None, None, number_of_ticks, trading_hours)
            .expect("historical ticks trade request failed");

        // Test standard iterator
        let mut ticks = Vec::new();
        for tick in tick_subscription.iter() {
            ticks.push(tick);
        }

        // Should have received all 5 ticks from both messages
        assert_eq!(ticks.len(), 5, "Expected 5 ticks in total");

        // Check specific values from first and last ticks
        assert_eq!(ticks[0].price, 11.63, "First tick price");
        assert_eq!(ticks[0].exchange, "ISLAND", "First tick exchange");

        assert_eq!(ticks[4].price, 11.67, "Last tick price");
        assert_eq!(ticks[4].exchange, "BATS", "Last tick exchange");
    }

    #[test]
    fn test_tick_subscription_owned_iterator() {
        // Test that the owned iterator works correctly
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["98\09000\02\01681133400\00\011.70\024547\0ISLAND\0 O X\01681133401\00\011.71\0179\0FINRA\0\01\0".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let tick_subscription = client
            .historical_ticks_trade(&contract, None, None, 10, TradingHours::Regular)
            .expect("historical ticks trade request failed");

        // Convert to owned iterator
        let ticks: Vec<TickLast> = tick_subscription.into_iter().collect();

        assert_eq!(ticks.len(), 2, "Expected 2 ticks from owned iterator");
        assert_eq!(ticks[0].price, 11.70, "First tick price");
        assert_eq!(ticks[1].price, 11.71, "Second tick price");
    }

    #[test]
    fn test_tick_subscription_bid_ask() {
        // Create a message bus with bid/ask tick data
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "97\09000\03\01681133399\00\011.63\011.83\02800\0100\01681133400\00\011.64\011.84\02900\0200\01681133401\00\011.65\011.85\03000\0300\01\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let tick_subscription = client
            .historical_ticks_bid_ask(&contract, None, None, 10, TradingHours::Regular, false)
            .expect("historical ticks bid_ask request failed");

        // Collect ticks
        let ticks: Vec<TickBidAsk> = tick_subscription.iter().collect();

        assert_eq!(ticks.len(), 3, "Expected 3 bid/ask ticks");

        // Check first tick
        assert_eq!(ticks[0].price_bid, 11.63, "First tick bid price");
        assert_eq!(ticks[0].price_ask, 11.83, "First tick ask price");
        assert_eq!(ticks[0].size_bid, 2800, "First tick bid size");
        assert_eq!(ticks[0].size_ask, 100, "First tick ask size");

        // Check last tick
        assert_eq!(ticks[2].price_bid, 11.65, "Last tick bid price");
        assert_eq!(ticks[2].price_ask, 11.85, "Last tick ask price");
    }

    #[test]
    fn test_tick_subscription_midpoint() {
        // Create a message bus with midpoint tick data
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["96\09000\03\01681133398\00\091.36\00\01681133399\00\091.37\00\01681133400\00\091.38\00\01\0".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

        let contract = Contract::stock("MSFT").build();
        let tick_subscription = client
            .historical_ticks_mid_point(&contract, None, None, 10, TradingHours::Regular)
            .expect("historical ticks mid_point request failed");

        // Collect ticks
        let ticks: Vec<TickMidpoint> = tick_subscription.iter().collect();

        assert_eq!(ticks.len(), 3, "Expected 3 midpoint ticks");

        // Check specific tick values
        assert_eq!(ticks[0].price, 91.36, "First tick price");
        assert_eq!(ticks[1].price, 91.37, "Second tick price");
        assert_eq!(ticks[2].price, 91.38, "Third tick price");
    }

    #[test]
    fn test_historical_data_time_zone_handling() {
        // Test with explicit Eastern time zone data
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Format: historical data with NY timezone in the response
                "17\09000\020230413  09:30:00\020230415  16:00:00\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0".to_owned()
            ],
        });

        // Create a client with a time zone specifically set to NY
        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

        let contract = Contract::stock("MSFT").build();
        let interval_end = datetime!(2023-04-15 16:00:00 UTC);
        let duration = 2.days();
        let bar_size = BarSize::Day;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let historical_data = client
            .historical_data(&contract, Some(interval_end), duration, bar_size, what_to_show, trading_hours)
            .expect("historical data request failed");

        // Assert that time zones are correctly handled
        let ny_zone = time_tz::timezones::db::america::NEW_YORK;

        // Start time should be 9:30 AM ET
        assert_eq!(
            historical_data.start,
            datetime!(2023-04-13 09:30:00).assume_timezone(ny_zone).unwrap(),
            "historical_data.start should be in NY timezone"
        );

        // End time should be 4:00 PM ET
        assert_eq!(
            historical_data.end,
            datetime!(2023-04-15 16:00:00).assume_timezone(ny_zone).unwrap(),
            "historical_data.end should be in NY timezone"
        );
    }

    #[test]
    fn test_time_zone_fallback() {
        // Test the time_zone function's fallback behavior
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        // Create a client without a time zone (should fall back to UTC)
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        // Test that the function returns UTC when client.time_zone is None
        assert_eq!(
            time_zone(&client),
            time_tz::timezones::db::UTC,
            "time_zone should fall back to UTC when client.time_zone is None"
        );

        // Create a client with a time zone set to NY
        let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

        // Test that the function returns the client's time zone when it is set
        assert_eq!(
            time_zone(&client),
            time_tz::timezones::db::america::NEW_YORK,
            "time_zone should return the client's time zone when it is set"
        );
    }

    #[test]
    fn test_historical_data_streaming_with_updates() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Initial historical data (message type 17)
                "17\09000\020230315  09:30:00\020230315  10:30:00\01\01678886400\0185.50\0186.00\0185.25\0185.75\01000\0185.70\0100\0".to_owned(),
                // Streaming update (message type 90)
                "90\09000\0-1\01678890000\0185.80\0186.10\0185.60\0185.90\0500\0185.85\050\0".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let subscription = client
            .historical_data_streaming(
                &contract,
                Duration::days(1),
                BarSize::Hour,
                Some(WhatToShow::Trades),
                TradingHours::Regular,
                true,
            )
            .expect("streaming request should succeed");

        // First: receive initial historical data
        let update1 = subscription.next();
        assert!(update1.is_some(), "Should receive initial historical data");
        match update1.unwrap() {
            HistoricalBarUpdate::Historical(data) => {
                assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
                assert_eq!(data.bars[0].open, 185.50, "Wrong open price");
            }
            _ => panic!("Expected Historical variant"),
        }

        // Second: receive streaming update
        let update2 = subscription.next();
        assert!(update2.is_some(), "Should receive streaming update");
        match update2.unwrap() {
            HistoricalBarUpdate::Update(bar) => {
                assert_eq!(bar.open, 185.80, "Wrong open price in update");
                assert_eq!(bar.high, 186.10, "Wrong high price in update");
                assert_eq!(bar.close, 185.90, "Wrong close price in update");
            }
            _ => panic!("Expected Update variant"),
        }

        // Verify request message was sent
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
    }

    #[test]
    fn test_historical_data_streaming_keep_up_to_date_false() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Initial historical data only
                "17\09000\020230315  09:30:00\020230315  10:30:00\01\01678886400\0185.50\0186.00\0185.25\0185.75\01000\0185.70\0100\0".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let subscription = client
            .historical_data_streaming(
                &contract,
                Duration::days(1),
                BarSize::Hour,
                Some(WhatToShow::Trades),
                TradingHours::Regular,
                false, // keep_up_to_date = false
            )
            .expect("streaming request should succeed");

        // Receive initial historical data
        let update1 = subscription.next();
        assert!(update1.is_some(), "Should receive initial historical data");
        match update1.unwrap() {
            HistoricalBarUpdate::Historical(data) => {
                assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
            }
            _ => panic!("Expected Historical variant"),
        }

        // Verify request message was sent
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
    }

    #[test]
    fn test_historical_data_streaming_error_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Error response
                "4\02\09000\0162\0Historical Market Data Service error message:No market data permissions.\0".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let subscription = client
            .historical_data_streaming(
                &contract,
                Duration::days(1),
                BarSize::Hour,
                Some(WhatToShow::Trades),
                TradingHours::Regular,
                true,
            )
            .expect("streaming request should succeed");

        // Should return None due to error
        let update = subscription.next();
        assert!(update.is_none(), "Should return None on error");

        // Error should be accessible
        let error = subscription.error();
        assert!(error.is_some(), "Error should be stored");
        assert!(
            error.unwrap().to_string().contains("No market data permissions"),
            "Error should contain the message"
        );
    }

    #[test]
    fn test_streaming_subscription_sends_cancel_on_drop() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let internal = message_bus.send_request(9000, &[]).unwrap();

        {
            let _subscription = HistoricalDataStreamingSubscription::new(
                internal,
                server_versions::SIZE_RULES,
                time_tz::timezones::db::UTC,
                9000,
                message_bus.clone(),
            );
            // subscription dropped here
        }

        let messages = message_bus.request_messages.read().unwrap();
        // First message is the send_request call, second is the cancel
        let cancel_msg = messages.last().expect("should have cancel message");
        assert_proto_msg_id(cancel_msg, OutgoingMessages::CancelHistoricalData);
    }

    #[test]
    fn test_streaming_subscription_cancel_prevents_duplicate_on_drop() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let internal = message_bus.send_request(9001, &[]).unwrap();

        {
            let subscription = HistoricalDataStreamingSubscription::new(
                internal,
                server_versions::SIZE_RULES,
                time_tz::timezones::db::UTC,
                9001,
                message_bus.clone(),
            );

            // Explicit cancel
            subscription.cancel();

            // Drop should not send a second cancel
        }

        let messages = message_bus.request_messages.read().unwrap();
        let cancel_count = messages
            .iter()
            .filter(|m| {
                if m.len() >= 4 {
                    let msg_id = i32::from_be_bytes([m[0], m[1], m[2], m[3]]);
                    msg_id == OutgoingMessages::CancelHistoricalData as i32 + 200
                } else {
                    false
                }
            })
            .count();
        assert_eq!(cancel_count, 1, "should send cancel only once");
    }
}
