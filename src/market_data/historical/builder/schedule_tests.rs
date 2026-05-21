// A minimal valid HistoricalSchedule response (msg type 106) so the
// `.fetch()` terminal returns Ok and the test can proceed to the request
// assertion. The schedule body is intentionally one minimal session.
const SCHEDULE_RESPONSE: &str = "106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0";

#[cfg(feature = "sync")]
mod sync_tests {
    use std::sync::{Arc, RwLock};
    use time::macros::datetime;

    use super::SCHEDULE_RESPONSE;
    use crate::client::blocking::Client;
    use crate::common::test_utils::helpers::{assert_request, request_message_count};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;

    fn stub_client() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![SCHEDULE_RESPONSE.to_owned()],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(bus.clone(), server_versions::HISTORICAL_SCHEDULE);
        (client, bus)
    }

    #[test]
    fn defaults_anchor_at_now() {
        let (client, bus) = stub_client();
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
        let (client, bus) = stub_client();
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 0:00 UTC);

        client.historical_schedules(&contract, 30.days()).ending(end).fetch().expect("fetch failed");

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
    use std::sync::{Arc, RwLock};
    use time::macros::datetime;

    use super::SCHEDULE_RESPONSE;
    use crate::common::test_utils::helpers::assert_request;
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::Client;

    fn stub_client() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![SCHEDULE_RESPONSE.to_owned()],
            ordered_responses: vec![],
        });
        let client = Client::stubbed(bus.clone(), server_versions::HISTORICAL_SCHEDULE);
        (client, bus)
    }

    #[tokio::test]
    async fn defaults_anchor_at_now() {
        let (client, bus) = stub_client();
        let contract = Contract::stock("AAPL").build();

        client
            .historical_schedules(&contract, 7.days())
            .fetch()
            .await
            .expect("fetch failed");

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
        let (client, bus) = stub_client();
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
