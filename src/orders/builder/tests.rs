// Integration tests for the order builder module
// These tests span multiple modules and test interaction between components

#[cfg(all(test, feature = "sync"))]
mod sync_integration_tests {
    use crate::common::test_utils::helpers::create_blocking_test_client;
    use crate::contracts::{Contract, Currency, Exchange, Symbol};
    use crate::orders::OrderBuilder;
    use crate::orders::{Action, OcaType, TimeInForce};

    fn create_stock_contract(symbol: &str) -> Contract {
        Contract {
            symbol: Symbol::from(symbol),
            security_type: crate::contracts::SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        }
    }

    #[test]
    fn test_full_order_workflow() {
        let (client, _bus) = create_blocking_test_client();
        let contract = create_stock_contract("AAPL");

        // Create multiple orders
        let orders = [
            OrderBuilder::new(&client, &contract).buy(100).market().build().unwrap(),
            OrderBuilder::new(&client, &contract).sell(50).limit(150.00).build().unwrap(),
            OrderBuilder::new(&client, &contract).buy(200).stop_limit(145.00, 148.00).build().unwrap(),
        ];

        // Verify orders have correct properties
        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[0].order_type, "MKT");
        assert_eq!(orders[0].total_quantity, 100.0);

        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[1].order_type, "LMT");
        assert_eq!(orders[1].limit_price, Some(150.00));

        assert_eq!(orders[2].order_type, "STP LMT");
        assert_eq!(orders[2].aux_price, Some(145.00));
        assert_eq!(orders[2].limit_price, Some(148.00));
    }

    #[test]
    fn test_complex_order_combinations() {
        let (client, _bus) = create_blocking_test_client();
        let contract = create_stock_contract("MSFT");

        // Test complex order with multiple attributes
        let order = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .hidden()
            .outside_rth()
            .good_till_date("20240630 23:59:59")
            .account("TEST123")
            .algo("VWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00")
            .oca_group("TestGroup", 1)
            .build()
            .unwrap();

        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
        assert!(order.hidden);
        assert!(order.outside_rth);
        assert_eq!(order.tif, TimeInForce::GoodTilDate);
        assert_eq!(order.good_till_date, "20240630 23:59:59");
        assert_eq!(order.account, "TEST123");
        assert_eq!(order.algo_strategy, "VWAP");
        assert_eq!(order.algo_params.len(), 2);
        assert_eq!(order.oca_group, "TestGroup");
        assert_eq!(order.oca_type, OcaType::CancelWithBlock);
    }
}

#[cfg(all(test, feature = "async"))]
mod async_integration_tests {
    use crate::common::test_utils::helpers::create_test_client;
    use crate::contracts::{Contract, Currency, Exchange, Symbol};
    use crate::orders::Action;
    use crate::orders::OrderBuilder;

    fn create_stock_contract(symbol: &str) -> Contract {
        Contract {
            symbol: Symbol::from(symbol),
            security_type: crate::contracts::SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_async_full_order_workflow() {
        let (client, _bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        // Create multiple orders
        let orders = [
            OrderBuilder::new(&client, &contract).buy(100).market().build().unwrap(),
            OrderBuilder::new(&client, &contract).sell(50).limit(150.00).build().unwrap(),
            OrderBuilder::new(&client, &contract).buy(200).stop_limit(145.00, 148.00).build().unwrap(),
        ];

        // Verify orders have correct properties
        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[0].order_type, "MKT");
        assert_eq!(orders[0].total_quantity, 100.0);

        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[1].order_type, "LMT");
        assert_eq!(orders[1].limit_price, Some(150.00));

        assert_eq!(orders[2].order_type, "STP LMT");
        assert_eq!(orders[2].aux_price, Some(145.00));
        assert_eq!(orders[2].limit_price, Some(148.00));
    }
}
