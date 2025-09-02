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
    fn test_build_with_stop_price_fallback() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test Stop order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(95.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.aux_price, Some(95.50)); // Stop price goes to aux_price
    }

    #[test]
    fn test_build_with_limit_price_required() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test StopLimit order
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop_limit(95.00, 96.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP LMT");
        assert_eq!(order.aux_price, Some(95.00));  // Stop price
        assert_eq!(order.limit_price, Some(96.00)); // Limit price
    }

    #[test]
    fn test_build_trailing_stop_with_percent() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .sell(100)
            .trailing_stop(5.0, 95.0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "TRAIL");
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(95.0));
    }

    #[test]
    fn test_build_with_transmit_flag() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Default transmit should be true
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert!(order.transmit);
        
        // Test do_not_transmit
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .do_not_transmit();
        
        let order = builder.build().unwrap();
        assert!(!order.transmit);
    }

    #[test]
    fn test_build_with_parent_id() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .parent(12345);
        
        let order = builder.build().unwrap();
        assert_eq!(order.parent_id, 12345);
    }

    #[test]
    fn test_build_with_oca_group() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("GROUP1", 1);
        
        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "GROUP1");
        assert_eq!(order.oca_type, 1);
    }

    #[test]
    fn test_build_with_account() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .account("DU123456");
        
        let order = builder.build().unwrap();
        assert_eq!(order.account, "DU123456");
    }

    #[test]
    fn test_build_with_algo_strategy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("ADAPTIVE")
            .algo_param("adaptivePriority", "Normal");
        
        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "ADAPTIVE");
        assert_eq!(order.algo_params.len(), 1);
    }

    #[test]
    fn test_build_with_what_if_flag() {
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
    fn test_build_with_discretionary_amount() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .discretionary(50.00, 0.10);
        
        let order = builder.build().unwrap();
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.discretionary_amt, 0.10);
    }

    #[test]
    fn test_build_with_sweep_to_fill() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .sweep_to_fill(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.sweep_to_fill);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_build_with_block_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .block(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.block_order);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_build_order_default_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        
        // Check defaults
        assert!(order.transmit);
        assert!(!order.hidden);
        assert!(!order.outside_rth);
        assert!(!order.sweep_to_fill);
        assert!(!order.block_order);
        assert!(!order.not_held);
        assert!(!order.all_or_none);
        assert!(!order.what_if);
        assert_eq!(order.parent_id, 0);
        assert_eq!(order.oca_group, "");
        assert_eq!(order.account, "");
        assert_eq!(order.algo_strategy, "");
        assert_eq!(order.algo_params.len(), 0);
    }

    #[test]
    fn test_time_in_force_good_till_date() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_till_date("20240630 23:59:59 EST");
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "GTD");
        assert_eq!(order.good_till_date, "20240630 23:59:59 EST");
    }

    #[test]
    fn test_error_missing_action() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // No action set (no buy/sell)
        let builder = OrderBuilder::new(&client, &contract)
            .market();
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("action"));
    }

    #[test]
    fn test_error_missing_quantity() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Use buy with invalid quantity
        let builder = OrderBuilder::new(&client, &contract)
            .buy(0)  // Zero quantity
            .market();
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid quantity"));
    }

    #[test]
    fn test_error_negative_quantity() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(-10.0)
            .market();
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid quantity"));
    }

    #[test]
    fn test_error_negative_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(-10.0);
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid price"));
    }
}