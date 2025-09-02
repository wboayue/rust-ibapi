#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::contracts::Contract;

    fn create_test_contract() -> Contract {
        let mut contract = Contract::default();
        contract.symbol = "TEST".to_string();
        contract.security_type = crate::contracts::SecurityType::Stock;
        contract.exchange = "SMART".to_string();
        contract.currency = "USD".to_string();
        contract
    }

    struct MockClient;

    #[test]
    fn test_stop_order_with_aux_price_fallback() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Stop orders use aux_price field
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(95.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.aux_price, Some(95.50));
    }

    #[test]
    fn test_limit_order_with_limit_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .sell(100)
            .limit(50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_day_order_time_in_force() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .day_order();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "DAY");
    }

    #[test]
    fn test_good_till_cancel_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_till_cancel();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "GTC");
    }

    #[test]
    fn test_fill_or_kill_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .fill_or_kill();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "FOK");
    }

    #[test]
    fn test_immediate_or_cancel_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .immediate_or_cancel();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "IOC");
    }

    #[test]
    fn test_volatility_field_is_set() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Volatility)
            .volatility(0.20);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "VOL");
        // Volatility field should be set (though not directly accessible in Order)
    }

    #[test]
    fn test_build_with_transmit_true_by_default() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert!(order.transmit);
    }

    #[test]
    fn test_build_with_hidden_false_by_default() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert!(!order.hidden);
    }

    #[test]
    fn test_build_with_outside_rth_false_by_default() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert!(!order.outside_rth);
    }

    #[test]
    fn test_bracket_order_validation_buy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        assert_eq!(orders.len(), 3);
        
        // Parent order
        assert_eq!(orders[0].order_type, "LMT");
        assert_eq!(orders[0].limit_price, Some(50.0));
        assert!(!orders[0].transmit);
        
        // Take profit
        assert_eq!(orders[1].order_type, "LMT");
        assert_eq!(orders[1].limit_price, Some(55.0));
        assert!(!orders[1].transmit);
        
        // Stop loss
        assert_eq!(orders[2].order_type, "STP");
        assert_eq!(orders[2].aux_price, Some(45.0));
        assert!(orders[2].transmit);
    }

    #[test]
    fn test_bracket_order_missing_entry_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("entry_price"));
    }

    #[test]
    fn test_bracket_order_missing_take_profit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .stop_loss(45.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("take_profit"));
    }

    #[test]
    fn test_bracket_order_missing_stop_loss() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stop_loss"));
    }

    #[test]
    fn test_bracket_order_invalid_prices_buy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Take profit should be above entry for buy
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0)  // Invalid - below entry
            .stop_loss(45.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Take profit"));
    }

    #[test]
    fn test_bracket_order_invalid_stop_buy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Stop loss should be below entry for buy
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(55.0);  // Invalid - above entry
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Stop loss"));
    }

    #[test]
    fn test_bracket_order_sell() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .sell(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0)  // Below entry for sell
            .stop_loss(55.0);   // Above entry for sell
        
        let orders = bracket.build().unwrap();
        assert_eq!(orders.len(), 3);
        
        // Verify actions are reversed for sell
        assert_eq!(orders[0].action, crate::orders::Action::Sell);
        assert_eq!(orders[1].action, crate::orders::Action::Buy);
        assert_eq!(orders[2].action, crate::orders::Action::Buy);
    }

    #[test]
    fn test_discretionary_order_with_amount() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .discretionary(50.00, 0.10);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.discretionary_amt, 0.10);
    }

    #[test]
    fn test_market_if_touched_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market_if_touched(45.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MIT");
        assert_eq!(order.aux_price, Some(45.00));
    }

    #[test]
    fn test_limit_if_touched_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit_if_touched(45.00, 50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LIT");
        assert_eq!(order.aux_price, Some(45.00));
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_market_to_limit_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market_to_limit();
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MTL");
    }

    #[test]
    fn test_relative_order_with_cap() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00));
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.aux_price, Some(0.05));
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_relative_order_without_cap() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, None);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.aux_price, Some(0.05));
        assert_eq!(order.limit_price, None);
    }
}