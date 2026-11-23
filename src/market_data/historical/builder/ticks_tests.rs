// The decoder is not exercised; each test asserts that the right wire request
// is sent, so the stub needs no responses.
#[cfg(feature = "sync")]
mod sync_tests {
    use time::macros::datetime;

    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client_with_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::{IgnoreSize, TradingHours};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_ticks_request;

    #[test]
    fn last_terminal_round_trip() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 100)
            .starting(start)
            .last()
            .expect("last terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .start(Some(start))
                .number_of_ticks(100)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[test]
    fn mid_point_terminal_round_trip() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 16:00:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 50)
            .ending(end)
            .trading_hours(TradingHours::Extended)
            .mid_point()
            .expect("mid_point terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end(Some(end))
                .number_of_ticks(50)
                .what_to_show(WhatToShow::MidPoint)
                .use_rth(false),
        );
    }

    #[test]
    fn bid_ask_terminal_passes_ignore_size_yes() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 25)
            .starting(start)
            .bid_ask(IgnoreSize::Yes)
            .expect("bid_ask terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .start(Some(start))
                .number_of_ticks(25)
                .what_to_show(WhatToShow::BidAsk)
                .use_rth(true)
                .ignore_size(true),
        );
    }

    #[test]
    fn bid_ask_terminal_passes_ignore_size_no() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .historical_ticks(&contract, 10)
            .bid_ask(IgnoreSize::No)
            .expect("bid_ask terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_ticks(10)
                .what_to_show(WhatToShow::BidAsk)
                .use_rth(true)
                .ignore_size(false),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use time::macros::datetime;

    use crate::common::test_utils::helpers::{assert_request, create_test_client_with_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::{IgnoreSize, TradingHours};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_ticks_request;

    #[tokio::test]
    async fn last_terminal_round_trip() {
        let (client, bus) = create_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 100)
            .starting(start)
            .last()
            .await
            .expect("last terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .start(Some(start))
                .number_of_ticks(100)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn mid_point_terminal_round_trip() {
        let (client, bus) = create_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let end = datetime!(2026-04-15 16:00:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 50)
            .ending(end)
            .trading_hours(TradingHours::Extended)
            .mid_point()
            .await
            .expect("mid_point terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .end(Some(end))
                .number_of_ticks(50)
                .what_to_show(WhatToShow::MidPoint)
                .use_rth(false),
        );
    }

    #[tokio::test]
    async fn bid_ask_terminal_passes_ignore_size_yes() {
        let (client, bus) = create_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 25)
            .starting(start)
            .bid_ask(IgnoreSize::Yes)
            .await
            .expect("bid_ask terminal failed");

        assert_request(
            &bus,
            0,
            &historical_ticks_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .start(Some(start))
                .number_of_ticks(25)
                .what_to_show(WhatToShow::BidAsk)
                .use_rth(true)
                .ignore_size(true),
        );
    }
}

// The stub serves the same page to every request. The page's earliest tick is after `start`, so
// paging asks for the ticks before it; the repeated page then holds nothing before that end, and
// paging stops.
#[cfg(feature = "sync")]
mod sync_between_tests {
    use time::macros::datetime;

    use crate::common::test_utils::helpers::{
        assert_request, create_blocking_test_client_with_ordered_proto_responses, create_blocking_test_client_with_version, proto_response,
        request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::TradingHours;
    use crate::messages::IncomingMessages;
    use crate::server_versions;
    use crate::testdata::builders::market_data::{historical_ticks_last_response, historical_ticks_request, tick_last};
    use crate::testdata::builders::ResponseProtoEncoder;
    use crate::Error;

    #[test]
    fn between_pages_back_to_the_earliest_second() {
        let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                // 2026-11-13 14:28:35 and 14:29:59 UTC
                .tick(tick_last(1_794_580_115, 7_100.25, 2.0, "CME"))
                .tick(tick_last(1_794_580_199, 7_101.50, 3.0, "CME"))
                .done(true)
                .encode_proto(),
        )]);
        let contract = Contract::stock("AAPL").build();

        let ticks = client
            .historical_ticks(&contract, 1000)
            .between(datetime!(2026-11-13 14:26:44 UTC), datetime!(2026-11-13 14:29:59.5 UTC))
            .trading_hours(TradingHours::Extended)
            .last()
            .expect("between failed");

        let prices: Vec<f64> = ticks.iter().map(|tick| tick.price).collect();
        assert_eq!(prices, vec![7_100.25, 7_101.50]);
        assert_eq!(request_message_count(&bus), 2);
        for (index, end) in [datetime!(2026-11-13 14:30:00 UTC), datetime!(2026-11-13 14:28:35 UTC)]
            .into_iter()
            .enumerate()
        {
            assert_request(
                &bus,
                index,
                &historical_ticks_request()
                    .request_id(TEST_REQ_ID_FIRST + index as i32)
                    .contract(&contract)
                    .end(Some(end))
                    .number_of_ticks(1000)
                    .what_to_show(WhatToShow::Trades)
                    .use_rth(false),
            );
        }
    }

    #[test]
    fn between_rejects_end_not_after_start() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let time = datetime!(2026-11-13 14:30 UTC);

        let result = client.historical_ticks(&contract, 1000).between(time, time).mid_point();

        assert!(matches!(result, Err(Error::InvalidArgument(_))), "got {:?}", result.err());
        assert_eq!(request_message_count(&bus), 0);
    }
}

