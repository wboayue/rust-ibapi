use super::*;
use crate::client::blocking::Client;
use crate::common::test_utils::helpers::{
    assert_decimal_parse_error, assert_request, assert_tws_error_message, count_proto_msgs, create_blocking_test_client_with_ordered_proto_responses,
    proto_error_response, proto_response, request_message_count, TEST_REQ_ID_FIRST,
};
use crate::contracts::Contract;
use crate::market_data::historical::common::test_tables::{HMDS_CONNECTED_MESSAGE, PRE_DATA_NOTICES};
use crate::market_data::historical::BarTimestamp;
use crate::market_data::historical::{TickBidAsk, TickLast, TickMidpoint, ToDuration};
use crate::market_data::{IgnoreSize, TradingHours};
use crate::messages::{IncomingMessages, OutgoingMessages, HMDS_QUERY_MESSAGE_CODE};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::market_data::{
    head_timestamp_request, head_timestamp_response, histogram_data_request, histogram_data_response, histogram_entry, historical_data_bar,
    historical_data_daily_bar, historical_data_end_response, historical_data_request, historical_data_response, historical_data_update_response,
    historical_schedule_response, historical_ticks_bid_ask_response, historical_ticks_last_response, historical_ticks_request,
    historical_ticks_response, tick_bid_ask, tick_last, tick_midpoint,
};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;
use time::macros::{date, datetime};
use time::OffsetDateTime;
use time_tz::{self, PrimitiveDateTimeExt, Tz};

// Feeds a single frame of a message type the request doesn't expect: it is
// skipped, the stream closes before the answer, and the request is cancelled
// at TWS with `cancel`.
fn assert_skips_undeclared<F, T>(server_version: i32, response: crate::messages::ResponseMessage, cancel: OutgoingMessages, call: F)
where
    F: FnOnce(Client) -> Result<T, Error>,
{
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![response]));
    let client = Client::stubbed(message_bus.clone(), server_version);
    let Err(err) = call(client) else {
        panic!("expected UnexpectedEndOfStream");
    };
    assert!(
        matches!(err, Error::UnexpectedEndOfStream),
        "expected UnexpectedEndOfStream, got: {err:?}"
    );
    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(count_proto_msgs(&messages, cancel), 1, "no answer, so the request is cancelled");
}

#[test]
fn head_timestamp() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HeadTimestamp,
        head_timestamp_response().unix_timestamp(1678323335).encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let head_timestamp = client
        .head_timestamp(&contract, what_to_show, trading_hours)
        .expect("head timestamp request failed");

    assert_eq!(head_timestamp, OffsetDateTime::from_unix_timestamp(1678323335).unwrap(), "bar.date");

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
#[test]
fn head_timestamp_reads_past_hmds_query_notice() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_error_response(
            TEST_REQ_ID_FIRST,
            HMDS_QUERY_MESSAGE_CODE,
            "Historical Market Data Service query message:no items retrieved",
        ),
        proto_response(
            IncomingMessages::HeadTimestamp,
            head_timestamp_response().unix_timestamp(1678323335).encode_proto(),
        ),
    ]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let head_timestamp = client
        .head_timestamp(&Contract::stock("MSFT").build(), WhatToShow::Trades, TradingHours::Regular)
        .expect("the 165 must not fail the call");

    assert_eq!(head_timestamp, OffsetDateTime::from_unix_timestamp(1678323335).unwrap());
}

#[test]
fn histogram_data_malformed_size_fails_the_request() {
    // End-to-end through the public API: a malformed size must fail the request
    // rather than silently decoding as 0 (issue #716).
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(125.50, 0.0).size_wire(Some("abc")))
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);
    let contract = Contract::stock("MSFT").build();

    assert_decimal_parse_error(client.histogram_data(&contract, TradingHours::Regular, BarSize::Day), "abc");
}

