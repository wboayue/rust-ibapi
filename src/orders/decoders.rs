use super::*;

pub fn decode_open_order(server_version: i32, message: &mut ResponseMessage) -> Result<OpenOrder> {
    message.skip(); // message type

    if server_version < server_versions::ORDER_CONTAINER {
        message.skip(); // message version
    }

    let mut open_order = OpenOrder::default();

    let contract = &mut open_order.contract;
    let order = &mut open_order.order;
    let order_state = &mut open_order.order_state;

    // Order Id

    open_order.order_id = message.next_int()?;
    order.order_id = open_order.order_id;

    // Contract fields

    contract.contract_id = message.next_int()?;
    contract.symbol = message.next_string()?;

    let security_type = message.next_string()?;
    contract.security_type = SecurityType::from(&security_type);

    contract.last_trade_date_or_contract_month = message.next_string()?;
    contract.strike = message.next_double()?;
    contract.right = message.next_string()?;
    contract.multiplier = message.next_string()?;
    contract.exchange = message.next_string()?;
    contract.currency = message.next_string()?;
    contract.local_symbol = message.next_string()?;
    contract.trading_class = message.next_string()?;

    // Order fields

    let action = message.next_string()?;
    order.action = Action::from(&action);

    order.total_quantity = message.next_double()?;
    order.order_type = message.next_string()?;
    order.limit_price = message.next_optional_double()?;
    order.aux_price = message.next_optional_double()?;
    order.tif = message.next_string()?;
    order.oca_group = message.next_string()?;
    order.account = message.next_string()?;

    let open_close = message.next_string()?;
    order.open_close = OrderOpenClose::from(&open_close);

    order.origin = message.next_int()?;
    order.order_ref = message.next_string()?;
    order.client_id = message.next_int()?;
    order.perm_id = message.next_int()?;
    order.outside_rth = message.next_bool()?;
    order.hidden = message.next_bool()?;
    order.discretionary_amt = message.next_double()?;
    order.good_after_time = message.next_string()?;

    message.skip(); // skip deprecated shares_allocation field

    order.fa_group = message.next_string()?;
    order.fa_method = message.next_string()?;
    order.fa_percentage = message.next_string()?;
    if server_version < server_versions::FA_PROFILE_DESUPPORT {
        order.fa_percentage = message.next_string()?;
    }

    if server_version > server_versions::MODELS_SUPPORT {
        order.model_code = message.next_string()?;
    }

    order.good_till_date = message.next_string()?;
    let rule_80_a = message.next_string()?;
    order.rule_80_a = Rule80A::from(&rule_80_a);
    order.percent_offset = message.next_optional_double()?;
    order.settling_firm = message.next_string()?;

    // Short sale params
    order.short_sale_slot = message.next_int()?;
    order.designated_location = message.next_string()?;
    order.exempt_code = message.next_int()?;

    order.auction_strategy = message.next_optional_int()?;

    // Box order paramas
    order.starting_price = message.next_optional_double()?;
    order.stock_ref_price = message.next_optional_double()?;
    order.delta = message.next_optional_double()?;

    // Peg to STK or volume order params
    order.stock_range_lower = message.next_optional_double()?;
    order.stock_range_upper = message.next_optional_double()?;

    order.display_size = message.next_optional_int()?;
    order.block_order = message.next_bool()?;
    order.sweep_to_fill = message.next_bool()?;
    order.all_or_none = message.next_bool()?;
    order.min_qty = message.next_optional_int()?;
    order.oca_type = message.next_int()?;

    message.skip(); // ETradeOnly
    message.skip(); // FirmQuoteOnly
    message.skip(); // NbboPriceCap

    order.parent_id = message.next_int()?;
    order.trigger_method = message.next_int()?;

    // Volatility order params
    order.volatility = message.next_optional_double()?;
    order.volatility_type = message.next_optional_int()?;
    order.delta_neutral_order_type = message.next_string()?;
    order.delta_neutral_aux_price = message.next_optional_double()?;

    if order.is_delta_neutral() {
        order.delta_neutral_con_id = message.next_int()?;
        order.delta_neutral_settling_firm = message.next_string()?;
        order.delta_neutral_clearing_account = message.next_string()?;
        order.delta_neutral_clearing_intent = message.next_string()?;
        order.delta_neutral_open_close = message.next_string()?;
        order.delta_neutral_short_sale = message.next_bool()?;
        order.delta_neutral_short_sale_slot = message.next_int()?;
        order.delta_neutral_designated_location = message.next_string()?;
    }

    order.continuous_update = message.next_bool()?;
    order.reference_price_type = message.next_optional_int()?;

    // Trail parameters
    order.trail_stop_price = message.next_optional_double()?;
    order.trailing_percent = message.next_optional_double()?;

    // Basic points
    order.basis_points = message.next_optional_double()?;
    order.basis_points_type = message.next_optional_int()?;

    // Combo Legs
    contract.combo_legs_description = message.next_string()?;

    let combo_legs_count = message.next_int()?;
    for _ in 0..combo_legs_count {
        let contract_id = message.next_int()?;
        let ratio = message.next_int()?;
        let action = message.next_string()?;
        let exchange = message.next_string()?;
        let open_close = message.next_int()?;
        let short_sale_slot = message.next_int()?;
        let designated_location = message.next_string()?;
        let exempt_code = message.next_int()?;

        contract.combo_legs.push(ComboLeg {
            contract_id,
            ratio,
            action,
            exchange,
            open_close: ComboLegOpenClose::from_i32(open_close),
            short_sale_slot,
            designated_location,
            exempt_code,
        });
    }

    // smart combo routing params
    let order_combo_legs_count = message.next_int()?;
    for _ in 0..order_combo_legs_count {
        let price = message.next_optional_double()?;

        order.order_combo_legs.push(OrderComboLeg { price });
    }

    let smart_combo_routing_params_count = message.next_int()?;
    for _ in 0..smart_combo_routing_params_count {
        order.smart_combo_routing_params.push(TagValue {
            tag: message.next_string()?,
            value: message.next_string()?,
        });
    }

    // scale order params
    order.scale_init_level_size = message.next_optional_int()?;
    order.scale_subs_level_size = message.next_optional_int()?;
    order.scale_price_increment = message.next_optional_double()?;

    if let Some(scale_price_increment) = order.scale_price_increment {
        if scale_price_increment > 0.0 {
            order.scale_price_adjust_value = message.next_optional_double()?;
            order.scale_price_adjust_interval = message.next_optional_int()?;
            order.scale_profit_offset = message.next_optional_double()?;
            order.scale_auto_reset = message.next_bool()?;
            order.scale_init_position = message.next_optional_int()?;
            order.scale_init_fill_qty = message.next_optional_int()?;
            order.scale_random_percent = message.next_bool()?;
        }
    }

    // hedge params
    order.hedge_type = message.next_string()?;
    if !order.hedge_type.is_empty() {
        order.hedge_param = message.next_string()?;
    }

    order.opt_out_smart_routing = message.next_bool()?;

    order.clearing_account = message.next_string()?;
    order.clearing_intent = message.next_string()?;

    order.not_held = message.next_bool()?;

    // delta neutral
    let has_delta_neutral_contract = message.next_bool()?;
    if has_delta_neutral_contract {
        contract.delta_neutral_contract = Some(DeltaNeutralContract {
            contract_id: message.next_int()?,
            delta: message.next_double()?,
            price: message.next_double()?,
        });
    }

    // algo params
    order.algo_strategy = message.next_string()?;
    if !order.algo_strategy.is_empty() {
        let algo_params_count = message.next_int()?;
        for _ in 0..algo_params_count {
            order.algo_params.push(TagValue {
                tag: message.next_string()?,
                value: message.next_string()?,
            });
        }
    }

    order.solicited = message.next_bool()?;

    // what_if and comission
    order.what_if = message.next_bool()?;
    order_state.status = message.next_string()?;
    if server_version >= server_versions::WHAT_IF_EXT_FIELDS {
        order_state.initial_margin_before = message.next_optional_double()?;
        order_state.maintenance_margin_before = message.next_optional_double()?;
        order_state.equity_with_loan_before = message.next_optional_double()?;
        order_state.initial_margin_change = message.next_optional_double()?;
        order_state.maintenance_margin_change = message.next_optional_double()?;
        order_state.equity_with_loan_change = message.next_optional_double()?;
    }
    order_state.initial_margin_after = message.next_optional_double()?;
    order_state.maintenance_margin_after = message.next_optional_double()?;
    order_state.equity_with_loan_after = message.next_optional_double()?;
    order_state.commission = message.next_optional_double()?;
    order_state.minimum_commission = message.next_optional_double()?;
    order_state.maximum_commission = message.next_optional_double()?;
    order_state.commission_currency = message.next_string()?;
    order_state.warning_text = message.next_string()?;

    // vol randomize flags
    order.randomize_size = message.next_bool()?;
    order.randomize_price = message.next_bool()?;

    if server_version >= server_versions::PEGGED_TO_BENCHMARK {
        if order.order_type == "PEG BENCH" {
            order.reference_contract_id = message.next_int()?;
            order.is_pegged_change_amount_decrease = message.next_bool()?;
            order.pegged_change_amount = message.next_optional_double()?;
            order.reference_change_amount = message.next_optional_double()?;
            order.reference_exchange = message.next_string()?;
        }
    }

    // Conditions
    if server_version >= server_versions::PEGGED_TO_BENCHMARK {
        let conditions_count = message.next_int()?;
        for _ in 0..conditions_count {
            let order_condition = message.next_int()?;
            order.conditions.push(OrderCondition::from_i32(order_condition));
        }
        if conditions_count > 0 {
            order.conditions_ignore_rth = message.next_bool()?;
            order.conditions_cancel_order = message.next_bool()?;
        }
    }

    // Adjusted order params
    if server_version >= server_versions::PEGGED_TO_BENCHMARK {
        order.adjusted_order_type = message.next_string()?;
        order.trigger_price = message.next_optional_double()?;
        order.trail_stop_price = message.next_optional_double()?;
        order.lmt_price_offset = message.next_optional_double()?;
        order.adjusted_stop_price = message.next_optional_double()?;
        order.adjusted_stop_limit_price = message.next_optional_double()?;
        order.adjusted_trailing_amount = message.next_optional_double()?;
        order.adjustable_trailing_unit = message.next_int()?;
    }

    if server_version >= server_versions::SOFT_DOLLAR_TIER {
        order.soft_dollar_tier = SoftDollarTier {
            name: message.next_string()?,
            value: message.next_string()?,
            display_name: message.next_string()?,
        };
    }

    if server_version >= server_versions::CASH_QTY {
        order.cash_qty = message.next_optional_double()?;
    }

    if server_version >= server_versions::AUTO_PRICE_FOR_HEDGE {
        order.dont_use_auto_price_for_hedge = message.next_bool()?;
    }

    if server_version >= server_versions::ORDER_CONTAINER {
        order.is_oms_container = message.next_bool()?;
    }

    if server_version >= server_versions::D_PEG_ORDERS {
        order.discretionary_up_to_limit_price = message.next_bool()?;
    }

    if server_version >= server_versions::PRICE_MGMT_ALGO {
        order.use_price_mgmt_algo = message.next_bool()?;
    }

    if server_version >= server_versions::DURATION {
        order.duration = message.next_optional_int()?;
    }

    if server_version >= server_versions::POST_TO_ATS {
        order.post_to_ats = message.next_optional_int()?;
    }

    if server_version >= server_versions::AUTO_CANCEL_PARENT {
        order.auto_cancel_parent = message.next_bool()?;
    }

    if server_version >= server_versions::PEGBEST_PEGMID_OFFSETS {
        order.min_trade_qty = message.next_optional_int()?;
        order.min_compete_size = message.next_optional_int()?;
        order.compete_against_best_offset = message.next_optional_double()?;
        order.mid_offset_at_whole = message.next_optional_double()?;
        order.mid_offset_at_half = message.next_optional_double()?;
    }

    Ok(open_order)
}

