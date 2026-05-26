use super::*;
use crate::client::blocking::Client;
use crate::common::test_utils::helpers::{
    assert_proto_msg_id, assert_request, assert_request_msg_id, count_proto_msgs, proto_error_response, proto_response, request_message_count,
    TEST_REQ_ID_FIRST,
};
use crate::contracts::Contract;
use crate::market_data::historical::BarTimestamp;
use crate::market_data::historical::{TickBidAsk, TickLast, TickMidpoint, ToDuration};
use crate::market_data::{IgnoreSize, TradingHours};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::protocol::{Features, ProtocolFeature};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::market_data::{
    head_timestamp_request, head_timestamp_response, histogram_data_request, histogram_data_response, histogram_entry, historical_data_bar,
    historical_data_daily_bar, historical_data_end_response, historical_data_request, historical_data_response, historical_data_update_response,
    historical_schedule_response, historical_tick_bid_ask, historical_tick_last, historical_tick_mid, historical_ticks_bid_ask_response,
    historical_ticks_last_response, historical_ticks_request, historical_ticks_response,
};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::{Arc, RwLock};
use time::macros::{date, datetime};
use time::OffsetDateTime;
use time_tz::{self, PrimitiveDateTimeExt, Tz};

// Pins server_version one below `feature.min_version` and asserts the call fails
// with the feature name in the error. Custom helper (vs. `.expect_err`) because
// subscription return types don't implement Debug.
fn assert_version_check_fails<F, T>(feature: ProtocolFeature, call: F)
where
    F: FnOnce(Client) -> Result<T, Error>,
{
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, feature.min_version - 1);
    let Err(err) = call(client) else {
        panic!("expected version-check failure ({})", feature.name);
    };
    assert!(err.to_string().contains(feature.name), "expected '{}', got: {err}", feature.name);
}

// Feeds a single wire-format response, asserts the call fails with
// `Error::UnexpectedResponse`. Pairs with `assert_version_check_fails`.
fn assert_unexpected_response<F, T>(server_version: i32, response: &str, call: F)
where
    F: FnOnce(Client) -> Result<T, Error>,
{
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![response.to_owned()]));
    let client = Client::stubbed(message_bus, server_version);
    let Err(err) = call(client) else {
        panic!("expected UnexpectedResponse failure");
    };
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got: {err:?}");
}

#[test]
fn test_head_timestamp() {
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

#[test]
fn test_histogram_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistogramData,
        histogram_data_response()
            .entry(histogram_entry(125.50, 1000))
            .entry(histogram_entry(126.00, 2000))
            .entry(histogram_entry(126.50, 3000))
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
    assert_eq!(histogram_data[0].size, 1000, "histogram_data[0].size");

    assert_eq!(histogram_data[1].price, 126.00, "histogram_data[1].price");
    assert_eq!(histogram_data[1].size, 2000, "histogram_data[1].size");

    assert_eq!(histogram_data[2].price, 126.50, "histogram_data[2].price");
    assert_eq!(histogram_data[2].size, 3000, "histogram_data[2].size");

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
fn test_historical_data() {
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
    assert_eq!(historical_data.bars[0].volume, 948837.22, "bar.volume");
    assert_eq!(historical_data.bars[0].wap, 184.869, "bar.wap");
    assert_eq!(historical_data.bars[0].count, 324891, "bar.count");

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
            .what_to_show(Some(what_to_show))
            .use_rth(trading_hours.use_rth()),
    );
}

#[test]
fn test_historical_schedules() {
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
            .what_to_show(Some(WhatToShow::Schedule))
            .use_rth(true),
    );
}

#[test]
fn test_historical_ticks_bid_ask() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = datetime!(2023-04-01 09:30:00 UTC);
    let end = datetime!(2023-04-01 16:00:00 UTC);
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let _tick_subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .starting(start)
        .ending(end)
        .trading_hours(trading_hours)
        .bid_ask(IgnoreSize::Yes)
        .expect("historical ticks bid ask request failed");

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
            .ignore_size(true),
    );
}

#[test]
fn test_historical_ticks_mid_point() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = datetime!(2023-04-01 09:30:00 UTC);
    let end = datetime!(2023-04-01 16:00:00 UTC);
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let _tick_subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .starting(start)
        .ending(end)
        .trading_hours(trading_hours)
        .mid_point()
        .expect("historical ticks mid point request failed");

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
            .what_to_show(WhatToShow::MidPoint)
            .use_rth(trading_hours.use_rth()),
    );
}

