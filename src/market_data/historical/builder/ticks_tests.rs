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

// A stub built with ordered responses serves the same page to every request. When the page's
// earliest tick is after `start`, paging asks for the ticks before it; the repeated page then holds
// nothing before that end, and paging stops as at an empty page.
#[cfg(feature = "sync")]
mod sync_between_tests {
    use time::macros::datetime;
    use time::OffsetDateTime;

    use crate::common::test_utils::helpers::{
        assert_request, assert_tws_error_message, count_proto_msgs, create_blocking_test_client_with_ordered_proto_responses,
        create_blocking_test_client_with_responses_per_request, create_blocking_test_client_with_version, proto_error_response, proto_response,
        request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::{PagingStop, TickLast, WhatToShow};
    use crate::market_data::TradingHours;
    use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::{historical_ticks_last_response, historical_ticks_request, tick_last};
    use crate::testdata::builders::ResponseProtoEncoder;
    use crate::Error;

    const START: OffsetDateTime = datetime!(2026-11-13 14:26:44 UTC);
    const END: OffsetDateTime = datetime!(2026-11-13 14:29:59.5 UTC);

    // Trades at 14:28:35 and 14:29:59 UTC
    fn page() -> ResponseMessage {
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                .tick(tick_last(1_794_580_115, 7_100.25, 2.0, "CME"))
                .tick(tick_last(1_794_580_199, 7_101.50, 3.0, "CME"))
                .done(true)
                .encode_proto(),
        )
    }

    fn empty_page() -> ResponseMessage {
        proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response().done(true).encode_proto(),
        )
    }

    fn prices(ticks: &[TickLast]) -> Vec<f64> {
        ticks.iter().map(|tick| tick.price).collect()
    }

    // A failed page's subscription also writes a cancel when dropped, so only the requests are counted
    fn assert_request_ends(bus: &MessageBusStub, contract: &Contract, ends: &[OffsetDateTime]) {
        assert_eq!(
            count_proto_msgs(&bus.request_messages(), OutgoingMessages::RequestHistoricalTicks),
            ends.len()
        );
        for (index, end) in ends.iter().enumerate() {
            assert_request(
                bus,
                index,
                &historical_ticks_request()
                    .request_id(TEST_REQ_ID_FIRST + index as i32)
                    .contract(contract)
                    .end(Some(*end))
                    .number_of_ticks(1000)
                    .what_to_show(WhatToShow::Trades)
                    .use_rth(false),
            );
        }
    }

    #[test]
    fn between_pages_back_to_the_earliest_second() {
        let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![page()]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect("between failed");

        assert_eq!(prices(&result.ticks), vec![7_100.25, 7_101.50]);
        assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
        assert_eq!(result.stop, PagingStop::EmptyPage);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC), datetime!(2026-11-13 14:28:35 UTC)]);
    }

    #[test]
    fn between_stops_at_a_page_reaching_the_start() {
        let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
            IncomingMessages::HistoricalTickLast,
            historical_ticks_last_response()
                // 14:26:43, before the start
                .tick(tick_last(1_794_580_003, 7_099.75, 1.0, "CME"))
                .tick(tick_last(1_794_580_115, 7_100.25, 2.0, "CME"))
                .done(true)
                .encode_proto(),
        )]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect("between failed");

        assert_eq!(prices(&result.ticks), vec![7_100.25]);
        assert_eq!(result.from, Some(START));
        assert_eq!(result.stop, PagingStop::ReachedStart);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC)]);
    }

    #[test]
    fn between_stops_at_an_empty_first_page() {
        let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![empty_page()]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect("between failed");

        assert!(result.ticks.is_empty());
        assert_eq!(result.from, None);
        assert_eq!(result.stop, PagingStop::EmptyPage);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC)]);
    }

    #[test]
    fn between_stops_at_an_empty_page_after_a_page() {
        let (client, bus) = create_blocking_test_client_with_responses_per_request(vec![vec![page()], vec![empty_page()]]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect("between failed");

        assert_eq!(prices(&result.ticks), vec![7_100.25, 7_101.50]);
        assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
        assert_eq!(result.stop, PagingStop::EmptyPage);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC), datetime!(2026-11-13 14:28:35 UTC)]);
    }

    #[test]
    fn between_failing_on_the_first_page_keeps_nothing() {
        let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_error_response(
            TEST_REQ_ID_FIRST,
            162,
            "Historical Market Data Service error message:No market data permissions",
        )]);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect_err("between succeeded");

        assert!(error.ticks.is_empty());
        assert_eq!(error.from, None);
        assert_tws_error_message(Error::from(error), 162, "No market data permissions");
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC)]);
    }

    #[test]
    fn between_failing_after_a_page_keeps_its_ticks() {
        let (client, bus) = create_blocking_test_client_with_responses_per_request(vec![
            vec![page()],
            vec![proto_error_response(
                TEST_REQ_ID_FIRST + 1,
                162,
                "Historical Market Data Service error message:pacing violation",
            )],
        ]);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .trading_hours(TradingHours::Extended)
            .last()
            .expect_err("between succeeded");

        assert_eq!(prices(&error.ticks), vec![7_100.25, 7_101.50]);
        assert_eq!(error.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
        assert_tws_error_message(error.error, 162, "pacing violation");
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-13 14:30:00 UTC), datetime!(2026-11-13 14:28:35 UTC)]);
    }

    #[test]
    fn between_rejects_end_not_after_start() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let time = datetime!(2026-11-13 14:30 UTC);

        let error = client
            .historical_ticks(&contract, 1000)
            .between(time, time)
            .mid_point()
            .expect_err("end == start was accepted");

        assert!(matches!(error.error, Error::InvalidArgument(_)), "got {:?}", error.error);
        assert!(error.ticks.is_empty());
        assert_eq!(error.from, None);
        assert_eq!(request_message_count(&bus), 0);
    }
}