#[test]
fn histogram_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(125.50, 1000.0))
            .entry(histogram_entry(126.00, 2000.0))
            .entry(histogram_entry(126.50, 3000.0))
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let trading_hours = TradingHours::Regular;
    let period = BarSize::Day;

    let histogram_data = client
        .histogram_data(&contract, trading_hours, period)
        .expect("histogram data request failed");

    // Assert Response
    assert_eq!(histogram_data.len(), 3, "histogram_data.len()");

    assert_eq!(histogram_data[0].price, 125.50, "histogram_data[0].price");
    assert_eq!(histogram_data[0].size, Some(1000.0), "histogram_data[0].size");

    assert_eq!(histogram_data[1].price, 126.00, "histogram_data[1].price");
    assert_eq!(histogram_data[1].size, Some(2000.0), "histogram_data[1].size");

    assert_eq!(histogram_data[2].price, 126.50, "histogram_data[2].price");
    assert_eq!(histogram_data[2].size, Some(3000.0), "histogram_data[2].size");

    // Assert Request
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

#[test]
fn historical_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_681_344_000) // 2023-04-13 UTC
                        .ohlc(182.94, 186.50, 180.94, 185.90)
                        .volume(948_837.22)
                        .wap(184.869)
                        .count(324_891),
                )
                .bar(
                    historical_data_daily_bar("20230414")
                        .ohlc(183.88, 186.28, 182.01, 185.00)
                        .volume(810_998.27)
                        .wap(183.9865)
                        .count(277_547),
                )
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230413 16:31:22 UTC")
                .end_date_str("20230415 16:31:22 UTC")
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let end_date = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let historical_data = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .duration(duration)
        .ending(end_date)
        .fetch()
        .expect("historical data request failed");

    // Assert Response

    assert_eq!(historical_data.start, datetime!(2023-04-13 16:31:22 UTC), "historical_data.start");
    assert_eq!(historical_data.end, datetime!(2023-04-15 16:31:22 UTC), "historical_data.end");
    assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");

    assert_eq!(
        historical_data.bars[0].date,
        BarTimestamp::DateTime(datetime!(2023-04-13 00:00:00 UTC)),
        "bar.date"
    );
    assert_eq!(historical_data.bars[0].open, 182.94, "bar.open");
    assert_eq!(historical_data.bars[0].high, 186.50, "bar.high");
    assert_eq!(historical_data.bars[0].low, 180.94, "bar.low");
    assert_eq!(historical_data.bars[0].close, 185.90, "bar.close");
    assert_eq!(historical_data.bars[0].volume, Some(948837.22), "bar.volume");
    assert_eq!(historical_data.bars[0].wap, Some(184.869), "bar.wap");
    assert_eq!(historical_data.bars[0].count, Some(324891), "bar.count");

    assert_eq!(historical_data.bars[1].date, BarTimestamp::Date(date!(2023 - 04 - 14)), "daily bar.date");
    assert_eq!(historical_data.bars[1].open, 183.88, "bar[1].open");

    // Assert Request
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
            .use_rth(trading_hours.use_rth()),
    );
}

#[test]
fn historical_schedules() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalSchedule,
        historical_schedule_response().encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let end_date = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 7.days();

    let schedule = client
        .historical_schedules(&contract, duration)
        .ending(end_date)
        .fetch()
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

#[test]
fn historical_data_adjusted_last_validation() {
    let message_bus = Arc::new(MessageBusStub::default());

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let end_date = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::AdjustedLast;

    // This should return an error because AdjustedLast can't be used with end_date
    let result = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .duration(duration)
        .ending(end_date)
        .fetch();
    assert!(result.is_err(), "Expected error due to AdjustedLast with end_date");

    match result {
        Err(Error::InvalidArgument(_)) => {
            // This is the expected error type
        }
        _ => panic!("Expected InvalidArgument error"),
    }
}

#[test]
fn historical_data_error_response() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        200,
        "No security definition has been found for the request",
    )]));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    // This should return an error because the server sent an error response
    let result = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .trading_hours(trading_hours)
        .duration(duration)
        .fetch();
    assert!(result.is_err(), "Expected error due to error response from server");
}

/// TWS sends these on the error callback and then delivers the bars: 2188 (the
/// account may not stream the up-to-the-second tail, but the historical data
/// itself is served) and 165 "HMDS server connection was successful" (the farm
/// connected on demand, #1136). Classified as an error either would end the
/// request before `HistoricalData` arrived, and the caller would see no bars.
#[test]
fn historical_data_notice_before_bars_is_skipped() {
    for (code, message) in PRE_DATA_NOTICES {
        let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![
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
        ]);

        let result = client
            .historical_data(&Contract::stock("SPY").build(), BarSize::Day)
            .duration(Duration::days(1))
            .ending(datetime!(2023-03-15 16:00:00 UTC))
            .fetch();

        let data = result.unwrap_or_else(|e| panic!("{code} must not abort the request: {e:?}"));
        assert_eq!(data.bars.len(), 1, "bars sent after {code} should be returned");
    }
}

