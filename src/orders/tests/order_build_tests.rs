use super::*;

/// Tests for basic order types like market, limit, and stop orders
#[cfg(test)]
mod basic_order_tests {
    use super::*;

    #[test]
    fn test_market_order() {
        let order = market_order(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, None);
        assert_eq!(order.aux_price, None);

        // Test sell order
        let order = market_order(Action::Sell, 200.0);
        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.total_quantity, 200.0);
    }

    #[test]
    fn test_limit_order() {
        let order = limit_order(Action::Buy, 100.0, 50.25);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.25));

        // Test sell order
        let order = limit_order(Action::Sell, 200.0, 60.50);
        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.limit_price, Some(60.50));
    }

    #[test]
    fn test_stop_order() {
        let order = stop(Action::Sell, 100.0, 45.0);

        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0)); // Stop price
        assert_eq!(order.limit_price, None);
    }

    #[test]
    fn test_stop_limit_order() {
        let order = stop_limit(Action::Sell, 100.0, 45.0, 44.0);

        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.order_type, "STP LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(45.0));
        assert_eq!(order.aux_price, Some(44.0)); // Stop trigger price
    }

    #[test]
    fn test_limit_if_touched() {
        let order = limit_if_touched(Action::Buy, 100.0, 52.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LIT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(52.0));
        assert_eq!(order.aux_price, Some(50.0)); // Trigger price
    }

    #[test]
    fn test_market_if_touched() {
        let order = market_if_touched(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MIT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(50.0)); // Trigger price
    }
}

#[cfg(test)]
mod time_based_order_tests {
    use super::*;

    #[test]
    fn test_market_on_close() {
        let order = market_on_close(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MOC");
        assert_eq!(order.total_quantity, 100.0);
    }

    #[test]
    fn test_market_on_open() {
        let order = market_on_open(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.tif, "OPG");
    }

    #[test]
    fn test_limit_on_close() {
        let order = limit_on_close(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LOC");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
    }

    #[test]
    fn test_limit_on_open() {
        let order = limit_on_open(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.tif, "OPG");
    }
}

#[cfg(test)]
mod complex_order_tests {
    use super::*;

    #[test]
    fn test_bracket_order() {
        let orders = bracket_order(1000, Action::Buy, 100.0, 50.0, 55.0, 45.0);

        assert_eq!(orders.len(), 3);

        // Parent order
        let parent = &orders[0];
        assert_eq!(parent.order_id, 1000);
        assert_eq!(parent.action, Action::Buy);
        assert_eq!(parent.order_type, "LMT");
        assert_eq!(parent.total_quantity, 100.0);
        assert_eq!(parent.limit_price, Some(50.0));
        assert!(!parent.transmit);

        // Take profit order
        let take_profit = &orders[1];
        assert_eq!(take_profit.order_id, 1001);
        assert_eq!(take_profit.action, Action::Sell);
        assert_eq!(take_profit.order_type, "LMT");
        assert_eq!(take_profit.total_quantity, 100.0);
        assert_eq!(take_profit.limit_price, Some(55.0));
        assert_eq!(take_profit.parent_id, 1000);
        assert!(!take_profit.transmit);

        // Stop loss order
        let stop_loss = &orders[2];
        assert_eq!(stop_loss.order_id, 1002);
        assert_eq!(stop_loss.action, Action::Sell);
        assert_eq!(stop_loss.order_type, "STP");
        assert_eq!(stop_loss.total_quantity, 100.0);
        assert_eq!(stop_loss.aux_price, Some(45.0));
        assert_eq!(stop_loss.parent_id, 1000);
        assert!(stop_loss.transmit);
    }

    #[test]
    fn test_one_cancels_all() {
        let order1 = limit_order(Action::Buy, 100.0, 50.0);
        let order2 = limit_order(Action::Sell, 100.0, 52.0);
        let orders = one_cancels_all("TestOCA", vec![order1, order2], 2);

        for order in &orders {
            assert_eq!(order.oca_group, "TestOCA");
            assert_eq!(order.oca_type, 2);
        }

        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[0].limit_price, Some(50.0));

        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[1].limit_price, Some(52.0));
    }

