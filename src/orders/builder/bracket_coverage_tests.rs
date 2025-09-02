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

    // Test bracket order validation and building for BUY orders
    #[test]
    fn test_bracket_order_buy_complete_flow() {
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
        
        // Verify parent order
        let parent = &orders[0];
        assert_eq!(parent.action, Action::Buy);
        assert_eq!(parent.order_type, "LMT");
        assert_eq!(parent.limit_price, Some(50.0));
        assert_eq!(parent.total_quantity, 100.0);
        assert!(!parent.transmit);
        
        // Verify take profit order
        let tp = &orders[1];
        assert_eq!(tp.action, Action::Sell);
        assert_eq!(tp.order_type, "LMT");
        assert_eq!(tp.limit_price, Some(55.0));
        assert_eq!(tp.total_quantity, 100.0);
        assert_eq!(tp.parent_id, parent.order_id);
        assert!(!tp.transmit);
        
        // Verify stop loss order
        let sl = &orders[2];
        assert_eq!(sl.action, Action::Sell);
        assert_eq!(sl.order_type, "STP");
        assert_eq!(sl.aux_price, Some(45.0));
        assert_eq!(sl.total_quantity, 100.0);
        assert_eq!(sl.parent_id, parent.order_id);
        assert!(sl.transmit);
    }

    // Test bracket order validation and building for SELL orders
    #[test]
    fn test_bracket_order_sell_complete_flow() {
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
        
        // Verify parent order
        let parent = &orders[0];
        assert_eq!(parent.action, Action::Sell);
        assert_eq!(parent.order_type, "LMT");
        assert_eq!(parent.limit_price, Some(50.0));
        assert_eq!(parent.total_quantity, 100.0);
        assert!(!parent.transmit);
        
        // Verify take profit order
        let tp = &orders[1];
        assert_eq!(tp.action, Action::Buy);
        assert_eq!(tp.order_type, "LMT");
        assert_eq!(tp.limit_price, Some(45.0));
        assert_eq!(tp.total_quantity, 100.0);
        assert_eq!(tp.parent_id, parent.order_id);
        assert!(!tp.transmit);
        
        // Verify stop loss order
        let sl = &orders[2];
        assert_eq!(sl.action, Action::Buy);
        assert_eq!(sl.order_type, "STP");
        assert_eq!(sl.aux_price, Some(55.0));
        assert_eq!(sl.total_quantity, 100.0);
        assert_eq!(sl.parent_id, parent.order_id);
        assert!(sl.transmit);
    }

    // Test bracket order with large quantity
    #[test]
    fn test_bracket_order_large_quantity() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(10000)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        
        assert_eq!(orders.len(), 3);
        
        // All orders should have same quantity
        assert_eq!(orders[0].total_quantity, 10000.0);
        assert_eq!(orders[1].total_quantity, 10000.0);
        assert_eq!(orders[2].total_quantity, 10000.0);
    }

    // Test bracket order with fractional prices
    #[test]
    fn test_bracket_order_fractional_prices() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.25)
            .take_profit(55.75)
            .stop_loss(45.50);
        
        let orders = bracket.build().unwrap();
        
        assert_eq!(orders.len(), 3);
        
        assert_eq!(orders[0].limit_price, Some(50.25));
        assert_eq!(orders[1].limit_price, Some(55.75));
        assert_eq!(orders[2].aux_price, Some(45.50));
    }

    // Test bracket order parent ID propagation
    #[test]
    fn test_bracket_order_parent_id_propagation() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        
        let parent_id = orders[0].order_id;
        assert_eq!(orders[1].parent_id, parent_id);
        assert_eq!(orders[2].parent_id, parent_id);
    }

    // Test bracket order transmit flags
    #[test]
    fn test_bracket_order_transmit_flags() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        
        // Parent and take profit should not transmit
        assert!(!orders[0].transmit);
        assert!(!orders[1].transmit);
        
        // Stop loss should transmit (last in chain)
        assert!(orders[2].transmit);
    }

    // Test bracket order action reversal
    #[test]
    fn test_bracket_order_action_reversal() {
        let client = MockClient;
        let contract = create_test_contract();
        
        // Test buy bracket
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        
        // Parent is Buy, children should be Sell
        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[2].action, Action::Sell);
        
        // Test sell bracket
        let bracket = OrderBuilder::new(&client, &contract)
            .sell(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0)
            .stop_loss(55.0);
        
        let orders = bracket.build().unwrap();
        
        // Parent is Sell, children should be Buy
        assert_eq!(orders[0].action, Action::Sell);
        assert_eq!(orders[1].action, Action::Buy);
        assert_eq!(orders[2].action, Action::Buy);
    }

    // Test bracket order types are correct
    #[test]
    fn test_bracket_order_types() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0);
        
        let orders = bracket.build().unwrap();
        
        // Parent should be limit
        assert_eq!(orders[0].order_type, "LMT");
        
        // Take profit should be limit
        assert_eq!(orders[1].order_type, "LMT");
        
        // Stop loss should be stop
        assert_eq!(orders[2].order_type, "STP");
    }

    // Test invalid bracket order - take profit wrong side for buy
    #[test]
    fn test_bracket_order_invalid_take_profit_buy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0)  // Wrong - below entry for buy
            .stop_loss(45.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Take profit"));
    }

    // Test invalid bracket order - stop loss wrong side for buy
    #[test]
    fn test_bracket_order_invalid_stop_loss_buy() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(55.0);  // Wrong - above entry for buy
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Stop loss"));
    }

    // Test invalid bracket order - take profit wrong side for sell
    #[test]
    fn test_bracket_order_invalid_take_profit_sell() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .sell(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)  // Wrong - above entry for sell
            .stop_loss(55.0);
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Take profit"));
    }

    // Test invalid bracket order - stop loss wrong side for sell
    #[test]
    fn test_bracket_order_invalid_stop_loss_sell() {
        let client = MockClient;
        let contract = create_test_contract();
        
        let bracket = OrderBuilder::new(&client, &contract)
            .sell(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(45.0)
            .stop_loss(45.0);  // Wrong - below entry for sell
        
        let result = bracket.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Stop loss"));
    }
}