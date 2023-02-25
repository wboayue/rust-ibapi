use super::*;
use crate::client::stub::ClientStub;
use crate::contracts::{contract_samples, Contract, SecurityType};

#[test]
fn place_market_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![
        "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1236109||0||100|1376327563|0|0|0||1376327563.0/DU1236109/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|PreSubmitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
        "3|13|PreSubmitted|0|100|0|1376327563|0|0|100||0||".to_owned(),
        "11|-1|13|76792991|TSLA|STK||0.0|||ISLAND|USD|TSLA|NMS|00025b46.63f8f39c.01.01|20230224  12:04:56|DU1236109|ISLAND|BOT|100|196.52|1376327563|100|0|100|196.52|||||2||".to_owned(),
        "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1236109||0||100|1376327563|0|0|0||1376327563.0/DU1236109/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
        "3|13|Filled|100|0|196.52|1376327563|0|196.52|100||0||".to_owned(),
        "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1236109||0||100|1376327563|0|0|0||1376327563.0/DU1236109/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.0|||USD||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
        "59|1|00025b46.63f8f39c.01.01|1.0|USD|1.7976931348623157E308|1.7976931348623157E308|||".to_owned(),
    ];

    let contract = Contract {
        symbol: "TSLA".to_owned(),
        security_type: SecurityType::Stock,
        exchange: "SMART".to_owned(),
        currency: "USD".to_owned(),
        ..Contract::default()
    };

    let order_id = 13;
    let order = order_builder::market_order(super::Action::Buy, 100.0);

    let result = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|13|0|TSLA|STK||0|||SMART||USD|||||BUY|100|MKT|||||||0||1|0|0|0|0|0|0|0||0||||||||0||-1|0|||0|||0|0||0||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
    );

    assert!(result.is_ok(), "failed to place order: {:?}", result.err());

    let mut notifications = result.unwrap();

    if let Some(OrderNotification::OpenOrder(notification)) = notifications.next() {
        assert_eq!(notification.order_id, 13, "notification.order_id");

        let contract = &notification.contract;
        let order = &notification.order;
        let order_state = &notification.order_state;

        assert_eq!(contract.contract_id, 76792991, "contract.contract_id");
        assert_eq!(contract.symbol, "TSLA", "contract.symbol");
        assert_eq!(contract.security_type, SecurityType::Stock, "contract.security_type");
        assert_eq!(
            contract.last_trade_date_or_contract_month, "",
            "contract.last_trade_date_or_contract_month"
        );
        assert_eq!(contract.strike, 0.0, "contract.strike");
        assert_eq!(contract.right, "?", "contract.right");
        assert_eq!(contract.multiplier, "", "contract.multiplier");
        assert_eq!(contract.exchange, "SMART", "contract.exchange");
        assert_eq!(contract.currency, "USD", "contract.currency");
        assert_eq!(contract.local_symbol, "TSLA", "contract.local_symbol");
        assert_eq!(contract.trading_class, "NMS", "contract.trading_class");

        assert_eq!(order.order_id, 13, "order.order_id");
        assert_eq!(order.action, Action::Buy, "order.action");
        assert_eq!(order.total_quantity, 100.0, "order.total_quantity");
        assert_eq!(order.order_type, "MKT", "order.order_type");
        assert_eq!(order.limit_price, Some(0.0), "order.limit_price");
        assert_eq!(order.aux_price, Some(0.0), "order.aux_price");
        assert_eq!(order.tif, "DAY", "order.tif");
        assert_eq!(order.oca_group, "", "order.oca_group");
        assert_eq!(order.account, "DU1236109", "order.account");
        assert_eq!(order.open_close, None, "order.open_close");
        assert_eq!(order.origin, 0, "order.origin");
        assert_eq!(order.order_ref, "", "order.order_ref");
        assert_eq!(order.client_id, 100, "order.client_id");
        assert_eq!(order.perm_id, 1376327563, "order.perm_id");
        assert_eq!(order.outside_rth, false, "order.outside_rth");
        assert_eq!(order.hidden, false, "order.hidden");
        assert_eq!(order.discretionary_amt, 0.0, "order.discretionary_amt");
        assert_eq!(order.good_after_time, "", "order.good_after_time");
        assert_eq!(order.fa_group, "", "order.fa_group");
        assert_eq!(order.fa_method, "", "order.fa_method");
        assert_eq!(order.fa_percentage, "", "order.fa_percentage");
        assert_eq!(order.fa_profile, "", "order.fa_profile");
        assert_eq!(order.model_code, "", "order.model_code");
        assert_eq!(order.good_till_date, "", "order.good_till_date");
        assert_eq!(order.rule_80_a, None, "order.rule_80_a");
        assert_eq!(order.percent_offset, None, "order.percent_offset");
        assert_eq!(order.settling_firm, "", "order.settling_firm");
        assert_eq!(order.short_sale_slot, 0, "order.short_sale_slot");
        assert_eq!(order.designated_location, "", "order.designated_location");
        assert_eq!(order.exempt_code, -1, "order.exempt_code");
        assert_eq!(order.auction_strategy, Some(0), "order.auction_strategy");
        assert_eq!(order.starting_price, None, "order.starting_price");
        assert_eq!(order.stock_ref_price, None, "order.stock_ref_price");
        assert_eq!(order.delta, None, "order.delta");
        assert_eq!(order.stock_range_lower, None, "order.stock_range_lower");
        assert_eq!(order.stock_range_upper, None, "order.stock_range_upper");
        assert_eq!(order.display_size, None, "order.display_size");
        assert_eq!(order.block_order, false, "order.block_order");
        assert_eq!(order.sweep_to_fill, false, "order.sweep_to_fill");
        assert_eq!(order.all_or_none, false, "order.all_or_none");
        assert_eq!(order.min_qty, None, "order.min_qty");
        assert_eq!(order.oca_type, 3, "order.oca_type");
        assert_eq!(order.parent_id, 0, "order.parent_id");
        assert_eq!(order.trigger_method, 0, "order.trigger_method");
        assert_eq!(order.volatility, None, "order.volatility");
        assert_eq!(order.volatility_type, Some(0), "order.volatility_type");
        assert_eq!(order.delta_neutral_order_type, "None", "order.delta_neutral_order_type");
        assert_eq!(order.delta_neutral_aux_price, None, "order.delta_neutral_aux_price");
        assert_eq!(order.delta_neutral_con_id, 0, "order.delta_neutral_con_id");
        assert_eq!(order.delta_neutral_settling_firm, "", "order.delta_neutral_settling_firm");
        assert_eq!(order.delta_neutral_clearing_account, "", "order.delta_neutral_clearing_account");
        assert_eq!(order.delta_neutral_clearing_intent, "", "order.delta_neutral_clearing_intent");
        assert_eq!(order.delta_neutral_open_close, "?", "order.delta_neutral_open_close");
        assert_eq!(order.delta_neutral_short_sale, false, "order.delta_neutral_short_sale");
        assert_eq!(order.delta_neutral_short_sale_slot, 0, "order.delta_neutral_short_sale_slot");
        assert_eq!(order.delta_neutral_designated_location, "", "order.delta_neutral_designated_location");
        assert_eq!(order.continuous_update, false, "order.continuous_update");
        assert_eq!(order.reference_price_type, Some(0), "order.reference_price_type");
        assert_eq!(order.trail_stop_price, None, "order.trail_stop_price");
        assert_eq!(order.trailing_percent, None, "order.trailing_percent");
        assert_eq!(order.basis_points, None, "order.basis_points");
        assert_eq!(order.basis_points_type, None, "order.basis_points_type");
        assert_eq!(contract.combo_legs_description, "", "contract.combo_legs_description");
        assert_eq!(contract.combo_legs.len(), 0, "contract.combo_legs.len()");
        assert_eq!(order.order_combo_legs.len(), 0, "order.order_combo_legs.len()");
        assert_eq!(order.smart_combo_routing_params.len(), 0, "order.smart_combo_routing_params.len()");
        assert_eq!(order.scale_init_level_size, None, "order.scale_init_level_size");
        assert_eq!(order.scale_subs_level_size, None, "order.scale_subs_level_size");
        assert_eq!(order.scale_price_increment, None, "order.scale_price_increment");
        assert_eq!(order.hedge_type, "", "order.hedge_type");
        assert_eq!(order.opt_out_smart_routing, false, "order.opt_out_smart_routing");
        assert_eq!(order.clearing_account, "", "order.clearing_account");
        assert_eq!(order.clearing_intent, "IB", "order.clearing_intent");
        assert_eq!(order.not_held, false, "order.not_held");
        assert_eq!(order.algo_strategy, "", "order.algo_strategy");
        assert_eq!(order.algo_params.len(), 0, "order.algo_params.len()");
        assert_eq!(order.solicited, false, "order.solicited");
        assert_eq!(order.what_if, false, "order.what_if");
        assert_eq!(order_state.status, "PreSubmitted", "order_state.status");
        assert_eq!(order_state.initial_margin_before, None, "order_state.initial_margin_before");
        assert_eq!(order_state.maintenance_margin_before, None, "order_state.maintenance_margin_before");
        assert_eq!(order_state.equity_with_loan_before, None, "order_state.equity_with_loan_before");
        assert_eq!(order_state.initial_margin_change, None, "order_state.initial_margin_change");
        assert_eq!(order_state.maintenance_margin_change, None, "order_state.maintenance_margin_change");
        assert_eq!(order_state.equity_with_loan_change, None, "order_state.equity_with_loan_change");
        assert_eq!(order_state.initial_margin_after, None, "order_state.initial_margin_after");
        assert_eq!(order_state.maintenance_margin_after, None, "order_state.maintenance_margin_after");
        assert_eq!(order_state.equity_with_loan_after, None, "order_state.equity_with_loan_after");
        assert_eq!(order_state.commission, None, "order_state.commission");
        assert_eq!(order_state.minimum_commission, None, "order_state.minimum_commission");
        assert_eq!(order_state.maximum_commission, None, "order_state.maximum_commission");
        assert_eq!(order_state.commission_currency, "", "order_state.commission_currency");
        assert_eq!(order_state.warning_text, "", "order_state.warning_text");
        assert_eq!(order.randomize_size, false, "order.randomize_size");
        assert_eq!(order.randomize_price, false, "order.randomize_price");
        assert_eq!(order.conditions.len(), 0, "order.conditions.len()");
        assert_eq!(order.adjusted_order_type, "None", "order.adjusted_order_type");
        assert_eq!(order.trigger_price, None, "order.trigger_price");
        assert_eq!(order.trail_stop_price, None, "order.trail_stop_price");
        assert_eq!(order.lmt_price_offset, None, "order.lmt_price_offset");
        assert_eq!(order.adjusted_stop_price, None, "order.adjusted_stop_price");
        assert_eq!(order.adjusted_stop_limit_price, None, "order.adjusted_stop_limit_price");
        assert_eq!(order.adjusted_trailing_amount, None, "order.adjusted_trailing_amount");
        assert_eq!(order.adjustable_trailing_unit, 0, "order.adjustable_trailing_unit");
        assert_eq!(order.soft_dollar_tier.name, "", "order.soft_dollar_tier.name");
        assert_eq!(order.soft_dollar_tier.value, "", "order.soft_dollar_tier.value");
        assert_eq!(order.soft_dollar_tier.display_name, "", "order.soft_dollar_tier.display_name");
        assert_eq!(order.cash_qty, Some(0.0), "order.cash_qty");
        assert_eq!(order.dont_use_auto_price_for_hedge, true, "order.dont_use_auto_price_for_hedge");
        assert_eq!(order.is_oms_container, false, "order.is_oms_container");
        assert_eq!(order.discretionary_up_to_limit_price, false, "order.discretionary_up_to_limit_price");
        assert_eq!(order.use_price_mgmt_algo, false, "order.use_price_mgmt_algo");
        assert_eq!(order.duration, None, "order.duration");
        assert_eq!(order.post_to_ats, None, "order.post_to_ats");
        assert_eq!(order.auto_cancel_parent, false, "order.auto_cancel_parent");
        assert_eq!(order.min_trade_qty, None, "order.min_trade_qty");
        assert_eq!(order.min_compete_size, None, "order.min_compete_size");
        assert_eq!(order.compete_against_best_offset, None, "order.compete_against_best_offset");
        assert_eq!(order.mid_offset_at_whole, None, "order.mid_offset_at_whole");
        assert_eq!(order.mid_offset_at_half, None, "order.mid_offset_at_half");
    } else {
        assert!(false, "expected an open order notification");
    }

    // 3 order status
    // 11 execution data
}

