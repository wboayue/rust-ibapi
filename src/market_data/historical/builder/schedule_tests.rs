const SCHEDULE_RESPONSE: &str = "106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0";

#[cfg(feature = "sync")]
mod sync_tests {
    use time::macros::datetime;

    use super::SCHEDULE_RESPONSE;
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client_with_responses_and_version, request_message_count};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_data_request;

    #[test]
    fn defaults_anchor_at_now() {
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![SCHEDULE_RESPONSE.to_owned()], server_versions::HISTORICAL_SCHEDULE);
        let contract = Contract::stock("AAPL").build();

        client.historical_schedules(&contract, 7.days()).fetch().expect("fetch failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &historical_data_request()
                .contract(&contract)
                .end_date(None)
                .duration(Duration::days(7))
                .bar_size(BarSize::Day)
                .what_to_show(Some(WhatToShow::Schedule))
                .use_rth(true),
        );
    }

    #[test]
    fn ending_anchors_at_explicit_date() {
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![SCHEDULE_RESPONSE.to_owned()], server_versions::HISTORICAL_SCHEDULE);
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 0:00 UTC);

        client
            .historical_schedules(&contract, 30.days())
            .ending(end)
            .fetch()
            .expect("fetch failed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(30))
                .bar_size(BarSize::Day)
                .what_to_show(Some(WhatToShow::Schedule))
                .use_rth(true),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use time::macros::datetime;

    use super::SCHEDULE_RESPONSE;
    use crate::common::test_utils::helpers::{assert_request, create_test_client_with_responses_and_version};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_data_request;

    #[tokio::test]
    async fn defaults_anchor_at_now() {
        let (client, bus) = create_test_client_with_responses_and_version(vec![SCHEDULE_RESPONSE.to_owned()], server_versions::HISTORICAL_SCHEDULE);
        let contract = Contract::stock("AAPL").build();

        client.historical_schedules(&contract, 7.days()).fetch().await.expect("fetch failed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .contract(&contract)
                .end_date(None)
                .duration(Duration::days(7))
                .bar_size(BarSize::Day)
                .what_to_show(Some(WhatToShow::Schedule))
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn ending_anchors_at_explicit_date() {
        let (client, bus) = create_test_client_with_responses_and_version(vec![SCHEDULE_RESPONSE.to_owned()], server_versions::HISTORICAL_SCHEDULE);
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 0:00 UTC);

        client
            .historical_schedules(&contract, 30.days())
            .ending(end)
            .fetch()
            .await
            .expect("fetch failed");

        assert_request(
            &bus,
            0,
            &historical_data_request()
                .contract(&contract)
                .end_date(Some(end))
                .duration(Duration::days(30))
                .bar_size(BarSize::Day)
                .what_to_show(Some(WhatToShow::Schedule))
                .use_rth(true),
        );
    }
}