    #[test]
    fn test_trailing_stop_order() {
        let order = trailing_stop(Action::Sell, 100.0, 5.0, 45.0);

        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.order_type, "TRAIL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.trailing_percent, Some(5.0));
        assert_eq!(order.trail_stop_price, Some(45.0));
    }

    #[test]
    fn test_trailing_stop_limit_order() {
        let order = trailing_stop_limit(Action::Sell, 100.0, 2.0, 5.0, 45.0);

        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.order_type, "TRAIL LIMIT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price_offset, Some(2.0));
        assert_eq!(order.aux_price, Some(5.0)); // Trailing amount
        assert_eq!(order.trail_stop_price, Some(45.0));
    }
}

#[cfg(test)]
mod combo_order_tests {
    use super::*;

    #[test]
    fn test_combo_market_order() {
        let order = combo_market_order(Action::Buy, 100.0, true);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.total_quantity, 100.0);

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
    }

    #[test]
    fn test_combo_limit_order() {
        let order = combo_limit_order(Action::Buy, 100.0, 50.0, true);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
    }

    #[test]
    fn test_relative_limit_combo() {
        let order = relative_limit_combo(Action::Buy, 100.0, 50.0, true);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "REL + LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
    }

    #[test]
    fn test_limit_order_for_combo_with_leg_prices() {
        let leg_prices = vec![50.0, 45.0];
        let order = limit_order_for_combo_with_leg_prices(Action::Buy, 100.0, leg_prices, true);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);

        // Check leg prices
        assert_eq!(order.order_combo_legs.len(), 2);
        assert_eq!(order.order_combo_legs[0].price, Some(50.0));
        assert_eq!(order.order_combo_legs[1].price, Some(45.0));

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
    }
}

#[cfg(test)]
mod specialized_order_tests {
    use super::*;