pub fn decode_order_status(server_version: i32, message: &mut ResponseMessage) -> Result<OrderStatus> {
    message.skip(); // message type

    if server_version < server_versions::MARKET_CAP_PRICE {
        message.skip(); // message version
    };

    let mut order_status = OrderStatus::default();

    order_status.order_id = message.next_int()?;
    order_status.status = message.next_string()?;
    order_status.filled = message.next_double()?;
    order_status.remaining = message.next_double()?;
    order_status.average_fill_price = message.next_double()?;
    order_status.perm_id = message.next_int()?;
    order_status.parent_id = message.next_int()?;
    order_status.last_fill_price = message.next_double()?;
    order_status.client_id = message.next_int()?;
    order_status.why_held = message.next_string()?;

    if server_version >= server_versions::MARKET_CAP_PRICE {
        order_status.market_cap_price = message.next_double()?;
    }

    Ok(order_status)
}

pub fn decode_execution_data(server_version: i32, message: &mut ResponseMessage) -> Result<ExecutionData> {
    message.skip(); // message type

    if server_version < server_versions::LAST_LIQUIDITY {
        message.skip(); // message version
    };

    let mut execution_data = ExecutionData::default();
    let contract = &mut execution_data.contract;
    let execution = &mut execution_data.execution;

    execution_data.request_id = message.next_int()?;
    execution.order_id = message.next_int()?;
    contract.contract_id = message.next_int()?;
    contract.symbol = message.next_string()?;
    let secutity_type = message.next_string()?;
    contract.security_type = SecurityType::from(&secutity_type);
    contract.last_trade_date_or_contract_month = message.next_string()?;
    contract.strike = message.next_double()?;
    contract.right = message.next_string()?;
    contract.multiplier = message.next_string()?;
    contract.exchange = message.next_string()?;
    contract.currency = message.next_string()?;
    contract.local_symbol = message.next_string()?;
    contract.trading_class = message.next_string()?;
    execution.execution_id = message.next_string()?;
    execution.time = message.next_string()?;
    execution.account_number = message.next_string()?;
    execution.exchange = message.next_string()?;
    execution.side = message.next_string()?;
    execution.shares = message.next_double()?;
    execution.price = message.next_double()?;
    execution.perm_id = message.next_int()?;
    execution.client_id = message.next_int()?;
    execution.liquidation = message.next_int()?;
    execution.cumulative_quantity = message.next_double()?;
    execution.average_price = message.next_double()?;
    execution.order_reference = message.next_string()?;
    execution.ev_rule = message.next_string()?;
    execution.ev_multiplier = message.next_optional_double()?;

    if server_version >= server_versions::MODELS_SUPPORT {
        execution.model_code = message.next_string()?;
    }

    if server_version >= server_versions::LAST_LIQUIDITY {
        execution.last_liquidity = Liquidity::from_i32(message.next_int()?);
    }

    Ok(execution_data)
}

pub fn decode_commission_report(_server_version: i32, message: &mut ResponseMessage) -> Result<CommissionReport> {
    message.skip(); // message type
    message.skip(); // message version

    let mut report = CommissionReport::default();

    report.execution_id = message.next_string()?;
    report.commission = message.next_double()?;
    report.currency = message.next_string()?;
    report.realized_pnl = message.next_optional_double()?;
    report.yields = message.next_optional_double()?;
    report.yield_redemption_date = message.next_string()?; // TODO: date as string?

    Ok(report)
}
