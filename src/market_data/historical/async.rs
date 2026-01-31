use log::{debug, warn};
use std::collections::VecDeque;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::client::ClientRequestBuilders;
use crate::contracts::Contract;
use crate::messages::IncomingMessages;
use crate::protocol::{check_version, Features};
use crate::transport::AsyncInternalSubscription;
use crate::{Client, Error, MAX_RETRIES};

use super::common::{decoders, encoders};
use super::{
    BarSize, Duration, HistogramEntry, HistoricalBarUpdate, HistoricalData, Schedule, TickBidAsk, TickDecoder, TickLast, TickMidpoint, WhatToShow,
};
use crate::market_data::TradingHours;

// === Public API Functions ===

/// Returns the timestamp of earliest available historical data for a contract and data type.
pub async fn head_timestamp(
    client: &Client,
    contract: &Contract,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
) -> Result<OffsetDateTime, Error> {
    check_version(client.server_version(), Features::HEAD_TIMESTAMP)?;

    let builder = client.request();
    let request = encoders::encode_request_head_timestamp(builder.request_id(), contract, what_to_show, trading_hours.use_rth())?;
    let mut subscription = builder.send_raw(request).await?;

    match subscription.next().await {
        Some(Ok(mut message)) if message.message_type() == IncomingMessages::HeadTimestamp => Ok(decoders::decode_head_timestamp(&mut message)?),
        Some(Ok(message)) => Err(Error::UnexpectedResponse(message)),
        Some(Err(e)) => Err(e),
        None => {
            // Connection might have been reset, retry
            Box::pin(head_timestamp(client, contract, what_to_show, trading_hours)).await
        }
    }
}

/// Requests historical data for a contract.
///
/// # See Also
/// * [TWS API Documentation](https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration)
/// * IB also recommends [IBKR Campus](https://ibkrcampus.com/ibkr-api-page/trader-workstation-api/)
pub async fn historical_data(
    client: &Client,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    trading_hours: TradingHours,
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
            trading_hours.use_rth(),
            false,
            Vec::<crate::contracts::TagValue>::default(),
        )?;

        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) if message.message_type() == IncomingMessages::HistoricalData => {
                return decoders::decode_historical_data(client.server_version(), time_zone(client), &mut message)
            }
            Some(Ok(message)) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
            Some(Ok(message)) => return Err(Error::UnexpectedResponse(message)),
            Some(Err(e)) => return Err(e),
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
    client: &Client,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    trading_hours: TradingHours,
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
        trading_hours.use_rth(),
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
    trading_hours: TradingHours,
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
        trading_hours.use_rth(),
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
    trading_hours: TradingHours,
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
        trading_hours.use_rth(),
        false,
    )?;
    let subscription = builder.send_raw(request).await?;

    Ok(TickSubscription::new(subscription))
}

