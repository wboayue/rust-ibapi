#[cfg(feature = "sync")]
mod sync_tests {
    use crate::common::test_utils::helpers::{assert_request, create_blocking_test_client, request_message_count};
    use crate::contracts::{Contract, TagValue};
    use crate::market_data::realtime::WhatToShow;
    use crate::market_data::TradingHours;
    use crate::testdata::builders::market_data::realtime_bars_request;

    #[test]
    fn defaults_match_trades_regular_no_options() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.realtime_bars(&contract).subscribe().expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &realtime_bars_request().contract(&contract).what_to_show(WhatToShow::Trades).use_rth(true),
        );
    }

    #[test]
    fn override_what_to_show() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .realtime_bars(&contract)
            .what_to_show(WhatToShow::MidPoint)
            .subscribe()
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .contract(&contract)
                .what_to_show(WhatToShow::MidPoint)
                .use_rth(true),
        );
    }

    #[test]
    fn extended_hours_clears_use_rth() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client
            .realtime_bars(&contract)
            .trading_hours(TradingHours::Extended)
            .subscribe()
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(false),
        );
    }

    #[test]
    fn options_round_trip() {
        let (client, bus) = create_blocking_test_client();
        let contract = Contract::stock("AAPL").build();
        let options = vec![TagValue {
            tag: "XYZ".to_owned(),
            value: "1".to_owned(),
        }];

        let _sub = client
            .realtime_bars(&contract)
            .options(options.clone())
            .subscribe()
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true)
                .options(options),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::common::test_utils::helpers::{assert_request, create_test_client};
    use crate::contracts::{Contract, TagValue};
    use crate::market_data::realtime::WhatToShow;
    use crate::market_data::TradingHours;
    use crate::testdata::builders::market_data::realtime_bars_request;

    #[tokio::test]
    async fn defaults_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.realtime_bars(&contract).subscribe().await.expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request().contract(&contract).what_to_show(WhatToShow::Trades).use_rth(true),
        );
    }

    #[tokio::test]
    async fn full_chain_async() {
        let (client, bus) = create_test_client();
        let contract = Contract::stock("AAPL").build();
        let options = vec![TagValue {
            tag: "K".to_owned(),
            value: "V".to_owned(),
        }];

        let _sub = client
            .realtime_bars(&contract)
            .what_to_show(WhatToShow::Bid)
            .trading_hours(TradingHours::Extended)
            .options(options.clone())
            .subscribe()
            .await
            .expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .contract(&contract)
                .what_to_show(WhatToShow::Bid)
                .use_rth(false)
                .options(options),
        );
    }
}
