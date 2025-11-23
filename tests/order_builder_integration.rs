#[cfg(test)]
mod order_builder_tests {
    use ibapi::contracts::{Contract, Currency, Exchange, Symbol};
    use ibapi::orders::{Action, OrderBuilder};

    #[cfg(feature = "sync")]
    use ibapi::orders::OcaType;

    fn create_stock_contract(symbol: &str) -> Contract {
        Contract {
            symbol: Symbol::from(symbol),
            security_type: ibapi::contracts::SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        }
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_order_builder_basic_sync() {
        // This test verifies the builder API compiles and produces correct orders
        // It doesn't connect to TWS, just tests the builder logic

        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test market order
        let builder = OrderBuilder::new(&client, &contract).buy(100).market();

        let order = builder.build().unwrap();
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.order_type, "MKT");

        // Test limit order
        let builder = OrderBuilder::new(&client, &contract)
            .sell(200)
            .limit(150.50)
            .good_till_cancel()
            .hidden()
            .outside_rth();

        let order = builder.build().unwrap();
        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.total_quantity, 200.0);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(150.50));
        assert_eq!(order.tif, ibapi::orders::TimeInForce::GoodTilCanceled);
        assert!(order.hidden);
        assert!(order.outside_rth);

        // Test stop-limit order
        let builder = OrderBuilder::new(&client, &contract).buy(50).stop_limit(45.0, 45.50).account("DU123456");

        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP LMT");
        assert_eq!(order.aux_price, Some(45.0)); // Stop price
        assert_eq!(order.limit_price, Some(45.50));
        assert_eq!(order.account, "DU123456");
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_bracket_order_builder_sync() {
        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test bracket order
        let bracket_builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);

        let orders = bracket_builder.build().unwrap();
        assert_eq!(orders.len(), 3);

        // Parent order
        let parent = &orders[0];
        assert_eq!(parent.action, Action::Buy);
        assert_eq!(parent.order_type, "LMT");
        assert_eq!(parent.total_quantity, 100.0);
        assert_eq!(parent.limit_price, Some(50.0));
        assert!(!parent.transmit);

        // Take profit order
        let take_profit = &orders[1];
        assert_eq!(take_profit.action, Action::Sell);
        assert_eq!(take_profit.order_type, "LMT");
        assert_eq!(take_profit.total_quantity, 100.0);
        assert_eq!(take_profit.limit_price, Some(55.0));
        assert!(!take_profit.transmit);

        // Stop loss order
        let stop_loss = &orders[2];
        assert_eq!(stop_loss.action, Action::Sell);
        assert_eq!(stop_loss.order_type, "STP");
        assert_eq!(stop_loss.total_quantity, 100.0);
        assert_eq!(stop_loss.aux_price, Some(45.0));
        assert!(stop_loss.transmit);
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_advanced_order_types_sync() {
        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test trailing stop
        let builder = OrderBuilder::new(&client, &contract).sell(100).trailing_stop(5.0, 95.0);

        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "TRAIL");
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(95.0));

        // Test discretionary order
        let builder = OrderBuilder::new(&client, &contract).buy(100).discretionary(50.0, 0.10);

        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.discretionary_amt, 0.10);

        // Test sweep to fill
        let builder = OrderBuilder::new(&client, &contract).buy(100).sweep_to_fill(50.0);

        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.sweep_to_fill);

        // Test midprice order
        let builder = OrderBuilder::new(&client, &contract).buy(100).midprice(Some(50.0));

        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.limit_price, Some(50.0));
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_order_validation_sync() {
        use ibapi::orders::builder::ValidationError;

        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test missing action
        let builder = OrderBuilder::new(&client, &contract).market();
        assert!(builder.build().is_err());

        // Test missing quantity
        let builder = OrderBuilder::new(&client, &contract).market();
        assert!(builder.build().is_err());

        // Test invalid quantity
        let builder = OrderBuilder::new(&client, &contract).buy(-100).market();

        let result = builder.build();
        assert!(result.is_err());
        if let Err(ValidationError::InvalidQuantity(q)) = result {
            assert_eq!(q, -100.0);
        } else {
            panic!("Expected InvalidQuantity error");
        }

        // Test invalid price (NaN)
        let builder = OrderBuilder::new(&client, &contract).buy(100).limit(f64::NAN);

        let result = builder.build();
        assert!(result.is_err());
        if let Err(ValidationError::InvalidPrice(p)) = result {
            assert!(p.is_nan());
        } else {
            panic!("Expected InvalidPrice error");
        }

        // Test missing limit price for limit order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(ibapi::orders::builder::OrderType::Limit);

        let result = builder.build();
        assert!(result.is_err());
        if let Err(ValidationError::MissingRequiredField(field)) = result {
            assert_eq!(field, "limit_price");
        } else {
            panic!("Expected MissingRequiredField error");
        }
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_oca_order_builder_sync() {
        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test OCA group
        let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.0).oca_group("TestOCA", 1);

        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "TestOCA");
        assert_eq!(order.oca_type, OcaType::CancelWithBlock);
    }

    #[test]
    #[cfg(feature = "sync")]
    fn test_algo_order_builder_sync() {
        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test algorithmic order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.0)
            .algo("VWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00");

        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "VWAP");
        assert_eq!(order.algo_params.len(), 2);
        assert_eq!(order.algo_params[0].tag, "startTime");
        assert_eq!(order.algo_params[0].value, "09:30:00");
        assert_eq!(order.algo_params[1].tag, "endTime");
        assert_eq!(order.algo_params[1].value, "16:00:00");
    }

    #[tokio::test]
    #[cfg(feature = "async")]
    async fn test_order_builder_basic_async() {
        struct MockClient;

        let client = MockClient;
        let contract = create_stock_contract("AAPL");

        // Test market order
        let builder = OrderBuilder::new(&client, &contract).buy(100).market();

        let order = builder.build().unwrap();
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.order_type, "MKT");

        // Test limit order
        let builder = OrderBuilder::new(&client, &contract).sell(200).limit(150.50).good_till_cancel();

        let order = builder.build().unwrap();
        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.total_quantity, 200.0);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(150.50));
        assert_eq!(order.tif, ibapi::orders::TimeInForce::GoodTilCanceled);
    }
}