    #[test]
    fn test_pegged_to_market() {
        let order = pegged_to_market(Action::Buy, 100.0, 0.05);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG MKT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(0.05));
    }

    #[test]
    fn test_volatility_order() {
        let order = volatility(Action::Buy, 100.0, 0.04, 1);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "VOL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.volatility, Some(0.04));
        assert_eq!(order.volatility_type, Some(1));
    }

    #[test]
    fn test_auction_limit() {
        let order = auction_limit(Action::Buy, 100.0, 50.0, 2);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.auction_strategy, Some(2));
    }

    #[test]
    fn test_auction_relative() {
        let order = auction_relative(Action::Buy, 100.0, 0.05);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(0.05));
    }

    #[test]
    fn test_block_order() {
        let order = block(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.block_order);
    }

    #[test]
    fn test_box_top() {
        let order = box_top(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "BOX TOP");
        assert_eq!(order.total_quantity, 100.0);
    }

    #[test]
    fn test_sweep_to_fill() {
        let order = sweep_to_fill(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.sweep_to_fill);
    }

    #[test]
    fn test_discretionary() {
        let order = discretionary(Action::Buy, 100.0, 50.0, 0.1);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.discretionary_amt, 0.1);
    }

    #[test]
    fn test_midpoint_match() {
        let order = midpoint_match(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.total_quantity, 100.0);
    }

    #[test]
    fn test_midprice() {
        let order = midprice(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
    }

    #[test]
    fn test_pegged_to_benchmark() {
        let order = pegged_to_benchmark(
            Action::Buy,
            100.0,
            50.0,     // starting_price
            false,    // pegged_change_amount_decrease
            0.02,     // pegged_change_amount
            0.01,     // reference_change_amount
            12345,    // reference_contract_id
            "ISLAND", // reference_exchange
            49.0,     // stock_reference_price
            48.0,     // reference_contract_lower_range
            52.0,     // reference_contract_upper_range
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG BENCH");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.starting_price, Some(50.0));
        assert_eq!(order.is_pegged_change_amount_decrease, false);
        assert_eq!(order.pegged_change_amount, Some(0.02));
        assert_eq!(order.reference_change_amount, Some(0.01));
        assert_eq!(order.reference_contract_id, 12345);
        assert_eq!(order.reference_exchange, "ISLAND");
        assert_eq!(order.stock_ref_price, Some(49.0));
        assert_eq!(order.stock_range_lower, Some(48.0));
        assert_eq!(order.stock_range_upper, Some(52.0));
    }
}

#[cfg(test)]
mod pegged_order_tests {
    use super::*;

    #[test]
    fn test_peg_best_order() {
        let order = peg_best_order(
            Action::Buy,
            100.0, // quantity
            50.0,  // limit_price
            10,    // min_trade_qty
            20,    // min_compete_size
            0.01,  // compete_against_best_offset
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG BEST");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.not_held);
        assert_eq!(order.min_trade_qty, Some(10));
        assert_eq!(order.min_compete_size, Some(20));
        assert_eq!(order.compete_against_best_offset, Some(0.01));
    }

    #[test]
    fn test_peg_best_up_to_mid() {
        let order = peg_best_up_to_mid_order(
            Action::Buy,
            100.0, // quantity
            50.0,  // limit_price
            10,    // min_trade_qty
            20,    // min_compete_size
            0.01,  // mid_offset_at_whole
            0.005, // mid_offset_at_half
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG BEST");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.not_held);
        assert_eq!(order.min_trade_qty, Some(10));
        assert_eq!(order.min_compete_size, Some(20));
        assert_eq!(order.compete_against_best_offset, COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID);
        assert_eq!(order.mid_offset_at_whole, Some(0.01));
        assert_eq!(order.mid_offset_at_half, Some(0.005));
    }

    #[test]
    fn test_peg_mid_order() {
        let order = peg_mid_order(
            Action::Buy,
            100.0, // quantity
            50.0,  // limit_price
            10,    // min_trade_qty
            0.01,  // mid_offset_at_whole
            0.005, // mid_offset_at_half
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG MID");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.not_held);
        assert_eq!(order.min_trade_qty, Some(10));
        assert_eq!(order.mid_offset_at_whole, Some(0.01));
        assert_eq!(order.mid_offset_at_half, Some(0.005));
    }
}

#[cfg(test)]
mod miscellaneous_order_tests {
    use super::*;

    #[test]
    fn test_limit_order_with_cash_qty() {
        let order = limit_order_with_cash_qty(Action::Buy, 50.0, 5000.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.cash_qty, Some(5000.0));
    }

    #[test]
    fn test_limit_order_with_manual_order_time() {
        let order = limit_order_with_manual_order_time(Action::Buy, 100.0, 50.0, "20240101 10:00:00");

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.manual_order_time, "20240101 10:00:00");
    }

    #[test]
    fn test_market_with_protection() {
        let order = market_with_protection(Action::Buy, 100.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT PRT");
        assert_eq!(order.total_quantity, 100.0);
    }

    #[test]
    fn test_stop_with_protection() {
        let order = stop_with_protection(Action::Sell, 100.0, 45.0);

        assert_eq!(order.action, Action::Sell);
        assert_eq!(order.order_type, "STP PRT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0));
    }

    #[test]
    fn test_ibkrats_limit_order() {
        let order = limit_ibkrats(Action::Buy, 100.0, 50.0);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.not_held);
    }

    #[test]
    fn test_market_f_hedge() {
        let order = market_f_hedge(1001, Action::Buy);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MKT");
        assert_eq!(order.total_quantity, 0.0);
        assert_eq!(order.parent_id, 1001);
        assert_eq!(order.hedge_type, "F");
    }
}

#[cfg(test)]
mod adjustable_order_tests {
    use super::*;

    #[test]
    fn test_attach_adjustable_to_stop() {
        let parent = stop(Action::Buy, 100.0, 50.0);
        let order = attach_adjustable_to_stop(
            &parent, 45.0, // attached_order_stop_price
            48.0, // trigger_price
            46.0, // adjusted_stop_price
        );

        assert_eq!(order.action, Action::Sell); // Opposite of parent
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0));
        assert_eq!(order.parent_id, parent.order_id);
        assert_eq!(order.trigger_price, Some(48.0));
        assert_eq!(order.adjusted_order_type, "STP");
        assert_eq!(order.adjusted_stop_price, Some(46.0));
    }

    #[test]
    fn test_attach_adjustable_to_stop_limit() {
        let parent = stop(Action::Buy, 100.0, 50.0);
        let order = attach_adjustable_to_stop_limit(
            &parent, 45.0, // attached_order_stop_price
            48.0, // trigger_price
            46.0, // adjusted_stop_price
            47.0, // adjusted_stop_limit_price
        );

        assert_eq!(order.action, Action::Sell); // Opposite of parent
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0));
        assert_eq!(order.parent_id, parent.order_id);
        assert_eq!(order.trigger_price, Some(48.0));
        assert_eq!(order.adjusted_order_type, "STP LMT");
        assert_eq!(order.adjusted_stop_price, Some(46.0));
        assert_eq!(order.adjusted_stop_limit_price, Some(47.0));
    }

    #[test]
    fn test_attach_adjustable_to_trail() {
        let parent = stop(Action::Buy, 100.0, 50.0);
        let order = attach_adjustable_to_trail(
            &parent, 45.0, // attached_order_stop_price
            48.0, // trigger_price
            46.0, // adjusted_stop_price
            0.02, // adjusted_trail_amount
            100,  // trail_unit (percentage)
        );

        assert_eq!(order.action, Action::Sell); // Opposite of parent
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0));
        assert_eq!(order.parent_id, parent.order_id);
        assert_eq!(order.trigger_price, Some(48.0));
        assert_eq!(order.adjusted_order_type, "TRAIL");
        assert_eq!(order.adjusted_stop_price, Some(46.0));
        assert_eq!(order.adjusted_trailing_amount, Some(0.02));
        assert_eq!(order.adjustable_trailing_unit, 100);
    }
}

