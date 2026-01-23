use super::*;
use crate::contracts::{Contract, Currency, Exchange, Symbol};
use crate::market_data::TradingHours;
use crate::orders::Action;

fn create_test_contract() -> Contract {
    Contract {
        symbol: Symbol::from("TEST"),
        security_type: crate::contracts::SecurityType::Stock,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    }
}

struct MockClient;

#[test]
fn test_stop_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(95.50);

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

    let builder = OrderBuilder::new(&client, &contract).sell(100).trailing_stop_limit(5.0, 95.0, 0.50);

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

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_if_touched(99.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIT");
    assert_eq!(order.aux_price, Some(99.50));
}

#[test]
fn test_limit_if_touched() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_if_touched(99.50, 100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LIT");
    assert_eq!(order.aux_price, Some(99.50));
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_market_to_limit() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_to_limit();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MTL");
}

#[test]
fn test_block_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).block(50.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.limit_price, Some(50.00));
    assert!(order.block_order);
}

#[test]
fn test_relative_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).relative(0.05, Some(100.00));

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "REL");
    assert_eq!(order.aux_price, Some(0.05));
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_passive_relative() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).passive_relative(0.05);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "PASSV REL");
    assert_eq!(order.aux_price, Some(0.05));
}

#[test]
fn test_midprice_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).midprice(Some(50.00));

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIDPRICE");
    assert_eq!(order.limit_price, Some(50.00));
}

#[test]
fn test_midprice_order_without_price_cap() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).midprice(None);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIDPRICE");
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_at_auction() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).at_auction(100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MTL");
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_discretionary_amount() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).discretionary(50.00, 0.25);

    let order = builder.build().unwrap();
    assert_eq!(order.limit_price, Some(50.00));
    assert_eq!(order.discretionary_amt, 0.25);
}

#[test]
fn test_sweep_to_fill() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).sweep_to_fill(50.00);

    let order = builder.build().unwrap();
    assert!(order.sweep_to_fill);
    assert_eq!(order.limit_price, Some(50.00));
}

#[test]
fn test_time_conditions() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test Day order
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().day_order();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, crate::orders::TimeInForce::Day);

    // Test Good Till Cancel
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().good_till_cancel();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, crate::orders::TimeInForce::GoodTilCanceled);

    // Test Immediate or Cancel
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().immediate_or_cancel();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, crate::orders::TimeInForce::ImmediateOrCancel);

    // Test Fill or Kill
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().fill_or_kill();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, crate::orders::TimeInForce::FillOrKill);
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
    assert_eq!(order.tif, crate::orders::TimeInForce::ImmediateOrCancel);
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
    assert_eq!(order.tif, crate::orders::TimeInForce::GoodTilDate);
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
fn test_order_attributes() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).hidden().outside_rth();

    let order = builder.build().unwrap();
    assert!(order.hidden);
    assert!(order.outside_rth);
}

#[test]
fn test_not_held_flag() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().not_held();

    let order = builder.build().unwrap();
    assert!(order.not_held);
}

#[test]
fn test_all_or_none_flag() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().all_or_none();

    let order = builder.build().unwrap();
    assert!(order.all_or_none);
}

#[test]
fn test_account() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().account("DU123456");

    let order = builder.build().unwrap();
    assert_eq!(order.account, "DU123456");
}

#[test]
fn test_parent_id() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).parent(999);

    let order = builder.build().unwrap();
    assert_eq!(order.parent_id, 999);
}

#[test]
fn test_oca_group_settings() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).oca_group("TEST_OCA", 2);

    let order = builder.build().unwrap();
    assert_eq!(order.oca_group, "TEST_OCA");
    assert_eq!(order.oca_type, crate::orders::OcaType::ReduceWithBlock);
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

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).what_if();

    let order = builder.build().unwrap();
    assert!(order.what_if);
}

#[test]
fn test_custom_order_type() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).order_type(OrderType::PeggedToStock);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "PEG STK");
}