#[cfg(feature = "async")]
mod async_between_tests {
    use time::macros::datetime;

    use crate::common::test_utils::helpers::{
        assert_request, create_test_client_with_ordered_proto_responses, proto_response, request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::IgnoreSize;
    use crate::messages::IncomingMessages;
    use crate::testdata::builders::market_data::{historical_ticks_bid_ask_response, historical_ticks_request, tick_bid_ask};
    use crate::testdata::builders::ResponseProtoEncoder;

    #[tokio::test]
    async fn between_pages_back_to_the_earliest_second() {
        let (client, bus) = create_test_client_with_ordered_proto_responses(vec![proto_response(
            IncomingMessages::HistoricalTickBidAsk,
            historical_ticks_bid_ask_response()
                // 2026-11-13 14:28:35 and 14:29:59 UTC
                .tick(tick_bid_ask(1_794_580_115, 7_100.00, 7_100.25, 4.0, 5.0))
                .tick(tick_bid_ask(1_794_580_199, 7_101.25, 7_101.50, 6.0, 7.0))
                .done(true)
                .encode_proto(),
        )]);
        let contract = Contract::stock("AAPL").build();

        let ticks = client
            .historical_ticks(&contract, 1000)
            .between(datetime!(2026-11-13 14:26:44 UTC), datetime!(2026-11-13 14:29:59.5 UTC))
            .bid_ask(IgnoreSize::Yes)
            .await
            .expect("between failed");

        let bids: Vec<f64> = ticks.iter().map(|tick| tick.price_bid).collect();
        assert_eq!(bids, vec![7_100.00, 7_101.25]);
        // The earliest second of a bid/ask page is partial, so the second request ends after it;
        // the repeated page then holds only that second, kept as it is
        let ends = [
            datetime!(2026-11-13 14:30:00 UTC),
            datetime!(2026-11-13 14:28:36 UTC),
            datetime!(2026-11-13 14:28:35 UTC),
        ];
        assert_eq!(request_message_count(&bus), ends.len());
        for (index, end) in ends.into_iter().enumerate() {
            assert_request(
                &bus,
                index,
                &historical_ticks_request()
                    .request_id(TEST_REQ_ID_FIRST + index as i32)
                    .contract(&contract)
                    .end(Some(end))
                    .number_of_ticks(1000)
                    .what_to_show(WhatToShow::BidAsk)
                    .use_rth(true)
                    .ignore_size(true),
            );
        }
    }
}
