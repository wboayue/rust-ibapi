use crate::common::test_utils::helpers::{assert_request, proto_error_response, proto_response, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::Contract;
use crate::market_data::historical::{BarSize, BarTimestamp, Duration, HistoricalData, WhatToShow};
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::market_data::{
    historical_data_bar, historical_data_end_response, historical_data_request, historical_data_response, HistoricalDataBarFields,
};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::Error;
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
const SPLIT_START: OffsetDateTime = datetime!(2026-11-23 15:00 UTC);
const SPLIT_END: OffsetDateTime = datetime!(2026-11-23 16:30 UTC);
const SPLIT_REQUESTS: [(OffsetDateTime, i32); 3] = [
    (SPLIT_END, 2000),
    (datetime!(2026-11-23 15:56:40 UTC), 2000),
    (datetime!(2026-11-23 15:23:20 UTC), 1400),
];
const SPLIT_BARS: [OffsetDateTime; 6] = [
    datetime!(2026-11-23 15:00:00 UTC),
    datetime!(2026-11-23 15:23:19 UTC),
    datetime!(2026-11-23 15:23:20 UTC),
    datetime!(2026-11-23 15:56:39 UTC),
    datetime!(2026-11-23 15:56:40 UTC),
    datetime!(2026-11-23 16:29:59 UTC),
];
const NO_DATA_MESSAGE: &str = "Historical Market Data Service error message:HMDS query returned no data: AAPL@SMART Trades";
const PACING_MESSAGE: &str = "Historical Market Data Service error message:pacing violation";
const OTHER_SPLIT_ERRORS: [(i32, &str); 4] = [
    (162, PACING_MESSAGE),
    (162, "Historical Market Data Service error message:API historical data query cancelled"),
    (162, "Historical Market Data Service error message:invalid step: 1"),
    (200, NO_DATA_MESSAGE),
];

fn bars_at(times: &[OffsetDateTime]) -> Vec<ResponseMessage> {
    response_pair(
        times
            .iter()
            .map(|time| historical_data_bar(time.unix_timestamp()).ohlc(1.0, 1.0, 1.0, 1.0))
            .collect(),
    )
}

fn split_replies() -> Vec<Vec<ResponseMessage>> {
    vec![
        bars_at(&[datetime!(2026-11-23 15:56:40 UTC), datetime!(2026-11-23 16:29:59 UTC)]),
        bars_at(&[
            datetime!(2026-11-23 15:23:20 UTC),
            datetime!(2026-11-23 15:56:39 UTC),
            datetime!(2026-11-23 15:56:40 UTC),
        ]),
        bars_at(&[
            datetime!(2026-11-23 14:59:59 UTC),
            datetime!(2026-11-23 15:00:00 UTC),
            datetime!(2026-11-23 15:23:19 UTC),
        ]),
    ]
}

fn no_data_reply(request_offset: i32) -> Vec<ResponseMessage> {
    vec![proto_error_response(TEST_REQ_ID_FIRST + request_offset, 162, NO_DATA_MESSAGE)]
}

fn split_bus(replies: Vec<Vec<ResponseMessage>>) -> std::sync::Arc<MessageBusStub> {
    std::sync::Arc::new(MessageBusStub::with_responses_per_request(replies))
}

/// The first `count` requests of the split range, each under its own request id.
fn assert_split_requests(bus: &MessageBusStub, contract: &Contract, count: usize) {
    assert_eq!(request_message_count(bus), count);
    for (index, (end, seconds)) in SPLIT_REQUESTS.into_iter().take(count).enumerate() {
        assert_request(
            bus,
            index,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST + index as i32)
                .contract(contract)
                .end_date(Some(end))
                .duration(Duration::seconds(seconds))
                .bar_size(BarSize::Sec)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }
}

fn assert_split_bars(data: &HistoricalData, expected_dates: &[OffsetDateTime]) {
    let dates: Vec<BarTimestamp> = data.bars.iter().map(|bar| bar.date).collect();
    assert_eq!(dates, expected_dates.iter().copied().map(BarTimestamp::from).collect::<Vec<_>>());
    assert_eq!((data.start, data.end), (SPLIT_START, SPLIT_END));
}

fn assert_split_failure(result: Result<HistoricalData, Error>, code: i32, message: &str, expected_dates: &[OffsetDateTime]) {
    let Err(Error::BarsBetween(partial)) = result else {
        panic!("expected BarsBetween, got: {result:?}");
    };
    assert_notice_error(partial.error, TEST_REQ_ID_FIRST + 1, code, message);
    assert_eq!((partial.fetched.start, partial.fetched.end), (SPLIT_REQUESTS[1].0, SPLIT_END));
    let dates: Vec<_> = partial.fetched.bars.iter().map(|bar| bar.date).collect();
    assert_eq!(dates, expected_dates.iter().copied().map(BarTimestamp::from).collect::<Vec<_>>());
}

fn assert_notice_error(error: Error, request_id: i32, code: i32, message: &str) {
    let Error::Notice(notice) = error else {
        panic!("expected notice error, got: {error:?}");
    };
    assert_eq!(
        (notice.request_id, notice.code, notice.message.as_str()),
        (Some(request_id), code, message)
    );
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
    fn ending_before_duration_anchors_end_date() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 16:00:00 UTC);

        client
            .historical_data(&contract, BarSize::Hour)
            .ending(end)
            .duration(2.days())
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
                .use_rth(true),
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
    fn between_on_continuous_future_sends_no_end_date() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::continuous_futures("ES").build();
        let now = time::OffsetDateTime::now_utc();
        // 10 days less an hour back: 10 whole days plus one session, whatever the exact time.
        let start = now - time::Duration::days(10) + time::Duration::hours(1);

        client
            .historical_data(&contract, BarSize::Hour)
            .between(start, start + time::Duration::days(2))
            .fetch()
            .expect("fetch should succeed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(None)
                .duration(Duration::days(11))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[test]
    fn ending_on_continuous_future_errors() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::continuous_futures("ES").build();
        let ending = || {
            client
                .historical_data(&contract, BarSize::Hour)
                .duration(2.days())
                .ending(datetime!(2026-04-15 0:00 UTC))
        };

        let fetched = ending().fetch();
        assert!(
            matches!(fetched, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {fetched:?}"
        );
        let Err(err) = ending().subscribe() else {
            panic!("expected InvalidArgument")
        };
        assert!(matches!(err, Error::InvalidArgument(_)), "got: {err}");
        // Ending before duration reaches the same check.
        let reordered = client
            .historical_data(&contract, BarSize::Hour)
            .ending(datetime!(2026-04-15 0:00 UTC))
            .duration(2.days())
            .fetch();
        assert!(matches!(reordered, Err(Error::InvalidArgument(_))), "got: {reordered:?}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
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
    fn between_splits_one_second_range_over_requests() {
        let bus = super::split_bus(super::split_replies());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .expect("fetch should succeed");

        super::assert_split_requests(&bus, &contract, 3);
        super::assert_split_bars(&data, &super::SPLIT_BARS);
    }

    #[test]
    fn between_split_no_data_page_preserves_populated_pages() {
        let mut replies = super::split_replies();
        replies[1] = super::no_data_reply(1);
        let bus = super::split_bus(replies);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .expect("fetch should skip the no-data page");

        super::assert_split_requests(&bus, &contract, 3);
        let bars = super::SPLIT_BARS;
        super::assert_split_bars(&data, &[bars[0], bars[1], bars[4], bars[5]]);
    }

    #[test]
    fn between_split_no_data_on_every_page_returns_empty() {
        let bus = super::split_bus((0..3).map(super::no_data_reply).collect());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .expect("fetch should return the empty range");

        super::assert_split_requests(&bus, &contract, 3);
        super::assert_split_bars(&data, &[]);
    }

    #[test]
    fn between_split_pacing_error_after_empty_first_page_reports_progress() {
        let bus = super::split_bus(vec![
            super::no_data_reply(0),
            vec![proto_error_response(TEST_REQ_ID_FIRST + 1, 162, super::PACING_MESSAGE)],
        ]);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch();

        super::assert_split_failure(result, 162, super::PACING_MESSAGE, &[]);
        super::assert_split_requests(&bus, &contract, 2);
    }

    #[test]
    fn between_split_propagates_other_errors_with_partial_bars() {
        let contract = Contract::stock("AAPL").build();
        for (code, message) in super::OTHER_SPLIT_ERRORS {
            let replies = vec![
                super::split_replies().remove(0),
                vec![proto_error_response(TEST_REQ_ID_FIRST + 1, code, message)],
            ];
            let bus = super::split_bus(replies);
            let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);

            let result = client
                .historical_data(&contract, BarSize::Sec)
                .between(super::SPLIT_START, super::SPLIT_END)
                .fetch();

            super::assert_split_failure(result, code, message, &super::SPLIT_BARS[4..]);
            super::assert_split_requests(&bus, &contract, 2);
        }
    }

    #[test]
    fn between_single_request_propagates_no_data_error() {
        let bus = super::split_bus(vec![super::no_data_reply(0)]);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_REQUESTS[1].0, super::SPLIT_END)
            .fetch()
            .expect_err("fetch should return the original notice");

        super::assert_notice_error(error, TEST_REQ_ID_FIRST, 162, super::NO_DATA_MESSAGE);
        super::assert_split_requests(&bus, &contract, 1);
    }

    #[test]
    fn between_within_one_second_bar_limit_sends_one_request() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-11-23 16:30 UTC);

        client
            .historical_data(&contract, BarSize::Sec)
            .between(end - time::Duration::seconds(2000), end)
            .fetch()
            .expect("fetch should succeed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::seconds(2000))
                .bar_size(BarSize::Sec)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[test]
    fn between_one_request_cannot_serve_errors_up_front() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
        let stock = Contract::stock("AAPL").build();
        let future = Contract::continuous_futures("ES").build();
        let now = time::OffsetDateTime::now_utc();
        let (start, end) = (now - time::Duration::seconds(3000), now - time::Duration::seconds(2500));

        let Err(subscribed) = client
            .historical_data(&stock, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .subscribe()
        else {
            panic!("expected InvalidArgument")
        };
        let fetched = client.historical_data(&future, BarSize::Sec).between(start, end).fetch();
        let Err(future_subscribed) = client.historical_data(&future, BarSize::Sec).between(start, end).subscribe() else {
            panic!("expected InvalidArgument")
        };

        assert!(matches!(subscribed, Error::InvalidArgument(_)), "got: {subscribed}");
        assert!(matches!(fetched, Err(Error::InvalidArgument(_))), "got: {fetched:?}");
        assert!(matches!(future_subscribed, Error::InvalidArgument(_)), "got: {future_subscribed}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
    }

    #[test]
    fn between_needing_more_than_thirty_requests_errors_up_front() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let end = super::SPLIT_END;

        let at_limit = client
            .historical_data(&contract, BarSize::Sec)
            .between(end - time::Duration::seconds(60_001), end)
            .fetch();

        assert!(matches!(at_limit, Err(Error::InvalidArgument(_))), "got: {at_limit:?}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
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

        let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(bars)))) = subscription.next() else {
            panic!("expected bars first");
        };
        assert_eq!(bars.len(), 1);
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

    /// With updates, `End` is mid-stream: drop after it must still cancel, or
    /// the request keeps streaming at TWS.
    #[test]
    fn subscribe_with_updates_dropped_after_end_cancels() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe_with_updates()
            .unwrap();
        assert!(matches!(
            subscription.next(),
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(_))))
        ));
        assert!(matches!(
            subscription.next(),
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))
        ));
        drop(subscription);

        assert_eq!(cancels(&bus), 1, "End does not end a keep-up-to-date request");
    }

    /// With updates, only 200 is verified to end the request; 162 keeps the cancel.
    #[test]
    fn subscribe_with_updates_rejection_162_still_cancels() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
            TEST_REQ_ID_FIRST,
            162,
            "Historical Market Data Service error message:HMDS query returned no data",
        )]));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe_with_updates()
            .unwrap();
        assert!(matches!(subscription.next(), Some(Err(Error::Notice(notice))) if notice.code == 162));
        drop(subscription);

        assert_eq!(cancels(&bus), 1);
    }

    #[test]
    fn subscribe_rejects_invalid_arguments_unsent() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let adjusted_with_end = client
            .historical_data(&contract, BarSize::Hour)
            .what_to_show(WhatToShow::AdjustedLast)
            .duration(1.days())
            .ending(datetime!(2026-04-15 0:00 UTC))
            .subscribe();
        let zero_limit = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .buffer_limit(0)
            .subscribe();

        for result in [adjusted_with_end, zero_limit] {
            let Err(err) = result else { panic!("expected InvalidArgument") };
            assert!(matches!(err, Error::InvalidArgument(_)), "got: {err}");
        }
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
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
    async fn between_on_continuous_future_sends_no_end_date() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::continuous_futures("ES").build();
        let now = time::OffsetDateTime::now_utc();
        // 10 days less an hour back: 10 whole days plus one session, whatever the exact time.
        let start = now - time::Duration::days(10) + time::Duration::hours(1);

        client
            .historical_data(&contract, BarSize::Hour)
            .between(start, start + time::Duration::days(2))
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
                .duration(Duration::days(11))
                .bar_size(BarSize::Hour)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn ending_on_continuous_future_errors() {
        let (client, bus) = create_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::continuous_futures("ES").build();
        let ending = || {
            client
                .historical_data(&contract, BarSize::Hour)
                .duration(2.days())
                .ending(datetime!(2026-04-15 0:00 UTC))
        };

        let fetched = ending().fetch().await;
        assert!(
            matches!(fetched, Err(Error::InvalidArgument(_))),
            "expected InvalidArgument, got: {fetched:?}"
        );
        let Err(err) = ending().subscribe().await else {
            panic!("expected InvalidArgument")
        };
        assert!(matches!(err, Error::InvalidArgument(_)), "got: {err}");
        // Ending before duration reaches the same check.
        let reordered = client
            .historical_data(&contract, BarSize::Hour)
            .ending(datetime!(2026-04-15 0:00 UTC))
            .duration(2.days())
            .fetch()
            .await;
        assert!(matches!(reordered, Err(Error::InvalidArgument(_))), "got: {reordered:?}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
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
    async fn between_splits_one_second_range_over_requests() {
        let bus = super::split_bus(super::split_replies());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .await
            .expect("fetch should succeed");

        super::assert_split_requests(&bus, &contract, 3);
        super::assert_split_bars(&data, &super::SPLIT_BARS);
    }

    #[tokio::test(start_paused = true)]
    async fn between_split_no_data_page_preserves_populated_pages() {
        let mut replies = super::split_replies();
        replies[1] = super::no_data_reply(1);
        let bus = super::split_bus(replies);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .await
            .expect("fetch should skip the no-data page");

        super::assert_split_requests(&bus, &contract, 3);
        let bars = super::SPLIT_BARS;
        super::assert_split_bars(&data, &[bars[0], bars[1], bars[4], bars[5]]);
    }

    #[tokio::test(start_paused = true)]
    async fn between_split_no_data_on_every_page_returns_empty() {
        let bus = super::split_bus((0..3).map(super::no_data_reply).collect());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let data = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .await
            .expect("fetch should return the empty range");

        super::assert_split_requests(&bus, &contract, 3);
        super::assert_split_bars(&data, &[]);
    }

    #[tokio::test(start_paused = true)]
    async fn between_split_pacing_error_after_empty_first_page_reports_progress() {
        let bus = super::split_bus(vec![
            super::no_data_reply(0),
            vec![proto_error_response(TEST_REQ_ID_FIRST + 1, 162, super::PACING_MESSAGE)],
        ]);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .await;

        super::assert_split_failure(result, 162, super::PACING_MESSAGE, &[]);
        super::assert_split_requests(&bus, &contract, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn between_split_propagates_other_errors_with_partial_bars() {
        let contract = Contract::stock("AAPL").build();
        for (code, message) in super::OTHER_SPLIT_ERRORS {
            let replies = vec![
                super::split_replies().remove(0),
                vec![proto_error_response(TEST_REQ_ID_FIRST + 1, code, message)],
            ];
            let bus = super::split_bus(replies);
            let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);

            let result = client
                .historical_data(&contract, BarSize::Sec)
                .between(super::SPLIT_START, super::SPLIT_END)
                .fetch()
                .await;

            super::assert_split_failure(result, code, message, &super::SPLIT_BARS[4..]);
            super::assert_split_requests(&bus, &contract, 2);
        }
    }

    #[tokio::test]
    async fn between_single_request_propagates_no_data_error() {
        let bus = super::split_bus(vec![super::no_data_reply(0)]);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_REQUESTS[1].0, super::SPLIT_END)
            .fetch()
            .await
            .expect_err("fetch should return the original notice");

        super::assert_notice_error(error, TEST_REQ_ID_FIRST, 162, super::NO_DATA_MESSAGE);
        super::assert_split_requests(&bus, &contract, 1);
    }

    #[tokio::test]
    async fn between_within_one_second_bar_limit_sends_one_request() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-11-23 16:30 UTC);

        client
            .historical_data(&contract, BarSize::Sec)
            .between(end - time::Duration::seconds(2000), end)
            .fetch()
            .await
            .expect("fetch should succeed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::seconds(2000))
                .bar_size(BarSize::Sec)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn between_one_request_cannot_serve_errors_up_front() {
        let (client, bus) = create_test_client_with_version(server_versions::SIZE_RULES);
        let stock = Contract::stock("AAPL").build();
        let future = Contract::continuous_futures("ES").build();
        let now = time::OffsetDateTime::now_utc();
        let (start, end) = (now - time::Duration::seconds(3000), now - time::Duration::seconds(2500));

        let Err(subscribed) = client
            .historical_data(&stock, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .subscribe()
            .await
        else {
            panic!("expected InvalidArgument")
        };
        let fetched = client.historical_data(&future, BarSize::Sec).between(start, end).fetch().await;
        let Err(future_subscribed) = client.historical_data(&future, BarSize::Sec).between(start, end).subscribe().await else {
            panic!("expected InvalidArgument")
        };

        assert!(matches!(subscribed, Error::InvalidArgument(_)), "got: {subscribed}");
        assert!(matches!(fetched, Err(Error::InvalidArgument(_))), "got: {fetched:?}");
        assert!(matches!(future_subscribed, Error::InvalidArgument(_)), "got: {future_subscribed}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
    }

    #[tokio::test]
    async fn between_needing_more_than_thirty_requests_errors_up_front() {
        let (client, bus) = create_test_client_with_version(server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let end = super::SPLIT_END;

        let at_limit = client
            .historical_data(&contract, BarSize::Sec)
            .between(end - time::Duration::seconds(60_001), end)
            .fetch()
            .await;

        assert!(matches!(at_limit, Err(Error::InvalidArgument(_))), "got: {at_limit:?}");
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
    }

    /// Three split requests start at least 0.5 s apart (paused clock).
    #[tokio::test(start_paused = true)]
    async fn between_split_requests_are_spaced() {
        let bus = super::split_bus(super::split_replies());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();
        let started = tokio::time::Instant::now();

        client
            .historical_data(&contract, BarSize::Sec)
            .between(super::SPLIT_START, super::SPLIT_END)
            .fetch()
            .await
            .expect("split fetch");

        assert!(started.elapsed() >= std::time::Duration::from_secs(1), "took {:?}", started.elapsed());
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

        let Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(bars)))) = subscription.next().await else {
            panic!("expected bars first");
        };
        assert_eq!(bars.len(), 1);
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

    /// With updates, `End` is mid-stream: drop after it must still cancel, or
    /// the request keeps streaming at TWS.
    #[tokio::test]
    async fn subscribe_with_updates_dropped_after_end_cancels() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe_with_updates()
            .await
            .unwrap();
        assert!(matches!(
            subscription.next().await,
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(_))))
        ));
        assert!(matches!(
            subscription.next().await,
            Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))
        ));
        drop(subscription);
        settle().await;

        assert_eq!(cancels(&bus), 1, "End does not end a keep-up-to-date request");
    }

    /// With updates, only 200 is verified to end the request; 162 keeps the cancel.
    #[tokio::test]
    async fn subscribe_with_updates_rejection_162_still_cancels() {
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
            .subscribe_with_updates()
            .await
            .unwrap();
        assert!(matches!(subscription.next().await, Some(Err(Error::Notice(notice))) if notice.code == 162));
        drop(subscription);
        settle().await;

        assert_eq!(cancels(&bus), 1);
    }

    #[tokio::test]
    async fn subscribe_rejects_invalid_arguments_unsent() {
        let (client, bus) = client_with_data_pair();
        let contract = Contract::stock("AAPL").build();

        let adjusted_with_end = client
            .historical_data(&contract, BarSize::Hour)
            .what_to_show(WhatToShow::AdjustedLast)
            .duration(1.days())
            .ending(datetime!(2026-04-15 0:00 UTC))
            .subscribe()
            .await;
        let zero_limit = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .buffer_limit(0)
            .subscribe()
            .await;

        for result in [adjusted_with_end, zero_limit] {
            let Err(err) = result else { panic!("expected InvalidArgument") };
            assert!(matches!(err, Error::InvalidArgument(_)), "got: {err}");
        }
        assert_eq!(request_message_count(&bus), 0, "nothing sent");
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

mod bar_pages_tests {
    use super::super::BarPages;
    use crate::market_data::historical::{Bar, BarTimestamp, Duration};
    use time::macros::datetime;
    use time::OffsetDateTime;

    const LIMIT: time::Duration = time::Duration::seconds(2000);

    fn bars(times: &[OffsetDateTime]) -> Vec<Bar> {
        times
            .iter()
            .map(|time| Bar {
                date: BarTimestamp::DateTime(*time),
                ..Bar::default()
            })
            .collect()
    }

    fn dates(pages: BarPages) -> Vec<BarTimestamp> {
        pages.finish().bars.iter().map(|bar| bar.date).collect()
    }

    // TWS counts trading time, so a request ending Monday 14:40 returns Friday's last bars too
    // (live: 2026-11-23). Its earliest bar reaches back past `start`: one request covers the range.
    #[test]
    fn earliest_bar_before_start_ends_the_requests() {
        let mut pages = BarPages::new(datetime!(2026-11-20 20:50 UTC), datetime!(2026-11-23 14:40 UTC), LIMIT);

        assert_eq!(pages.next_request(), Some((datetime!(2026-11-23 14:40 UTC), Duration::seconds(2000))));
        pages.push(bars(&[
            datetime!(2026-11-20 20:36:40 UTC),
            datetime!(2026-11-20 20:59:59 UTC),
            datetime!(2026-11-23 14:30:00 UTC),
            datetime!(2026-11-23 14:39:59 UTC),
        ]));

        assert_eq!(pages.next_request(), None);
        assert_eq!(
            dates(pages),
            [
                datetime!(2026-11-20 20:59:59 UTC),
                datetime!(2026-11-23 14:30:00 UTC),
                datetime!(2026-11-23 14:39:59 UTC),
            ]
            .map(BarTimestamp::from)
        );
    }

    // An empty reply, or one whose earliest bar is later than one limit back (sparse bars), steps
    // back one limit: the request's trading-time duration covered at least that much wall clock.
    #[test]
    fn empty_or_sparse_reply_steps_back_one_limit() {
        let mut pages = BarPages::new(datetime!(2026-11-23 15:00 UTC), datetime!(2026-11-23 16:10 UTC), LIMIT);

        pages.push(Vec::new());
        assert_eq!(pages.next_request(), Some((datetime!(2026-11-23 15:36:40 UTC), Duration::seconds(2000))));
        pages.push(bars(&[datetime!(2026-11-23 15:05 UTC)]));
        assert_eq!(pages.next_request(), Some((datetime!(2026-11-23 15:03:20 UTC), Duration::seconds(200))));
        pages.push(bars(&[datetime!(2026-11-23 15:00 UTC)]));

        assert_eq!(pages.next_request(), None);
        assert_eq!(
            dates(pages),
            [datetime!(2026-11-23 15:00 UTC), datetime!(2026-11-23 15:05 UTC)].map(BarTimestamp::from)
        );
    }

    #[test]
    fn split_requests_start_at_least_half_a_second_apart() {
        use super::super::split_request_wait;
        use std::time::Duration as Wait;

        assert_eq!(split_request_wait(Wait::ZERO), Wait::from_millis(500));
        assert_eq!(split_request_wait(Wait::from_millis(200)), Wait::from_millis(300));
        assert_eq!(split_request_wait(Wait::from_secs(2)), Wait::ZERO);
    }

    // TWS rejects a duration under 30 S (error 321, live: 2026-11-23).
    #[test]
    fn last_request_asks_for_at_least_thirty_seconds() {
        let mut pages = BarPages::new(datetime!(2026-11-23 15:00:00 UTC), datetime!(2026-11-23 15:33:30 UTC), LIMIT);

        pages.push(bars(&[datetime!(2026-11-23 15:00:10 UTC)]));

        assert_eq!(pages.next_request(), Some((datetime!(2026-11-23 15:00:10 UTC), Duration::seconds(30))));
    }
}