#[cfg(feature = "async")]
mod async_between_tests {
    use time::macros::datetime;
    use time::OffsetDateTime;

    use crate::common::test_utils::helpers::{
        assert_request, assert_tws_error_message, count_proto_msgs, create_test_client_with_ordered_proto_responses,
        create_test_client_with_responses_per_request, proto_error_response, proto_response, request_message_count, TEST_REQ_ID_FIRST,
    };
    use crate::contracts::Contract;
    use crate::market_data::historical::{PagingStop, TickMidpoint, WhatToShow};
    use crate::market_data::IgnoreSize;
    use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::{
        historical_ticks_bid_ask_response, historical_ticks_request, historical_ticks_response, tick_bid_ask, tick_midpoint,
    };
    use crate::testdata::builders::ResponseProtoEncoder;
    use crate::Error;

    const START: OffsetDateTime = datetime!(2026-11-23 09:16:00 UTC);
    const END: OffsetDateTime = datetime!(2026-11-23 09:17:01 UTC);

    // Midpoints at 09:16:59, 09:17:00 and 09:17:01 UTC. TWS sends one tick of a midpoint page's
    // earliest second, so the page is complete from 09:17:00.
    fn page() -> ResponseMessage {
        proto_response(
            IncomingMessages::HistoricalTick,
            historical_ticks_response()
                .tick(tick_midpoint(1_795_425_419, 7_100.125))
                .tick(tick_midpoint(1_795_425_420, 7_100.375))
                .tick(tick_midpoint(1_795_425_421, 7_100.625))
                .done(true)
                .encode_proto(),
        )
    }

    fn empty_page() -> ResponseMessage {
        proto_response(IncomingMessages::HistoricalTick, historical_ticks_response().done(true).encode_proto())
    }

    fn prices(ticks: &[TickMidpoint]) -> Vec<f64> {
        ticks.iter().map(|tick| tick.price).collect()
    }

    // A failed page's subscription also writes a cancel when dropped, so only the requests are counted
    fn assert_request_ends(bus: &MessageBusStub, contract: &Contract, ends: &[OffsetDateTime]) {
        assert_eq!(
            count_proto_msgs(&bus.request_messages(), OutgoingMessages::RequestHistoricalTicks),
            ends.len()
        );
        for (index, end) in ends.iter().enumerate() {
            assert_request(
                bus,
                index,
                &historical_ticks_request()
                    .request_id(TEST_REQ_ID_FIRST + index as i32)
                    .contract(contract)
                    .end(Some(*end))
                    .number_of_ticks(1000)
                    .what_to_show(WhatToShow::MidPoint)
                    .use_rth(true),
            );
        }
    }

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

