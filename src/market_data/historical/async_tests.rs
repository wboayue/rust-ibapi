use super::*;
use crate::common::test_utils::helpers::{
    assert_proto_msg_id, assert_request, assert_request_msg_id, count_proto_msgs, proto_response, request_message_count, TEST_REQ_ID_FIRST,
};
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::market_data::historical::BarTimestamp;
use crate::market_data::historical::TickLast;
use crate::market_data::IgnoreSize;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::protocol::{Features, ProtocolFeature};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::common::RoutedItem;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::market_data::{
    head_timestamp_request, head_timestamp_response, histogram_data_request, histogram_data_response, histogram_entry, historical_data_bar,
    historical_data_daily_bar, historical_data_end_response, historical_data_request, historical_data_response, historical_data_update_response,
    historical_schedule_response, historical_session, historical_tick_bid_ask, historical_tick_last, historical_tick_mid,
    historical_ticks_bid_ask_response, historical_ticks_last_response, historical_ticks_request, historical_ticks_response,
};
use crate::testdata::builders::ResponseProtoEncoder;
use futures::StreamExt;
use std::sync::Arc;
use std::sync::RwLock;
use time::macros::{date, datetime};

fn test_contract() -> Contract {
    Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    }
}

// Pins server_version one below `feature.min_version` and asserts the call fails
// with the feature name in the error. Custom helper (vs. `.expect_err`) because
// subscription return types don't implement Debug.
async fn assert_version_check_fails<F, Fut, T>(feature: ProtocolFeature, call: F)
where
    F: FnOnce(Client) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, feature.min_version - 1);
    let Err(err) = call(client).await else {
        panic!("expected version-check failure ({})", feature.name);
    };
    assert!(err.to_string().contains(feature.name), "expected '{}', got: {err}", feature.name);
}

// Feeds a single wire-format response, asserts the call fails with
// `Error::UnexpectedResponse`. Pairs with `assert_version_check_fails`.
async fn assert_unexpected_response<F, Fut, T>(server_version: i32, response: &str, call: F)
where
    F: FnOnce(Client) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![response.to_owned()]));
    let client = Client::stubbed(message_bus, server_version);
    let Err(err) = call(client).await else {
        panic!("expected UnexpectedResponse failure");
    };
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got: {err:?}");
}

#[tokio::test]
async fn test_head_timestamp() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HeadTimestamp,
        head_timestamp_response().unix_timestamp(1_678_838_400).encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let result = client.head_timestamp(&contract, what_to_show, trading_hours).await;
    assert!(result.is_ok(), "head_timestamp should succeed");

    let timestamp = result.unwrap();
    assert_eq!(timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &head_timestamp_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .what_to_show(what_to_show)
            .use_rth(trading_hours.use_rth()),
    );
}

#[tokio::test]
async fn test_histogram_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(185.50, 100))
            .entry(histogram_entry(185.75, 150))
            .entry(histogram_entry(186.00, 200))
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();
    let trading_hours = TradingHours::Regular;
    let period = BarSize::Day;

    let result = client.histogram_data(&contract, trading_hours, period).await;
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

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &histogram_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .use_rth(trading_hours.use_rth())
            .period(period),
    );
}