#[test]
fn test_volatility_order_missing_volatility() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).order_type(OrderType::Volatility);
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
fn test_pegged_order_fields() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let order = builder.build().unwrap();
    // Check defaults are properly set
    assert_eq!(order.min_trade_qty, None);
    assert_eq!(order.min_compete_size, None);
    assert_eq!(order.compete_against_best_offset, None);
    assert_eq!(order.mid_offset_at_whole, None);
    assert_eq!(order.mid_offset_at_half, None);
}

#[test]
fn test_validation_errors() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test zero quantity
    let builder = OrderBuilder::new(&client, &contract).buy(0).market();

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::InvalidQuantity(0.0))));

    // Test missing order type
    let builder = OrderBuilder::new(&client, &contract).buy(100);

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::MissingRequiredField("order_type"))));

    // Test invalid stop price (NaN)
    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(f64::NAN);

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::InvalidPrice(_))));
}

#[test]
fn test_validation_edge_cases() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test with zero stop price (should be valid)
    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(0.0);

    let result = builder.build();
    assert!(result.is_ok());

    // Test limit price of zero (should be valid for some order types)
    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(0.0);

    let result = builder.build();
    assert!(result.is_ok());
}

// ===== Bracket Order Tests =====

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
        .stop_loss(55.0); // Higher for sell

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
fn test_bracket_order_validation_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    // Valid buy bracket
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    assert!(bracket.build().is_ok());
}

#[test]
fn test_bracket_order_missing_entry() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract).buy(100).bracket().take_profit(55.0).stop_loss(45.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("entry"));
}

#[test]
fn test_bracket_order_missing_take_profit() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract).buy(100).bracket().entry_limit(50.0).stop_loss(45.0);

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

    // Take profit below entry for buy
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(45.0) // Invalid: below entry
        .stop_loss(45.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Take profit (45) must be above entry (50)"));
}

#[test]
fn test_bracket_order_invalid_stop_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    // Stop loss above entry for buy
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(55.0); // Invalid: above entry

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Stop loss (55) must be below entry (50)"));
}

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
    assert_eq!(orders[0].total_quantity, 10000.0);
    assert_eq!(orders[1].total_quantity, 10000.0);
    assert_eq!(orders[2].total_quantity, 10000.0);
}

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
    assert_eq!(orders[0].limit_price, Some(50.25));
    assert_eq!(orders[1].limit_price, Some(55.75));
    assert_eq!(orders[2].aux_price, Some(45.50));
}

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

    // Stop loss should transmit (last order)
    assert!(orders[2].transmit);
}

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
    assert_eq!(orders[0].action, Action::Sell);
    assert_eq!(orders[1].action, Action::Buy);
    assert_eq!(orders[2].action, Action::Buy);
}

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

    // Check order types
    assert_eq!(orders[0].order_type, "LMT"); // Parent is limit
    assert_eq!(orders[1].order_type, "LMT"); // Take profit is limit
    assert_eq!(orders[2].order_type, "STP"); // Stop loss is stop
}

#[test]
fn test_bracket_order_inherits_outside_rth() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test with outside_rth enabled
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .outside_rth()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should inherit outside_rth from parent
    assert!(orders[0].outside_rth, "Parent should have outside_rth");
    assert!(orders[1].outside_rth, "Take profit should inherit outside_rth");
    assert!(orders[2].outside_rth, "Stop loss should inherit outside_rth");

    // Test without outside_rth (default)
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should have outside_rth = false
    assert!(!orders[0].outside_rth);
    assert!(!orders[1].outside_rth);
    assert!(!orders[2].outside_rth);
}

#[test]
fn test_bracket_order_with_missing_action() {
    let client = MockClient;
    let contract = create_test_contract();

    // Create builder without setting action
    let mut builder = OrderBuilder::new(&client, &contract);
    builder.quantity = Some(100.0);

    let bracket = builder.bracket();

    let result = bracket.entry_limit(50.0).take_profit(55.0).stop_loss(45.0).build();

    assert!(result.is_err());
}

// ===== Market Entry Bracket Order Tests =====

