#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::contracts::Contract;
    use crate::orders::Action;
    use crate::market_data::TradingHours;

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
    fn test_passive_relative() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .passive_relative(0.05);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "PASSV REL");
        assert_eq!(order.aux_price, Some(0.05));
    }

    #[test]
    fn test_time_in_force_method() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .time_in_force(TimeInForce::ImmediateOrCancel);
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "IOC");
    }

    #[test]
    fn test_good_till_date() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_till_date("20240630 23:59:59");
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "GTD");
        assert_eq!(order.good_till_date, "20240630 23:59:59");
    }

    #[test]
    fn test_trading_hours_method() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test with Regular hours
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .trading_hours(TradingHours::Regular);
        
        let order = builder.build().unwrap();
        assert!(!order.outside_rth);
        
        // Test with Extended hours
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .trading_hours(TradingHours::Extended);
        
        let order = builder.build().unwrap();
        assert!(order.outside_rth);
    }

    #[test]
    fn test_order_special_flags() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .sweep_to_fill(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.sweep_to_fill);
        assert_eq!(order.limit_price, Some(50.00));
        
        // Test block order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .block(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.block_order);
        
        // Test all_or_none
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
    fn test_not_held_flag() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        // Check that not_held is set in the build process
        assert!(!order.not_held); // Default should be false
    }

    #[test]
    fn test_all_or_none_flag() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        // Check that all_or_none is set in the build process
        assert!(!order.all_or_none); // Default should be false
    }

    #[test]
    fn test_pegged_order_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test that the build process properly initializes pegged order fields
        // These fields would normally be set through methods that don't exist yet
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00);
        
        let order = builder.build().unwrap();
        // Check defaults are properly set
        assert_eq!(order.min_trade_qty, None);
        assert_eq!(order.min_compete_size, None);
        assert_eq!(order.compete_against_best_offset, None);
        assert_eq!(order.mid_offset_at_whole, None);
        assert_eq!(order.mid_offset_at_half, None);
    }

    #[test]
    fn test_order_field_setters() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .parent(999)
            .account("TEST_ACCOUNT");
        
        let order = builder.build().unwrap();
        assert_eq!(order.parent_id, 999);
        assert_eq!(order.account, "TEST_ACCOUNT");
    }

    #[test]
    fn test_oca_group_settings() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("TEST_OCA", 2);
        
        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "TEST_OCA");
        assert_eq!(order.oca_type, 2);
    }

    #[test]
    fn test_algo_order_settings() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("TWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "15:30:00")
            .algo_param("allowPastEndTime", "1");
        
        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "TWAP");
        assert_eq!(order.algo_params.len(), 3);
        assert_eq!(order.algo_params[0].tag, "startTime");
        assert_eq!(order.algo_params[1].tag, "endTime");
        assert_eq!(order.algo_params[2].tag, "allowPastEndTime");
    }

    #[test]
    fn test_what_if_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .what_if();
        
        let order = builder.build().unwrap();
        assert!(order.what_if);
    }

    #[test]
    fn test_discretionary_amount() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .discretionary(50.00, 0.25);
        
        let order = builder.build().unwrap();
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.discretionary_amt, 0.25);
    }




    #[test]
    fn test_bracket_order_build_details() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test Buy bracket order
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        assert_eq!(orders.len(), 3);
        
        // Verify parent order details
        let parent = &orders[0];
        assert_eq!(parent.action, Action::Buy);
        assert_eq!(parent.order_type, "LMT");
        assert_eq!(parent.limit_price, Some(50.0));
        assert!(!parent.transmit);
        
        // Verify take profit details
        let tp = &orders[1];
        assert_eq!(tp.action, Action::Sell); // Reverse of Buy
        assert_eq!(tp.order_type, "LMT");
        assert_eq!(tp.limit_price, Some(55.0));
        assert_eq!(tp.parent_id, parent.order_id);
        assert!(!tp.transmit);
        
        // Verify stop loss details
        let sl = &orders[2];
        assert_eq!(sl.action, Action::Sell); // Reverse of Buy
        assert_eq!(sl.order_type, "STP");
        assert_eq!(sl.aux_price, Some(45.0));
        assert_eq!(sl.parent_id, parent.order_id);
        assert!(sl.transmit);
    }

    #[test]
    fn test_bracket_order_sell() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test Sell bracket order
        let bracket = OrderBuilder::new(&client, &contract)
            .sell(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0) // Lower for sell
            .stop_loss(55.0);  // Higher for sell
        
        let orders = bracket.build().unwrap();
        assert_eq!(orders.len(), 3);
        
        // Verify actions are reversed for sell bracket
        let parent = &orders[0];
        assert_eq!(parent.action, Action::Sell);
        
        let tp = &orders[1];
        assert_eq!(tp.action, Action::Buy); // Reverse of Sell
        
        let sl = &orders[2];
        assert_eq!(sl.action, Action::Buy); // Reverse of Sell
    }

    #[test]
    fn test_validation_edge_cases() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test with zero stop price (should be valid)
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(0.0);
        
        let result = builder.build();
        assert!(result.is_ok());
        
        // Test limit price of zero (should be valid for some order types)
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(0.0);
        
        let result = builder.build();
        assert!(result.is_ok());
    }




}