/// The window frame ahead of the bars is out of order.
#[test]
fn historical_data_unexpected_response() {
    let (result, bus) = fetch_daily(vec![proto_response(
        IncomingMessages::HistoricalDataEnd,
        historical_data_end_response().encode_proto(),
    )]);
    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{result:?}");
    // The window frame ended the request at TWS: nothing to cancel.
    assert_eq!(historical_requests_and_cancels(&bus), (1, 0));
}

/// A message type the request doesn't expect is skipped, not taken for a frame.
#[test]
fn historical_data_skips_undeclared_message() {
    let (result, bus) = fetch_daily(vec![proto_response(IncomingMessages::MarketDataType, Vec::new())]);
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "{result:?}");
    assert_eq!(historical_requests_and_cancels(&bus), (1, 1));
}

#[test]
fn historical_data_time_zone_handling() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(
                    historical_data_bar(1_681_344_000)
                        .ohlc(182.94, 186.50, 180.94, 185.90)
                        .volume(948_837.22)
                        .wap(184.869)
                        .count(324_891),
                )
                .bar(
                    historical_data_bar(1_681_430_400)
                        .ohlc(183.88, 186.28, 182.01, 185.00)
                        .volume(810_998.27)
                        .wap(183.9865)
                        .count(277_547),
                )
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230413 09:30:00 US/Eastern")
                .end_date_str("20230415 16:00:00 US/Eastern")
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let end_date = datetime!(2023-04-15 16:00:00 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Day;
    let what_to_show = WhatToShow::Trades;

    let historical_data = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .duration(duration)
        .ending(end_date)
        .fetch()
        .expect("historical data request failed");

    let ny_zone = time_tz::timezones::db::america::NEW_YORK;

    assert_eq!(
        historical_data.start,
        datetime!(2023-04-13 09:30:00).assume_timezone(ny_zone).unwrap(),
        "start carried in HistoricalDataEnd's embedded TZ"
    );

    assert_eq!(
        historical_data.end,
        datetime!(2023-04-15 16:00:00).assume_timezone(ny_zone).unwrap(),
        "end carried in HistoricalDataEnd's embedded TZ"
    );
}

#[test]
fn historical_data_streaming_with_updates() {
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

    let mut client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let update1 = subscription.next_data();
    assert!(update1.is_some(), "Should receive initial historical data");
    match update1.unwrap().expect("expected Ok") {
        HistoricalDataFrame::Bars(bars) => {
            assert_eq!(bars.len(), 1, "Should have 1 initial bar");
            assert_eq!(bars[0].open, 185.50, "Wrong open price");
        }
        _ => panic!("Expected Bars variant"),
    }

    // Second: receive streaming update
    let update2 = subscription.next_data();
    assert!(update2.is_some(), "Should receive streaming update");
    match update2.unwrap().expect("expected Ok") {
        HistoricalDataFrame::Update(bar) => {
            assert_eq!(bar.open, 185.80, "Wrong open price in update");
            assert_eq!(bar.high, 186.10, "Wrong high price in update");
            assert_eq!(bar.close, 185.90, "Wrong close price in update");
        }
        _ => panic!("Expected Update variant"),
    }

    // Verify request message was sent
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

// The dispatcher classifies the error frame; this asserts that *this API* hands
// it to the caller rather than swallowing it. The transport tests prove
// delivery, not consumption — see blocking `matching_symbols` (#735).
#[test]
fn historical_data_streaming_error_response() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]);
    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .expect("streaming request should succeed");

    let Some(Err(err)) = subscription.next_data() else {
        panic!("error should arrive as Some(Err(_))");
    };
    assert_tws_error_message(err, 162, "No market data permissions");
}