#[test]
fn place_limit_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![];

    let order_id = 12;
    let contract = contract_samples::future_with_local_symbol();
    let order = order_builder::limit_order(super::Action::Buy, 10.0, 500.00);

    let results = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|12|0||FUT|202303|0|||EUREX||EUR|FGBL MAR 23||||BUY|10|LMT|500||||||0||1|0|0|0|0|0|0|0||0||||||||0||-1|0|||0|||0|0||0||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
    );

    assert!(results.is_ok(), "failed to place order: {:?}", results.err());
}

#[test]
fn place_combo_market_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![];

    let order_id = 12; // get next order id
    let contract = contract_samples::smart_future_combo_contract();
    let order = order_builder::combo_market_order(Action::Sell, 150.0, true);

    let results = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|12|0|WTI|BAG||0|||SMART||USD|||||SELL|150|MKT|||||||0||1|0|0|0|0|0|0|0|2|55928698|1|BUY|IPE|0|0||0|55850663|1|SELL|IPE|0|0||0|0|1|NonGuaranteed|1||0||||||||0||-1|0|||0|||0|0||0||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
    );

    assert!(results.is_ok(), "failed to place order: {:?}", results.err());
}

// 11:49:32:189 <- 3-12-0-AAPL-STK--0.0---SMART--USD-----BUY-100-MKT-------0--1-0-0-0-0-0-0-0--0--------0---1-0---0---0-0--0------0-----0-----------0---0-0---0--0-0-0-0--1.7976931348623157e+308-1.7976931348623157e+308-1.7976931348623157e+308-1.7976931348623157e+308-1.7976931348623157e+308-0----1.7976931348623157e+308-----0-0-0--2147483647-2147483647-0-
// 11:49:32:797 -> -- �5-12-265598-AAPL-STK--0-?--SMART-USD-AAPL-NMS-BUY-100-MKT-0.0-0.0-DAY--DU1236109--0--123-45587459-0-0-0--45587459.0/DU1236109/100----------0---1-0------2147483647-0-0-0--3-0-0--0-0--0-None--0----?-0-0--0-0------0-0-0-2147483647-2147483647---0--IB-0-0--0-0-Submitted-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308------0-0-0-None-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-0----0-1-0-0-0---0----+3-12-Submitted-0-100-0-45587459-0-0-123--0-
// 11:49:32:834 -> ---�11--1-12-265598-AAPL-STK--0.0---ISLAND-USD-AAPL-NMS-0000e0d5.64305db8.01.01-20230223  11:49:33-DU1236109-ISLAND-BOT-100-149.23-45587459-123-0-100-149.23-----2-
// 11:49:32:835 -> -- �5-12-265598-AAPL-STK--0-?--SMART-USD-AAPL-NMS-BUY-100-MKT-0.0-0.0-DAY--DU1236109--0--123-45587459-0-0-0--45587459.0/DU1236109/100----------0---1-0------2147483647-0-0-0--3-0-0--0-0--0-None--0----?-0-0--0-0------0-0-0-2147483647-2147483647---0--IB-0-0--0-0-Filled-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308------0-0-0-None-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-0----0-1-0-0-0---0----23-12-Filled-100-0-149.23-45587459-0-149.23-123--0-
// 11:49:32:836 -> -- �5-12-265598-AAPL-STK--0-?--SMART-USD-AAPL-NMS-BUY-100-MKT-0.0-0.0-DAY--DU1236109--0--123-45587459-0-0-0--45587459.0/DU1236109/100----------0---1-0------2147483647-0-0-0--3-0-0--0-0--0-None--0----?-0-0--0-0------0-0-0-2147483647-2147483647---0--IB-0-0--0-0-Filled-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.0---USD--0-0-0-None-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-1.7976931348623157E308-0----0-1-0-0-0---0----23-12-Filled-100-0-149.23-45587459-0-149.23-123--0-
// 11:49:32:837 -> ---T59-1-0000e0d5.64305db8.01.01-1.0-USD-1.7976931348623157E308-1.7976931348623157E308--