#[tokio::test]
async fn test_historical_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_678_886_400)
                        .ohlc(185.50, 186.00, 185.25, 185.75)
                        .volume(1000.0)
                        .wap(185.70)
                        .count(100),
                )
                .bar(
                    historical_data_daily_bar("20230315")
                        .ohlc(185.75, 186.25, 185.50, 186.00)
                        .volume(1500.0)
                        .wap(185.85)
                        .count(150),
                )
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230315 09:30:00 UTC")
                .end_date_str("20230315 10:30:00 UTC")
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);

    let contract = test_contract();
    let end_date = datetime!(2023-03-15 16:00:00 UTC);
    let duration = Duration::seconds(3600);
    let bar_size = BarSize::Min30;
    let what_to_show = WhatToShow::Trades;

    let result = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .duration(duration)
        .ending(end_date)
        .fetch()
        .await;
    assert!(result.is_ok(), "historical_data should succeed");

    let data = result.unwrap();
    assert_eq!(data.bars.len(), 2, "Should receive 2 bars");

    // Verify first bar
    let bar = &data.bars[0];
    // 1678886400 = 2023-03-15 13:20:00 UTC
    assert_eq!(
        bar.date,
        BarTimestamp::DateTime(datetime!(2023-03-15 13:20:00 UTC)),
        "Wrong date for first bar"
    );
    assert_eq!(bar.open, 185.50, "Wrong open for first bar");
    assert_eq!(bar.high, 186.00, "Wrong high for first bar");
    assert_eq!(bar.low, 185.25, "Wrong low for first bar");
    assert_eq!(bar.close, 185.75, "Wrong close for first bar");
    assert_eq!(bar.volume, 1000.0, "Wrong volume for first bar");
    assert_eq!(bar.wap, 185.70, "Wrong WAP for first bar");
    assert_eq!(bar.count, 100, "Wrong count for first bar");

    // Verify second bar (daily bar — YYYYMMDD wire format)
    let bar = &data.bars[1];
    assert_eq!(bar.date, BarTimestamp::Date(date!(2023 - 03 - 15)), "Wrong date for second bar");
    assert_eq!(bar.open, 185.75, "Wrong open for second bar");
    assert_eq!(bar.high, 186.25, "Wrong high for second bar");
    assert_eq!(bar.low, 185.50, "Wrong low for second bar");
    assert_eq!(bar.close, 186.00, "Wrong close for second bar");
    assert_eq!(bar.volume, 1500.0, "Wrong volume for second bar");
    assert_eq!(bar.wap, 185.85, "Wrong WAP for second bar");
    assert_eq!(bar.count, 150, "Wrong count for second bar");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .end_date(Some(end_date))
            .duration(duration)
            .bar_size(bar_size)
            .what_to_show(Some(what_to_show))
            .use_rth(true),
    );
}

#[tokio::test]
async fn test_historical_data_version_check() {
    let mut contract = test_contract();
    contract.trading_class = "ES".to_owned();
    assert_version_check_fails(Features::TRADING_CLASS, |c| async move {
        c.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).fetch().await
    })
    .await;
}

#[tokio::test]
async fn test_historical_data_adjusted_last_validation() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("AAPL").build();
    let end_date = datetime!(2023-03-15 16:00:00 UTC);

    let result = client
        .historical_data(&contract, BarSize::Day)
        .what_to_show(WhatToShow::AdjustedLast)
        .duration(Duration::days(1))
        .ending(end_date)
        .fetch()
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
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = test_contract();

    let result = client.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).fetch().await;
    assert!(result.is_err(), "Should fail with error response");
    assert!(
        result.unwrap_err().to_string().contains("No market data permissions"),
        "Error should contain the error message"
    );
}

#[tokio::test]
async fn test_historical_data_unexpected_response() {
    // 1 = TickPrice — wrong type for historical_data.
    assert_unexpected_response(server_versions::SIZE_RULES, "1|2|9000|1|185.50|100|7|", |c| async move {
        c.historical_data(&test_contract(), BarSize::Hour)
            .duration(Duration::days(1))
            .fetch()
            .await
    })
    .await;
}

#[tokio::test]
async fn test_historical_schedules() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalSchedule,
        historical_schedule_response()
            .start_date_time("20230313-09:30:00")
            .end_date_time("20230315-16:00:00")
            .time_zone("UTC")
            .sessions(vec![
                historical_session("20230313-09:30:00", "20230313-16:00:00", "20230313"),
                historical_session("20230314-09:30:00", "20230314-16:00:00", "20230314"),
                historical_session("20230315-09:30:00", "20230315-16:00:00", "20230315"),
            ])
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = Contract::stock("AAPL").build();
    let end_date = datetime!(2023-03-15 16:00:00 UTC);
    let duration = Duration::days(3);

    let result = client.historical_schedules(&contract, duration).ending(end_date).fetch().await;
    assert!(result.is_ok(), "historical_schedules should succeed");

    let schedule = result.unwrap();
    assert_eq!(schedule.time_zone, "UTC", "Wrong time zone");
    // Check that we have sessions
    assert!(!schedule.sessions.is_empty(), "Should have at least 1 session");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .end_date(Some(end_date))
            .duration(duration)
            .bar_size(BarSize::Day)
            .what_to_show(Some(WhatToShow::Schedule))
            .use_rth(true),
    );
}

