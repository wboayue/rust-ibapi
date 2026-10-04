use crate::orders::builder::{BracketPrices, TrailBy, ValidationError};
use crate::orders::common::order_builder::*;
use crate::orders::{Action, Order};

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
        assert_eq!(order.tif, TimeInForce::OnOpen);
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
        assert_eq!(order.tif, TimeInForce::OnOpen);
    }
}

#[cfg(test)]
mod complex_order_tests {
    use super::*;

    #[test]
    fn test_bracket_order() {
        let prices = BracketPrices {
            entry: 50.0,
            take_profit: 55.0,
            stop_loss: 45.0,
        };
        let orders = bracket_order(1000, Action::Buy, 100.0, prices).unwrap();

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
    fn test_bracket_order_rejects_non_finite_prices() {
        let valid = BracketPrices {
            entry: 50.0,
            take_profit: 55.0,
            stop_loss: 45.0,
        };
        let cases = [
            BracketPrices { entry: f64::NAN, ..valid },
            BracketPrices {
                take_profit: f64::INFINITY,
                ..valid
            },
            BracketPrices {
                stop_loss: f64::NAN,
                ..valid
            },
        ];

        for prices in cases {
            let err = bracket_order(1000, Action::Buy, 100.0, prices).unwrap_err();
            assert!(matches!(err, ValidationError::InvalidPrice(_)), "{prices:?}");
        }
    }

    #[test]
    fn test_bracket_order_rejects_prices_on_the_wrong_side() {
        // (action, take_profit, stop_loss) around an entry of 50.0
        let cases = [
            (Action::Buy, 45.0, 40.0),
            (Action::Buy, 55.0, 52.0),
            (Action::Sell, 55.0, 60.0),
            (Action::Sell, 45.0, 48.0),
        ];

        for (action, take_profit, stop_loss) in cases {
            let prices = BracketPrices {
                entry: 50.0,
                take_profit,
                stop_loss,
            };
            let err = bracket_order(1000, action, 100.0, prices).unwrap_err();
            assert!(matches!(err, ValidationError::InvalidBracketOrder(_)), "{action:?} {prices:?}");
        }
    }

    #[test]
    fn test_one_cancels_all() {
        let order1 = limit_order(Action::Buy, 100.0, 50.0);
        let order2 = limit_order(Action::Sell, 100.0, 52.0);
        let orders = one_cancels_all("TestOCA", vec![order1, order2], OcaType::ReduceWithBlock);

        for order in &orders {
            assert_eq!(order.oca_group, "TestOCA");
            assert_eq!(order.oca_type, OcaType::ReduceWithBlock);
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
}

#[cfg(test)]
mod combo_order_tests {
    use super::*;

    #[test]
    fn test_combo_market_order() {
        let order = non_guaranteed(combo_market_order(Action::Buy, 100.0));

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
        let order = non_guaranteed(combo_limit_order(Action::Buy, 100.0, 50.0));

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
        let order = non_guaranteed(relative_limit_combo(Action::Buy, 100.0, 50.0));

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
        let order = non_guaranteed(limit_order_for_combo_with_leg_prices(Action::Buy, 100.0, leg_prices));

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
        let order = volatility(Action::Buy, 100.0, 0.04, VolatilityType::Daily);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "VOL");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.volatility, Some(0.04));
        assert_eq!(order.volatility_type, Some(VolatilityType::Daily));
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
    fn test_midprice_with_cap() {
        let order = midprice(Action::Buy, 100.0, Some(50.0));

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, Some(50.0));
    }

    #[test]
    fn test_midprice_without_cap() {
        let order = midprice(Action::Buy, 100.0, None);

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "MIDPRICE");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.limit_price, None);
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

    fn attach(to: AdjustTo) -> Order {
        let mut parent = stop(Action::Buy, 100.0, 50.0);
        parent.order_id = 7;
        attach_adjustable_stop(&parent, 45.0, Adjustment { trigger_price: 48.0, to })
    }

    fn assert_attached_stop(order: &Order) {
        assert_eq!(order.action, Action::Sell); // Opposite of parent
        assert_eq!(order.order_type, "STP");
        assert_eq!(order.total_quantity, 100.0);
        assert_eq!(order.aux_price, Some(45.0));
        assert_eq!(order.parent_id, 7);
        assert_eq!(order.trigger_price, Some(48.0));
    }

    #[test]
    fn test_adjust_to_stop() {
        let order = attach(AdjustTo::Stop { stop_price: 46.0 });

        assert_attached_stop(&order);
        assert_eq!(order.adjusted_order_type, "STP");
        assert_eq!(order.adjusted_stop_price, Some(46.0));
        assert_eq!(order.adjusted_stop_limit_price, None);
    }

    #[test]
    fn test_adjust_to_stop_limit() {
        let order = attach(AdjustTo::StopLimit {
            stop_price: 46.0,
            limit_price: 47.0,
        });

        assert_attached_stop(&order);
        assert_eq!(order.adjusted_order_type, "STP LMT");
        assert_eq!(order.adjusted_stop_price, Some(46.0));
        assert_eq!(order.adjusted_stop_limit_price, Some(47.0));
    }

    #[test]
    fn test_adjust_to_trail() {
        // (trail, expected adjustable_trailing_unit, expected adjusted_trailing_amount)
        let cases = [(TrailBy::Amount(0.5), 0, 0.5), (TrailBy::Percent(2.0), 100, 2.0)];

        for (trail, unit, amount) in cases {
            let order = attach(AdjustTo::Trail { stop_price: 46.0, trail });

            assert_attached_stop(&order);
            assert_eq!(order.adjusted_order_type, "TRAIL");
            assert_eq!(order.adjusted_stop_price, Some(46.0));
            assert_eq!(order.adjustable_trailing_unit, unit, "{trail:?}");
            assert_eq!(order.adjusted_trailing_amount, Some(amount), "{trail:?}");
        }
    }
}

#[cfg(test)]
mod additional_specialized_order_tests {
    use super::*;

    #[test]
    fn test_relative_market_combo() {
        let order = non_guaranteed(relative_market_combo(Action::Buy, 100.0));

        assert_eq!(order.action, Action::Buy);
        assert_eq!(order.order_type, "REL + MKT");
        assert_eq!(order.total_quantity, 100.0);

        // Check non-guaranteed params
        assert_eq!(order.smart_combo_routing_params.len(), 1);
        assert_eq!(order.smart_combo_routing_params[0].tag, "NonGuaranteed");
        assert_eq!(order.smart_combo_routing_params[0].value, "1");
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
        assert_eq!(order.tif, TimeInForce::Auction);
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