/// 165 "HMDS server connection was successful" ahead of the bars is a notice:
/// the stream yields it and keeps going (#1136).
#[test]
fn historical_data_streaming_hmds_connected_notice_then_data() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![
        proto_error_response(9000, HMDS_QUERY_MESSAGE_CODE, HMDS_CONNECTED_MESSAGE),
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(historical_data_bar(1_678_886_400).ohlc(185.50, 186.00, 185.25, 185.75))
                .encode_proto(),
        ),
    ]);

    let subscription = client
        .historical_data(&Contract::stock("SPY").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .expect("streaming request should succeed");

    let Some(Ok(SubscriptionItem::Notice(notice))) = subscription.next() else {
        panic!("165 should arrive as a notice");
    };
    assert_eq!((notice.code, notice.message.as_str()), (HMDS_QUERY_MESSAGE_CODE, HMDS_CONNECTED_MESSAGE));
    let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(bars)))) = subscription.next() else {
        panic!("bars should follow the notice");
    };
    assert_eq!(bars.len(), 1);
}

#[test]
fn streaming_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub::default());

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let _subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .subscribe_with_updates()
            .expect("streaming request should succeed");
    }

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send exactly one cancel message on drop"
    );
}

#[test]
fn streaming_subscription_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub::default());

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(Duration::days(1))
            .subscribe_with_updates()
            .expect("streaming request should succeed");

        subscription.cancel();
    }

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send cancel only once"
    );
}

#[test]
fn head_timestamp_skips_undeclared_message() {
    // 17 = HistoricalData — wrong type for head_timestamp (expects 88).
    assert_skips_undeclared(
        server_versions::SIZE_RULES,
        proto_response(IncomingMessages::HistoricalData, Vec::new()),
        OutgoingMessages::CancelHeadTimestamp,
        |c| c.head_timestamp(&Contract::stock("MSFT").build(), WhatToShow::Trades, TradingHours::Regular),
    );
}

#[test]
fn head_timestamp_end_of_stream() {
    // Empty response set → channel closes → subscription.next() yields None.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let result = client.head_timestamp(&Contract::stock("MSFT").build(), WhatToShow::Trades, TradingHours::Regular);
    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
}

#[test]
fn historical_data_with_end_message() {
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
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .expect("historical_data should succeed");

    assert_eq!(data.bars.len(), 1, "should have one bar");
    assert_eq!(data.start, datetime!(2023-03-15 09:30:00 UTC), "start populated from end-message");
    assert_eq!(data.end, datetime!(2023-03-15 10:30:00 UTC), "end populated from end-message");
}

#[test]
fn historical_data_end_of_stream_is_not_retried() {
    // Empty responses → channel closes immediately → subscription.next() yields
    // None. A closed stream is not a reset: it fails on the first attempt.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch();

    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
    assert_eq!(
        historical_requests_and_cancels(&message_bus),
        (1, 1),
        "a closed stream must not be retried"
    );
}

#[test]
fn historical_data_retries_a_connection_reset() {
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
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch()
        .expect("the reset must be retried, not surfaced");

    assert_eq!(data.bars.len(), 1);
    let (requests, _) = historical_requests_and_cancels(&message_bus);
    assert_eq!(requests, 2, "the retry must re-send the request");
}

#[test]
fn historical_data_error_message_response() {
    // Type 4 = IncomingMessages::Error — exercises the explicit Error arm
    // (Err::from(message)), distinct from the UnexpectedResponse arm.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch();

    let err = result.expect_err("expected error from server");
    assert!(err.to_string().contains("No market data permissions"), "got: {err}");
    // TWS ended the query with its error; a cancel would draw 366.
    assert_eq!(historical_requests_and_cancels(&message_bus), (1, 0));
}

#[test]
fn historical_schedules_ending_now() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalSchedule,
        historical_schedule_response().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let duration = 7.days();

    let schedule = client
        .historical_schedules(&contract, duration)
        .fetch()
        .expect("historical schedules ending now should succeed");

    assert_eq!(schedule.sessions.len(), 1);
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &historical_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .duration(duration)
            .bar_size(BarSize::Day)
            .what_to_show(WhatToShow::Schedule)
            .use_rth(true),
    );
}

#[test]
fn historical_schedule_skips_undeclared_message() {
    assert_skips_undeclared(
        server_versions::HISTORICAL_SCHEDULE,
        proto_response(IncomingMessages::HistoricalData, Vec::new()),
        OutgoingMessages::CancelHistoricalData,
        |c| c.historical_schedules(&Contract::stock("MSFT").build(), Duration::days(1)).fetch(),
    );
}