#[tokio::test]
async fn test_tick_subscription_methods() {
    // mask = 10 (binary 1010) → bid_past_low=true,  ask_past_high=false
    // mask = 11 (binary 1011) → bid_past_low=true,  ask_past_high=true
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalTickBidAsk,
            historical_ticks_bid_ask_response()
                .tick(historical_tick_bid_ask(1_678_838_400, 185.50, 186.00, 100, 200).bid_past_low(true))
                .tick(
                    historical_tick_bid_ask(1_678_838_401, 185.55, 186.05, 105, 205)
                        .bid_past_low(true)
                        .ask_past_high(true),
                )
                .done(false)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalTickBidAsk,
            historical_ticks_bid_ask_response()
                .tick(historical_tick_bid_ask(1_678_838_500, 185.75, 186.25, 150, 250).bid_past_low(true))
                .done(true)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();

    let mut subscription = client
        .historical_ticks(&contract, 3)
        .bid_ask(IgnoreSize::No)
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
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(historical_tick_bid_ask(1_678_838_400, 185.50, 186.00, 100, 200))
            .tick(historical_tick_bid_ask(1_678_838_401, 185.60, 186.10, 110, 210))
            .tick(historical_tick_bid_ask(1_678_838_402, 185.70, 186.20, 120, 220))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();

    let mut subscription = client
        .historical_ticks(&contract, 3)
        .bid_ask(IgnoreSize::No)
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
    // bid_past_low = true, ask_past_high = false
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(historical_tick_bid_ask(1_678_838_400, 185.50, 186.00, 100, 200).bid_past_low(true))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();
    let start = datetime!(2023-03-15 09:00:00 UTC);
    let end = datetime!(2023-03-15 10:00:00 UTC);
    let number_of_ticks = 1;
    let trading_hours = TradingHours::Regular;

    let mut subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .starting(start)
        .ending(end)
        .trading_hours(trading_hours)
        .bid_ask(IgnoreSize::No)
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

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .start(Some(start))
            .end(Some(end))
            .number_of_ticks(number_of_ticks)
            .what_to_show(WhatToShow::BidAsk)
            .use_rth(trading_hours.use_rth())
            .ignore_size(false),
    );
}

#[tokio::test]
async fn test_tick_subscription_midpoint() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTick,
        historical_ticks_response()
            .tick(historical_tick_mid(1_678_838_400, 185.75, 100))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();

    let mut subscription = client
        .historical_ticks(&contract, 1)
        .mid_point()
        .await
        .expect("Failed to create midpoint tick subscription");

    let tick = subscription.next().await.expect("Should receive a tick");
    assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
    assert_eq!(tick.price, 185.75, "Wrong midpoint price");
    assert_eq!(tick.size, 100, "Wrong size");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .number_of_ticks(1)
            .what_to_show(WhatToShow::MidPoint)
            .use_rth(true),
    );
}

#[tokio::test]
async fn test_historical_ticks_trade() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(historical_tick_last(1_678_838_400, 185.50, 100, "ISLAND").special_conditions("APR"))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
    let contract = test_contract();

    let mut subscription = client
        .historical_ticks(&contract, 1)
        .trade()
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

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .number_of_ticks(1)
            .what_to_show(WhatToShow::Trades)
            .use_rth(true),
    );
}

#[tokio::test]
async fn test_historical_data_time_zone_handling() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_678_886_400)
                        .ohlc(185.50, 186.00, 185.25, 185.75)
                        .volume(1000.0)
                        .wap(185.70)
                        .count(100),
                )
                .encode_proto(),
        ),
        proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_HISTORICAL_DATA);

    let contract = test_contract();
    let result = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::seconds(3600))
        .fetch()
        .await;

    assert!(result.is_ok(), "historical_data should succeed with timezone");
    let data = result.unwrap();
    assert_eq!(data.bars.len(), 1, "Should receive 1 bar");

    let bar = &data.bars[0];
    assert_eq!(
        bar.date,
        BarTimestamp::DateTime(datetime!(2023-03-15 13:20:00 UTC)),
        "Timestamp should match"
    );
}

#[tokio::test]
async fn test_historical_data_streaming_with_updates() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_678_886_400)
                        .ohlc(185.50, 186.00, 185.25, 185.75)
                        .volume(1000.0)
                        .wap(185.70)
                        .count(100),
                )
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataUpdate,
            historical_data_update_response()
                .bar(
                    historical_data_bar(1_678_890_000)
                        .ohlc(185.80, 186.10, 185.60, 185.90)
                        .volume(500.0)
                        .wap(185.85)
                        .count(50),
                )
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .stream()
        .await
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let Some(Ok(SubscriptionItem::Data(HistoricalBarUpdate::Historical(data)))) = subscription.next().await else {
        panic!("Expected Historical variant");
    };
    assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
    assert_eq!(data.bars[0].open, 185.50, "Wrong open price");

    // Second: receive streaming update
    let Some(Ok(SubscriptionItem::Data(HistoricalBarUpdate::Update(bar)))) = subscription.next().await else {
        panic!("Expected Update variant");
    };
    assert_eq!(bar.open, 185.80, "Wrong open price in update");
    assert_eq!(bar.high, 186.10, "Wrong high price in update");
    assert_eq!(bar.close, 185.90, "Wrong close price in update");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .duration(Duration::days(1))
            .bar_size(BarSize::Hour)
            .what_to_show(Some(WhatToShow::Trades))
            .use_rth(true)
            .keep_up_to_date(true),
    );
}

