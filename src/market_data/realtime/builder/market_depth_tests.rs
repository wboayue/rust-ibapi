#[cfg(feature = "sync")]
mod sync_tests {
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client, request_message_count, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::SmartDepth;
    use crate::testdata::builders::market_data::market_depth_request;

    #[test]
    fn default_smart_depth_is_no() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.market_depth(&contract, 5).subscribe().expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &market_depth_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_rows(5)
                .smart_depth(false),
        );
    }

    #[test]
    fn smart_depth_yes() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .market_depth(&contract, 10)
            .smart_depth(SmartDepth::Yes)
            .subscribe()
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &market_depth_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_rows(10)
                .smart_depth(true),
        );
    }

    #[test]
    fn smart_depth_no_explicit() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .market_depth(&contract, 3)
            .smart_depth(SmartDepth::No)
            .subscribe()
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &market_depth_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_rows(3)
                .smart_depth(false),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::common::test_utils::helpers::{assert_request, create_test_client, request_message_count, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::SmartDepth;
    use crate::testdata::builders::market_data::market_depth_request;

    #[tokio::test]
    async fn default_smart_depth_is_no_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.market_depth(&contract, 5).subscribe().await.expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &market_depth_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_rows(5)
                .smart_depth(false),
        );
    }

    #[tokio::test]
    async fn smart_depth_yes_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .market_depth(&contract, 10)
            .smart_depth(SmartDepth::Yes)
            .subscribe()
            .await
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &market_depth_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .number_of_rows(10)
                .smart_depth(true),
        );
    }
}
