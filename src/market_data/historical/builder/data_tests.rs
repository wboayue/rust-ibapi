use crate::common::test_utils::helpers::proto_response;
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::testdata::builders::market_data::{historical_data_bar, historical_data_end_response, historical_data_response};
use crate::testdata::builders::ResponseProtoEncoder;

/// Minimal HistoricalData + HistoricalDataEnd proto pair for happy-path
/// fetches. Tests in this file only need the call to succeed; the bar/end
/// payloads are not assertion targets.
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

        // 7 days in seconds = 604800.
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::seconds(604800))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
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

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::seconds(604800))
                .bar_size(BarSize::Hour)
                .what_to_show(Some(WhatToShow::Trades))
                .use_rth(true),
        );
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