#[test]
fn test_historical_ticks_trade() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = datetime!(2023-04-01 09:30:00 UTC);
    let end = datetime!(2023-04-01 16:00:00 UTC);
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let _tick_subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .starting(start)
        .ending(end)
        .trading_hours(trading_hours)
        .trade()
        .expect("historical ticks trade request failed");

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
            .what_to_show(WhatToShow::Trades)
            .use_rth(trading_hours.use_rth()),
    );
}

#[test]
fn test_historical_data_version_check() {
    // Test with a server version that doesn't support trading class
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
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
    let result = client
        .historical_data(&contract, bar_size)
        .duration(duration)
        .ending(end_date)
        .trading_hours(trading_hours)
        .fetch();
    assert!(result.is_err(), "Expected error due to server version incompatibility");
}

#[test]
fn test_historical_data_adjusted_last_validation() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

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
fn test_historical_data_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Respond with an error message
            "3\09000\0200\0No security definition has been found for the request\0".to_owned(),
        ],
        ordered_responses: vec![],
    });

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

#[test]
fn test_historical_data_unexpected_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Respond with an unexpected message type (using market data type message)
            "58\09000\02\0".to_owned(),
        ],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    // This should return an error because the server sent an unexpected response
    let result = client
        .historical_data(&contract, bar_size)
        .what_to_show(what_to_show)
        .trading_hours(trading_hours)
        .duration(duration)
        .fetch();
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
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let tick_subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .trading_hours(trading_hours)
        .trade()
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
    // First batch: 3 ticks, done = false. Second batch: 2 ticks, done = true.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                .tick(historical_tick_last(1_681_133_400, 11.63, 24_547, "ISLAND").special_conditions(" O X"))
                .tick(historical_tick_last(1_681_133_401, 11.64, 179, "FINRA"))
                .tick(historical_tick_last(1_681_133_402, 11.65, 200, "NYSE"))
                .done(false)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                .tick(historical_tick_last(1_681_133_403, 11.66, 100, "ARCA"))
                .tick(historical_tick_last(1_681_133_404, 11.67, 300, "BATS"))
                .done(true)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let tick_subscription = client
        .historical_ticks(&contract, number_of_ticks)
        .trading_hours(trading_hours)
        .trade()
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
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(historical_tick_last(1_681_133_400, 11.70, 24_547, "ISLAND").special_conditions(" O X"))
            .tick(historical_tick_last(1_681_133_401, 11.71, 179, "FINRA"))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks(&contract, 10)
        .trade()
        .expect("historical ticks trade request failed");

    // Convert to owned iterator
    let ticks: Vec<TickLast> = tick_subscription.into_iter().collect();

    assert_eq!(ticks.len(), 2, "Expected 2 ticks from owned iterator");
    assert_eq!(ticks[0].price, 11.70, "First tick price");
    assert_eq!(ticks[1].price, 11.71, "Second tick price");
}

#[test]
fn test_tick_subscription_bid_ask() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(historical_tick_bid_ask(1_681_133_399, 11.63, 11.83, 2_800, 100))
            .tick(historical_tick_bid_ask(1_681_133_400, 11.64, 11.84, 2_900, 200))
            .tick(historical_tick_bid_ask(1_681_133_401, 11.65, 11.85, 3_000, 300))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks(&contract, 10)
        .bid_ask(IgnoreSize::No)
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
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTick,
        historical_ticks_response()
            .tick(historical_tick_mid(1_681_133_398, 91.36, 0))
            .tick(historical_tick_mid(1_681_133_399, 91.37, 0))
            .tick(historical_tick_mid(1_681_133_400, 91.38, 0))
            .done(true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks(&contract, 10)
        .mid_point()
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
fn test_historical_data_streaming_with_updates() {
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
        .stream()
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let update1 = subscription.next_data();
    assert!(update1.is_some(), "Should receive initial historical data");
    match update1.unwrap().expect("expected Ok") {
        HistoricalBarUpdate::Historical(data) => {
            assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
            assert_eq!(data.bars[0].open, 185.50, "Wrong open price");
        }
        _ => panic!("Expected Historical variant"),
    }

    // Second: receive streaming update
    let update2 = subscription.next_data();
    assert!(update2.is_some(), "Should receive streaming update");
    match update2.unwrap().expect("expected Ok") {
        HistoricalBarUpdate::Update(bar) => {
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
            .what_to_show(Some(WhatToShow::Trades))
            .use_rth(true)
            .keep_up_to_date(true),
    );
}

#[test]
fn test_historical_data_streaming_error_response() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]));

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data(&contract, BarSize::Hour)
        .duration(Duration::days(1))
        .stream()
        .expect("streaming request should succeed");

    // Error now flows via the Err arm of next_data()
    let update = subscription.next_data();
    match update {
        Some(Err(e)) => assert!(
            e.to_string().contains("No market data permissions"),
            "Error should contain the message, got: {e}"
        ),
        other => panic!("Expected Some(Err(_)), got {other:?}"),
    }
}