/// Requests histogram data for a contract.
pub async fn histogram_data(
    client: &Client,
    contract: &Contract,
    trading_hours: TradingHours,
    period: BarSize,
) -> Result<Vec<HistogramEntry>, Error> {
    check_version(client.server_version(), Features::HISTOGRAM)?;

    loop {
        let builder = client.request();
        let request = encoders::encode_request_histogram_data(builder.request_id(), contract, trading_hours.use_rth(), period)?;
        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => return decoders::decode_histogram_data(&mut message),
            Some(Err(e)) => return Err(e),
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

// === Historical Data Streaming with keepUpToDate ===

/// Requests historical data for a contract with optional streaming updates.
///
/// When `keep_up_to_date` is `true`, this function requests historical bars and then
/// continues to receive streaming updates for the current (incomplete) bar. IBKR sends
/// updates approximately every 4-6 seconds until the bar completes, at which point a
/// new bar begins.
///
/// When `keep_up_to_date` is `false`, only the initial historical data is returned
/// and the subscription ends after delivering the data.
///
/// **Important IBKR behavior (when keepUpToDate=true):**
/// - The same timestamp bar is sent multiple times as it builds (with updated OHLCV)
/// - When a NEW timestamp appears, the previous bar is considered complete
/// - Supported `what_to_show` values: Trades, Midpoint, Bid, Ask only
///
/// # Arguments
/// * `client` - The IBKR client connection
/// * `contract` - The contract to request data for
/// * `duration` - How far back to request initial historical data
/// * `bar_size` - The bar size (e.g., Min15 for 15-minute bars)
/// * `what_to_show` - The data type (Trades, Midpoint, Bid, or Ask)
/// * `trading_hours` - Whether to use regular trading hours only
/// * `keep_up_to_date` - If true, continue receiving streaming updates after initial data
///
/// # Returns
/// A `HistoricalDataStreamingSubscription` that yields `HistoricalBarUpdate` values
///
/// # Example
/// ```no_run
/// use ibapi::Client;
/// use ibapi::contracts::Contract;
/// use ibapi::market_data::historical::{
///     BarSize, Duration, HistoricalBarUpdate, WhatToShow, historical_data_streaming
/// };
/// use ibapi::market_data::TradingHours;
///
/// # async fn example() -> Result<(), ibapi::Error> {
/// let client = Client::connect("127.0.0.1:4002", 100).await?;
/// let contract = Contract::stock("SPY").build();
///
/// let mut subscription = historical_data_streaming(
///     &client,
///     &contract,
///     Duration::days(1),
///     BarSize::Min15,
///     Some(WhatToShow::Trades),
///     TradingHours::Regular,
///     true,  // keep_up_to_date
/// ).await?;
///
/// while let Some(update) = subscription.next().await {
///     match update {
///         HistoricalBarUpdate::Historical(data) => {
///             println!("Received {} historical bars", data.bars.len());
///         }
///         HistoricalBarUpdate::Update(bar) => {
///             println!("Bar update: {} close={}", bar.date, bar.close);
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # See Also
/// * [IBKR Campus - keepUpToDate](https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/#hist-keepUp-date)
pub async fn historical_data_streaming(
    client: &Client,
    contract: &Contract,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    trading_hours: TradingHours,
    keep_up_to_date: bool,
) -> Result<HistoricalDataStreamingSubscription, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    // Note: end_date must be None when keepUpToDate=true (IBKR requirement)
    let builder = client.request();
    let request = encoders::encode_request_historical_data(
        client.server_version(),
        builder.request_id(),
        contract,
        None, // end_date must be None for keepUpToDate
        duration,
        bar_size,
        what_to_show,
        trading_hours.use_rth(),
        keep_up_to_date,
        Vec::<crate::contracts::TagValue>::default(),
    )?;

    let subscription = builder.send_raw(request).await?;

    // Get the timezone directly to avoid lifetime issues
    // time_zone(client) returns a reference tied to client's lifetime,
    // but we need a 'static reference for the subscription struct
    let tz: &'static Tz = client.time_zone.unwrap_or_else(|| {
        warn!("server timezone unknown. assuming UTC, but that may be incorrect!");
        time_tz::timezones::db::UTC
    });

    Ok(HistoricalDataStreamingSubscription::new(subscription, client.server_version(), tz))
}

/// Async subscription for streaming historical data with keepUpToDate=true.
///
/// This subscription first yields the initial historical bars as a `Historical` variant,
/// then continues to yield streaming updates for the current bar as `Update` variants.
pub struct HistoricalDataStreamingSubscription {
    messages: AsyncInternalSubscription,
    server_version: i32,
    time_zone: &'static Tz,
    error: Option<Error>,
}

impl HistoricalDataStreamingSubscription {
    fn new(messages: AsyncInternalSubscription, server_version: i32, time_zone: &'static Tz) -> Self {
        Self {
            messages,
            server_version,
            time_zone,
            error: None,
        }
    }

