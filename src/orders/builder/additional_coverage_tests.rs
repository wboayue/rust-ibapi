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

    // Test line 495: fallback to limit_price when order type doesn't explicitly require it
    #[test]
    fn test_limit_price_fallback_for_non_limit_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Market orders don't have limit price
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.limit_price, None);
    }

    // Test that stop orders properly use aux_price
    #[test]
    fn test_stop_order_uses_aux_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop(45.0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP");
        // Stop orders use aux_price for stop price
        assert_eq!(order.aux_price, Some(45.0));
    }

    // Test trailing stop with both percent and price
    #[test]
    fn test_trailing_stop_with_percent_and_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .trailing_stop(5.0, 95.0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "TRAIL");
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(95.0));
    }

    // Test good till date properly sets both tif and date
    #[test]
    fn test_good_till_date_properly_set() {
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

    // Test various field setters that get transferred to Order
    #[test]
    fn test_all_field_setters() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .parent(999)
            .account("ACCT123")
            .hidden()
            .outside_rth()
            .do_not_transmit();
        
        let order = builder.build().unwrap();
        
        // Verify all fields are set
        assert_eq!(order.parent_id, 999);
        assert_eq!(order.account, "ACCT123");
        assert!(order.hidden);
        assert!(order.outside_rth);
        assert!(!order.transmit);
        assert_eq!(order.limit_price, Some(50.00));
    }

    // Test oca group with type 0
    #[test]
    fn test_oca_group_with_zero_type() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("GROUP1", 0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "GROUP1");
        assert_eq!(order.oca_type, 0);
    }

    // Test good_after_time field
    #[test]
    fn test_good_after_time_field() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_after_time("20240630 09:30:00");
        
        let order = builder.build().unwrap();
        assert_eq!(order.good_after_time, "20240630 09:30:00");
    }

    // Test algo strategy with params
    #[test]
    fn test_algo_strategy_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("ADAPTIVE")
            .algo_param("adaptivePriority", "Normal")
            .algo_param("forceCompletion", "0");
        
        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "ADAPTIVE");
        assert_eq!(order.algo_params.len(), 2);
        assert_eq!(order.algo_params[0].tag, "adaptivePriority");
        assert_eq!(order.algo_params[0].value, "Normal");
    }

    // Test what_if field
    #[test]
    fn test_what_if_field() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .what_if();
        
        let order = builder.build().unwrap();
        assert!(order.what_if);
    }

    // Test pegged order fields
    #[test]
    fn test_all_pegged_order_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .min_trade_qty(5)
            .min_compete_size(100)
            .compete_against_best_offset(0.01)
            .mid_offset_at_whole(0.005)
            .mid_offset_at_half(0.0025);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.aux_price, Some(0.05));
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.min_trade_qty, Some(5));
        assert_eq!(order.min_compete_size, Some(100));
        assert_eq!(order.compete_against_best_offset, Some(0.01));
        assert_eq!(order.mid_offset_at_whole, Some(0.005));
        assert_eq!(order.mid_offset_at_half, Some(0.0025));
    }

    // Test stop limit order with both prices
    #[test]
    fn test_stop_limit_both_prices() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .stop_limit(45.00, 50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "STP LMT");
        assert_eq!(order.aux_price, Some(45.00));  // Stop price
        assert_eq!(order.limit_price, Some(50.00)); // Limit price
    }

    // Test trailing stop limit with all fields
    #[test]
    fn test_trailing_stop_limit_all_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .trailing_stop_limit(5.0, 95.0, 0.50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "TRAIL LIMIT");
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(95.0));
        assert_eq!(order.limit_price_offset, Some(0.50));
    }

    // Test GTD with actual date
    #[test]
    fn test_good_till_date_with_actual_date() {
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

    // Test good_till_date being set via good_till_time when not GTD
    #[test]
    fn test_good_till_time_when_not_gtd() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .day_order() // Not GTD
            .good_till_time("20240630 16:00:00");
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "DAY");
        assert_eq!(order.good_till_date, "20240630 16:00:00");
    }

    // Test volatility order with volatility set
    #[test]
    fn test_volatility_order_complete() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Volatility)
            .volatility(0.15);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "VOL");
        // Volatility is set internally
    }

    // Test special flags combination
    #[test]
    fn test_special_flags_combination() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .sweep_to_fill(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.sweep_to_fill);
        assert!(!order.block_order);
        assert!(!order.not_held);
        assert!(!order.all_or_none);
    }

    // Test block order flag
    #[test]
    fn test_block_order_flag() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .block(50.00);
        
        let order = builder.build().unwrap();
        assert!(order.block_order);
        assert!(!order.sweep_to_fill);
    }

    // Test default values for unset fields
    #[test]
    fn test_default_field_values() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        
        // Check defaults
        assert!(order.transmit); // Default true
        assert!(!order.hidden); // Default false
        assert!(!order.outside_rth); // Default false
        assert!(!order.sweep_to_fill); // Default false
        assert!(!order.block_order); // Default false
        assert!(!order.not_held); // Default false
        assert!(!order.all_or_none); // Default false
        assert!(!order.what_if); // Default false
        assert_eq!(order.parent_id, 0); // Default 0
        assert_eq!(order.oca_group, ""); // Default empty
        assert_eq!(order.account, ""); // Default empty
        assert_eq!(order.algo_strategy, ""); // Default empty
    }
}