#[test]
fn historical_schedule_end_of_stream() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_SCHEDULE);
    let result = client.historical_schedules(&Contract::stock("MSFT").build(), Duration::days(1)).fetch();
    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
}

#[test]
fn histogram_data_empty_response_returns_empty_vec() {
    // None on the first subscription.next() → returns an empty Vec.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let histogram = client
        .histogram_data(&Contract::stock("MSFT").build(), TradingHours::Regular, BarSize::Day)
        .expect("histogram_data should succeed with empty result");

    assert!(histogram.is_empty(), "expected empty histogram, got {histogram:?}");
}

/// The historical data requests and cancels written to `bus`.
fn historical_requests_and_cancels(bus: &MessageBusStub) -> (usize, usize) {
    let messages = bus.request_messages.read().unwrap();
    (
        count_proto_msgs(&messages, OutgoingMessages::RequestHistoricalData),
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
    )
}

/// Fetch one day of daily bars against `responses`.
fn fetch_daily(responses: Vec<crate::messages::ResponseMessage>) -> (Result<HistoricalData, Error>, Arc<MessageBusStub>) {
    let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(responses);
    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Day)
        .duration(Duration::seconds(86_400))
        .fetch();
    (result, bus)
}

/// A decoded batch alone cannot prove a finite request completed.
#[test]
fn historical_data_requires_end_after_a_batch() {
    for populated in [false, true] {
        let response = historical_data_response();
        let response = if populated {
            response.bar(historical_data_daily_bar("20230315").ohlc(1.0, 2.0, 0.5, 1.5))
        } else {
            response
        };
        let (result, bus) = fetch_daily(vec![proto_response(IncomingMessages::HistoricalData, response.encode_proto())]);
        assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "populated={populated}: {result:?}");
        assert_eq!(historical_requests_and_cancels(&bus), (1, 1), "bare closure must not replay the request");
    }
}

/// A second data frame is not the completion marker.
#[test]
fn historical_data_requires_end_message_type() {
    let response = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let (result, bus) = fetch_daily(vec![response.clone(), response]);
    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{result:?}");
    assert_eq!(historical_requests_and_cancels(&bus), (1, 1));
}

/// An update frame on a request sent without updates is out of protocol:
/// `fetch` fails rather than treat it as bars or the window, and cancels.
#[test]
fn historical_data_update_frame_is_unexpected() {
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let update = proto_response(
        IncomingMessages::HistoricalDataUpdate,
        historical_data_update_response()
            .bar(historical_data_bar(1_678_890_000).ohlc(1.0, 2.0, 0.5, 1.5))
            .encode_proto(),
    );
    for (label, responses) in [("in place of bars", vec![update.clone()]), ("in place of the window", vec![bars, update])] {
        let (result, bus) = fetch_daily(responses);
        assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{label}: {result:?}");
        assert_eq!(historical_requests_and_cancels(&bus), (1, 1), "{label}");
    }
}

/// A decode failure before the window frame cancels the request at TWS.
#[test]
fn historical_data_local_error_cancels() {
    let (result, bus) = fetch_daily(vec![proto_response(IncomingMessages::HistoricalData, vec![0xff])]);
    assert!(result.is_err(), "malformed bars must fail");
    assert_eq!(historical_requests_and_cancels(&bus), (1, 1));
}

/// `one_shot_timeout` bounds both reads: bars, then silence, times out
/// and cancels the request at TWS.
#[test]
fn historical_data_one_shot_timeout_cancels_between_frames() {
    let bars = proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto());
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![bars]).keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3).with_one_shot_timeout(std::time::Duration::from_millis(20));

    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Day)
        .duration(Duration::seconds(86_400))
        .fetch();

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert_eq!(historical_requests_and_cancels(&bus), (1, 1));
}

/// A reconnect that outlasts `one_shot_timeout` ends the fetch without a resend.
#[test]
fn historical_data_one_shot_timeout_skips_resend_past_deadline() {
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
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Day)
        .duration(Duration::seconds(86_400))
        .fetch();

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert!(start.elapsed() < RECONNECT / 2, "the reconnect wait must end at the deadline");
    assert_eq!(historical_requests_and_cancels(&bus).0, 1, "no resend past the deadline");
}

