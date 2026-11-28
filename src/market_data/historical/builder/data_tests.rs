use crate::common::test_utils::helpers::{proto_error_response, proto_response, TEST_REQ_ID_FIRST};
use crate::market_data::historical::{BarTimestamp, HistoricalData};
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::testdata::builders::market_data::{
    historical_data_bar, historical_data_end_response, historical_data_response, historical_data_update_response, HistoricalDataBarFields,
};
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

/// 10299 ahead of the bars: TWS substitutes the `whatToShow` the error names and sends that
/// series, as for crypto `TRADES` (live 2026-11-24).
const SUBSTITUTED_MESSAGE: &str = "Expected what to show is AGGTRADES, please use that instead of TRADES.";

fn substituted_response_pair() -> Vec<ResponseMessage> {
    let mut responses = vec![proto_error_response(TEST_REQ_ID_FIRST, 10299, SUBSTITUTED_MESSAGE)];
    responses.extend(historical_data_response_pair());
    responses
}

fn substituted_stream_responses() -> Vec<ResponseMessage> {
    let mut responses = substituted_response_pair();
    responses.push(proto_response(
        IncomingMessages::HistoricalDataUpdate,
        historical_data_update_response()
            .bar(historical_data_bar(1_678_890_000).ohlc(186.0, 186.5, 185.9, 186.25))
            .encode_proto(),
    ));
    responses
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

    #[test]
    fn subscribe_yields_10299_as_a_notice_then_the_bars() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_response_pair()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let subscription = client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe().unwrap();
        let notice = subscription.next();
        let bars = subscription.next();
        let end = subscription.next();
        let after = subscription.next();
        drop(subscription);

        assert!(
            matches!(&notice, Some(Ok(SubscriptionItem::Notice(n))) if n.code == 10299),
            "got {notice:?}"
        );
        assert!(
            matches!(&bars, Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(b)))) if b.len() == 1),
            "got {bars:?}"
        );
        assert!(
            matches!(&end, Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))),
            "got {end:?}"
        );
        assert!(after.is_none());
        assert_eq!(cancels(&bus), 0, "no cancel after End");
    }

    #[test]
    fn fetch_returns_the_bars_after_10299() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let data = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .fetch()
            .expect("10299 must not fail the fetch");

        assert_eq!(data.bars.len(), 1);
        assert_eq!(data.bars[0].close, 185.75);
    }

    #[test]
    fn stream_yields_10299_as_a_notice_then_bars_and_updates() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_stream_responses()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let subscription = client
            .historical_data(&contract, BarSize::Min)
            .duration(1.days())
            .subscribe_with_updates()
            .unwrap();
        let mut items = Vec::new();
        for _ in 0..4 {
            items.push(subscription.next());
        }

        assert!(
            matches!(&items[0], Some(Ok(SubscriptionItem::Notice(n))) if n.code == 10299),
            "got {:?}",
            items[0]
        );
        assert!(
            matches!(&items[1], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(b)))) if b.len() == 1),
            "got {:?}",
            items[1]
        );
        assert!(
            matches!(&items[2], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))),
            "got {:?}",
            items[2]
        );
        assert!(
            matches!(&items[3], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(b)))) if b.close == 186.25),
            "got {:?}",
            items[3]
        );
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

    #[tokio::test]
    async fn subscribe_yields_10299_as_a_notice_then_the_bars() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_response_pair()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .subscribe()
            .await
            .unwrap();
        let notice = subscription.next().await;
        let bars = subscription.next().await;
        let end = subscription.next().await;
        let after = subscription.next().await;
        drop(subscription);

        assert!(
            matches!(&notice, Some(Ok(SubscriptionItem::Notice(n))) if n.code == 10299),
            "got {notice:?}"
        );
        assert!(
            matches!(&bars, Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(b)))) if b.len() == 1),
            "got {bars:?}"
        );
        assert!(
            matches!(&end, Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))),
            "got {end:?}"
        );
        assert!(after.is_none());
        assert_eq!(cancels(&bus), 0, "no cancel after End");
    }

    #[tokio::test]
    async fn fetch_returns_the_bars_after_10299() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_response_pair()));
        let client = Client::stubbed(bus, server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let data = client
            .historical_data(&contract, BarSize::Hour)
            .duration(1.days())
            .fetch()
            .await
            .expect("10299 must not fail the fetch");

        assert_eq!(data.bars.len(), 1);
        assert_eq!(data.bars[0].close, 185.75);
    }

    #[tokio::test]
    async fn stream_yields_10299_as_a_notice_then_bars_and_updates() {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(super::substituted_stream_responses()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::crypto("BTC").build();

        let mut subscription = client
            .historical_data(&contract, BarSize::Min)
            .duration(1.days())
            .subscribe_with_updates()
            .await
            .unwrap();
        let mut items = Vec::new();
        for _ in 0..4 {
            items.push(subscription.next().await);
        }

        assert!(
            matches!(&items[0], Some(Ok(SubscriptionItem::Notice(n))) if n.code == 10299),
            "got {:?}",
            items[0]
        );
        assert!(
            matches!(&items[1], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(b)))) if b.len() == 1),
            "got {:?}",
            items[1]
        );
        assert!(
            matches!(&items[2], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })))),
            "got {:?}",
            items[2]
        );
        assert!(
            matches!(&items[3], Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(b)))) if b.close == 186.25),
            "got {:?}",
            items[3]
        );
    }
}