    /// Get the next update from the streaming subscription.
    ///
    /// Returns:
    /// - `Some(HistoricalBarUpdate::Historical(data))` - Initial batch of historical bars (always first)
    /// - `Some(HistoricalBarUpdate::Update(bar))` - Streaming bar update
    /// - `None` - Subscription ended (connection closed or error)
    pub async fn next(&mut self) -> Option<HistoricalBarUpdate> {
        loop {
            match self.messages.next().await {
                Some(Ok(mut message)) => {
                    match message.message_type() {
                        IncomingMessages::HistoricalData => {
                            // Initial historical data batch
                            match decoders::decode_historical_data(self.server_version, self.time_zone, &mut message) {
                                Ok(data) => {
                                    return Some(HistoricalBarUpdate::Historical(data));
                                }
                                Err(e) => {
                                    self.error = Some(e);
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
                                    self.error = Some(e);
                                    return None;
                                }
                            }
                        }
                        IncomingMessages::Error => {
                            self.error = Some(Error::from(message));
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
                    self.error = Some(e);
                    return None;
                }
                None => {
                    // Channel closed
                    return None;
                }
            }
        }
    }

    /// Returns the last error that occurred, if any.
    pub fn error(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::ToField;
    use std::sync::Arc;
    use std::sync::RwLock;
    use time::macros::datetime;

    #[tokio::test]
    async fn test_head_timestamp() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["88|9000|1678838400|".to_owned()], // 2023-03-15 00:00:00 UTC
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        let result = head_timestamp(&client, &contract, what_to_show, trading_hours).await;
        assert!(result.is_ok(), "head_timestamp should succeed");

        let timestamp = result.unwrap();
        assert_eq!(timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request.fields[0], OutgoingMessages::RequestHeadTimestamp.to_field(), "message.type");
        assert_eq!(request.fields[1], "9000", "message.request_id");
        assert_eq!(request.fields[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(request.fields[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(request.fields[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            request.fields[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(request.fields[6], contract.strike.to_field(), "message.strike");
        assert_eq!(request.fields[7], contract.right, "message.right");
        assert_eq!(request.fields[8], contract.multiplier, "message.multiplier");
        assert_eq!(request.fields[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(request.fields[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(request.fields[11], contract.currency.to_field(), "message.currency");
        assert_eq!(request.fields[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(request.fields[13], contract.trading_class, "message.trading_class");
        assert_eq!(request.fields[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(request.fields[15], trading_hours.use_rth().to_field(), "message.use_rth");
        assert_eq!(request.fields[16], what_to_show.to_field(), "message.what_to_show");
        assert_eq!(request.fields[17], "2", "message.date_format");
    }

    #[tokio::test]
    async fn test_histogram_data() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["96|9000|3|185.50|100|185.75|150|186.00|200|".to_owned()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_HISTOGRAM);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let trading_hours = TradingHours::Regular;
        let period = BarSize::Day;

        let result = histogram_data(&client, &contract, trading_hours, period).await;
        assert!(result.is_ok(), "histogram_data should succeed");

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3, "Should receive 3 histogram entries");

        // Verify first entry
        assert_eq!(entries[0].price, 185.50, "Wrong price for first entry");
        assert_eq!(entries[0].size, 100, "Wrong size for first entry");

        // Verify second entry
        assert_eq!(entries[1].price, 185.75, "Wrong price for second entry");
        assert_eq!(entries[1].size, 150, "Wrong size for second entry");

        // Verify third entry
        assert_eq!(entries[2].price, 186.00, "Wrong price for third entry");
        assert_eq!(entries[2].size, 200, "Wrong size for third entry");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request.fields[0],
            OutgoingMessages::RequestHistogramData.to_field(),
            "message.message_type"
        );
        assert_eq!(request.fields[1], "9000", "message.request_id");
        assert_eq!(request.fields[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(request.fields[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(request.fields[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            request.fields[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(request.fields[6], contract.strike.to_field(), "message.strike");
        assert_eq!(request.fields[7], contract.right, "message.right");
        assert_eq!(request.fields[8], contract.multiplier, "message.multiplier");
        assert_eq!(request.fields[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(request.fields[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(request.fields[11], contract.currency.to_field(), "message.currency");
        assert_eq!(request.fields[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(request.fields[13], contract.trading_class, "message.trading_class");
        assert_eq!(request.fields[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(request.fields[15], trading_hours.use_rth().to_field(), "message.use_rth");
        assert_eq!(request.fields[16], period.to_field(), "message.duration");
    }

    #[tokio::test]
    async fn test_historical_data() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "17|9000|20230315  09:30:00|20230315  10:30:00|2|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|1678890000|185.75|186.25|185.50|186.00|1500|185.85|150|"
                    .to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        // Set client timezone for test
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));
        let duration = Duration::seconds(3600);
        let bar_size = BarSize::Min30;
        let what_to_show = Some(WhatToShow::Trades);
        let trading_hours = TradingHours::Regular;

        let result = historical_data(&client, &contract, end_date, duration, bar_size, what_to_show, trading_hours).await;
        assert!(result.is_ok(), "historical_data should succeed");

        let data = result.unwrap();
        assert_eq!(data.bars.len(), 2, "Should receive 2 bars");

        // Verify first bar
        let bar = &data.bars[0];
        // 1678886400 = 2023-03-15 13:20:00 UTC
        assert_eq!(bar.date, datetime!(2023-03-15 13:20:00 UTC), "Wrong date for first bar");
        assert_eq!(bar.open, 185.50, "Wrong open for first bar");
        assert_eq!(bar.high, 186.00, "Wrong high for first bar");
        assert_eq!(bar.low, 185.25, "Wrong low for first bar");
        assert_eq!(bar.close, 185.75, "Wrong close for first bar");
        assert_eq!(bar.volume, 1000.0, "Wrong volume for first bar");
        assert_eq!(bar.wap, 185.70, "Wrong WAP for first bar");
        assert_eq!(bar.count, 100, "Wrong count for first bar");

        // Verify second bar
        let bar = &data.bars[1];
        // 1678890000 = 2023-03-15 14:20:00 UTC
        assert_eq!(bar.date, datetime!(2023-03-15 14:20:00 UTC), "Wrong date for second bar");
        assert_eq!(bar.open, 185.75, "Wrong open for second bar");
        assert_eq!(bar.high, 186.25, "Wrong high for second bar");
        assert_eq!(bar.low, 185.50, "Wrong low for second bar");
        assert_eq!(bar.close, 186.00, "Wrong close for second bar");
        assert_eq!(bar.volume, 1500.0, "Wrong volume for second bar");
        assert_eq!(bar.wap, 185.85, "Wrong WAP for second bar");
        assert_eq!(bar.count, 150, "Wrong count for second bar");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request.fields[0],
            OutgoingMessages::RequestHistoricalData.to_field(),
            "Wrong message type"
        );
    }

    #[tokio::test]
    async fn test_historical_data_version_check() {
        let message_bus = Arc::new(MessageBusStub::default());
        let client = Client::stubbed(message_bus, server_versions::TRADING_CLASS - 1);

        let mut contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        contract.trading_class = "ES".to_string(); // Requires TRADING_CLASS version

        let result = historical_data(&client, &contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular).await;
        assert!(result.is_err(), "Should fail version check");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("trading class"),
            "Error should mention trading class feature: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_historical_data_adjusted_last_validation() {
        let message_bus = Arc::new(MessageBusStub::default());
        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract::stock("AAPL").build();
        let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));

        let result = historical_data(
            &client,
            &contract,
            end_date,
            Duration::days(1),
            BarSize::Day,
            Some(WhatToShow::AdjustedLast),
            TradingHours::Regular,
        )
        .await;

        assert!(result.is_err(), "Should fail when end_date is provided with AdjustedLast");
        assert!(
            result.unwrap_err().to_string().contains("end_date must be None"),
            "Error should mention end_date restriction"
        );
    }

    #[tokio::test]
    async fn test_historical_data_error_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["4|2|9000|162|Historical Market Data Service error message:No market data permissions.|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let result = historical_data(&client, &contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular).await;
        assert!(result.is_err(), "Should fail with error response");
        assert!(
            result.unwrap_err().to_string().contains("No market data permissions"),
            "Error should contain the error message"
        );
    }

    #[tokio::test]
    async fn test_historical_data_unexpected_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["1|2|9000|1|185.50|100|7|".to_owned()], // Wrong message type
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let result = historical_data(&client, &contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular).await;
        assert!(result.is_err(), "Should fail with unexpected response");
        matches!(result.unwrap_err(), Error::UnexpectedResponse(_));
    }

    #[tokio::test]
    async fn test_historical_schedule() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "106|9000|20230313-09:30:00|20230315-16:00:00|UTC|3|20230313-09:30:00|20230313-16:00:00|20230313|20230314-09:30:00|20230314-16:00:00|20230314|20230315-09:30:00|20230315-16:00:00|20230315|"
                    .to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
        let contract = Contract::stock("AAPL").build();
        let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));
        let duration = Duration::days(3);

        let result = historical_schedule(&client, &contract, end_date, duration).await;
        assert!(result.is_ok(), "historical_schedule should succeed");

        let schedule = result.unwrap();
        assert_eq!(schedule.time_zone, "UTC", "Wrong time zone");
        // Check that we have sessions
        assert!(!schedule.sessions.is_empty(), "Should have at least 1 session");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request.fields[0], OutgoingMessages::RequestHistoricalData.to_field(), "message.type");
        assert_eq!(request.fields[1], "9000", "message.request_id"); // request_id will be generated
                                                                     // The rest of the fields follow the same pattern as historical data request
    }

    #[tokio::test]
    async fn test_tick_subscription_methods() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // HistoricalTickBidAsk = 97
                // First response with 2 ticks, not done
                // Format: message_type|request_id|num_ticks|timestamp|mask|bid|ask|bid_size|ask_size|...|done
                "97|9000|2|1678838400|10|185.50|186.00|100|200|1678838401|11|185.55|186.05|105|205|0|".to_owned(),
                // Second response with 1 tick, done
                "97|9000|1|1678838500|10|185.75|186.25|150|250|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let mut subscription = historical_ticks_bid_ask(&client, &contract, None, None, 3, TradingHours::Regular, false)
            .await
            .expect("Failed to create tick subscription");

        // Get first tick
        let tick1 = subscription.next().await;
        assert!(tick1.is_some(), "Should receive first tick");
        let tick1 = tick1.unwrap();
        assert_eq!(tick1.price_bid, 185.50, "Wrong bid price for first tick");
        assert_eq!(tick1.price_ask, 186.00, "Wrong ask price for first tick");
        assert_eq!(tick1.size_bid, 100, "Wrong bid size for first tick");
        assert_eq!(tick1.size_ask, 200, "Wrong ask size for first tick");
        assert!(tick1.tick_attribute_bid_ask.bid_past_low, "Wrong bid past low for first tick");
        assert!(!tick1.tick_attribute_bid_ask.ask_past_high, "Wrong ask past high for first tick");

        // Get second tick
        let tick2 = subscription.next().await;
        assert!(tick2.is_some(), "Should receive second tick");
        let tick2 = tick2.unwrap();
        assert_eq!(tick2.price_bid, 185.55, "Wrong bid price for second tick");
        assert_eq!(tick2.price_ask, 186.05, "Wrong ask price for second tick");

        // Get third tick
        let tick3 = subscription.next().await;
        assert!(tick3.is_some(), "Should receive third tick");
        let tick3 = tick3.unwrap();
        assert_eq!(tick3.price_bid, 185.75, "Wrong bid price for third tick");

        // Should be done now
        let tick4 = subscription.next().await;
        assert!(tick4.is_none(), "Should not receive more ticks after done");
    }

    #[tokio::test]
    async fn test_tick_subscription_buffer_and_iteration() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // HistoricalTickBidAsk = 97
                // Response with 3 ticks at once, done = true
                "97|9000|3|1678838400|8|185.50|186.00|100|200|1678838401|9|185.60|186.10|110|210|1678838402|10|185.70|186.20|120|220|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let mut subscription = historical_ticks_bid_ask(&client, &contract, None, None, 3, TradingHours::Regular, false)
            .await
            .expect("Failed to create tick subscription");

        // Should receive all 3 ticks from buffer
        let mut ticks = Vec::new();
        while let Some(tick) = subscription.next().await {
            ticks.push(tick);
        }

        assert_eq!(ticks.len(), 3, "Should receive exactly 3 ticks");
        assert_eq!(ticks[0].price_bid, 185.50, "Wrong bid price for first tick");
        assert_eq!(ticks[1].price_bid, 185.60, "Wrong bid price for second tick");
        assert_eq!(ticks[2].price_bid, 185.70, "Wrong bid price for third tick");
    }

    #[tokio::test]
    async fn test_tick_subscription_bid_ask() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // HistoricalTickBidAsk = 97
                // mask = 2 (binary 10) = bid_past_low = true, ask_past_high = false
                "97|9000|1|1678838400|2|185.50|186.00|100|200|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let start = Some(datetime!(2023-03-15 09:00:00 UTC));
        let end = Some(datetime!(2023-03-15 10:00:00 UTC));
        let number_of_ticks = 1;
        let trading_hours = TradingHours::Regular;
        let ignore_size = false;

        let mut subscription = historical_ticks_bid_ask(&client, &contract, start, end, number_of_ticks, trading_hours, ignore_size)
            .await
            .expect("Failed to create bid/ask tick subscription");

        let tick = subscription.next().await.expect("Should receive a tick");
        assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
        assert_eq!(tick.price_bid, 185.50, "Wrong bid price");
        assert_eq!(tick.price_ask, 186.00, "Wrong ask price");
        assert_eq!(tick.size_bid, 100, "Wrong bid size");
        assert_eq!(tick.size_ask, 200, "Wrong ask size");
        assert!(tick.tick_attribute_bid_ask.bid_past_low, "Wrong bid past low");
        assert!(!tick.tick_attribute_bid_ask.ask_past_high, "Wrong ask past high");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request.fields[0], OutgoingMessages::RequestHistoricalTicks.to_field(), "message.type");
        assert_eq!(request.fields[1], "9000", "message.request_id");
        assert_eq!(request.fields[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(request.fields[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(request.fields[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            request.fields[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(request.fields[6], contract.strike.to_field(), "message.strike");
        assert_eq!(request.fields[7], contract.right, "message.right");
        assert_eq!(request.fields[8], contract.multiplier, "message.multiplier");
        assert_eq!(request.fields[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(request.fields[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(request.fields[11], contract.currency.to_field(), "message.currency");
        assert_eq!(request.fields[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(request.fields[13], contract.trading_class, "message.trading_class");
        assert_eq!(request.fields[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(request.fields[15], start.to_field(), "message.start");
        assert_eq!(request.fields[16], end.to_field(), "message.end");
        assert_eq!(request.fields[17], number_of_ticks.to_field(), "message.number_of_ticks");
        assert_eq!(request.fields[18], "BID_ASK", "message.what_to_show");
        assert_eq!(request.fields[19], trading_hours.use_rth().to_field(), "message.use_rth");
        assert_eq!(request.fields[20], "0", "message.ignore_size"); // false = 0
        assert_eq!(request.fields[21], "", "message.misc_options");
    }

    #[tokio::test]
    async fn test_tick_subscription_midpoint() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // HistoricalTick = 96 (for midpoint)
                // Format: message_type|request_id|num_ticks|timestamp|skip|price|size|...|done
                "96|9000|1|1678838400|0|185.75|100|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let mut subscription = historical_ticks_mid_point(&client, &contract, None, None, 1, TradingHours::Regular)
            .await
            .expect("Failed to create midpoint tick subscription");

        let tick = subscription.next().await.expect("Should receive a tick");
        assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
        assert_eq!(tick.price, 185.75, "Wrong midpoint price");
        assert_eq!(tick.size, 100, "Wrong size");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        let request = &request_messages[0];
        assert_eq!(request.fields[18], "MIDPOINT", "message.what_to_show");
    }

    #[tokio::test]
    async fn test_historical_ticks_trade() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // HistoricalTickLast = 98
                // Format: message_type|request_id|num_ticks|timestamp|mask|price|size|exchange|conditions|...|done
                "98|9000|1|1678838400|0|185.50|100|ISLAND|APR|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };

        let mut subscription = historical_ticks_trade(&client, &contract, None, None, 1, TradingHours::Regular)
            .await
            .expect("Failed to create trade tick subscription");

        let tick = subscription.next().await.expect("Should receive a tick");
        assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
        assert_eq!(tick.price, 185.50, "Wrong trade price");
        assert_eq!(tick.size, 100, "Wrong trade size");
        assert_eq!(tick.exchange, "ISLAND", "Wrong exchange");
        assert_eq!(tick.special_conditions, "APR", "Wrong special conditions");
        assert!(!tick.tick_attribute_last.past_limit, "Wrong past limit");
        assert!(!tick.tick_attribute_last.unreported, "Wrong unreported");

        // Verify request message
        let request_messages = message_bus.request_messages.read().unwrap();
        let request = &request_messages[0];
        assert_eq!(request.fields[18], "TRADES", "message.what_to_show");
    }

    #[tokio::test]
    async fn test_historical_data_time_zone_handling() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        // Set client timezone to Eastern
        client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let result = historical_data(
            &client,
            &contract,
            None,
            Duration::seconds(3600),
            BarSize::Hour,
            None,
            TradingHours::Regular,
        )
        .await;

        assert!(result.is_ok(), "historical_data should succeed with timezone");
        let data = result.unwrap();
        assert_eq!(data.bars.len(), 1, "Should receive 1 bar");

        // The timestamp should be parsed in the client's timezone
        // 1678886400 = 2023-03-15 12:00:00 UTC = 2023-03-15 08:00:00 EDT
        let bar = &data.bars[0];
        assert_eq!(bar.date.unix_timestamp(), 1678886400, "Timestamp should match");
    }

    #[tokio::test]
    async fn test_time_zone_fallback() {
        let mut client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::SIZE_RULES);
        // Client without timezone set
        client.time_zone = None;

        let tz = time_zone(&client);
        assert_eq!(tz, time_tz::timezones::db::UTC, "Should fallback to UTC when timezone not set");
    }

    #[tokio::test]
    async fn test_historical_data_streaming_with_updates() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Initial historical data (message type 17)
                "17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned(),
                // Streaming update (message type 90)
                "90|9000|-1|1678890000|185.80|186.10|185.60|185.90|500|185.85|50|".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let mut subscription = historical_data_streaming(
            &client,
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .await
        .expect("streaming request should succeed");

        // First: receive initial historical data
        let update1 = subscription.next().await;
        assert!(update1.is_some(), "Should receive initial historical data");
        match update1.unwrap() {
            HistoricalBarUpdate::Historical(data) => {
                assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
                assert_eq!(data.bars[0].open, 185.50, "Wrong open price");
            }
            _ => panic!("Expected Historical variant"),
        }

        // Second: receive streaming update
        let update2 = subscription.next().await;
        assert!(update2.is_some(), "Should receive streaming update");
        match update2.unwrap() {
            HistoricalBarUpdate::Update(bar) => {
                assert_eq!(bar.open, 185.80, "Wrong open price in update");
                assert_eq!(bar.high, 186.10, "Wrong high price in update");
                assert_eq!(bar.close, 185.90, "Wrong close price in update");
            }
            _ => panic!("Expected Update variant"),
        }

        // Verify request message includes keepUpToDate=true
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request");
        // keepUpToDate is at field index 21 (for non-bag contracts)
        assert_eq!(request_messages[0].fields[21], "1", "Request should have keepUpToDate=true at field[21]");
    }

    #[tokio::test]
    async fn test_historical_data_streaming_keep_up_to_date_false() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Initial historical data only
                "17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let mut subscription = historical_data_streaming(
            &client,
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            false, // keep_up_to_date = false
        )
        .await
        .expect("streaming request should succeed");

        // Receive initial historical data
        let update1 = subscription.next().await;
        assert!(update1.is_some(), "Should receive initial historical data");
        match update1.unwrap() {
            HistoricalBarUpdate::Historical(data) => {
                assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
            }
            _ => panic!("Expected Historical variant"),
        }

        // Verify request message includes keepUpToDate=false
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Should send one request");
        // keepUpToDate is at field index 21 (for non-bag contracts)
        assert_eq!(request_messages[0].fields[21], "0", "Request should have keepUpToDate=false at field[21]");
    }

    #[tokio::test]
    async fn test_historical_data_streaming_error_response() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Error response
                "4|2|9000|162|Historical Market Data Service error message:No market data permissions.|".to_owned(),
            ],
        });

        let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        client.time_zone = Some(time_tz::timezones::db::UTC);

        let contract = Contract::stock("SPY").build();

        let mut subscription = historical_data_streaming(
            &client,
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .await
        .expect("streaming request should succeed");

        // Should return None due to error
        let update = subscription.next().await;
        assert!(update.is_none(), "Should return None on error");

        // Error should be accessible
        let error = subscription.error();
        assert!(error.is_some(), "Error should be stored");
        assert!(
            error.unwrap().to_string().contains("No market data permissions"),
            "Error should contain the message"
        );
    }
}
