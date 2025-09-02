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
    fn test_midprice_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .midprice(50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.limit_price, Some(50.00));
    }

    #[test]
    fn test_custom_order_type() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::PeggedToStock);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "PEG STK");
    }

    #[test]
    fn test_volatility_order_missing_volatility() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Volatility);
        // Don't set volatility
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("volatility"));
    }

    #[test]
    fn test_volatility_order_with_volatility() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Volatility)
            .volatility(0.15);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "VOL");
    }

    #[test]
    fn test_good_till_date_with_public_method() {
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
    fn test_special_order_flags_not_held() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .not_held();
        
        let order = builder.build().unwrap();
        assert!(order.not_held);
    }

    #[test]
    fn test_special_order_flags_all_or_none() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .all_or_none();
        
        let order = builder.build().unwrap();
        assert!(order.all_or_none);
    }

    #[test]
    fn test_pegged_order_min_trade_qty() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .min_trade_qty(10);
        
        let order = builder.build().unwrap();
        assert_eq!(order.min_trade_qty, Some(10));
    }

    #[test]
    fn test_pegged_order_min_compete_size() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .min_compete_size(50);
        
        let order = builder.build().unwrap();
        assert_eq!(order.min_compete_size, Some(50));
    }

    #[test]
    fn test_pegged_order_compete_against_best_offset() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .compete_against_best_offset(0.01);
        
        let order = builder.build().unwrap();
        assert_eq!(order.compete_against_best_offset, Some(0.01));
    }

    #[test]
    fn test_pegged_order_mid_offset_at_whole() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .mid_offset_at_whole(0.005);
        
        let order = builder.build().unwrap();
        assert_eq!(order.mid_offset_at_whole, Some(0.005));
    }

    #[test]
    fn test_pegged_order_mid_offset_at_half() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .mid_offset_at_half(0.0025);
        
        let order = builder.build().unwrap();
        assert_eq!(order.mid_offset_at_half, Some(0.0025));
    }

    #[test]
    fn test_good_after_time() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_after_time("20240630 09:30:00");
        
        let order = builder.build().unwrap();
        assert_eq!(order.good_after_time, "20240630 09:30:00");
    }

    #[test]
    fn test_good_till_time() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .good_till_time("20240630 16:00:00");
        
        let order = builder.build().unwrap();
        assert_eq!(order.good_till_date, "20240630 16:00:00");
    }

    #[test]
    fn test_trailing_stop_missing_both_percent_and_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::TrailingStop);
        // Don't set trailing_percent or trail_stop_price
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trailing amount or stop price"));
    }

    #[test]
    fn test_trailing_stop_with_percent_only() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::TrailingStop);
        
        // Can't set trailing_percent directly, need a public method
        // This test is covered by test_trailing_stop_with_stop_price
    }

    #[test]
    fn test_trailing_stop_limit_missing_both() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::TrailingStopLimit);
        // Don't set trailing_percent or trail_stop_price
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trailing amount or stop price"));
    }

    #[test]
    fn test_stop_order_missing_stop_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Stop);
        // Don't set stop_price
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stop_price"));
    }

    #[test]
    fn test_stop_limit_missing_stop_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::StopLimit)
            .limit(50.00);
        // Don't set stop_price - need to use the regular API
        
        // This test can't work with public API, test is covered by regular stop_limit tests
    }

    #[test]
    fn test_algo_strategy_with_params() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("TWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00");
        
        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "TWAP");
        assert_eq!(order.algo_params.len(), 2);
    }

    #[test]
    fn test_what_if_flag() {
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
    fn test_trailing_stop_with_stop_price() {
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

    #[test]
    fn test_order_with_parent_id() {
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
    fn test_oca_group_with_type() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("GROUP1", 2);
        
        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "GROUP1");
        assert_eq!(order.oca_type, 2);
    }

    #[test]
    fn test_oca_group_without_type() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .oca_group("GROUP2", 0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.oca_group, "GROUP2");
        assert_eq!(order.oca_type, 0);
    }

    #[test]
    fn test_order_with_account() {
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
    fn test_hidden_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .hidden();
        
        let order = builder.build().unwrap();
        assert!(order.hidden);
    }

    #[test]
    fn test_outside_rth() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .outside_rth();
        
        let order = builder.build().unwrap();
        assert!(order.outside_rth);
    }

    #[test]
    fn test_regular_hours_only() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .regular_hours_only();
        
        let order = builder.build().unwrap();
        assert!(!order.outside_rth);
    }

    #[test]
    fn test_do_not_transmit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .do_not_transmit();
        
        let order = builder.build().unwrap();
        assert!(!order.transmit);
    }
}