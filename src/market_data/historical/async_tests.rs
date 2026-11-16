use super::*;
use crate::common::test_utils::helpers::{
    assert_decimal_parse_error, assert_request, assert_tws_error_message, count_proto_msgs, create_test_client_with_ordered_proto_responses,
    proto_error_response, proto_response, request_message_count, test_notice, TEST_REQ_ID_FIRST,
};
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::market_data::historical::common::test_tables::{HMDS_CONNECTED_MESSAGE, PRE_DATA_NOTICES};
use crate::market_data::historical::BarTimestamp;
use crate::market_data::historical::TickLast;
use crate::market_data::IgnoreSize;
use crate::messages::{IncomingMessages, OutgoingMessages, HMDS_QUERY_MESSAGE_CODE};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::common::RoutedItem;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::market_data::{
    head_timestamp_request, head_timestamp_response, histogram_data_request, histogram_data_response, histogram_entry, historical_data_bar,
    historical_data_daily_bar, historical_data_end_response, historical_data_request, historical_data_response, historical_data_update_response,
    historical_schedule_response, historical_session, historical_ticks_bid_ask_response, historical_ticks_last_response, historical_ticks_request,
    historical_ticks_response, tick_bid_ask, tick_last, tick_midpoint,
};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::transport::AsyncInternalSubscription;
use futures::StreamExt;
use std::sync::Arc;
use time::macros::{date, datetime};

fn test_contract() -> Contract {
    Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: Some("202303".parse().unwrap()),
        ..Contract::default()
    }
}

// Feeds a single response of another message type, asserts the call fails with
// `Error::UnexpectedResponse`. Pairs with `assert_version_check_fails`.
async fn assert_unexpected_response<F, Fut, T>(server_version: i32, response: crate::messages::ResponseMessage, call: F)
where
    F: FnOnce(Client) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![response]));
    let client = Client::stubbed(message_bus, server_version);
    let Err(err) = call(client).await else {
        panic!("expected UnexpectedResponse failure");
    };
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got: {err:?}");
}

// Feeds a single frame of a message type the request doesn't expect: it is
// skipped, the stream closes before the answer, and the request is cancelled
// at TWS with `cancel`.
async fn assert_skips_undeclared<F, Fut, T>(server_version: i32, response: crate::messages::ResponseMessage, cancel: OutgoingMessages, call: F)
where
    F: FnOnce(Client) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![response]));
    let client = Client::stubbed(message_bus.clone(), server_version);
    let Err(err) = call(client).await else {
        panic!("expected UnexpectedEndOfStream");
    };
    assert!(
        matches!(err, Error::UnexpectedEndOfStream),
        "expected UnexpectedEndOfStream, got: {err:?}"
    );
    // Drop spawns its cancel; give it the chance to run.
    tokio::task::yield_now().await;
    let messages = message_bus.request_messages.read().unwrap().clone();
    assert_eq!(count_proto_msgs(&messages, cancel), 1, "no answer, so the request is cancelled");
}

#[tokio::test]
async fn head_timestamp() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HeadTimestamp,
        head_timestamp_response().unix_timestamp(1_678_838_400).encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
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

/// A 165 bound to a one-shot is a warning: the call reads past it to its answer.
#[tokio::test]
async fn head_timestamp_reads_past_hmds_query_notice() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_error_response(
            TEST_REQ_ID_FIRST,
            HMDS_QUERY_MESSAGE_CODE,
            "Historical Market Data Service query message:no items retrieved",
        ),
        proto_response(
            IncomingMessages::HeadTimestamp,
            head_timestamp_response().unix_timestamp(1_678_838_400).encode_proto(),
        ),
    ]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let timestamp = client
        .head_timestamp(&test_contract(), WhatToShow::Trades, TradingHours::Regular)
        .await
        .expect("the 165 must not fail the call");

    assert_eq!(timestamp, datetime!(2023-03-15 00:00:00 UTC));
}

#[tokio::test]
async fn histogram_data_malformed_size_fails_the_request() {
    // End-to-end through the public API: a malformed size must fail the request
    // rather than silently decoding as 0 (issue #716).
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(185.50, 0.0).size_wire(Some("abc")))
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let result = client.histogram_data(&test_contract(), TradingHours::Regular, BarSize::Day).await;
    assert_decimal_parse_error(result, "abc");
}

