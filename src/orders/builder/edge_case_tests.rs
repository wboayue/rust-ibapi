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
    fn test_good_after_time_is_set() {
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
    fn test_sweep_to_fill_order() {
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
    fn test_block_order() {
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
    fn test_at_auction_order() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .at_auction(50.00);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "MTL");
        assert_eq!(order.limit_price, Some(50.00));
        assert_eq!(order.tif, "AUC");
    }

    #[test]
    fn test_sell_action() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .sell(200)
            .market();
        
        let order = builder.build().unwrap();
        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.total_quantity, 200.0);
    }

    #[test]
    fn test_trailing_stop_limit_with_offset() {
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

    #[test]
    fn test_algo_order_with_multiple_params() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .algo("VWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00")
            .algo_param("maxPctVol", "0.1")
            .algo_param("noTakeLiq", "1");
        
        let order = builder.build().unwrap();
        assert_eq!(order.algo_strategy, "VWAP");
        assert_eq!(order.algo_params.len(), 4);
    }

    #[test]
    fn test_order_with_not_held() {
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
    fn test_order_with_all_or_none() {
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
    fn test_order_with_volatility() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .order_type(OrderType::Volatility)
            .volatility(0.25);
        
        let order = builder.build().unwrap();
        assert_eq!(order.order_type, "VOL");
        // Volatility is set internally
    }

    #[test]
    fn test_pegged_order_with_all_offsets() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .relative(0.05, Some(50.00))
            .min_trade_qty(10)
            .min_compete_size(50)
            .compete_against_best_offset(0.01)
            .mid_offset_at_whole(0.005)
            .mid_offset_at_half(0.0025);
        
        let order = builder.build().unwrap();
        assert_eq!(order.min_trade_qty, Some(10));
        assert_eq!(order.min_compete_size, Some(50));
        assert_eq!(order.compete_against_best_offset, Some(0.01));
        assert_eq!(order.mid_offset_at_whole, Some(0.005));
        assert_eq!(order.mid_offset_at_half, Some(0.0025));
    }

    #[test]
    fn test_order_default_transmit_is_true() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market();
        
        let order = builder.build().unwrap();
        assert!(order.transmit);
    }

    #[test]
    fn test_order_do_not_transmit() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .market()
            .do_not_transmit();
        
        let order = builder.build().unwrap();
        assert!(!order.transmit);
    }

    #[test]
    fn test_passive_relative_order() {
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
    fn test_order_with_parent() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .parent(54321);
        
        let order = builder.build().unwrap();
        assert_eq!(order.parent_id, 54321);
    }

    #[test]
    fn test_order_with_account() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .account("TEST123");
        
        let order = builder.build().unwrap();
        assert_eq!(order.account, "TEST123");
    }

    #[test]
    fn test_bracket_order_with_missing_action() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let builder = OrderBuilder::new(&client, &contract);
        // Don't set buy/sell action - just set quantity via bracket
        
        let bracket = builder.bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("action"));
    }
}