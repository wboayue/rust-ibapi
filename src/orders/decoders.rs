use super::*;

struct OrderDecoder {
    server_version: i32,
    message: ResponseMessage,
    order_id: i32,
    contract: Contract,
    order: Order,
    order_state: OrderState,
}

impl OrderDecoder {
    fn new(server_version: i32, mut message: ResponseMessage) -> Self {
        message.skip(); // message type

        if server_version < server_versions::ORDER_CONTAINER {
            message.skip(); // message version
        }

        Self {
            server_version,
            message,
            order_id: -1,
            contract: Contract::default(),
            order: Order::default(),
            order_state: OrderState::default(),
        }
    }

    fn read_order_id(&mut self) -> Result<()> {
        self.order_id = self.message.next_int()?;
        self.order.order_id = self.order_id;

        Ok(())
    }

    fn read_contract_fields(&mut self) -> Result<()> {
        let message = &mut self.message;
        let contract = &mut self.contract;

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

        Ok(())
    }

    fn read_action(&mut self) -> Result<()> {
        let action = self.message.next_string()?;
        self.order.action = Action::from(&action);

        Ok(())
    }

    fn read_total_quantity(&mut self) -> Result<()> {
        self.order.total_quantity = self.message.next_double()?;
        Ok(())
    }

    fn read_order_type(&mut self) -> Result<()> {
        self.order.order_type = self.message.next_string()?;
        Ok(())
    }