#[tokio::test]
async fn histogram_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(185.50, 100.0))
            .entry(histogram_entry(185.75, 150.0))
            .entry(histogram_entry(186.00, 200.0))
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
    let contract = test_contract();
    let trading_hours = TradingHours::Regular;
    let period = BarSize::Day;

    let result = client.histogram_data(&contract, trading_hours, period).await;
    assert!(result.is_ok(), "histogram_data should succeed");

    let entries = result.unwrap();
    assert_eq!(entries.len(), 3, "Should receive 3 histogram entries");

    // Verify first entry
    assert_eq!(entries[0].price, 185.50, "Wrong price for first entry");
    assert_eq!(entries[0].size, Some(100.0), "Wrong size for first entry");

    // Verify second entry
    assert_eq!(entries[1].price, 185.75, "Wrong price for second entry");
    assert_eq!(entries[1].size, Some(150.0), "Wrong size for second entry");

    // Verify third entry
    assert_eq!(entries[2].price, 186.00, "Wrong price for third entry");
    assert_eq!(entries[2].size, Some(200.0), "Wrong size for third entry");

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
async fn historical_data() {
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

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

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
    assert_eq!(bar.volume, Some(1000.0), "Wrong volume for first bar");
    assert_eq!(bar.wap, Some(185.70), "Wrong WAP for first bar");
    assert_eq!(bar.count, Some(100), "Wrong count for first bar");

    // Verify second bar (daily bar — YYYYMMDD wire format)
    let bar = &data.bars[1];
    assert_eq!(bar.date, BarTimestamp::Date(date!(2023 - 03 - 15)), "Wrong date for second bar");
    assert_eq!(bar.open, 185.75, "Wrong open for second bar");
    assert_eq!(bar.high, 186.25, "Wrong high for second bar");
    assert_eq!(bar.low, 185.50, "Wrong low for second bar");
    assert_eq!(bar.close, 186.00, "Wrong close for second bar");
    assert_eq!(bar.volume, Some(1500.0), "Wrong volume for second bar");
    assert_eq!(bar.wap, Some(185.85), "Wrong WAP for second bar");
    assert_eq!(bar.count, Some(150), "Wrong count for second bar");

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
            .what_to_show(what_to_show)
            .use_rth(true),
    );
}

#[tokio::test]
async fn historical_data_adjusted_last_validation() {
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
async fn historical_data_error_response() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    let contract = test_contract();

    let result = client.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).fetch().await;
    assert!(result.is_err(), "Should fail with error response");
    assert!(
        result.unwrap_err().to_string().contains("No market data permissions"),
        "Error should contain the error message"
    );
    // TWS ended the query with its error; a cancel would draw 366.
    assert_eq!(historical_requests_and_cancels(&message_bus).await, (1, 0));
}

/// TWS sends these on the error callback and then delivers the bars: 2188 (the
/// account may not stream the up-to-the-second tail, but the historical data
/// itself is served) and 165 "HMDS server connection was successful" (the farm
/// connected on demand, #1136). Classified as an error either would end the
/// request before `HistoricalData` arrived, and the caller would see no bars.
#[tokio::test]
async fn historical_data_notice_before_bars_is_skipped() {
    for (code, message) in PRE_DATA_NOTICES {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
            proto_error_response(9000, code, message),
            proto_response(
                IncomingMessages::HistoricalData,
                historical_data_response()
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
                    .end_date_str("20230315 16:00:00 UTC")
                    .encode_proto(),
            ),
        ]));

        let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

        let result = client
            .historical_data(&test_contract(), BarSize::Day)
            .duration(Duration::days(1))
            .ending(datetime!(2023-03-15 16:00:00 UTC))
            .fetch()
            .await;

        let data = result.unwrap_or_else(|e| panic!("{code} must not abort the request: {e:?}"));
        assert_eq!(data.bars.len(), 1, "bars sent after {code} should be returned");
    }
}

#[tokio::test]
async fn historical_data_unexpected_response() {
    // The window frame ahead of the bars is out of order.
    assert_unexpected_response(
        server_versions::SIZE_RULES,
        proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto()),
        |c| async move {
            c.historical_data(&test_contract(), BarSize::Hour)
                .duration(Duration::days(1))
                .fetch()
                .await
        },
    )
    .await;
}

#[tokio::test]
async fn historical_schedules() {
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

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
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
            .what_to_show(WhatToShow::Schedule)
            .use_rth(true),
    );
}