#[cfg(test)]
mod additional_specialized_order_tests {
    use super::*;

    #[test]
    fn test_relative_market_combo() {
        let order = relative_market_combo(Action::Buy, 100.0, true);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "REL + MKT");
        assert_eq!(order.total_quantity, 100.0);

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
    }

    #[test]
    fn test_auction_pegged_to_stock() {
        let order = auction_pegged_to_stock(
            Action::Buy,
            100.0, // quantity
            50.0,  // starting_price
            0.5,   // delta
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG STK");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.starting_price, Some(50.0));
        assert_eq!(order.delta, Some(0.5));
    }

    #[test]
    fn test_pegged_to_stock() {
        let order = pegged_to_stock(
            Action::Buy,
            100.0, // quantity
            0.5,   // delta
            50.0,  // stock_ref_price
            49.0,  // starting_price
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PEG STK");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.delta, Some(0.5));
        assert_eq!(order.stock_ref_price, Some(50.0));
        assert_eq!(order.starting_price, Some(49.0));
    }

    #[test]
    fn test_relative_pegged_to_primary() {
        let order = relative_pegged_to_primary(
            Action::Buy,
            100.0, // quantity
            50.0,  // price_cap
            0.01,  // offset_amount
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "REL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.aux_price, Some(0.01));
    }

    #[test]
    fn test_passive_relative() {
        let order = passive_relative(
            Action::Buy,
            100.0, // quantity
            0.01,  // offset
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "PASSV REL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(0.01));
    }

    #[test]
    fn test_at_auction() {
        let order = at_auction(
            Action::Buy,
            100.0, // quantity
            50.0,  // price
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MTL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert_eq!(order.tif, "AUC");
    }

    #[test]
    fn test_what_if_limit_order() {
        let order = what_if_limit_order(
            Action::Buy,
            100.0, // quantity
            50.0,  // price
        );

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
        assert!(order.what_if);
    }
}