    fn read_limit_price(&mut self) -> Result<()> {
        self.order.limit_price = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_aux_price(&mut self) -> Result<()> {
        self.order.aux_price = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_tif(&mut self) -> Result<()> {
        self.order.tif = self.message.next_string()?;
        Ok(())
    }

    fn read_oca_group(&mut self) -> Result<()> {
        self.order.oca_group = self.message.next_string()?;
        Ok(())
    }

    fn read_account(&mut self) -> Result<()> {
        self.order.account = self.message.next_string()?;
        Ok(())
    }

    fn read_open_close(&mut self) -> Result<()> {
        let open_close = self.message.next_string()?;
        self.order.open_close = OrderOpenClose::from(&open_close);
        Ok(())
    }

    fn read_origin(&mut self) -> Result<()> {
        self.order.origin = self.message.next_int()?;
        Ok(())
    }

    fn read_order_ref(&mut self) -> Result<()> {
        self.order.order_ref = self.message.next_string()?;
        Ok(())
    }

    fn read_client_id(&mut self) -> Result<()> {
        self.order.client_id = self.message.next_int()?;
        Ok(())
    }

    fn read_perm_id(&mut self) -> Result<()> {
        self.order.perm_id = self.message.next_int()?;
        Ok(())
    }

    fn read_outside_rth(&mut self) -> Result<()> {
        self.order.outside_rth = self.message.next_bool()?;
        Ok(())
    }

    fn read_hidden(&mut self) -> Result<()> {
        self.order.hidden = self.message.next_bool()?;
        Ok(())
    }

    fn read_discretionary_amt(&mut self) -> Result<()> {
        self.order.discretionary_amt = self.message.next_double()?;
        Ok(())
    }

    fn read_good_after_time(&mut self) -> Result<()> {
        self.order.good_after_time = self.message.next_string()?;
        Ok(())
    }

    // skips deprecated shares_allocation field
    fn skip_shares_allocation(&mut self) {
        self.message.skip();
    }

    fn read_fa_params(&mut self) -> Result<()> {
        self.order.fa_group = self.message.next_string()?;
        self.order.fa_method = self.message.next_string()?;
        self.order.fa_percentage = self.message.next_string()?;
        if self.server_version < server_versions::FA_PROFILE_DESUPPORT {
            self.order.fa_percentage = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_model_code(&mut self) -> Result<()> {
        if self.server_version >= server_versions::MODELS_SUPPORT {
            self.order.model_code = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_good_till_date(&mut self) -> Result<()> {
        self.order.good_till_date = self.message.next_string()?;
        Ok(())
    }

    fn decode_rule_80_a(&mut self) -> Result<()> {
        let rule_80_a = self.message.next_string()?;
        self.order.rule_80_a = Rule80A::from(&rule_80_a);
        Ok(())
    }

    fn decode_percent_offset(&mut self) -> Result<()> {
        self.order.percent_offset = self.message.next_optional_double()?;
        Ok(())
    }

    fn decode_settling_firm(&mut self) -> Result<()> {
        self.order.settling_firm = self.message.next_string()?;
        Ok(())
    }

    fn decode_short_sale_params(&mut self) -> Result<()> {
        self.order.short_sale_slot = self.message.next_int()?;
        self.order.designated_location = self.message.next_string()?;
        self.order.exempt_code = self.message.next_int()?;
        Ok(())
    }

    fn decode_auction_strategy(&mut self) -> Result<()> {
        self.order.auction_strategy = self.message.next_optional_int()?;
        Ok(())
    }

    fn decode_box_order_params(&mut self) -> Result<()> {
        self.order.starting_price = self.message.next_optional_double()?;
        self.order.stock_ref_price = self.message.next_optional_double()?;
        self.order.delta = self.message.next_optional_double()?;
        Ok(())
    }

    fn decode_peg_to_stock_or_vol_order_params(&mut self) -> Result<()> {
        self.order.stock_range_lower = self.message.next_optional_double()?;
        self.order.stock_range_upper = self.message.next_optional_double()?;
        Ok(())
    }

    fn decode_display_size(&mut self) -> Result<()> {
        self.order.display_size = self.message.next_optional_int()?;
        Ok(())
    }

    fn decode_block_order(&mut self) -> Result<()> {
        self.order.block_order = self.message.next_bool()?;
        Ok(())
    }

    fn decode_sweep_to_fill(&mut self) -> Result<()> {
        self.order.sweep_to_fill = self.message.next_bool()?;
        Ok(())
    }

    fn decode_all_or_none(&mut self) -> Result<()> {
        self.order.all_or_none = self.message.next_bool()?;
        Ok(())
    }

    fn decode_min_qty(&mut self) -> Result<()> {
        self.order.min_qty = self.message.next_optional_int()?;
        Ok(())
    }

    fn decode_oca_type(&mut self) -> Result<()> {
        self.order.oca_type = self.message.next_int()?;
        Ok(())
    }

    fn skip_etrade_only(&mut self) {
        self.message.skip();
    }

    fn skip_firm_quote_only(&mut self) {
        self.message.skip();
    }

    fn skip_nbbo_price_cap(&mut self) {
        self.message.skip();
    }

    fn decode_parent_id(&mut self) -> Result<()> {
        self.order.parent_id = self.message.next_int()?;
        Ok(())
    }

    fn decode_trigger_method(&mut self) -> Result<()> {
        self.order.trigger_method = self.message.next_int()?;
        Ok(())
    }

    fn decode_volatility_order_params(&mut self) -> Result<()> {
        self.order.volatility = self.message.next_optional_double()?;
        self.order.volatility_type = self.message.next_optional_int()?;
        self.order.delta_neutral_order_type = self.message.next_string()?;
        self.order.delta_neutral_aux_price = self.message.next_optional_double()?;

        if self.order.is_delta_neutral() {
            self.order.delta_neutral_con_id = self.message.next_int()?;
            self.order.delta_neutral_settling_firm = self.message.next_string()?;
            self.order.delta_neutral_clearing_account = self.message.next_string()?;
            self.order.delta_neutral_clearing_intent = self.message.next_string()?;
            self.order.delta_neutral_open_close = self.message.next_string()?;
            self.order.delta_neutral_short_sale = self.message.next_bool()?;
            self.order.delta_neutral_short_sale_slot = self.message.next_int()?;
            self.order.delta_neutral_designated_location = self.message.next_string()?;
        }

        self.order.continuous_update = self.message.next_bool()?;
        self.order.reference_price_type = self.message.next_optional_int()?;

        Ok(())
    }

    fn decode_trail_params(&mut self) -> Result<()> {
        self.order.trail_stop_price = self.message.next_optional_double()?;
        self.order.trailing_percent = self.message.next_optional_double()?;
        Ok(())
    }

    fn decode_basis_points(&mut self) -> Result<()> {
        self.order.basis_points = self.message.next_optional_double()?;
        self.order.basis_points_type = self.message.next_optional_int()?;
        Ok(())
    }

    fn decode_combo_legs(&mut self) -> Result<()> {
        self.contract.combo_legs_description = self.message.next_string()?;

        let combo_legs_count = self.message.next_int()?;
        for _ in 0..combo_legs_count {
            let contract_id = self.message.next_int()?;
            let ratio = self.message.next_int()?;
            let action = self.message.next_string()?;
            let exchange = self.message.next_string()?;
            let open_close = self.message.next_int()?;
            let short_sale_slot = self.message.next_int()?;
            let designated_location = self.message.next_string()?;
            let exempt_code = self.message.next_int()?;

            self.contract.combo_legs.push(ComboLeg {
                contract_id,
                ratio,
                action,
                exchange,
                open_close: ComboLegOpenClose::from(open_close),
                short_sale_slot,
                designated_location,
                exempt_code,
            });
        }

        let order_combo_legs_count = self.message.next_int()?;
        for _ in 0..order_combo_legs_count {
            let price = self.message.next_optional_double()?;

            self.order.order_combo_legs.push(OrderComboLeg { price });
        }

        Ok(())
    }

    fn decode_smart_combo_routing_params(&mut self) -> Result<()> {
        // smart combo routing params
        let smart_combo_routing_params_count = self.message.next_int()?;
        for _ in 0..smart_combo_routing_params_count {
            self.order.smart_combo_routing_params.push(TagValue {
                tag: self.message.next_string()?,
                value: self.message.next_string()?,
            });
        }

        Ok(())
    }

    fn decode_scale_order_params(&mut self) -> Result<()> {
        self.order.scale_init_level_size = self.message.next_optional_int()?;
        self.order.scale_subs_level_size = self.message.next_optional_int()?;
        self.order.scale_price_increment = self.message.next_optional_double()?;

        if let Some(scale_price_increment) = self.order.scale_price_increment {
            if scale_price_increment > 0.0 {
                self.order.scale_price_adjust_value = self.message.next_optional_double()?;
                self.order.scale_price_adjust_interval = self.message.next_optional_int()?;
                self.order.scale_profit_offset = self.message.next_optional_double()?;
                self.order.scale_auto_reset = self.message.next_bool()?;
                self.order.scale_init_position = self.message.next_optional_int()?;
                self.order.scale_init_fill_qty = self.message.next_optional_int()?;
                self.order.scale_random_percent = self.message.next_bool()?;
            }
        }

        Ok(())
    }

    fn decode_hedge_params(&mut self) -> Result<()> {
        self.order.hedge_type = self.message.next_string()?;
        if !self.order.hedge_type.is_empty() {
            self.order.hedge_param = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_opt_out_smart_routing(&mut self) -> Result<()> {
        self.order.opt_out_smart_routing = self.message.next_bool()?;
        Ok(())
    }

    fn read_clearing_params(&mut self) -> Result<()> {
        self.order.clearing_account = self.message.next_string()?;
        self.order.clearing_intent = self.message.next_string()?;
        Ok(())
    }

    fn read_not_held(&mut self) -> Result<()> {
        self.order.not_held = self.message.next_bool()?;
        Ok(())
    }

    fn read_delta_neutral(&mut self) -> Result<()> {
        let has_delta_neutral_contract = self.message.next_bool()?;
        if has_delta_neutral_contract {
            self.contract.delta_neutral_contract = Some(DeltaNeutralContract {
                contract_id: self.message.next_int()?,
                delta: self.message.next_double()?,
                price: self.message.next_double()?,
            });
        }
        Ok(())
    }

    fn read_algo_params(&mut self) -> Result<()> {
        self.order.algo_strategy = self.message.next_string()?;
        if !self.order.algo_strategy.is_empty() {
            let algo_params_count = self.message.next_int()?;
            for _ in 0..algo_params_count {
                self.order.algo_params.push(TagValue {
                    tag: self.message.next_string()?,
                    value: self.message.next_string()?,
                });
            }
        }

        Ok(())
    }

    fn read_solicited(&mut self) -> Result<()> {
        self.order.solicited = self.message.next_bool()?;
        Ok(())
    }
}
pub fn decode_open_order(server_version: i32, mut message: ResponseMessage) -> Result<OpenOrder> {
    let mut decoder = OrderDecoder::new(server_version, message);

    let mut open_order = OpenOrder::default();

    decoder.read_order_id()?;

    // Contract fields
    decoder.read_contract_fields()?;

    // Order fields
    decoder.read_action()?;
    decoder.read_total_quantity()?;
    decoder.read_order_type()?;
    decoder.read_limit_price()?;
    decoder.read_aux_price()?;
    decoder.read_tif()?;
    decoder.read_oca_group()?;
    decoder.read_account()?;
    decoder.read_open_close()?;
    decoder.read_origin()?;
    decoder.read_order_ref()?;
    decoder.read_client_id()?;
    decoder.read_perm_id()?;
    decoder.read_outside_rth()?;
    decoder.read_hidden()?;
    decoder.read_discretionary_amt()?;
    decoder.read_good_after_time()?;
    decoder.skip_shares_allocation();
    decoder.read_fa_params()?;
    decoder.read_model_code()?;
    decoder.read_good_till_date()?;
    decoder.decode_rule_80_a()?;
    decoder.decode_percent_offset()?;
    decoder.decode_settling_firm()?;
    decoder.decode_short_sale_params()?;
    decoder.decode_auction_strategy()?;
    decoder.decode_box_order_params()?;
    decoder.decode_peg_to_stock_or_vol_order_params()?;
    decoder.decode_display_size()?;
    decoder.decode_block_order()?;
    decoder.decode_sweep_to_fill()?;
    decoder.decode_all_or_none()?;
    decoder.decode_min_qty()?;
    decoder.decode_oca_type()?;
    decoder.skip_etrade_only();
    decoder.skip_firm_quote_only();
    decoder.skip_nbbo_price_cap();
    decoder.decode_parent_id()?;
    decoder.decode_trigger_method()?;
    decoder.decode_volatility_order_params()?;
    decoder.decode_trail_params()?;
    decoder.decode_basis_points()?;
    decoder.decode_combo_legs()?;
    decoder.decode_smart_combo_routing_params()?;
    decoder.decode_scale_order_params()?;
    decoder.decode_hedge_params()?;
    decoder.read_opt_out_smart_routing()?;
    decoder.read_clearing_params()?;
    decoder.read_not_held()?;
    decoder.read_delta_neutral()?;
    decoder.read_algo_params()?;
    decoder.read_solicited()?;

    open_order.order_id = decoder.order_id;
    open_order.contract = Box::new(decoder.contract);
    open_order.order = Box::new(decoder.order);
    open_order.order_state = Box::new(decoder.order_state);

    let order = &mut open_order.order;
    let order_state = &mut open_order.order_state;

    let mut message = decoder.message.clone();

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

    if server_version >= server_versions::PEGGED_TO_BENCHMARK && order.order_type == "PEG BENCH" {
        order.reference_contract_id = message.next_int()?;
        order.is_pegged_change_amount_decrease = message.next_bool()?;
        order.pegged_change_amount = message.next_optional_double()?;
        order.reference_change_amount = message.next_optional_double()?;
        order.reference_exchange = message.next_string()?;
    }

    // Conditions
    if server_version >= server_versions::PEGGED_TO_BENCHMARK {
        let conditions_count = message.next_int()?;
        for _ in 0..conditions_count {
            let order_condition = message.next_int()?;
            order.conditions.push(OrderCondition::from(order_condition));
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
        execution.last_liquidity = Liquidity::from(message.next_int()?);
    }

    Ok(execution_data)
}

pub fn decode_commission_report(_server_version: i32, message: &mut ResponseMessage) -> Result<CommissionReport> {
    message.skip(); // message type
    message.skip(); // message version

    Ok(CommissionReport {
        execution_id: message.next_string()?,
        commission: message.next_double()?,
        currency: message.next_string()?,
        realized_pnl: message.next_optional_double()?,
        yields: message.next_optional_double()?,
        yield_redemption_date: message.next_string()?, // TODO: use date type?
    })
}

pub fn decode_completed_orders(_server_version: i32, message: &mut ResponseMessage) -> Result<()> {
    // // read contract fields
    // eOrderDecoder.readContractFields();

    // // read order fields
    // eOrderDecoder.readAction();
    // eOrderDecoder.readTotalQuantity();
    // eOrderDecoder.readOrderType();
    // eOrderDecoder.readLmtPrice();
    // eOrderDecoder.readAuxPrice();
    // eOrderDecoder.readTIF();
    // eOrderDecoder.readOcaGroup();
    // eOrderDecoder.readAccount();
    // eOrderDecoder.readOpenClose();
    // eOrderDecoder.readOrigin();
    // eOrderDecoder.readOrderRef();
    // eOrderDecoder.readPermId();
    // eOrderDecoder.readOutsideRth();
    // eOrderDecoder.readHidden();
    // eOrderDecoder.readDiscretionaryAmount();
    // eOrderDecoder.readGoodAfterTime();
    // eOrderDecoder.readFAParams();
    // eOrderDecoder.readModelCode();
    // eOrderDecoder.readGoodTillDate();
    // eOrderDecoder.readRule80A();
    // eOrderDecoder.readPercentOffset();
    // eOrderDecoder.readSettlingFirm();
    // eOrderDecoder.readShortSaleParams();
    // eOrderDecoder.readBoxOrderParams();
    // eOrderDecoder.readPegToStkOrVolOrderParams();
    // eOrderDecoder.readDisplaySize();
    // eOrderDecoder.readSweepToFill();
    // eOrderDecoder.readAllOrNone();
    // eOrderDecoder.readMinQty();
    // eOrderDecoder.readOcaType();
    // eOrderDecoder.readTriggerMethod();
    // eOrderDecoder.readVolOrderParams(false);
    // eOrderDecoder.readTrailParams();
    // eOrderDecoder.readComboLegs();
    // eOrderDecoder.readSmartComboRoutingParams();
    // eOrderDecoder.readScaleOrderParams();
    // eOrderDecoder.readHedgeParams();
    // eOrderDecoder.readClearingParams();
    // eOrderDecoder.readNotHeld();
    // eOrderDecoder.readDeltaNeutral();
    // eOrderDecoder.readAlgoParams();
    // eOrderDecoder.readSolicited();
    // eOrderDecoder.readOrderStatus();
    // eOrderDecoder.readVolRandomizeFlags();
    // eOrderDecoder.readPegToBenchParams();
    // eOrderDecoder.readConditions();
    // eOrderDecoder.readStopPriceAndLmtPriceOffset();
    // eOrderDecoder.readCashQty();
    // eOrderDecoder.readDontUseAutoPriceForHedge();
    // eOrderDecoder.readIsOmsContainer();
    // eOrderDecoder.readAutoCancelDate();
    // eOrderDecoder.readFilledQuantity();
    // eOrderDecoder.readRefFuturesConId();
    // eOrderDecoder.readAutoCancelParent();
    // eOrderDecoder.readShareholder();
    // eOrderDecoder.readImbalanceOnly();
    // eOrderDecoder.readRouteMarketableToBbo();
    // eOrderDecoder.readParentPermId();
    // eOrderDecoder.readCompletedTime();
    // eOrderDecoder.readCompletedStatus();
    // eOrderDecoder.readPegBestPegMidOrderAttributes();

    // eWrapper.completedOrder(contract, order, orderState);

    Ok(())
}

pub fn decode_open_orders(server_version: i32, message: ResponseMessage) -> Result<()> {
    let mut decoder = OrderDecoder::new(server_version, message);

    decoder.read_order_id()?;

    // Contract fields
    decoder.read_contract_fields()?;

    // Order fields
    decoder.read_action()?;

    // // read order fields
    // eOrderDecoder.readAction();
    // eOrderDecoder.readTotalQuantity();
    // eOrderDecoder.readOrderType();
    // eOrderDecoder.readLmtPrice();
    // eOrderDecoder.readAuxPrice();
    // eOrderDecoder.readTIF();
    // eOrderDecoder.readOcaGroup();
    // eOrderDecoder.readAccount();
    // eOrderDecoder.readOpenClose();
    // eOrderDecoder.readOrigin();
    // eOrderDecoder.readOrderRef();
    // eOrderDecoder.readClientId();
    // eOrderDecoder.readPermId();
    // eOrderDecoder.readOutsideRth();
    // eOrderDecoder.readHidden();
    // eOrderDecoder.readDiscretionaryAmount();
    // eOrderDecoder.readGoodAfterTime();
    // eOrderDecoder.skipSharesAllocation();
    // eOrderDecoder.readFAParams();
    // eOrderDecoder.readModelCode();
    // eOrderDecoder.readGoodTillDate();
    // eOrderDecoder.readRule80A();
    // eOrderDecoder.readPercentOffset();
    // eOrderDecoder.readSettlingFirm();
    // eOrderDecoder.readShortSaleParams();
    // eOrderDecoder.readAuctionStrategy();
    // eOrderDecoder.readBoxOrderParams();
    // eOrderDecoder.readPegToStkOrVolOrderParams();
    // eOrderDecoder.readDisplaySize();
    // eOrderDecoder.readOldStyleOutsideRth();
    // eOrderDecoder.readBlockOrder();
    // eOrderDecoder.readSweepToFill();
    // eOrderDecoder.readAllOrNone();
    // eOrderDecoder.readMinQty();
    // eOrderDecoder.readOcaType();
    // eOrderDecoder.skipETradeOnly();
    // eOrderDecoder.skipFirmQuoteOnly();
    // eOrderDecoder.skipNbboPriceCap();
    // eOrderDecoder.readParentId();
    // eOrderDecoder.readTriggerMethod();
    // eOrderDecoder.readVolOrderParams(true);
    // eOrderDecoder.readTrailParams();
    // eOrderDecoder.readBasisPoints();
    // eOrderDecoder.readComboLegs();
    // eOrderDecoder.readSmartComboRoutingParams();
    // eOrderDecoder.readScaleOrderParams();
    // eOrderDecoder.readHedgeParams();
    // eOrderDecoder.readOptOutSmartRouting();
    // eOrderDecoder.readClearingParams();
    // eOrderDecoder.readNotHeld();
    // eOrderDecoder.readDeltaNeutral();
    // eOrderDecoder.readAlgoParams();
    // eOrderDecoder.readSolicited();
    // eOrderDecoder.readWhatIfInfoAndCommission();
    // eOrderDecoder.readVolRandomizeFlags();
    // eOrderDecoder.readPegToBenchParams();
    // eOrderDecoder.readConditions();
    // eOrderDecoder.readAdjustedOrderParams();
    // eOrderDecoder.readSoftDollarTier();
    // eOrderDecoder.readCashQty();
    // eOrderDecoder.readDontUseAutoPriceForHedge();
    // eOrderDecoder.readIsOmsContainer();
    // eOrderDecoder.readDiscretionaryUpToLimitPrice();
    // eOrderDecoder.readUsePriceMgmtAlgo();
    // eOrderDecoder.readDuration();
    // eOrderDecoder.readPostToAts();
    // eOrderDecoder.readAutoCancelParent(MinServerVer.AUTO_CANCEL_PARENT);
    // eOrderDecoder.readPegBestPegMidOrderAttributes();

    // eWrapper.openOrder(order.OrderId, contract, order, orderState);

    Ok(())
}