#[test]
fn test_bracket_order_market_entry_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify parent order is market order
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Buy);
    assert_eq!(parent.order_type, "MKT");
    assert_eq!(parent.limit_price, None);
    assert!(!parent.transmit);

    // Verify take profit details
    let tp = &orders[1];
    assert_eq!(tp.action, Action::Sell);
    assert_eq!(tp.order_type, "LMT");
    assert_eq!(tp.limit_price, Some(55.0));
    assert_eq!(tp.parent_id, parent.order_id);
    assert!(!tp.transmit);

    // Verify stop loss details
    let sl = &orders[2];
    assert_eq!(sl.action, Action::Sell);
    assert_eq!(sl.order_type, "STP");
    assert_eq!(sl.aux_price, Some(45.0));
    assert_eq!(sl.parent_id, parent.order_id);
    assert!(sl.transmit);
}

#[test]
fn test_bracket_order_market_entry_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .sell(100)
        .bracket()
        .entry_market()
        .take_profit(45.0)
        .stop_loss(55.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify parent order is market order with Sell action
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Sell);
    assert_eq!(parent.order_type, "MKT");

    // Verify child orders have reversed action
    let tp = &orders[1];
    assert_eq!(tp.action, Action::Buy);

    let sl = &orders[2];
    assert_eq!(sl.action, Action::Buy);
}

#[test]
fn test_bracket_order_market_entry_inherits_outside_rth() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .outside_rth()
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should inherit outside_rth from parent
    assert!(orders[0].outside_rth, "Parent should have outside_rth");
    assert!(orders[1].outside_rth, "Take profit should inherit outside_rth");
    assert!(orders[2].outside_rth, "Stop loss should inherit outside_rth");
}

#[test]
fn test_bracket_order_market_entry_quantity_propagation() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(500)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should have the same quantity
    assert_eq!(orders[0].total_quantity, 500.0);
    assert_eq!(orders[1].total_quantity, 500.0);
    assert_eq!(orders[2].total_quantity, 500.0);
}

#[test]
fn test_bracket_order_market_entry_parent_id_propagation() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    let parent_id = orders[0].order_id;

    assert_eq!(orders[1].parent_id, parent_id);
    assert_eq!(orders[2].parent_id, parent_id);
}

#[test]
fn test_market_on_close() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_on_close();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MOC");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_limit_on_close() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_on_close(50.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LOC");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, Some(50.50));
}

#[test]
fn test_market_on_open() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_on_open();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MKT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.tif, crate::orders::TimeInForce::OnOpen);
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_limit_on_open() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_on_open(50.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, Some(50.50));
    assert_eq!(order.tif, crate::orders::TimeInForce::OnOpen);
}

#[test]
fn test_market_with_protection() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_with_protection();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MKT PRT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, None);
    assert_eq!(order.aux_price, None);
}

#[test]
fn test_stop_with_protection() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(100).stop_with_protection(95.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "STP PRT");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.aux_price, Some(95.00));
}

#[test]
fn test_market_on_close_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(200).market_on_close();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MOC");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 200.0);
}

#[test]
fn test_limit_on_close_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(200).limit_on_close(100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LOC");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 200.0);
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_stop_with_protection_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(50).stop_with_protection(105.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "STP PRT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 50.0);
    assert_eq!(order.aux_price, Some(105.00));
}

// ===== Conditional Order Tests =====

#[test]
fn test_single_price_condition() {
    use crate::orders::builder::price;
    use crate::orders::OrderCondition;

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);

    match &order.conditions[0] {
        OrderCondition::Price(c) => {
            assert_eq!(c.contract_id, 265598);
            assert_eq!(c.exchange, "SMART");
            assert_eq!(c.price, 150.0);
            assert!(c.is_more);
            assert!(c.is_conjunction);
        }
        _ => panic!("Expected Price condition"),
    }
}

#[test]
fn test_multiple_and_conditions() {
    use crate::orders::builder::{margin, price, time};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0))
        .and_condition(margin().greater_than(30))
        .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 3);

    // All conditions should have is_conjunction = true for AND logic
    for cond in &order.conditions {
        match cond {
            crate::orders::OrderCondition::Price(c) => assert!(c.is_conjunction),
            crate::orders::OrderCondition::Margin(c) => assert!(c.is_conjunction),
            crate::orders::OrderCondition::Time(c) => assert!(c.is_conjunction),
            _ => {}
        }
    }
}

