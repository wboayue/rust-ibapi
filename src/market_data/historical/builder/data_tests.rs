use crate::common::test_utils::helpers::proto_response;
use crate::market_data::historical::{BarTimestamp, HistoricalData};
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::testdata::builders::market_data::{historical_data_bar, historical_data_end_response, historical_data_response, HistoricalDataBarFields};
use crate::testdata::builders::ResponseProtoEncoder;
use time::macros::datetime;
use time::OffsetDateTime;

fn historical_data_response_pair() -> Vec<ResponseMessage> {
    response_pair(vec![historical_data_bar(1_678_886_400)
        .ohlc(185.50, 186.00, 185.25, 185.75)
        .volume(1000.0)
        .wap(185.70)
        .count(100)])
}

fn response_pair(bars: Vec<HistoricalDataBarFields>) -> Vec<ResponseMessage> {
    vec![
        proto_response(IncomingMessages::HistoricalData, historical_data_response().bars(bars).encode_proto()),
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
// and must trim. The first bar is overshoot from the prior session.
const TRIM_START: OffsetDateTime = datetime!(2026-04-15 00:00 UTC);
const TRIM_END: OffsetDateTime = datetime!(2026-04-16 00:00 UTC);
const TRIM_BAR_BEFORE: OffsetDateTime = datetime!(2026-04-14 19:00 UTC);
const TRIM_BARS_INSIDE: [OffsetDateTime; 2] = [datetime!(2026-04-15 13:30 UTC), datetime!(2026-04-15 19:00 UTC)];

fn overshooting_response_pair() -> Vec<ResponseMessage> {
    response_pair(
        std::iter::once(TRIM_BAR_BEFORE)
            .chain(TRIM_BARS_INSIDE)
            .map(|dt| historical_data_bar(dt.unix_timestamp()).ohlc(1.0, 1.0, 1.0, 1.0))
            .collect(),
    )
}

fn assert_trimmed(data: &HistoricalData) {
    let dates: Vec<BarTimestamp> = data.bars.iter().map(|bar| bar.date).collect();
    assert_eq!(
        dates,
        TRIM_BARS_INSIDE.map(BarTimestamp::from),
        "bars outside the requested range were not trimmed"
    );
    assert_eq!((data.start, data.end), (TRIM_START, TRIM_END), "window should report the requested range");
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

    use std::time::{Duration as StdDuration, Instant};

    use super::historical_data_response_pair;
    use crate::client::blocking::Client;
    use crate::common::test_utils::helpers::{
        assert_request, count_proto_msgs, create_blocking_test_client_with_version, proto_error_response, request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, HistoricalDataFrame, ToDuration, WhatToShow};
    use crate::market_data::TradingHours;
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::subscriptions::{Drained, SubscriptionItem};
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
                .what_to_show(WhatToShow::Trades)
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
                .what_to_show(WhatToShow::MidPoint)
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

        // Unit choice is covered by `covering_duration_tests`; this pins the wiring.
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[test]
    fn between_trims_bars_outside_range() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::overshooting_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();
        let data = client
            .historical_data(&contract, BarSize::Hour)
            .between(super::TRIM_START, super::TRIM_END)
            .fetch()
            .expect("fetch should succeed");

        super::assert_trimmed(&data);
    }

    #[test]
    fn fetch_without_date_spec_errors() {
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client.historical_data(&contract, BarSize::Hour).fetch();

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[test]
    fn between_with_duration_errors() {
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
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
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
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
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
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
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
            .stream();

        super::assert_stream_invalid_argument_sync(result);
    }

    // ---- subscribe ----

    fn bars_only_client() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()[..1].to_vec()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    fn cancels(bus: &MessageBusStub) -> usize {
        count_proto_msgs(&bus.request_messages(), OutgoingMessages::CancelHistoricalData)
    }

    #[test]
    fn subscribe_sends_once_under_request_id() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let (start, end) = (datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC));

        let request = client.historical_data(&contract, BarSize::Hour).between(start, end).buffer_limit(8);
        let request_id = request.request_id();
        assert_eq!(request_message_count(&bus), 0, "building the request sends nothing");

        let _subscription = request.subscribe().expect("subscribe failed");
        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(request_id)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true)
                .keep_up_to_date(false),
        );
        assert_eq!(*bus.buffer_limits.read().unwrap(), vec![8]);
    }

    #[test]
    fn subscribe_yields_bars_then_end_without_cancel() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let subscription = client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe().unwrap();

        let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(data)))) = subscription.next() else {
            panic!("expected bars first");
        };
        assert_eq!(data.bars.len(), 1);
        assert!(matches!(
            subscription.next(),
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { start, end }))) if end > start
        ));
        assert!(subscription.next().is_none(), "nothing follows End");

        drop(subscription);
        assert_eq!(cancels(&bus), 0, "no cancel after End");
    }

    #[test]
    fn subscribe_dropped_before_end_cancels() {
        let (client, bus) = bars_only_client();
        let contract = Contract::stock("AAPL").build();

        let subscription = client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe().unwrap();
        assert!(matches!(
            subscription.next(),
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(_))))
        ));
        drop(subscription);

        assert_eq!(cancels(&bus), 1);
    }

    #[test]
    fn subscribe_rejection_read_then_drained_writes_no_cancel() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
            TEST_REQ_ID_FIRST,
            162,
            "Historical Market Data Service error message:HMDS query returned no data",
        )]));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let subscription = client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe().unwrap();
        assert!(matches!(subscription.next(), Some(Err(Error::Notice(notice))) if notice.code == 162));
        let outcome = subscription.cancel_and_drain(Instant::now() + StdDuration::from_secs(2)).unwrap();

        assert!(matches!(outcome, Drained::Rejected(notice) if notice.code == 162));
        assert_eq!(cancels(&bus), 0, "the rejection ended the request at TWS");
    }

    #[test]
    fn subscribe_connection_reset_is_not_resent() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()).with_connection_resets(1));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let subscription = client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe().unwrap();

        assert!(matches!(subscription.next(), Some(Err(Error::ConnectionReset))));
        assert!(subscription.next().is_none());
        assert_eq!(request_message_count(&bus), 1, "subscribe never resends");
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use std::sync::Arc;
    use time::macros::datetime;

    use futures::StreamExt;
    use std::time::Duration as StdDuration;
    use tokio::time::Instant;

    use super::historical_data_response_pair;
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::{
        assert_request, count_proto_msgs, create_test_client_with_version, proto_error_response, request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, HistoricalDataFrame, ToDuration, WhatToShow};
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::subscriptions::{Drained, SubscriptionItem};
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
                .what_to_show(WhatToShow::Trades)
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

        // Unit choice is covered by `covering_duration_tests`; this pins the wiring.
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn between_trims_bars_outside_range() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::overshooting_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();
        let data = client
            .historical_data(&contract, BarSize::Hour)
            .between(super::TRIM_START, super::TRIM_END)
            .fetch()
            .await
            .expect("fetch should succeed");

        super::assert_trimmed(&data);
    }

    #[tokio::test]
    async fn fetch_without_date_spec_errors() {
        let (client, _bus) = create_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client.historical_data(&contract, BarSize::Hour).fetch().await;

        assert!(
            matches!(result, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn stream_rejects_ending() {
        let (client, _bus) = create_test_client_with_version(server_versions::SIZE_RULES);
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
        let (client, _bus) = create_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Hour)
            .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
            .stream()
            .await;

        super::assert_stream_invalid_argument_async(result);
    }

    // ---- subscribe ----

    fn bars_only_client() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()[..1].to_vec()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    /// Let a drop's spawned cancel run.
    async fn settle() {
        tokio::time::sleep(StdDuration::from_millis(20)).await;
    }

    fn cancels(bus: &MessageBusStub) -> usize {
        count_proto_msgs(&bus.request_messages(), OutgoingMessages::CancelHistoricalData)
    }

    #[tokio::test]
    async fn subscribe_sends_once_under_request_id() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let (start, end) = (datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC));

        let request = client.historical_data(&contract, BarSize::Hour).between(start, end).buffer_limit(8);
        let request_id = request.request_id();
        assert_eq!(request_message_count(&bus), 0, "building the request sends nothing");

        let _subscription = request.subscribe().await.expect("subscribe failed");
        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(request_id)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(8))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true)
                .keep_up_to_date(false),
        );
        assert_eq!(*bus.buffer_limits.read().unwrap(), vec![8]);
    }

    #[tokio::test]
    async fn subscribe_yields_bars_then_end_without_cancel() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe()
            .await
            .unwrap();

        let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(data)))) = subscription.next().await else {
            panic!("expected bars first");
        };
        assert_eq!(data.bars.len(), 1);
        assert!(matches!(
            subscription.next().await,
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { start, end }))) if end > start
        ));
        assert!(subscription.next().await.is_none(), "nothing follows End");

        drop(subscription);
        settle().await;
        assert_eq!(cancels(&bus), 0, "no cancel after End");
    }

    #[tokio::test]
    async fn subscribe_dropped_before_end_cancels() {
        let (client, bus) = bars_only_client();
        let contract = Contract::stock("AAPL").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe()
            .await
            .unwrap();
        assert!(matches!(
            subscription.next().await,
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(_))))
        ));
        drop(subscription);
        settle().await;

        assert_eq!(cancels(&bus), 1);
    }

    #[tokio::test]
    async fn subscribe_rejection_read_then_drained_writes_no_cancel() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
            TEST_REQ_ID_FIRST,
            162,
            "Historical Market Data Service error message:HMDS query returned no data",
        )]));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe()
            .await
            .unwrap();
        assert!(matches!(subscription.next().await, Some(Err(Error::Notice(notice))) if notice.code == 162));
        let outcome = subscription.cancel_and_drain(Instant::now() + StdDuration::from_secs(2)).await.unwrap();

        assert!(matches!(outcome, Drained::Rejected(notice) if notice.code == 162));
        settle().await;
        assert_eq!(cancels(&bus), 0, "the rejection ended the request at TWS");
    }

    #[tokio::test]
    async fn subscribe_connection_reset_is_not_resent() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(historical_data_response_pair()).with_connection_resets(1));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe()
            .await
            .unwrap();

        assert!(matches!(subscription.next().await, Some(Err(Error::ConnectionReset))));
        assert!(subscription.next().await.is_none());
        assert_eq!(request_message_count(&bus), 1, "subscribe never resends");
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