#[tokio::test]
async fn test_historical_data_streaming_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Error response
            "4|2|9000|162|Historical Market Data Service error message:No market data permissions.|".to_owned(),
        ],
        ordered_responses: vec![],
    });

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .stream()
        .await
        .expect("streaming request should succeed");

    // Should yield Some(Err(_)) — Subscription<T> surfaces errors through next().
    let Some(Err(err)) = subscription.next().await else {
        panic!("error should arrive as Some(Err(_))");
    };
    assert!(err.to_string().contains("No market data permissions"), "Error should contain the message");
}

#[tokio::test]
async fn test_tick_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let _subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9100, message_bus.clone());
        // dropped here, !done so cancel should fire
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 1, "should send cancel message on drop");
    assert_proto_msg_id(&messages[0], OutgoingMessages::CancelHistoricalTicks);
}

#[tokio::test]
async fn test_tick_subscription_explicit_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9101, message_bus.clone());
        subscription.cancel().await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 1, "should send cancel only once");
}

#[tokio::test]
async fn test_tick_subscription_drop_after_done_does_not_cancel() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let mut subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9102, message_bus.clone());
        subscription.done = true;
        // drop with done=true → no cancel
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 0, "completed subscription should not send cancel on drop");
}

#[tokio::test]
async fn test_streaming_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let _subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .stream()
            .await
            .expect("streaming request should succeed");
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send exactly one cancel message on drop"
    );
}

#[tokio::test]
async fn test_streaming_subscription_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .stream()
            .await
            .expect("streaming request should succeed");

        subscription.cancel().await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send cancel only once"
    );
}

#[tokio::test]
async fn test_head_timestamp_version_check() {
    assert_version_check_fails(Features::HEAD_TIMESTAMP, |c| async move {
        c.head_timestamp(&test_contract(), WhatToShow::Trades, TradingHours::Regular).await
    })
    .await;
}

#[tokio::test]
async fn test_head_timestamp_unexpected_response() {
    // 17 = HistoricalData — wrong type for head_timestamp.
    assert_unexpected_response(
        server_versions::BOND_ISSUERID,
        "17|9000|20230315  09:30:00|20230315  10:30:00|0|",
        |c| async move { c.head_timestamp(&test_contract(), WhatToShow::Trades, TradingHours::Regular).await },
    )
    .await;
}

#[tokio::test]
async fn test_historical_data_with_end_message() {
    // start/end always come on the follow-on HistoricalDataEnd (108) frame at floor 210.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_678_886_400)
                        .ohlc(185.50, 186.00, 185.25, 185.75)
                        .volume(1000.0)
                        .wap(185.70)
                        .count(100),
                )
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230315 09:30:00 UTC")
                .end_date_str("20230315 10:30:00 UTC")
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_HISTORICAL_DATA);

    let data = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await
        .expect("historical_data should succeed");

    assert_eq!(data.bars.len(), 1, "should have one bar");
    assert_eq!(data.start, datetime!(2023-03-15 09:30:00 UTC), "start populated from end-message");
    assert_eq!(data.end, datetime!(2023-03-15 10:30:00 UTC), "end populated from end-message");
}

#[tokio::test]
async fn test_historical_data_connection_reset_after_retries() {
    // Empty responses → broadcast channel closes immediately → subscription.next()
    // returns None on every retry → loop exhausts MAX_RETRIES and returns ConnectionReset.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(matches!(result, Err(Error::ConnectionReset)), "expected ConnectionReset, got {result:?}");
    // Each retry resends the request — MAX_RETRIES = 5.
    assert_eq!(request_message_count(&message_bus), 5, "should retry MAX_RETRIES times");
}

#[tokio::test]
async fn test_historical_schedules_ending_now_version_check() {
    assert_version_check_fails(Features::HISTORICAL_SCHEDULE, |c| async move {
        c.historical_schedules(&test_contract(), Duration::days(1)).fetch().await
    })
    .await;
}