#[test]
fn test_tick_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let internal = message_bus.send_request(9100, &[]).unwrap();

    {
        let _subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9100, message_bus.clone());
        // subscription dropped here, !done so cancel should fire
    }

    let messages = message_bus.request_messages.read().unwrap();
    let cancel_msg = messages.last().expect("should have cancel message");
    assert_proto_msg_id(cancel_msg, OutgoingMessages::CancelHistoricalTicks);
}

#[test]
fn test_tick_subscription_explicit_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let internal = message_bus.send_request(9101, &[]).unwrap();

    {
        let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9101, message_bus.clone());
        subscription.cancel();
    }

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalTicks),
        1,
        "should send cancel only once"
    );
}

#[test]
fn test_tick_subscription_drop_after_done_does_not_cancel() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let internal = message_bus.send_request(9102, &[]).unwrap();

    {
        let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9102, message_bus.clone());
        subscription.done.store(true, std::sync::atomic::Ordering::Relaxed);
        // drop with done=true → no cancel
    }

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalTicks),
        0,
        "completed subscription should not send cancel on drop"
    );
}

#[test]
fn test_streaming_subscription_sends_cancel_on_drop() {
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
fn test_streaming_subscription_cancel_prevents_duplicate_on_drop() {
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
fn test_head_timestamp_version_check() {
    assert_version_check_fails(Features::HEAD_TIMESTAMP, |c| {
        c.head_timestamp(&Contract::stock("MSFT").build(), WhatToShow::Trades, TradingHours::Regular)
    });
}

#[test]
fn test_head_timestamp_unexpected_response() {
    // 17 = HistoricalData — wrong type for head_timestamp (expects 88).
    assert_unexpected_response(server_versions::SIZE_RULES, "17|9000|20230315  09:30:00|20230315  10:30:00|0|", |c| {
        c.head_timestamp(&Contract::stock("MSFT").build(), WhatToShow::Trades, TradingHours::Regular)
    });
}

#[test]
fn test_head_timestamp_end_of_stream() {
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
fn test_historical_data_with_end_message() {
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
fn test_historical_data_connection_reset_after_retries() {
    // Empty responses → channel closes immediately → subscription.next() returns
    // None on every retry → loop exhausts MAX_RETRIES and returns ConnectionReset.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch();

    // The first None response returns UnexpectedEndOfStream (the loop early-exits).
    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
}

#[test]
fn test_historical_data_error_message_response() {
    // Type 4 = IncomingMessages::Error — exercises the explicit Error arm
    // (Err::from(message)), distinct from the UnexpectedResponse arm.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        162,
        "Historical Market Data Service error message:No market data permissions.",
    )]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let result = client
        .historical_data(&Contract::stock("MSFT").build(), BarSize::Hour)
        .duration(Duration::days(1))
        .fetch();

    let err = result.expect_err("expected error from server");
    assert!(err.to_string().contains("No market data permissions"), "got: {err}");
}

#[test]
fn test_historical_schedules_ending_now() {
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
            .what_to_show(Some(WhatToShow::Schedule))
            .use_rth(true),
    );
}

#[test]
fn test_historical_schedule_version_check() {
    assert_version_check_fails(Features::HISTORICAL_SCHEDULE, |c| {
        c.historical_schedules(&Contract::stock("MSFT").build(), Duration::days(1)).fetch()
    });
}

#[test]
fn test_historical_schedule_trading_class_version_check() {
    // contract.trading_class triggers the earlier TRADING_CLASS gate ahead of the
    // HISTORICAL_SCHEDULE gate — pin below TRADING_CLASS so the former fires first.
    let mut contract = Contract::stock("MSFT").build();
    contract.trading_class = "ES".to_owned();
    assert_version_check_fails(Features::TRADING_CLASS, move |c| {
        c.historical_schedules(&contract, Duration::days(1)).fetch()
    });
}

#[test]
fn test_historical_schedule_unexpected_response() {
    assert_unexpected_response(
        server_versions::HISTORICAL_SCHEDULE,
        "17|9000|20230315  09:30:00|20230315  10:30:00|0|",
        |c| c.historical_schedules(&Contract::stock("MSFT").build(), Duration::days(1)).fetch(),
    );
}

#[test]
fn test_historical_schedule_end_of_stream() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_SCHEDULE);
    let result = client.historical_schedules(&Contract::stock("MSFT").build(), Duration::days(1)).fetch();
    assert!(
        matches!(result, Err(Error::UnexpectedEndOfStream)),
        "expected UnexpectedEndOfStream, got {result:?}"
    );
}

