// Fixture-only responses for each tick type. Decoder is not exercised; the
// test asserts that the right wire request is sent.
const TRADE_RESPONSE: &str = "98\09000\01\01681133400\00\011.63\024547\0ISLAND\0 O X\01\0";
const BID_ASK_RESPONSE: &str = "97\09000\01\01681133399\00\011.63\011.83\02800\0100\01\0";
const MID_POINT_RESPONSE: &str = "96\09000\01\01681133398\00\091.36\00\01\0";

#[cfg(feature = "sync")]
mod sync_tests {
    use time::macros::datetime;

    use super::{BID_ASK_RESPONSE, MID_POINT_RESPONSE, TRADE_RESPONSE};
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::{IgnoreSize, TradingHours};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_ticks_request;

    #[test]
    fn trade_terminal_round_trip() {
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![TRADE_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 100)
            .starting(start)
            .trade()
            .expect("trade terminal failed");

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
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![MID_POINT_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
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
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![BID_ASK_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
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
        let (client, bus) =
            create_blocking_test_client_with_responses_and_version(vec![BID_ASK_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
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

    use super::{BID_ASK_RESPONSE, MID_POINT_RESPONSE, TRADE_RESPONSE};
    use crate::common::test_utils::helpers::{assert_request, create_test_client_with_responses_and_version, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::WhatToShow;
    use crate::market_data::{IgnoreSize, TradingHours};
    use crate::server_versions;
    use crate::testdata::builders::market_data::historical_ticks_request;

    #[tokio::test]
    async fn trade_terminal_round_trip() {
        let (client, bus) = create_test_client_with_responses_and_version(vec![TRADE_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
        let contract = Contract::stock("AAPL").build();
        let start = datetime!(2026-04-15 9:30:00 UTC);

        let _sub = client
            .historical_ticks(&contract, 100)
            .starting(start)
            .trade()
            .await
            .expect("trade terminal failed");

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
        let (client, bus) = create_test_client_with_responses_and_version(vec![MID_POINT_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
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
        let (client, bus) = create_test_client_with_responses_and_version(vec![BID_ASK_RESPONSE.to_owned()], server_versions::HISTORICAL_TICKS);
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
