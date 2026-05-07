#[cfg(feature = "sync")]
mod sync_tests {
    use crate::client::sync::Client;
    use crate::common::test_utils::helpers::{assert_request, request_message_count, TEST_REQ_ID_FIRST};
    use crate::contracts::{Contract, TagValue};
    use crate::market_data::realtime::WhatToShow;
    use crate::market_data::TradingHours;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::realtime_bars_request;
    use std::sync::{Arc, RwLock};

    fn stubbed_client() -> (Arc<MessageBusStub>, Client) {
        let bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        (bus, client)
    }

    #[test]
    fn defaults_match_trades_regular_no_options() {
        let (bus, client) = stubbed_client();
        let contract = Contract::stock("AAPL").build();

        let _sub = client.realtime_bars(&contract).subscribe().expect("subscribe failed");

        assert_eq!(request_message_count(&bus), 1);
        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[test]
    fn override_what_to_show() {
        let (bus, client) = stubbed_client();
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
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::MidPoint)
                .use_rth(true),
        );
    }

    #[test]
    fn extended_hours_clears_use_rth() {
        let (bus, client) = stubbed_client();
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
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(false),
        );
    }

    #[test]
    fn options_round_trip() {
        let (bus, client) = stubbed_client();
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
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true)
                .options(options),
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use crate::client::r#async::Client;
    use crate::common::test_utils::helpers::{assert_request, TEST_REQ_ID_FIRST};
    use crate::contracts::{Contract, TagValue};
    use crate::market_data::realtime::WhatToShow;
    use crate::market_data::TradingHours;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::market_data::realtime_bars_request;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn defaults_async() {
        let bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();

        let _sub = client.realtime_bars(&contract).subscribe().await.expect("subscribe failed");

        assert_request(
            &bus,
            0,
            &realtime_bars_request()
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::Trades)
                .use_rth(true),
        );
    }

    #[tokio::test]
    async fn full_chain_async() {
        let bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
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
                .request_id(TEST_REQ_ID_FIRST)
                .contract(&contract)
                .what_to_show(WhatToShow::Bid)
                .use_rth(false)
                .options(options),
        );
    }
}