#[tokio::test]
async fn historical_data_time_zone_handling() {
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

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

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
async fn historical_data_streaming_with_updates() {
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

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .await
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(bars)))) = subscription.next().await else {
        panic!("Expected Bars variant");
    };
    assert_eq!(bars.len(), 1, "Should have 1 initial bar");
    assert_eq!(bars[0].open, 185.50, "Wrong open price");

    // Second: receive streaming update
    let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(bar)))) = subscription.next().await else {
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
            .what_to_show(WhatToShow::Trades)
            .use_rth(true)
            .keep_up_to_date(true),
    );
}

// Async twin of the sync `historical_data_streaming_error_response`; see
// that test for why this path is covered per API rather than only at the transport.
#[tokio::test]
async fn historical_data_streaming_error_response() {
    let (client, _bus) = create_test_client_with_ordered_proto_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]);
    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .await
        .expect("streaming request should succeed");

    let Some(Err(err)) = subscription.next().await else {
        panic!("error should arrive as Some(Err(_))");
    };
    assert_tws_error_message(err, 162, "No market data permissions");
}

/// 165 "HMDS server connection was successful" ahead of the bars is a notice:
/// the stream yields it and keeps going (#1136).
#[tokio::test]
async fn historical_data_streaming_hmds_connected_notice_then_data() {
    let (client, _bus) = create_test_client_with_ordered_proto_responses(vec![
        proto_error_response(9000, HMDS_QUERY_MESSAGE_CODE, HMDS_CONNECTED_MESSAGE),
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(historical_data_bar(1_678_886_400).ohlc(185.50, 186.00, 185.25, 185.75))
                .encode_proto(),
        ),
    ]);

    let mut subscription = client
        .historical_data(&Contract::stock("SPY").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .await
        .expect("streaming request should succeed");

    let Some(Ok(SubscriptionItem::Notice(notice))) = subscription.next().await else {
        panic!("165 should arrive as a notice");
    };
    assert_eq!((notice.code, notice.message.as_str()), (HMDS_QUERY_MESSAGE_CODE, HMDS_CONNECTED_MESSAGE));
    let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(bars)))) = subscription.next().await else {
        panic!("bars should follow the notice");
    };
    assert_eq!(bars.len(), 1);
}

#[tokio::test]
async fn streaming_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub::default());

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let _subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .subscribe_with_updates()
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

/// The historical data requests and cancels written to `bus`, once the
/// cancels that drops spawn have run.
async fn historical_requests_and_cancels(bus: &MessageBusStub) -> (usize, usize) {
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let messages = bus.request_messages.read().unwrap();
    (
        count_proto_msgs(&messages, OutgoingMessages::RequestHistoricalData),
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
    )
}

/// Dropping the `fetch` future before the window frame cancels the request at TWS.
#[tokio::test]
async fn historical_data_dropped_fetch_cancels() {
    let bus = Arc::new(MessageBusStub::default().keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = test_contract();
    let fetch = client.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).fetch();
    let elapsed = tokio::time::timeout(tokio::time::Duration::from_millis(20), fetch).await;

    assert!(elapsed.is_err(), "the stub never answers");
    assert_eq!(historical_requests_and_cancels(&bus).await, (1, 1));
}

/// `one_shot_timeout` bounds both reads: bars, then silence, times out
/// and cancels the request at TWS.
#[tokio::test]
async fn historical_data_one_shot_timeout_cancels_between_frames() {
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![bars]).keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3).with_one_shot_timeout(std::time::Duration::from_millis(20));

    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert_eq!(historical_requests_and_cancels(&bus).await, (1, 1));
}

/// A reconnect that outlasts `one_shot_timeout` ends the fetch without a resend.
#[tokio::test]
async fn historical_data_one_shot_timeout_skips_resend_past_deadline() {
    // Far past the 20 ms `one_shot_timeout`: a fetch that waits it out has
    // ignored its deadline.
    const RECONNECT: std::time::Duration = std::time::Duration::from_secs(10);
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let end = proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto());
    let bus = Arc::new(
        MessageBusStub::with_ordered_responses(vec![bars, end])
            .with_connection_resets(1)
            .with_reconnect_delay(RECONNECT),
    );
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3).with_one_shot_timeout(std::time::Duration::from_millis(20));

    let start = std::time::Instant::now();
    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert!(start.elapsed() < RECONNECT / 2, "the reconnect wait must end at the deadline");
    assert_eq!(historical_requests_and_cancels(&bus).await.0, 1, "no resend past the deadline");
}

