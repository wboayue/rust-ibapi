const HISTORICAL_DATA_RESPONSE: &str =
    "17\09000\020230315  09:30:00\020230315  10:30:00\01\01678886400\0185.50\0186.00\0185.25\0185.75\01000\0185.70\0100\0";

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
    use time::macros::datetime;

    use super::HISTORICAL_DATA_RESPONSE;
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::market_data::TradingHours;
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::Error;

    #[test]
    fn duration_defaults_ending_at_now() {
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![HISTORICAL_DATA_RESPONSE.to_owned()], server_versions::SIZE_RULES);
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
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![HISTORICAL_DATA_RESPONSE.to_owned()], server_versions::SIZE_RULES);
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
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![HISTORICAL_DATA_RESPONSE.to_owned()], server_versions::SIZE_RULES);
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
    use time::macros::datetime;

    use super::HISTORICAL_DATA_RESPONSE;
    use crate::common::test_utils::helpers::{assert_request, create_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::Error;

    #[tokio::test]
    async fn duration_defaults_ending_at_now() {
        let (client, bus) = create_test_client_with_responses_and_version(vec![HISTORICAL_DATA_RESPONSE.to_owned()], server_versions::SIZE_RULES);
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
        let (client, bus) = create_test_client_with_responses_and_version(vec![HISTORICAL_DATA_RESPONSE.to_owned()], server_versions::SIZE_RULES);
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