#[tokio::test]
async fn test_historical_schedules_ending_now_trading_class_version_check() {
    // contract.trading_class triggers the earlier TRADING_CLASS gate ahead of the
    // HISTORICAL_SCHEDULE gate — pin below TRADING_CLASS so the former fires first.
    let mut contract = test_contract();
    contract.trading_class = "ES".to_owned();
    assert_version_check_fails(Features::TRADING_CLASS, |c| async move {
        c.historical_schedules(&contract, Duration::days(1)).fetch().await
    })
    .await;
}

#[tokio::test]
async fn test_historical_schedules_unexpected_response() {
    // 17 = HistoricalData — wrong type for the schedule decoder (expects 106).
    assert_unexpected_response(
        server_versions::BOND_ISSUERID,
        "17|9000|20230315  09:30:00|20230315  10:30:00|0|",
        |c| async move { c.historical_schedules(&test_contract(), Duration::days(3)).fetch().await },
    )
    .await;
}

#[tokio::test]
async fn test_historical_ticks_bid_ask_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| async move {
        c.historical_ticks(&test_contract(), 1).bid_ask(IgnoreSize::No).await
    })
    .await;
}

#[tokio::test]
async fn test_historical_ticks_mid_point_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| async move {
        c.historical_ticks(&test_contract(), 1).mid_point().await
    })
    .await;
}

#[tokio::test]
async fn test_historical_ticks_trade_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| async move {
        c.historical_ticks(&test_contract(), 1).trade().await
    })
    .await;
}

#[tokio::test]
async fn test_cancel_historical_ticks() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::CANCEL_CONTRACT_DATA);

    client.cancel_historical_ticks(9000).await.expect("cancel should succeed");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request_msg_id(&message_bus, 0, OutgoingMessages::CancelHistoricalTicks);
}

#[tokio::test]
async fn test_cancel_historical_ticks_version_check() {
    assert_version_check_fails(Features::CANCEL_CONTRACT_DATA, |c| async move { c.cancel_historical_ticks(9000).await }).await;
}

#[tokio::test]
async fn test_histogram_data_version_check() {
    assert_version_check_fails(Features::HISTOGRAM, |c| async move {
        c.histogram_data(&test_contract(), TradingHours::Regular, BarSize::Day).await
    })
    .await;
}

#[tokio::test]
async fn test_historical_data_streaming_trading_class_version_check() {
    let mut contract = test_contract();
    contract.trading_class = "ES".to_owned();
    assert_version_check_fails(Features::TRADING_CLASS, |c| async move {
        c.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).stream().await
    })
    .await;
}

#[tokio::test]
async fn test_tick_subscription_cancel_idempotent() {
    let message_bus = Arc::new(MessageBusStub::default());

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9200, message_bus.clone());
    subscription.cancel().await;
    subscription.cancel().await;

    drop(subscription);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 1, "cancel + cancel + drop should send exactly one message");
}

#[tokio::test]
async fn test_tick_subscription_skips_unexpected_message_then_yields() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        // HistoricalData is unexpected for a tick subscription → should be skipped.
        proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto()),
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                .tick(historical_tick_last(1_678_838_400, 185.50, 100, "ISLAND").special_conditions("APR"))
                .done(true)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_HISTORICAL_DATA);

    let mut subscription = client
        .historical_ticks(&test_contract(), 1)
        .trade()
        .await
        .expect("subscription should be created");

    let tick = subscription.next().await.expect("should receive tick after skipping unexpected");
    assert_eq!(tick.price, 185.50, "wrong price");
    assert!(subscription.next().await.is_none(), "should be done");
}

#[tokio::test]
async fn test_tick_subscription_errors_terminate_stream() {
    let message_bus = Arc::new(MessageBusStub::default());

    let (tx, rx) = tokio::sync::broadcast::channel(16);
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();
    drop(tx);

    let internal = AsyncInternalSubscription::new(rx);
    let mut subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9300, message_bus);

    assert!(subscription.next().await.is_none(), "error on channel terminates next()");
}

#[tokio::test]
async fn test_tick_subscription_returns_none_on_closed_channel() {
    // Empty response_messages closes the broadcast channel immediately; the first
    // fill_buffer sees None and next() returns None.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let mut subscription = client
        .historical_ticks(&test_contract(), 1)
        .mid_point()
        .await
        .expect("subscription should be created");

    assert!(subscription.next().await.is_none(), "closed channel yields None");
}
