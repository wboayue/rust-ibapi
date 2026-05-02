use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{debug, error, warn};
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
        let request_id = builder.request_id();
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
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
        let request_id = builder.request_id();
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
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
        let request_id = builder.request_id();
        let subscription = builder.send_raw(request)?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
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
mod tests;