/// The window frame ends the request at TWS: nothing to cancel.
#[tokio::test]
async fn historical_data_completed_fetch_sends_no_cancel() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(historical_data_bar(1_678_886_400).ohlc(185.50, 186.00, 185.25, 185.75))
                .encode_proto(),
        ),
        proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto()),
    ]));
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let data = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await
        .expect("completed request");

    assert_eq!(data.bars.len(), 1);
    assert_eq!(historical_requests_and_cancels(&bus).await, (1, 0));
}

/// A second batch of bars in place of the window frame cancels the request at TWS.
#[tokio::test]
async fn historical_data_out_of_order_frames_cancel() {
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![bars.clone(), bars]));
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{result:?}");
    assert_eq!(historical_requests_and_cancels(&bus).await, (1, 1));
}

/// A decode failure before the window frame cancels the request at TWS.
#[tokio::test]
async fn historical_data_local_error_cancels() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalData,
        vec![0xff],
    )]));
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(result.is_err(), "malformed bars must fail");
    assert_eq!(historical_requests_and_cancels(&bus).await, (1, 1));
}

#[tokio::test]
async fn streaming_subscription_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub::default());

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .subscribe_with_updates()
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
async fn head_timestamp_skips_undeclared_message() {
    // 17 = HistoricalData — wrong type for head_timestamp.
    assert_skips_undeclared(
        server_versions::BOND_ISSUERID,
        proto_response(IncomingMessages::HistoricalData, Vec::new()),
        OutgoingMessages::CancelHeadTimestamp,
        |c| async move { c.head_timestamp(&test_contract(), WhatToShow::Trades, TradingHours::Regular).await },
    )
    .await;
}

