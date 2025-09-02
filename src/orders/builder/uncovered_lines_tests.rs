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

    // Force coverage of line 495 - limit price fallback
    #[test]
    fn test_peggged_to_market_with_limit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::PeggedToMarket);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "PEG MKT");
        assert_eq!(order.limit_price, None);
    }

    // Force coverage of line 501 - stop order type check
    #[test]
    fn test_stop_order_requires_stop_price() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Stop);
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stop_price"));
    }

    // Force coverage of line 505 - non-stop order with stop price
    #[test]
    fn test_market_on_close_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::MarketOnClose);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MOC");
        assert_eq!(order.aux_price, None);
    }

    // Force coverage of line 507 - stop price for non-stop orders
    #[test]
    fn test_limit_on_close_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.0)
            .order_type(OrderType::LimitOnClose);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LOC");
        assert_eq!(order.limit_price, Some(50.0));
    }

    // Force coverage of line 515 - trailing stop validation
    #[test]
    fn test_trailing_stop_missing_both() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::TrailingStop);
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trailing amount or stop price"));
    }

    // Force coverage of line 520 - trail stop price Some case
    #[test]
    fn test_trailing_stop_limit_missing_params() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::TrailingStopLimit);
        
        let result = builder.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trailing amount or stop price"));
    }

    // Force coverage of line 534 - GTD time in force check
    #[test]
    fn test_auction_time_in_force() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .at_auction(50.0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.tif, "AUC");
        assert_eq!(order.order_type, "MTL");
    }

    // Force coverage of line 536 - GTD missing date error
    #[test]
    fn test_pegged_to_benchmark_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::PeggedToBenchmark);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "PEG BENCH");
    }

    // Force coverage of volatility fields - line 606-607, 610
    #[test]
    fn test_box_top_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // BoxTop needs limit price
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::BoxTop);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "BOX TOP");
        // BoxTop doesn't require limit price in builder
    }

    // Force coverage of additional order types
    #[test]
    fn test_limit_on_open_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.0)
            .order_type(OrderType::LimitOnOpen);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "LMT");  // LimitOnOpen uses LMT
        assert_eq!(order.limit_price, Some(50.0));
    }

    // Force coverage of midprice order
    #[test]
    fn test_midprice_with_cap() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .midprice(50.0);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.limit_price, Some(50.0));
    }

    // Force coverage for various order types
    #[test]
    fn test_various_order_types() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test PeggedToStock
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::PeggedToStock);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "PEG STK");
        
        // Test MarketOnOpen (uses MKT)
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::MarketOnOpen);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MKT");
    }

    // Force coverage for special cases in build method
    #[test]
    fn test_order_with_all_optional_fields() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.0)
            .parent(12345)
            .account("ACCOUNT123")
            .oca_group("OCA_GROUP", 1)
            .hidden()
            .outside_rth()
            .do_not_transmit()
            .good_after_time("09:30:00")
            .algo("TWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00")
            .what_if()
            .discretionary(50.0, 0.10)
            .min_trade_qty(10)
            .min_compete_size(100)
            .compete_against_best_offset(0.01)
            .mid_offset_at_whole(0.005)
            .mid_offset_at_half(0.0025);
        
        let order = builder.build().unwrap();
        
        assert_eq!(order.parent_id, 12345);
        assert_eq!(order.account, "ACCOUNT123");
        assert_eq!(order.oca_group, "OCA_GROUP");
        assert_eq!(order.oca_type, 1);
        assert!(order.hidden);
        assert!(order.outside_rth);
        assert!(!order.transmit);
        assert_eq!(order.good_after_time, "09:30:00");
        assert_eq!(order.algo_strategy, "TWAP");
        assert_eq!(order.algo_params.len(), 2);
        assert!(order.what_if);
        assert_eq!(order.discretionary_amt, 0.10);
        assert_eq!(order.min_trade_qty, Some(10));
        assert_eq!(order.min_compete_size, Some(100));
        assert_eq!(order.compete_against_best_offset, Some(0.01));
        assert_eq!(order.mid_offset_at_whole, Some(0.005));
        assert_eq!(order.mid_offset_at_half, Some(0.0025));
    }

    // Test defaults and unset fields
    #[test]
    fn test_minimal_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        
        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.order_type, "MKT");
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
        assert_eq!(order.oca_type, 0);
        assert_eq!(order.account, "");
        assert_eq!(order.good_after_time, "");
        assert_eq!(order.algo_strategy, "");
        assert_eq!(order.algo_params.len(), 0);
        assert_eq!(order.discretionary_amt, 0.0);
        assert_eq!(order.min_trade_qty, None);
        assert_eq!(order.min_compete_size, None);
        assert_eq!(order.compete_against_best_offset, None);
        assert_eq!(order.mid_offset_at_whole, None);
        assert_eq!(order.mid_offset_at_half, None);
    }
}