        let result = client
            .historical_ticks(&contract, 1000)
            .between(datetime!(2026-11-13 14:26:44 UTC), datetime!(2026-11-13 14:29:59.5 UTC))
            .bid_ask(IgnoreSize::Yes)
            .await
            .expect("between failed");

        let bids: Vec<f64> = result.ticks.iter().map(|tick| tick.price_bid).collect();
        assert_eq!(bids, vec![7_100.00, 7_101.25]);
        assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
        assert_eq!(result.stop, PagingStop::EmptyPage);
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

    #[tokio::test]
    async fn between_stops_at_a_page_reaching_the_start() {
        let (client, bus) = create_test_client_with_ordered_proto_responses(vec![proto_response(
            IncomingMessages::HistoricalTick,
            historical_ticks_response()
                // 09:15:59, so the page is complete from 09:16:00, the start
                .tick(tick_midpoint(1_795_425_359, 7_099.875))
                .tick(tick_midpoint(1_795_425_360, 7_100.0))
                .tick(tick_midpoint(1_795_425_420, 7_100.375))
                .done(true)
                .encode_proto(),
        )]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .mid_point()
            .await
            .expect("between failed");

        assert_eq!(prices(&result.ticks), vec![7_100.0, 7_100.375]);
        assert_eq!(result.from, Some(START));
        assert_eq!(result.stop, PagingStop::ReachedStart);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-23 09:17:02 UTC)]);
    }

    #[tokio::test]
    async fn between_stops_at_an_empty_first_page() {
        let (client, bus) = create_test_client_with_ordered_proto_responses(vec![empty_page()]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .mid_point()
            .await
            .expect("between failed");

        assert!(result.ticks.is_empty());
        assert_eq!(result.from, None);
        assert_eq!(result.stop, PagingStop::EmptyPage);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-23 09:17:02 UTC)]);
    }

    #[tokio::test]
    async fn between_stops_at_an_empty_page_after_a_page() {
        let (client, bus) = create_test_client_with_responses_per_request(vec![vec![page()], vec![empty_page()]]);
        let contract = Contract::stock("AAPL").build();

        let result = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .mid_point()
            .await
            .expect("between failed");

        assert_eq!(prices(&result.ticks), vec![7_100.375, 7_100.625]);
        assert_eq!(result.from, Some(datetime!(2026-11-23 09:17:00 UTC)));
        assert_eq!(result.stop, PagingStop::EmptyPage);
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-23 09:17:02 UTC), datetime!(2026-11-23 09:17:00 UTC)]);
    }

    #[tokio::test]
    async fn between_failing_on_the_first_page_keeps_nothing() {
        let (client, bus) = create_test_client_with_ordered_proto_responses(vec![proto_error_response(
            TEST_REQ_ID_FIRST,
            162,
            "Historical Market Data Service error message:No market data permissions",
        )]);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .mid_point()
            .await
            .expect_err("between succeeded");

        assert!(error.ticks.is_empty());
        assert_eq!(error.from, None);
        assert_tws_error_message(Error::from(error), 162, "No market data permissions");
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-23 09:17:02 UTC)]);
    }

    #[tokio::test]
    async fn between_failing_after_a_page_keeps_its_ticks() {
        let (client, bus) = create_test_client_with_responses_per_request(vec![
            vec![page()],
            vec![proto_error_response(
                TEST_REQ_ID_FIRST + 1,
                162,
                "Historical Market Data Service error message:pacing violation",
            )],
        ]);
        let contract = Contract::stock("AAPL").build();

        let error = client
            .historical_ticks(&contract, 1000)
            .between(START, END)
            .mid_point()
            .await
            .expect_err("between succeeded");

        assert_eq!(prices(&error.ticks), vec![7_100.375, 7_100.625]);
        assert_eq!(error.from, Some(datetime!(2026-11-23 09:17:00 UTC)));
        assert_tws_error_message(error.error, 162, "pacing violation");
        assert_request_ends(&bus, &contract, &[datetime!(2026-11-23 09:17:02 UTC), datetime!(2026-11-23 09:17:00 UTC)]);
    }
}