#[tokio::test]
async fn historical_data_with_end_message() {
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

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

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
async fn historical_data_end_of_stream_is_not_retried() {
    // Empty responses → broadcast channel closes immediately → subscription.next()
    // yields None. A closed stream is not a reset: it fails on the first attempt.
    // Before #744 this side retried it five times and reported ConnectionReset,
    // while the sync side failed immediately — the two had drifted apart.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await;

    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
    assert_eq!(request_message_count(&message_bus), 1, "a closed stream must not be retried");
}

#[tokio::test]
async fn historical_data_retries_a_connection_reset() {
    // The case the async side never retried: a routed ConnectionReset came back
    // through `Some(Err(e))` and returned straight to the caller.
    let message_bus = Arc::new(
        MessageBusStub::with_ordered_responses(vec![
            proto_response(
                IncomingMessages::HistoricalData,
                historical_data_response()
                    .bar(historical_data_bar(1_678_886_400).ohlc(185.50, 186.00, 185.25, 185.75))
                    .encode_proto(),
            ),
            proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto()),
        ])
        .with_connection_resets(1),
    );
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let data = client
        .historical_data(&test_contract(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .await
        .expect("the reset must be retried, not surfaced");

    assert_eq!(data.bars.len(), 1);
    assert_eq!(request_message_count(&message_bus), 2, "the retry must re-send the request");
}

#[tokio::test]
async fn historical_schedules_skips_undeclared_message() {
    // 17 = HistoricalData — wrong type for the schedule decoder (expects 106).
    assert_skips_undeclared(
        server_versions::BOND_ISSUERID,
        proto_response(IncomingMessages::HistoricalData, Vec::new()),
        OutgoingMessages::CancelHistoricalData,
        |c| async move { c.historical_schedules(&test_contract(), Duration::days(3)).fetch().await },
    )
    .await;
}

#[tokio::test]
async fn next_frame_fails_on_lag() {
    // A lag may have evicted the bars frame; skipping the gap would read the
    // window frame in its place (#1090).
    let (tx, rx) = tokio::sync::broadcast::channel(2);
    for code in [2100, 2101, 2102] {
        tx.send(RoutedItem::Notice(test_notice(code, "n"))).unwrap();
    }
    drop(tx);

    let mut subscription: Subscription<HistoricalDataFrame> = Subscription::new_from_internal(
        AsyncInternalSubscription::new(rx),
        Arc::new(MessageBusStub::default()),
        Some(9000),
        None,
        crate::subscriptions::DecoderContext::default(),
    );

    match request_helpers::next_frame(&mut subscription, None).await {
        Err(Error::Notice(n)) => assert_eq!(n.code, crate::messages::SUBSCRIPTION_LAG_CODE),
        other => panic!("expected lag error, got {other:?}"),
    }
}

/// Fetch one day of daily bars against `responses`.
async fn fetch_daily(responses: Vec<crate::messages::ResponseMessage>) -> (Result<HistoricalData, Error>, Arc<MessageBusStub>) {
    let (client, bus) = create_test_client_with_ordered_proto_responses(responses);
    let result = client
        .historical_data(&test_contract(), BarSize::Day)
        .duration(Duration::seconds(86_400))
        .fetch()
        .await;
    (result, bus)
}

/// A decoded batch alone cannot prove a finite request completed.
#[tokio::test]
async fn historical_data_requires_end_after_a_batch() {
    for populated in [false, true] {
        let response = historical_data_response();
        let response = if populated {
            response.bar(historical_data_daily_bar("20230315").ohlc(1.0, 2.0, 0.5, 1.5))
        } else {
            response
        };
        let (result, bus) = fetch_daily(vec![proto_response(IncomingMessages::HistoricalData, response.encode_proto())]).await;
        assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "populated={populated}: {result:?}");
        assert_eq!(request_message_count(&bus), 1, "bare closure must not replay the request");
    }
}

/// A second data frame is not the completion marker.
#[tokio::test]
async fn historical_data_requires_end_message_type() {
    let response = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let (result, bus) = fetch_daily(vec![response.clone(), response]).await;
    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{result:?}");
    assert_eq!(request_message_count(&bus), 1);
}

/// An update frame on a request sent without updates is out of protocol:
/// `fetch` fails rather than treat it as bars or the window, and cancels.
#[tokio::test]
async fn historical_data_update_frame_is_unexpected() {
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let update = proto_response(
        IncomingMessages::HistoricalDataUpdate,
        historical_data_update_response()
            .bar(historical_data_bar(1_678_890_000).ohlc(1.0, 2.0, 0.5, 1.5))
            .encode_proto(),
    );
    for (label, responses) in [("in place of bars", vec![update.clone()]), ("in place of the window", vec![bars, update])] {
        let (result, bus) = fetch_daily(responses).await;
        assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{label}: {result:?}");
        assert_eq!(historical_requests_and_cancels(&bus).await, (1, 1), "{label}");
    }
}

/// An empty result is valid when the broker explicitly completes the request.
#[tokio::test]
async fn historical_data_requires_end_accepts_completed_empty_batch() {
    let (result, bus) = fetch_daily(vec![
        proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto()),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230315 09:30:00 UTC")
                .end_date_str("20230315 10:30:00 UTC")
                .encode_proto(),
        ),
    ])
    .await;
    let result = result.unwrap();
    assert!(result.bars.is_empty());
    assert_eq!(result.start, datetime!(2023-03-15 09:30:00 UTC));
    assert_eq!(result.end, datetime!(2023-03-15 10:30:00 UTC));
    assert_eq!(request_message_count(&bus), 1);
}

// ---- buffer_limit ----------------------------------------------------------

#[tokio::test]
async fn historical_data_stream_buffer_limit_reaches_the_bus() {
    let (client, message_bus) = crate::common::test_utils::helpers::create_test_client();
    let contract = Contract::stock("SPY").build();

    let _bounded = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .buffer_limit(16)
        .subscribe_with_updates()
        .await
        .expect("bounded stream failed");
    let _unbounded = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .await
        .expect("unbounded stream failed");

    assert_eq!(
        *message_bus.buffer_limits.read().unwrap(),
        vec![16],
        "only the bounded request carries a limit"
    );
}

#[tokio::test]
async fn historical_data_fetch_rejects_buffer_limit() {
    let (client, message_bus) = crate::common::test_utils::helpers::create_test_client();
    let contract = Contract::stock("SPY").build();

    let result = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .buffer_limit(16)
        .fetch()
        .await;

    assert!(matches!(result, Err(Error::InvalidArgument(_))), "got {result:?}");
    assert!(message_bus.request_messages.read().unwrap().is_empty(), "nothing is sent");
}

// === Historical ticks: a one-shot read until TWS marks the reply `done` ===

/// The historical ticks requests and cancels written to `bus`, once the
/// cancels that drops spawn have run.
async fn tick_requests_and_cancels(bus: &MessageBusStub) -> (usize, usize) {
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let messages = bus.request_messages.read().unwrap();
    (
        count_proto_msgs(&messages, OutgoingMessages::RequestHistoricalTicks),
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalTicks),
    )
}

