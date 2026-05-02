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
use super::{
    BarSize, Duration, HistogramEntry, HistoricalBarUpdate, HistoricalData, Schedule, TickBidAsk, TickDecoder, TickLast, TickMidpoint, WhatToShow,
};
use crate::market_data::TradingHours;

// === Public API Functions ===

impl Client {
    /// Returns the timestamp of earliest available historical data for a contract and data type.
    pub async fn head_timestamp(&self, contract: &Contract, what_to_show: WhatToShow, trading_hours: TradingHours) -> Result<OffsetDateTime, Error> {
        check_version(self.server_version(), Features::HEAD_TIMESTAMP)?;

        let builder = self.request();
        let request = encoders::encode_request_head_timestamp(builder.request_id(), contract, what_to_show, trading_hours.use_rth())?;
        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HeadTimestamp => {
                Ok(decoders::decode_head_timestamp(&mut message, self.time_zone())?)
            }
            Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
            Some(Err(e)) => Err(e),
            None => {
                // Connection might have been reset, retry
                Box::pin(self.head_timestamp(contract, what_to_show, trading_hours)).await
            }
        }
    }

    /// Requests historical data for a contract.
    pub async fn historical_data(
        &self,
        contract: &Contract,
        end_date: Option<OffsetDateTime>,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: Option<WhatToShow>,
        trading_hours: TradingHours,
    ) -> Result<HistoricalData, Error> {
        common::validate_historical_data(self.server_version(), contract, end_date, what_to_show)?;

        for _ in 0..MAX_RETRIES {
            let builder = self.request();
            let request = encoders::encode_request_historical_data(
                builder.request_id(),
                contract,
                end_date,
                duration,
                bar_size,
                what_to_show,
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
                Some(Ok(message)) => return Err(Error::UnexpectedResponse(message)),
                Some(Err(e)) => return Err(e),
                None => continue, // Connection reset, retry
            }
        }

        Err(Error::ConnectionReset)
    }

    /// Requests historical schedule data for a contract.
    pub async fn historical_schedule(&self, contract: &Contract, end_date: Option<OffsetDateTime>, duration: Duration) -> Result<Schedule, Error> {
        if !contract.trading_class.is_empty() || contract.contract_id > 0 {
            check_version(self.server_version(), Features::TRADING_CLASS)?;
        }

        check_version(self.server_version(), Features::HISTORICAL_SCHEDULE)?;

        loop {
            let builder = self.request();
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
                Some(Ok(message)) => return Err(Error::UnexpectedResponse(message)),
                Some(Err(e)) => return Err(e),
                None => continue, // Connection reset, retry
            }
        }
    }

    /// Requests historical bid/ask tick data.
    pub async fn historical_ticks_bid_ask(
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
        let subscription = builder.send_raw(request).await?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
    }

    /// Requests historical midpoint tick data.
    pub async fn historical_ticks_mid_point(
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
        let subscription = builder.send_raw(request).await?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
    }

    /// Requests historical trade tick data.
    pub async fn historical_ticks_trade(
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
        let subscription = builder.send_raw(request).await?;

        Ok(TickSubscription::new(subscription, request_id, Arc::clone(&self.message_bus)))
    }

    /// Cancels an in-flight historical ticks request.
    pub async fn cancel_historical_ticks(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version(), Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_historical_ticks(request_id)?;
        self.send_message(message).await?;
        Ok(())
    }

    /// Requests histogram data for a contract.
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
    pub async fn historical_data_streaming(
        &self,
        contract: &Contract,
        duration: Duration,
        bar_size: BarSize,
        what_to_show: Option<WhatToShow>,
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
            what_to_show,
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

// === TickSubscription and related types ===

/// Async subscription for historical tick data
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

    /// Get the next tick from the subscription
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