#[test]
fn test_cancel_historical_ticks() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::CANCEL_CONTRACT_DATA);

    client.cancel_historical_ticks(9000).expect("cancel should succeed");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request_msg_id(&message_bus, 0, OutgoingMessages::CancelHistoricalTicks);
}

#[test]
fn test_cancel_historical_ticks_version_check() {
    assert_version_check_fails(Features::CANCEL_CONTRACT_DATA, |c| c.cancel_historical_ticks(9000));
}

#[test]
fn test_histogram_data_version_check() {
    assert_version_check_fails(Features::HISTOGRAM, |c| {
        c.histogram_data(&Contract::stock("MSFT").build(), TradingHours::Regular, BarSize::Day)
    });
}

#[test]
fn test_histogram_data_empty_response_returns_empty_vec() {
    // None on the first subscription.next() → returns an empty Vec.
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let histogram = client
        .histogram_data(&Contract::stock("MSFT").build(), TradingHours::Regular, BarSize::Day)
        .expect("histogram_data should succeed with empty result");

    assert!(histogram.is_empty(), "expected empty histogram, got {histogram:?}");
}

#[test]
fn test_historical_data_streaming_trading_class_version_check() {
    let mut contract = Contract::stock("MSFT").build();
    contract.trading_class = "ES".to_owned();
    assert_version_check_fails(Features::TRADING_CLASS, move |c| {
        c.historical_data(&contract, BarSize::Hour).duration(Duration::days(1)).stream()
    });
}

#[test]
fn test_historical_ticks_bid_ask_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| {
        c.historical_ticks(&Contract::stock("MSFT").build(), 1).bid_ask(IgnoreSize::No)
    });
}

#[test]
fn test_historical_ticks_mid_point_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| {
        c.historical_ticks(&Contract::stock("MSFT").build(), 1).mid_point()
    });
}

#[test]
fn test_historical_ticks_trade_version_check() {
    assert_version_check_fails(Features::HISTORICAL_TICKS, |c| {
        c.historical_ticks(&Contract::stock("MSFT").build(), 1).trade()
    });
}

#[test]
fn test_tick_subscription_try_next_drains_buffer() {
    // Pre-load a single batch with done=true so try_next() can drain without blocking.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(historical_tick_last(1_681_133_400, 12.00, 100, "NYSE"))
            .tick(historical_tick_last(1_681_133_401, 12.01, 200, "NYSE"))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client
        .historical_ticks(&Contract::stock("MSFT").build(), 10)
        .trade()
        .expect("subscription should be created");

    // Drain via try_iter — each .next() goes through try_next() → next_helper(try_next).
    let ticks: Vec<TickLast> = subscription.try_iter().collect();
    assert_eq!(ticks.len(), 2, "should drain both ticks via try_next");
    assert_eq!(ticks[0].price, 12.00);
    assert_eq!(ticks[1].price, 12.01);

    // After done, try_next returns None immediately.
    assert!(subscription.try_next().is_none(), "no more ticks expected after done");
}