/// One trade-tick frame holding one tick at `price`.
fn last_frame(price: f64, done: bool) -> crate::messages::ResponseMessage {
    proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(tick_last(1_681_133_400, price, 24.0, "ARCA").special_conditions("APR"))
            .done(done)
            .encode_proto(),
    )
}

/// Request ten trade ticks against `responses`.
async fn fetch_last(responses: Vec<crate::messages::ResponseMessage>) -> (Result<Vec<TickLast>, Error>, Arc<MessageBusStub>) {
    fetch_last_at(server_versions::CANCEL_CONTRACT_DATA, responses).await
}

async fn fetch_last_at(server_version: i32, responses: Vec<crate::messages::ResponseMessage>) -> (Result<Vec<TickLast>, Error>, Arc<MessageBusStub>) {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(responses));
    let client = Client::stubbed(bus.clone(), server_version);
    let result = client.historical_ticks(&test_contract(), 10).last().await;
    (result, bus)
}

fn prices(ticks: &[TickLast]) -> Vec<f64> {
    ticks.iter().map(|tick| tick.price).collect()
}

#[tokio::test]
async fn historical_ticks_last() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(11.63, true)]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = test_contract();
    let start = datetime!(2023-04-01 09:30:00 UTC);
    let end = datetime!(2023-04-01 16:00:00 UTC);

    let ticks = client
        .historical_ticks(&contract, 10)
        .starting(start)
        .ending(end)
        .trading_hours(TradingHours::Regular)
        .last()
        .await
        .expect("historical ticks request failed");

    assert_eq!(prices(&ticks), [11.63]);
    assert_eq!((ticks[0].exchange.as_str(), ticks[0].special_conditions.as_str()), ("ARCA", "APR"));
    assert_eq!(ticks[0].size, 24.0);
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .start(Some(start))
            .end(Some(end))
            .number_of_ticks(10)
            .what_to_show(WhatToShow::Trades)
            .use_rth(true),
    );
}

#[tokio::test]
async fn historical_ticks_mid_point() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTick,
        historical_ticks_response()
            .tick(tick_midpoint(1_681_133_398, 91.36))
            .tick(tick_midpoint(1_681_133_399, 91.37))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = test_contract();
    let end = datetime!(2023-04-01 16:00:00 UTC);

    let ticks = client
        .historical_ticks(&contract, 10)
        .ending(end)
        .mid_point()
        .await
        .expect("historical ticks request failed");

    assert_eq!(ticks.iter().map(|tick| tick.price).collect::<Vec<_>>(), [91.36, 91.37]);
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .end(Some(end))
            .number_of_ticks(10)
            .what_to_show(WhatToShow::MidPoint)
            .use_rth(true),
    );
}

#[tokio::test]
async fn historical_ticks_bid_ask() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(tick_bid_ask(1_681_133_399, 11.63, 11.83, 2_800.0, 100.0).bid_past_low(true))
            .tick(tick_bid_ask(1_681_133_400, 11.64, 11.84, 2_900.0, 200.0).ask_past_high(true))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = test_contract();
    let start = datetime!(2023-04-01 09:30:00 UTC);

    let ticks = client
        .historical_ticks(&contract, 10)
        .starting(start)
        .bid_ask(IgnoreSize::Yes)
        .await
        .expect("historical ticks request failed");

    assert_eq!(ticks.len(), 2);
    assert_eq!((ticks[0].price_bid, ticks[0].price_ask), (11.63, 11.83));
    assert_eq!((ticks[1].size_bid, ticks[1].size_ask), (2_900.0, 200.0));
    assert_eq!((ticks[0].attributes.bid_past_low, ticks[0].attributes.ask_past_high), (true, false));
    assert_eq!((ticks[1].attributes.bid_past_low, ticks[1].attributes.ask_past_high), (false, true));
    assert_request(
        &message_bus,
        0,
        &historical_ticks_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .start(Some(start))
            .number_of_ticks(10)
            .what_to_show(WhatToShow::BidAsk)
            .use_rth(true)
            .ignore_size(true),
    );
}

/// The reply is read frame by frame until the `done` one, which also ends the
/// request at TWS: no cancel is written.
#[tokio::test]
async fn historical_ticks_reads_frames_until_done() {
    let (result, bus) = fetch_last(vec![last_frame(1.0, false), last_frame(2.0, false), last_frame(3.0, true)]).await;
    assert_eq!(prices(&result.expect("three frames")), [1.0, 2.0, 3.0]);
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 0));
}

