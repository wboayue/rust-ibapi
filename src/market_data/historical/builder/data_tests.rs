use crate::common::test_utils::helpers::proto_response;
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::testdata::builders::market_data::{historical_data_bar, historical_data_end_response, historical_data_response};
use crate::testdata::builders::ResponseProtoEncoder;

fn historical_data_response_pair() -> Vec<ResponseMessage> {
    vec![
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
    ]
}

// Issue #835: IBKR counts durations in trading time, so `.between` over-fetches
// and must trim. Range [2026-04-15 00:00, 2026-04-16 00:00) UTC; the first bar
// (2026-04-14 19:00) is overshoot from the prior session.
const TRIM_RANGE_START: i64 = 1_776_211_200;
const TRIM_RANGE_END: i64 = 1_776_297_600;
const TRIM_BAR_BEFORE: i64 = 1_776_193_200;
const TRIM_BARS_INSIDE: [i64; 2] = [1_776_259_800, 1_776_279_600];

fn overshooting_response_pair() -> Vec<ResponseMessage> {
    let bars = std::iter::once(TRIM_BAR_BEFORE)
        .chain(TRIM_BARS_INSIDE)
        .map(|ts| historical_data_bar(ts).ohlc(1.0, 1.0, 1.0, 1.0))
        .collect();
    vec![
        proto_response(IncomingMessages::HistoricalData, historical_data_response().bars(bars).encode_proto()),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20260414 19:00:00 UTC")
                .end_date_str("20260416 00:00:00 UTC")
                .encode_proto(),
        ),
    ]
}

fn trim_range() -> (time::OffsetDateTime, time::OffsetDateTime) {
    (
        time::OffsetDateTime::from_unix_timestamp(TRIM_RANGE_START).unwrap(),
        time::OffsetDateTime::from_unix_timestamp(TRIM_RANGE_END).unwrap(),
    )
}

fn assert_trimmed(data: &crate::market_data::historical::HistoricalData) {
    let dates: Vec<i64> = data
        .bars
        .iter()
        .map(|bar| match bar.date {
            crate::market_data::historical::BarTimestamp::DateTime(dt) => dt.unix_timestamp(),
            other => panic!("expected intraday bar, got {other:?}"),
        })
        .collect();
    assert_eq!(dates, TRIM_BARS_INSIDE, "bars outside the requested range were not trimmed");
    assert_eq!((data.start, data.end), trim_range(), "window should report the requested range");
}

// `Subscription<T>` doesn't impl Debug, so `{:?}` formatting on `Result<Subscription<_>, _>`
// won't compile. These helpers match the Err arm manually for the .stream() terminals.
// Sync + async use their own variants because the `Subscription` type differs per feature.
#[cfg(feature = "sync")]
fn assert_stream_invalid_argument_sync(
    result: Result<crate::subscriptions::sync::Subscription<crate::market_data::historical::HistoricalBarUpdate>, crate::Error>,
) {
    let Err(err) = result else { panic!("expected InvalidArgument error") };
    assert!(matches!(err, crate::Error::InvalidArgument(_)), "expected InvalidArgument, got: {err}");
}

#[cfg(feature = "async")]
fn assert_stream_invalid_argument_async(
    result: Result<crate::subscriptions::r#async::Subscription<crate::market_data::historical::HistoricalBarUpdate>, crate::Error>,
) {
    let Err(err) = result else { panic!("expected InvalidArgument error") };
    assert!(matches!(err, crate::Error::InvalidArgument(_)), "expected InvalidArgument, got: {err}");
}

#[cfg(feature = "sync")]
mod sync_tests {
    use std::sync::Arc;
    use time::macros::datetime;

    use super::historical_data_response_pair;
    use crate::client::blocking::Client;
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::market_data::TradingHours;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::Error;

    fn client_with_data_pair() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    #[test]
    fn duration_defaults_ending_at_now() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        client
            .historical_data(&contract, BarSize::Hour)
            .duration(7.days())
            .fetch()
            .expect("fetch should succeed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(None)
                .duration(Duration::days(7))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
    }