/// An empty result is valid when the broker explicitly completes the request.
#[test]
fn historical_data_requires_end_accepts_completed_empty_batch() {
    let (result, bus) = fetch_daily(vec![
        proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto()),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230315 09:30:00 UTC")
                .end_date_str("20230315 10:30:00 UTC")
                .encode_proto(),
        ),
    ]);
    let result = result.unwrap();
    assert!(result.bars.is_empty());
    assert_eq!(result.start, datetime!(2023-03-15 09:30:00 UTC));
    assert_eq!(result.end, datetime!(2023-03-15 10:30:00 UTC));
    assert_eq!(request_message_count(&bus), 1);
}

// ---- buffer_limit ----------------------------------------------------------

#[test]
fn historical_data_stream_buffer_limit_reaches_the_bus() {
    let (client, message_bus) = crate::common::test_utils::helpers::create_blocking_test_client();
    let contract = Contract::stock("SPY").build();

    let _bounded = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .buffer_limit(16)
        .subscribe_with_updates()
        .expect("bounded stream failed");
    let _unbounded = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .subscribe_with_updates()
        .expect("unbounded stream failed");

    assert_eq!(
        *message_bus.buffer_limits.read().unwrap(),
        vec![16],
        "only the bounded request carries a limit"
    );
}

#[test]
fn historical_data_fetch_rejects_buffer_limit() {
    let (client, message_bus) = crate::common::test_utils::helpers::create_blocking_test_client();
    let contract = Contract::stock("SPY").build();

    let result = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .buffer_limit(16)
        .fetch();

    assert!(matches!(result, Err(Error::InvalidArgument(_))), "got {result:?}");
    assert!(message_bus.request_messages.read().unwrap().is_empty(), "nothing is sent");
}

// === Historical ticks: a one-shot read until TWS marks the reply `done` ===

/// The historical ticks requests and cancels written to `bus`.
fn tick_requests_and_cancels(bus: &MessageBusStub) -> (usize, usize) {
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
fn fetch_last(responses: Vec<crate::messages::ResponseMessage>) -> (Result<Vec<TickLast>, Error>, Arc<MessageBusStub>) {
    fetch_last_at(server_versions::CANCEL_CONTRACT_DATA, responses)
}

fn fetch_last_at(server_version: i32, responses: Vec<crate::messages::ResponseMessage>) -> (Result<Vec<TickLast>, Error>, Arc<MessageBusStub>) {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(responses));
    let client = Client::stubbed(bus.clone(), server_version);
    let result = client.historical_ticks(&Contract::stock("MSFT").build(), 10).last();
    (result, bus)
}

fn prices(ticks: &[TickLast]) -> Vec<f64> {
    ticks.iter().map(|tick| tick.price).collect()
}

#[test]
fn historical_ticks_last() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(11.63, true)]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let start = datetime!(2023-04-01 09:30:00 UTC);
    let end = datetime!(2023-04-01 16:00:00 UTC);

    let ticks = client
        .historical_ticks(&contract, 10)
        .starting(start)
        .ending(end)
        .trading_hours(TradingHours::Regular)
        .last()
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

#[test]
fn historical_ticks_mid_point() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTick,
        historical_ticks_response()
            .tick(tick_midpoint(1_681_133_398, 91.36))
            .tick(tick_midpoint(1_681_133_399, 91.37))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let end = datetime!(2023-04-01 16:00:00 UTC);

    let ticks: Vec<TickMidpoint> = client
        .historical_ticks(&contract, 10)
        .ending(end)
        .mid_point()
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

#[test]
fn historical_ticks_bid_ask() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(tick_bid_ask(1_681_133_399, 11.63, 11.83, 2_800.0, 100.0).bid_past_low(true))
            .tick(tick_bid_ask(1_681_133_400, 11.64, 11.84, 2_900.0, 200.0).ask_past_high(true))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let start = datetime!(2023-04-01 09:30:00 UTC);

    let ticks: Vec<TickBidAsk> = client
        .historical_ticks(&contract, 10)
        .starting(start)
        .bid_ask(IgnoreSize::Yes)
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
#[test]
fn historical_ticks_reads_frames_until_done() {
    let (result, bus) = fetch_last(vec![last_frame(1.0, false), last_frame(2.0, false), last_frame(3.0, true)]);
    assert_eq!(prices(&result.expect("three frames")), [1.0, 2.0, 3.0]);
    assert_eq!(tick_requests_and_cancels(&bus), (1, 0));
}

