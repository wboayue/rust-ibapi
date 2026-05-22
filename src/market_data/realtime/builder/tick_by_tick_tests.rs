#[cfg(feature = "sync")]
mod sync_tests {
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client, request_message_count, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::IgnoreSize;
    use crate::testdata::builders::market_data::tick_by_tick_request;

    #[test]
    fn last_sets_tick_type_last_and_ignore_size_false() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 5).last().expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("Last")
                .number_of_ticks(5)
                .ignore_size(false),
        );
    }

    #[test]
    fn all_last_sets_tick_type_all_last() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 7).all_last().expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("AllLast")
                .number_of_ticks(7)
                .ignore_size(false),
        );
    }

    #[test]
    fn bid_ask_ignore_size_yes() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 3).bid_ask(IgnoreSize::Yes).expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("BidAsk")
                .number_of_ticks(3)
                .ignore_size(true),
        );
    }

    #[test]
    fn bid_ask_ignore_size_no() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 3).bid_ask(IgnoreSize::No).expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("BidAsk")
                .number_of_ticks(3)
                .ignore_size(false),
        );
    }

    #[test]
    fn mid_point_sets_tick_type_midpoint() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 2).mid_point().expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("MidPoint")
                .number_of_ticks(2)
                .ignore_size(false),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::common::test_utils::helpers::{assert_request, create_test_client, request_message_count, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::IgnoreSize;
    use crate::testdata::builders::market_data::tick_by_tick_request;

    #[tokio::test]
    async fn last_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 5).last().await.expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("Last")
                .number_of_ticks(5)
                .ignore_size(false),
        );
    }

    #[tokio::test]
    async fn all_last_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 7).all_last().await.expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("AllLast")
                .number_of_ticks(7)
                .ignore_size(false),
        );
    }

    #[tokio::test]
    async fn bid_ask_ignore_size_yes_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .tick_by_tick(&contract, 3)
            .bid_ask(IgnoreSize::Yes)
            .await
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("BidAsk")
                .number_of_ticks(3)
                .ignore_size(true),
        );
    }

    #[tokio::test]
    async fn mid_point_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.tick_by_tick(&contract, 2).mid_point().await.expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &tick_by_tick_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .tick_type("MidPoint")
                .number_of_ticks(2)
                .ignore_size(false),
        );
    }
}