    #[test]
    fn ending_anchors_end_date() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 16:00:00 UTC);

        client
            .historical_data(&contract, BarSize::Hour)
            .duration(2.days())
            .ending(end)
            .what_to_show(WhatToShow::MidPoint)
            .trading_hours(TradingHours::Extended)
            .fetch()
            .expect("fetch should succeed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(2))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::MidPoint))
                .use_rth(false),
        );
    }

    #[test]
    fn between_computes_duration_from_range() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-08 0:00 UTC);
        let end = datetime!(2026-04-15 0:00 UTC);

        client
            .historical_data(&contract, BarSize::Hour)
            .between(start, end)
            .fetch()
            .expect("fetch should succeed");

        // 7 days exceeds IBKR's 86400 S ceiling; `N D` is session-aligned, so one extra
        // session covers a range whose ends fall mid-session (issue #835).
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
    }

    #[test]
    fn between_trims_bars_outside_range() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::overshooting_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();
        let (start, end) = super::trim_range();

        let data = client
            .historical_data(&contract, BarSize::Hour)
            .between(start, end)
            .fetch()
            .expect("fetch should succeed");

        super::assert_trimmed(&data);
    }

    #[test]
    fn fetch_without_date_spec_errors() {
        let (client, _bus) = create_blocking_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client.historical_data(&contract, BarSize::Hour).fetch();

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[test]
    fn between_with_duration_errors() {
        let (client, _bus) = create_blocking_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-08 0:00 UTC);
        let end = datetime!(2026-04-15 0:00 UTC);

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .duration(7.days())
            .between(start, end)
            .fetch();

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[test]
    fn between_with_inverted_range_errors() {
        let (client, _bus) = create_blocking_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .between(datetime!(2026-04-15 0:00 UTC), datetime!(2026-04-08 0:00 UTC))
            .fetch();

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[test]
    fn stream_rejects_ending() {
        let (client, _bus) = create_blocking_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .ending(datetime!(2026-04-15 0:00 UTC))
            .stream();

        super::assert_stream_invalid_argument_sync(result);
    }

    #[test]
    fn stream_rejects_between() {
        let (client, _bus) = create_blocking_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
            .stream();

        super::assert_stream_invalid_argument_sync(result);
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use std::sync::Arc;
    use time::macros::datetime;

    use super::historical_data_response_pair;
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::{assert_request, create_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::Error;

    fn client_with_data_pair() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    #[tokio::test]
    async fn duration_defaults_ending_at_now() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        client
            .historical_data(&contract, BarSize::Hour)
            .duration(7.days())
            .fetch()
            .await
            .expect("fetch should succeed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(None)
                .duration(Duration::days(7))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn between_computes_duration_from_range() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-08 0:00 UTC);
        let end = datetime!(2026-04-15 0:00 UTC);

        client
            .historical_data(&contract, BarSize::Hour)
            .between(start, end)
            .fetch()
            .await
            .expect("fetch should succeed");

        // 7 days exceeds IBKR's 86400 S ceiling; `N D` is session-aligned, so one extra
        // session covers a range whose ends fall mid-session (issue #835).
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn between_trims_bars_outside_range() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::overshooting_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();
        let (start, end) = super::trim_range();

        let data = client
            .historical_data(&contract, BarSize::Hour)
            .between(start, end)
            .fetch()
            .await
            .expect("fetch should succeed");

        super::assert_trimmed(&data);
    }

    #[tokio::test]
    async fn fetch_without_date_spec_errors() {
        let (client, _bus) = create_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client.historical_data(&contract, BarSize::Hour).fetch().await;

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn stream_rejects_ending() {
        let (client, _bus) = create_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .ending(datetime!(2026-04-15 0:00 UTC))
            .stream()
            .await;

        super::assert_stream_invalid_argument_async(result);
    }

    #[tokio::test]
    async fn stream_rejects_between() {
        let (client, _bus) = create_test_client_with_responses_and_version(vec![], server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
            .stream()
            .await;

        super::assert_stream_invalid_argument_async(result);
    }
}

mod covering_duration_tests {
    use super::super::covering_duration;
    use crate::market_data::historical::Duration;
    use time::Duration as Span;

    #[test]
    fn up_to_one_day_uses_seconds() {
        assert_eq!(covering_duration(Span::hours(1)), Duration::seconds(3600));
        assert_eq!(covering_duration(Span::days(1)), Duration::seconds(86_400));
        assert_eq!(covering_duration(Span::milliseconds(500)), Duration::seconds(1));
    }

    // IBKR rejects `S` above 86400 (code 321); `D` is session-aligned, so one extra
    // session covers a range that starts mid-session.
    #[test]
    fn over_one_day_uses_days_plus_one_session() {
        assert_eq!(covering_duration(Span::days(1) + Span::seconds(1)), Duration::days(3));
        assert_eq!(covering_duration(Span::days(3)), Duration::days(4));
        assert_eq!(covering_duration(Span::days(364)), Duration::days(365));
    }

    // IBKR rejects `D` above 365 (code 321); `Y` is calendar years.
    #[test]
    fn over_365_sessions_uses_years() {
        assert_eq!(covering_duration(Span::days(365)), Duration::years(1));
        assert_eq!(covering_duration(Span::days(365) + Span::seconds(1)), Duration::years(2));
        assert_eq!(covering_duration(Span::days(3 * 365)), Duration::years(3));
    }
}