/// An empty `done` frame is a complete, empty reply.
#[test]
fn historical_ticks_empty_reply() {
    let frame = proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response().done(true).encode_proto(),
    );
    let (result, bus) = fetch_last(vec![frame]);
    assert_eq!(result.expect("empty reply"), Vec::<TickLast>::new());
    assert_eq!(tick_requests_and_cancels(&bus), (1, 0));
}

/// A reply that stops before its `done` frame is never returned as complete,
/// and the request is cancelled at TWS.
#[test]
fn historical_ticks_closed_before_done_is_an_error() {
    for responses in [vec![], vec![last_frame(1.0, false)]] {
        let (result, bus) = fetch_last(responses);
        assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "{result:?}");
        assert_eq!(tick_requests_and_cancels(&bus), (1, 1));
    }
}

/// A server older than the cancel message gets none.
#[test]
fn historical_ticks_writes_no_cancel_below_its_server_version() {
    let (result, bus) = fetch_last_at(server_versions::CANCEL_CONTRACT_DATA - 1, vec![last_frame(1.0, false)]);
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus), (1, 0));
}

/// A TWS rejection is the error, with its code.
#[test]
fn historical_ticks_rejection_is_the_error() {
    let rejection = proto_error_response(TEST_REQ_ID_FIRST, 10187, "Failed to request historical ticks:Max length exceeded");
    let (result, _bus) = fetch_last(vec![rejection]);
    match result {
        Err(Error::Notice(notice)) => assert_eq!(
            (notice.code, notice.message.as_str()),
            (10187, "Failed to request historical ticks:Max length exceeded")
        ),
        other => panic!("expected the 10187 rejection, got {other:?}"),
    }
}

/// Notices TWS sends before the data are skipped, as `fetch()` skips them.
#[test]
fn historical_ticks_notice_before_reply_is_skipped() {
    for (code, message) in PRE_DATA_NOTICES {
        let (result, _bus) = fetch_last(vec![proto_error_response(TEST_REQ_ID_FIRST, code, message), last_frame(1.0, true)]);
        assert_eq!(prices(&result.unwrap_or_else(|e| panic!("code {code}: {e:?}"))), [1.0], "code {code}");
    }
}

/// A frame of another message type is skipped, not decoded.
#[test]
fn historical_ticks_skips_undeclared_message() {
    let other = proto_response(IncomingMessages::HistoricalDataEnd, historical_data_end_response().encode_proto());
    let (result, _bus) = fetch_last(vec![other, last_frame(1.0, true)]);
    assert_eq!(prices(&result.expect("the tick frame")), [1.0]);
}

/// A decode failure before the `done` frame cancels the request at TWS.
#[test]
fn historical_ticks_local_error_cancels() {
    let (result, bus) = fetch_last(vec![proto_response(IncomingMessages::HistoricalTickLast, vec![0xff])]);
    assert!(matches!(result, Err(Error::ProtobufDecode(_))), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus), (1, 1));
}

#[test]
fn historical_ticks_retries_a_connection_reset() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(1.0, true)]).with_connection_resets(1));
    let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

    let ticks = client
        .historical_ticks(&Contract::stock("MSFT").build(), 10)
        .last()
        .expect("the reset must be retried, not surfaced");

    assert_eq!(prices(&ticks), [1.0]);
    assert_eq!(tick_requests_and_cancels(&bus).0, 2, "the retry must re-send the request");
}

/// `one_shot_timeout` bounds every read: a frame, then silence, times out and
/// cancels the request at TWS.
#[test]
fn historical_ticks_one_shot_timeout_cancels_between_frames() {
    let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![last_frame(1.0, false)]).keeping_channels_open());
    let client = Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA).with_one_shot_timeout(std::time::Duration::from_millis(20));

    let result = client.historical_ticks(&Contract::stock("MSFT").build(), 10).last();

    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert_eq!(tick_requests_and_cancels(&bus), (1, 1));
}
