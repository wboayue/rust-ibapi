#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::contracts::Contract;
    use crate::orders::Action;

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
    fn test_stop_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(95.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.aux_price, Some(95.50));
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
    }

    #[test]
    fn test_trailing_stop_limit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .sell(100)
            .trailing_stop_limit(5.0, 95.0, 0.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "TRAIL LIMIT");
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(95.0));
        assert_eq!(order.limit_price_offset, Some(0.50));
    }

    #[test]
    fn test_market_if_touched() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market_if_touched(99.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MIT");
        assert_eq!(order.aux_price, Some(99.50));
    }

    #[test]
    fn test_limit_if_touched() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit_if_touched(99.50, 100.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LIT");
        assert_eq!(order.aux_price, Some(99.50));
        assert_eq!(order.limit_price, Some(100.00));
    }

    #[test]
    fn test_market_to_limit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market_to_limit();
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MTL");
    }

    #[test]
    fn test_block_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .block(50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
        assert!(order.block_order);
    }

    #[test]
    fn test_relative_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(100.00));
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.aux_price, Some(0.05));
        assert_eq!(order.limit_price, Some(100.00));
    }




    #[test]
    fn test_at_auction() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .at_auction(100.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MTL");
        assert_eq!(order.limit_price, Some(100.00));
    }

    #[test]
    fn test_time_conditions() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test Day order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .day_order();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "DAY");
        
        // Test Good Till Cancel
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .good_till_cancel();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "GTC");
        
        // Test Immediate or Cancel
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .immediate_or_cancel();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "IOC");
        
        // Test Fill or Kill
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .fill_or_kill();
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "FOK");
    }

    #[test]
    fn test_order_attributes() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .hidden()
            .outside_rth();
        
        let order = builder.build().unwrap();
        assert!(order.hidden);
        assert!(order.outside_rth);
    }

    #[test]
    fn test_account() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .account("DU123456");
        
        let order = builder.build().unwrap();
        assert_eq!(order.account, "DU123456");
    }


    #[test]
    fn test_validation_errors() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test zero quantity
        let builder = OrderBuilder::new(&client, &contract)
            .buy(0)
            .market();
        
        let result = builder.build();
        assert!(matches!(result, Err(ValidationError::InvalidQuantity(0.0))));
        
        // Test missing order type
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100);
        
        let result = builder.build();
        assert!(matches!(result, Err(ValidationError::MissingRequiredField("order_type"))));
        
        // Test invalid stop price
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(-10.0);
        
        let result = builder.build();
        assert!(matches!(result, Err(ValidationError::InvalidPrice(-10.0))));
        
    }
}