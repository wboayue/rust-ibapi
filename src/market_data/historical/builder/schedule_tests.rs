use crate::common::test_utils::helpers::proto_response;
use crate::messages::IncomingMessages;
use crate::messages::ResponseMessage;
use crate::testdata::builders::market_data::historical_schedule_response;
use crate::testdata::builders::ResponseProtoEncoder;

fn schedule_response() -> Vec<ResponseMessage> {
    vec![proto_response(
        IncomingMessages::HistoricalSchedule,
        historical_schedule_response().encode_proto(),
    )]
}

#[cfg(feature = "sync")]
mod sync_tests {
    use std::sync::Arc;
    use time::macros::datetime;

    use super::schedule_response;
    use crate::client::blocking::Client;
    use crate::common::test_utils::helpers::{assert_request, request_message_count};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;

    fn client_with_schedule() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(schedule_response()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    #[test]
    fn defaults_anchor_at_now() {
        let (client, bus) = client_with_schedule();
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
        let (client, bus) = client_with_schedule();
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
    use std::sync::Arc;
    use time::macros::datetime;

    use super::schedule_response;
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::assert_request;
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::historical_data_request;

    fn client_with_schedule() -> (Client, Arc<MessageBusStub>) {
        let bus = Arc::new(MessageBusStub::with_ordered_responses(schedule_response()));
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        (client, bus)
    }

    #[tokio::test]
    async fn defaults_anchor_at_now() {
        let (client, bus) = client_with_schedule();
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
        let (client, bus) = client_with_schedule();
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