/// An empty `done` frame is a complete, empty reply.
#[tokio::test]
async fn historical_ticks_empty_reply() {
    let frame = proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response().done(true).encode_proto(),
    );
    let (result, bus) = fetch_last(vec![frame]).await;
    assert_eq!(result.expect("empty reply"), Vec::<TickLast>::new());
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 0));
}

/// A reply that stops before its `done` frame is never returned as complete,
/// and the request is cancelled at TWS.
#[tokio::test]
async fn historical_ticks_closed_before_done_is_an_error() {
    for responses in [vec![], vec![last_frame(1.0, false)]] {
        let (result, bus) = fetch_last(responses).await;
        assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "{result:?}");
        assert_eq!(tick_requests_and_cancels(&bus).await, (1, 1));
    }
}

/// A server older than the cancel message gets none.
#[tokio::test]
async fn historical_ticks_writes_no_cancel_below_its_server_version() {
    let (result, bus) = fetch_last_at(server_versions::CANCEL_CONTRACT_DATA - 1, vec![last_frame(1.0, false)]).await;
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 0));
}

/// A TWS rejection is the error, with its code.
#[tokio::test]
async fn historical_ticks_rejection_is_the_error() {
    let rejection = proto_error_response(TEST_REQ_ID_FIRST, 10187, "Failed to request historical ticks:Max length exceeded");
    let (result, _bus) = fetch_last(vec![rejection]).await;
    match result {
        Err(Error::Notice(notice)) => assert_eq!(
            (notice.code, notice.message.as_str()),
            (10187, "Failed to request historical ticks:Max length exceeded")
        ),
        other => panic!("expected the 10187 rejection, got {other:?}"),
    }
}

/// Notices TWS sends before the data are skipped, as `fetch()` skips them.
#[tokio::test]
async fn historical_ticks_notice_before_reply_is_skipped() {
    for (code, message) in PRE_DATA_NOTICES {
        let (result, _bus) = fetch_last(vec![proto_error_response(TEST_REQ_ID_FIRST, code, message), last_frame(1.0, true)]).await;
        assert_eq!(prices(&result.unwrap_or_else(|e| panic!("code {code}: {e:?}"))), [1.0], "code {code}");
    }
}

/// A frame of another message type is skipped, not decoded.
#[tokio::test]
async fn historical_ticks_skips_undeclared_message() {
    let other = proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto());
    let (result, _bus) = fetch_last(vec![other, last_frame(1.0, true)]).await;
    assert_eq!(prices(&result.expect("the tick frame")), [1.0]);
}

/// A decode failure before the `done` frame cancels the request at TWS.
#[tokio::test]
async fn historical_ticks_local_error_cancels() {
    let (result, bus) = fetch_last(vec![proto_response(IncomingMessages::HistoricalTickLast, vec![0xff])]).await;
    assert!(matches!(result, Err(Error::ProtobufDecode(_))), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 1));
}

#[tokio::test]
async fn historical_ticks_retries_a_connection_reset() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(1.0, true)]).with_connection_resets(1));
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let ticks = client
        .historical_ticks(&test_contract(), 10)
        .last()
        .await
        .expect("the reset must be retried, not surfaced");

    assert_eq!(prices(&ticks), [1.0]);
    assert_eq!(tick_requests_and_cancels(&bus).await.0, 2, "the retry must re-send the request");
}

/// `one_shot_timeout` bounds every read: a frame, then silence, times out and
/// cancels the request at TWS.
#[tokio::test]
async fn historical_ticks_one_shot_timeout_cancels_between_frames() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(1.0, false)]).keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA).with_one_shot_timeout(std::time::Duration::from_millis(20));

    let result = client.historical_ticks(&test_contract(), 10).last().await;

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 1));
}

/// Dropping the terminal's future before the `done` frame cancels the request at TWS.
#[tokio::test]
async fn historical_ticks_dropped_future_cancels() {
    let bus = Arc::new(MessageBusStub::default().keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);

    let contract = test_contract();
    let request = client.historical_ticks(&contract, 10).last();
    let elapsed = tokio::time::timeout(tokio::time::Duration::from_millis(20), request).await;

    assert!(elapsed.is_err(), "the stub never answers");
    assert_eq!(tick_requests_and_cancels(&bus).await, (1, 1));
}
