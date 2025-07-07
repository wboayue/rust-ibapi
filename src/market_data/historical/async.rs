use log::{debug, warn};
use std::collections::VecDeque;
use time::OffsetDateTime;

use crate::client::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::transport::AsyncInternalSubscription;
use crate::{Client, Error, MAX_RETRIES};

use super::common::{decoders, encoders};
use super::{BarSize, Duration, HistogramEntry, HistoricalData, Schedule, TickBidAsk, TickDecoder, TickLast, TickMidpoint, WhatToShow};

// === Public API Functions ===

/// Returns the timestamp of earliest available historical data for a contract and data type.
pub async fn head_timestamp(client: &Client, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<OffsetDateTime, Error> {
    check_version(client.server_version(), Features::HEAD_TIMESTAMP)?;

    let builder = client.request();
    let request = encoders::encode_request_head_timestamp(builder.request_id(), contract, what_to_show, use_rth)?;
    let mut subscription = builder.send_raw(request).await?;

    match subscription.next().await {
        Some(mut message) if message.message_type() == IncomingMessages::HeadTimestamp => Ok(decoders::decode_head_timestamp(&mut message)?),
        Some(message) => Err(Error::UnexpectedResponse(message)),
        None => {
            // Connection might have been reset, retry
            Box::pin(head_timestamp(client, contract, what_to_show, use_rth)).await
        }
    }
}

/// Requests historical data for a contract.
/// https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
pub async fn historical_data(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
) -> Result<HistoricalData, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    if what_to_show == Some(WhatToShow::Schedule) {
        check_version(client.server_version(), Features::HISTORICAL_SCHEDULE)?;
    }

    if end_date.is_some() && what_to_show == Some(WhatToShow::AdjustedLast) {
        return Err(Error::InvalidArgument(
            "end_date must be None when requesting WhatToShow::AdjustedLast.".into(),
        ));
    }

    for _ in 0..MAX_RETRIES {
        let builder = client.request();
        let request = encoders::encode_request_historical_data(
            client.server_version(),
            builder.request_id(),
            contract,
            end_date,
            duration,
            bar_size,
            what_to_show,
            use_rth,
            false,
            Vec::<crate::contracts::TagValue>::default(),
        )?;

        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(mut message) if message.message_type() == IncomingMessages::HistoricalData => {
                return decoders::decode_historical_data(client.server_version(), time_zone(client), &mut message)
            }
            Some(message) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
            Some(message) => return Err(Error::UnexpectedResponse(message)),
            None => continue, // Connection reset, retry
        }
    }

    Err(Error::ConnectionReset)
}

pub(crate) fn time_zone(client: &Client) -> &time_tz::Tz {
    if let Some(tz) = client.time_zone {
        tz
    } else {
        warn!("server timezone unknown. assuming UTC, but that may be incorrect!");
        time_tz::timezones::db::UTC
    }
}

/// Requests historical schedule data for a contract.
pub async fn historical_schedule(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
) -> Result<Schedule, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    check_version(client.server_version(), Features::HISTORICAL_SCHEDULE)?;

    loop {
        let builder = client.request();
        let request = encoders::encode_request_historical_data(
            client.server_version(),
            builder.request_id(),
            contract,
            end_date,
            duration,
            BarSize::Day,
            Some(WhatToShow::Schedule),
            true,
            false,
            Vec::<crate::contracts::TagValue>::default(),
        )?;

        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(mut message) if message.message_type() == IncomingMessages::HistoricalSchedule => {
                return decoders::decode_historical_schedule(&mut message)
            }
            Some(message) => return Err(Error::UnexpectedResponse(message)),
            None => continue, // Connection reset, retry
        }
    }
}

/// Requests historical bid/ask tick data.
pub async fn historical_ticks_bid_ask(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
    ignore_size: bool,
) -> Result<TickSubscription<TickBidAsk>, Error> {
    check_version(client.server_version(), Features::HISTORICAL_TICKS)?;

    let builder = client.request();
    let request = encoders::encode_request_historical_ticks(
        builder.request_id(),
        contract,
        start,
        end,
        number_of_ticks,
        WhatToShow::BidAsk,
        use_rth,
        ignore_size,
    )?;
    let subscription = builder.send_raw(request).await?;

    Ok(TickSubscription::new(subscription))
}

/// Requests historical midpoint tick data.
pub async fn historical_ticks_mid_point(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickSubscription<TickMidpoint>, Error> {
    check_version(client.server_version(), Features::HISTORICAL_TICKS)?;

    let builder = client.request();
    let request = encoders::encode_request_historical_ticks(
        builder.request_id(),
        contract,
        start,
        end,
        number_of_ticks,
        WhatToShow::MidPoint,
        use_rth,
        false,
    )?;
    let subscription = builder.send_raw(request).await?;

    Ok(TickSubscription::new(subscription))
}

/// Requests historical trade tick data.
pub async fn historical_ticks_trade(
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: bool,
) -> Result<TickSubscription<TickLast>, Error> {
    check_version(client.server_version(), Features::HISTORICAL_TICKS)?;

    let builder = client.request();
    let request = encoders::encode_request_historical_ticks(
        builder.request_id(),
        contract,
        start,
        end,
        number_of_ticks,
        WhatToShow::Trades,
        use_rth,
        false,
    )?;
    let subscription = builder.send_raw(request).await?;

    Ok(TickSubscription::new(subscription))
}

/// Requests histogram data for a contract.
pub async fn histogram_data(client: &Client, contract: &Contract, use_rth: bool, period: BarSize) -> Result<Vec<HistogramEntry>, Error> {
    check_version(client.server_version(), Features::HISTOGRAM)?;

    loop {
        let builder = client.request();
        let request = encoders::encode_request_histogram_data(builder.request_id(), contract, use_rth, period)?;
        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(mut message) => return decoders::decode_histogram_data(&mut message),
            None => continue, // Connection reset, retry
        }
    }
}

// === TickSubscription and related types ===

/// Async subscription for historical tick data
pub struct TickSubscription<T: TickDecoder<T> + Send> {
    done: bool,
    messages: AsyncInternalSubscription,
    buffer: VecDeque<T>,
    error: Option<Error>,
}

impl<T: TickDecoder<T> + Send> TickSubscription<T> {
    fn new(messages: AsyncInternalSubscription) -> Self {
        Self {
            done: false,
            messages,
            buffer: VecDeque::new(),
            error: None,
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
            Some(mut message) if message.message_type() == T::MESSAGE_TYPE => {
                let (ticks, done) = T::decode(&mut message).unwrap();
                self.buffer.extend(ticks);
                self.done = done;
                Ok(())
            }
            Some(message) => {
                debug!("unexpected message: {:?}", message);
                Ok(())
            }
            None => Err(()),
        }
    }

    fn next_buffered(&mut self) -> Option<T> {
        self.buffer.pop_front()
    }

    fn set_error(&mut self, e: Error) {
        self.error = Some(e);
    }

    fn clear_error(&mut self) {
        self.error = None;
    }
}