#[test]
fn test_multiple_or_conditions() {
    use crate::orders::builder::{price, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").less_than(100.0))
        .or_condition(volume(265598, "SMART").greater_than(50_000_000));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 2);

    // First condition should have is_conjunction = false for OR with next
    match &order.conditions[0] {
        crate::orders::OrderCondition::Price(c) => assert!(!c.is_conjunction),
        _ => panic!("Expected Price condition"),
    }
}

#[test]
fn test_mixed_and_or_conditions() {
    use crate::orders::builder::{margin, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    // (price > 10 AND margin < 20) OR time > X OR volume > Y
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(123445, "SMART").greater_than(10.0))
        .and_condition(margin().less_than(20))
        .or_condition(time().greater_than("20251010 09:30:00 US/Eastern"))
        .or_condition(volume(123445, "SMART").greater_than(10_000_000));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 4);

    // Check conjunction flags: AND, OR, OR pattern
    match &order.conditions[0] {
        crate::orders::OrderCondition::Price(c) => assert!(c.is_conjunction), // AND with next
        _ => panic!("Expected Price condition"),
    }
    match &order.conditions[1] {
        crate::orders::OrderCondition::Margin(c) => assert!(!c.is_conjunction), // OR with next
        _ => panic!("Expected Margin condition"),
    }
    match &order.conditions[2] {
        crate::orders::OrderCondition::Time(c) => assert!(!c.is_conjunction), // OR with next
        _ => panic!("Expected Time condition"),
    }
}

#[test]
fn test_all_condition_types() {
    use crate::orders::builder::{execution, margin, percent_change, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0))
        .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
        .and_condition(margin().less_than(30))
        .and_condition(execution("MSFT", "STK", "SMART"))
        .and_condition(volume(76792991, "SMART").greater_than(50_000_000))
        .and_condition(percent_change(756733, "SMART").greater_than(2.0));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 6);
}

#[test]
fn test_condition_builder_conversion() {
    use crate::orders::builder::price;

    let client = MockClient;
    let contract = create_test_contract();

    // Test that condition builder auto-converts to OrderCondition via Into
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0)); // Builder should auto-convert

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);
}

#[test]
fn test_condition_with_existing_order_conditions() {
    use crate::orders::builder::price;
    use crate::orders::conditions::MarginCondition;
    use crate::orders::OrderCondition;

    let client = MockClient;
    let contract = create_test_contract();

    // Test mixing fluent API with manual conditions
    let mut builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0));

    // Manually add another condition
    builder.conditions.push(OrderCondition::Margin(MarginCondition {
        percent: 25,
        is_more: false,
        is_conjunction: true,
    }));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 2);
}

#[test]
fn test_less_than_conditions() {
    use crate::orders::builder::{margin, percent_change, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .sell(100)
        .market()
        .condition(price(265598, "SMART").less_than(140.0))
        .and_condition(margin().less_than(25))
        .and_condition(volume(265598, "SMART").less_than(1_000_000))
        .and_condition(percent_change(265598, "SMART").less_than(-2.0))
        .and_condition(time().less_than("20251230 09:30:00 US/Eastern"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 5);

    // Verify all are less_than (is_more = false)
    for cond in &order.conditions {
        match cond {
            crate::orders::OrderCondition::Price(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Margin(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Volume(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::PercentChange(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Time(c) => assert!(!c.is_more),
            _ => {}
        }
    }
}

#[test]
fn test_execution_condition_no_threshold() {
    use crate::orders::builder::execution;

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(execution("TSLA", "STK", "SMART"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);

    match &order.conditions[0] {
        crate::orders::OrderCondition::Execution(c) => {
            assert_eq!(c.symbol, "TSLA");
            assert_eq!(c.security_type, "STK");
            assert_eq!(c.exchange, "SMART");
        }
        _ => panic!("Expected Execution condition"),
    }
}
