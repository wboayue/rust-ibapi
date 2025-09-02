#[cfg(all(test, feature = "sync"))]
mod tests {
    use crate::client::sync::Client;
    use crate::common::test_utils::helpers::*;
    use crate::contracts::Contract;
    use crate::orders::{Action, Order, OrderBuilder, OrderState, OrderData, PlaceOrder};

    fn create_stock_contract(symbol: &str) -> Contract {
        let mut contract = Contract::default();
        contract.symbol = symbol.to_string();
        contract.security_type = crate::contracts::SecurityType::Stock;
        contract.exchange = "SMART".to_string();
        contract.currency = "USD".to_string();
        contract
    }

    #[test]
    fn test_order_builder_submit() {
        let (client, message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        // Mock the next_order_id response
        let mut responses = vec![];
        responses.push("9\x008\x00100\x00".to_string()); // NextValidId message
        
        let (client, _message_bus) = create_test_client_with_responses(responses);

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00);

        // Since we can't actually submit without proper mock infrastructure,
        // just test that the order builds correctly
        let order = builder.build_order().unwrap();
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_order_builder_build_order() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_till_cancel()
            .account("DU123456")
            .hidden()
            .outside_rth();

        let order = builder.build_order().unwrap();
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.tif, "GTC");
        assert_eq!(order.account, "DU123456");
        assert!(order.hidden);
        assert!(order.outside_rth);
    }

    #[test]
    fn test_order_builder_what_if() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .what_if();

        let order = builder.build_order().unwrap();
        assert_eq!(order.what_if, true);
    }

    #[test]
    fn test_bracket_order_builder() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

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
    fn test_algo_order() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("VWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00");

        let order = builder.build_order().unwrap();
        assert_eq!(order.algo_strategy, "VWAP");
        assert_eq!(order.algo_params.len(), 2);
        assert_eq!(order.algo_params[0].tag, "startTime");
        assert_eq!(order.algo_params[0].value, "09:30:00");
    }

    #[test]
    fn test_oca_group() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("TestOCA", 1);

        let order = builder.build_order().unwrap();
        assert_eq!(order.oca_group, "TestOCA");
        assert_eq!(order.oca_type, 1);
    }

    #[test]
    fn test_parent_order() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .parent(999);

        let order = builder.build_order().unwrap();
        assert_eq!(order.parent_id, 999);
    }

    #[test]
    fn test_do_not_transmit() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .do_not_transmit();

        let order = builder.build_order().unwrap();
        assert!(!order.transmit);
    }

    #[test]
    fn test_order_builder_with_invalid_quantity() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(-100)
            .limit(50.00);

        let result = builder.build_order();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid quantity"));
    }

    #[test]
    fn test_order_builder_with_invalid_price() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(-50.00);

        let result = builder.build_order();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid price"));
    }

    #[test]
    fn test_trading_hours() {
        let (client, _message_bus) = create_test_client();
        let contract = create_stock_contract("AAPL");

        // Test regular hours
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .regular_hours_only();

        let order = builder.build_order().unwrap();
        assert!(!order.outside_rth);

        // Test extended hours
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .outside_rth();

        let order = builder.build_order().unwrap();
        assert!(order.outside_rth);

        // Test with trading_hours enum
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .trading_hours(crate::market_data::TradingHours::Extended);

        let order = builder.build_order().unwrap();
        assert!(order.outside_rth);
    }
}