#[test]
fn test_tick_subscription_next_timeout_drains_buffer() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(historical_tick_last(1_681_133_400, 13.00, 100, "NYSE"))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client
        .historical_ticks(&Contract::stock("MSFT").build(), 10)
        .trade()
        .expect("subscription should be created");

    // Drive timeout_iter — each .next() goes through next_timeout() → next_helper.
    let ticks: Vec<TickLast> = subscription.timeout_iter(std::time::Duration::from_millis(50)).collect();
    assert_eq!(ticks.len(), 1, "should drain the single tick");
    assert_eq!(ticks[0].price, 13.00);

    // After done, next_timeout also returns None.
    assert!(
        subscription.next_timeout(std::time::Duration::from_millis(10)).is_none(),
        "no more ticks expected after done"
    );
}

#[test]
fn test_tick_subscription_fill_buffer_error_response_via_channel() {
    // Inject a RoutedItem::Error directly to exercise fill_buffer's Err arm and
    // set_error() — paths the MessageBusStub mock_request path can't reach.
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    let message_bus = Arc::new(MessageBusStub::default());

    let (sender, receiver) = channel::unbounded();
    let (signaler, _signal_rx) = channel::unbounded();
    sender.send(RoutedItem::Error(Error::Simple("injected".into()))).unwrap();
    drop(sender);

    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(9300).build();

    let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9300, message_bus);
    assert!(subscription.next().is_none(), "Err response → set_error → next returns None");
    subscription.done.store(true, Ordering::Relaxed); // prevent cancel-on-drop noise
}

#[test]
fn test_tick_subscription_fill_buffer_none_when_channel_closes() {
    // Channel closes with no done=true → fill_buffer(None) returns Err(()) → next returns None.
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    let message_bus = Arc::new(MessageBusStub::default());

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _signal_rx) = channel::unbounded();
    drop(sender); // empty + closed immediately

    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(9301).build();

    let subscription: TickSubscription<TickBidAsk> = TickSubscription::new(internal, 9301, message_bus);
    assert!(subscription.next().is_none(), "closed channel → fill_buffer None → next returns None");
    subscription.done.store(true, Ordering::Relaxed);
}

#[test]
fn test_tick_subscription_midpoint_try_iter_and_timeout_iter() {
    // Exercise try_next + next_timeout on the TickMidpoint monomorphization so
    // every (TickT × iterator) combination shows up in coverage data.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTick,
        historical_ticks_response()
            .tick(historical_tick_mid(1_681_133_400, 91.50, 0))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client
        .historical_ticks(&Contract::stock("MSFT").build(), 10)
        .mid_point()
        .expect("subscription should be created");

    let ticks_try: Vec<TickMidpoint> = subscription.try_iter().collect();
    assert_eq!(ticks_try.len(), 1);

    // Timeout iterator after exhaustion — exercises next_timeout None path.
    assert!(subscription.timeout_iter(std::time::Duration::from_millis(10)).next().is_none());
}

#[test]
fn test_tick_subscription_bid_ask_try_iter_and_timeout_iter() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalTickBidAsk,
        historical_ticks_bid_ask_response()
            .tick(historical_tick_bid_ask(1_681_133_399, 11.63, 11.83, 2_800, 100))
            .done(true)
            .encode_proto(),
    )]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client
        .historical_ticks(&Contract::stock("MSFT").build(), 10)
        .bid_ask(IgnoreSize::No)
        .expect("subscription should be created");

    let ticks_try: Vec<TickBidAsk> = subscription.try_iter().collect();
    assert_eq!(ticks_try.len(), 1);
    assert!(subscription.timeout_iter(std::time::Duration::from_millis(10)).next().is_none());
}

#[test]
fn test_tick_subscription_skips_unexpected_message_then_yields() {
    // First response is HistoricalData — fill_buffer should silently skip it
    // (the `Some(Ok(message))` non-match arm) and loop to the next batch.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(IncomingMessages::HistoricalData, historical_data_response().encode_proto()),
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                .tick(historical_tick_last(1_681_133_400, 14.00, 100, "NYSE"))
                .done(true)
                .encode_proto(),
        ),
    ]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);

    let subscription = client
        .historical_ticks(&Contract::stock("MSFT").build(), 1)
        .trade()
        .expect("subscription should be created");

    let tick = subscription.next().expect("should receive tick after skipping unexpected");
    assert_eq!(tick.price, 14.00, "wrong price");
    assert!(subscription.next().is_none(), "should be done");
}
