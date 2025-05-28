use crate::Error;

use super::*;

/// Helper struct for decoding order messages from TWS.
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

    fn read_order_id(&mut self) -> Result<(), Error> {
        self.order_id = self.message.next_int()?;
        self.order.order_id = self.order_id;

        Ok(())
    }

    fn read_contract_fields(&mut self) -> Result<(), Error> {
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

    fn read_action(&mut self) -> Result<(), Error> {
        let action = self.message.next_string()?;
        self.order.action = Action::from(&action);

        Ok(())
    }

    fn read_total_quantity(&mut self) -> Result<(), Error> {
        self.order.total_quantity = self.message.next_double()?;
        Ok(())
    }

    fn read_order_type(&mut self) -> Result<(), Error> {
        self.order.order_type = self.message.next_string()?;
        Ok(())
    }

    fn read_limit_price(&mut self) -> Result<(), Error> {
        self.order.limit_price = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_aux_price(&mut self) -> Result<(), Error> {
        self.order.aux_price = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_tif(&mut self) -> Result<(), Error> {
        self.order.tif = self.message.next_string()?;
        Ok(())
    }

    fn read_oca_group(&mut self) -> Result<(), Error> {
        self.order.oca_group = self.message.next_string()?;
        Ok(())
    }

    fn read_account(&mut self) -> Result<(), Error> {
        self.order.account = self.message.next_string()?;
        Ok(())
    }

    fn read_open_close(&mut self) -> Result<(), Error> {
        let open_close = self.message.next_string()?;
        self.order.open_close = OrderOpenClose::from(&open_close);
        Ok(())
    }

    fn read_origin(&mut self) -> Result<(), Error> {
        self.order.origin = self.message.next_int()?;
        Ok(())
    }

    fn read_order_ref(&mut self) -> Result<(), Error> {
        self.order.order_ref = self.message.next_string()?;
        Ok(())
    }

    fn read_client_id(&mut self) -> Result<(), Error> {
        self.order.client_id = self.message.next_int()?;
        Ok(())
    }

    fn read_perm_id(&mut self) -> Result<(), Error> {
        self.order.perm_id = self.message.next_int()?;
        Ok(())
    }

    fn read_outside_rth(&mut self) -> Result<(), Error> {
        self.order.outside_rth = self.message.next_bool()?;
        Ok(())
    }

    fn read_hidden(&mut self) -> Result<(), Error> {
        self.order.hidden = self.message.next_bool()?;
        Ok(())
    }

    fn read_discretionary_amt(&mut self) -> Result<(), Error> {
        self.order.discretionary_amt = self.message.next_double()?;
        Ok(())
    }

    fn read_good_after_time(&mut self) -> Result<(), Error> {
        self.order.good_after_time = self.message.next_string()?;
        Ok(())
    }

    // skips deprecated shares_allocation field
    fn skip_shares_allocation(&mut self) {
        self.message.skip();
    }

    fn read_fa_params(&mut self) -> Result<(), Error> {
        self.order.fa_group = self.message.next_string()?;
        self.order.fa_method = self.message.next_string()?;
        self.order.fa_percentage = self.message.next_string()?;
        if self.server_version < server_versions::FA_PROFILE_DESUPPORT {
            self.order.fa_percentage = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_model_code(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::MODELS_SUPPORT {
            self.order.model_code = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_good_till_date(&mut self) -> Result<(), Error> {
        self.order.good_till_date = self.message.next_string()?;
        Ok(())
    }

    fn read_rule_80_a(&mut self) -> Result<(), Error> {
        let rule_80_a = self.message.next_string()?;
        self.order.rule_80_a = Rule80A::from(&rule_80_a);
        Ok(())
    }

    fn read_percent_offset(&mut self) -> Result<(), Error> {
        self.order.percent_offset = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_settling_firm(&mut self) -> Result<(), Error> {
        self.order.settling_firm = self.message.next_string()?;
        Ok(())
    }

    fn read_short_sale_params(&mut self) -> Result<(), Error> {
        self.order.short_sale_slot = self.message.next_int()?;
        self.order.designated_location = self.message.next_string()?;
        self.order.exempt_code = self.message.next_int()?;
        Ok(())
    }

    fn read_auction_strategy(&mut self) -> Result<(), Error> {
        self.order.auction_strategy = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_box_order_params(&mut self) -> Result<(), Error> {
        self.order.starting_price = self.message.next_optional_double()?;
        self.order.stock_ref_price = self.message.next_optional_double()?;
        self.order.delta = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_peg_to_stock_or_vol_order_params(&mut self) -> Result<(), Error> {
        self.order.stock_range_lower = self.message.next_optional_double()?;
        self.order.stock_range_upper = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_display_size(&mut self) -> Result<(), Error> {
        self.order.display_size = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_block_order(&mut self) -> Result<(), Error> {
        self.order.block_order = self.message.next_bool()?;
        Ok(())
    }

    fn read_sweep_to_fill(&mut self) -> Result<(), Error> {
        self.order.sweep_to_fill = self.message.next_bool()?;
        Ok(())
    }

    fn read_all_or_none(&mut self) -> Result<(), Error> {
        self.order.all_or_none = self.message.next_bool()?;
        Ok(())
    }

    fn read_min_qty(&mut self) -> Result<(), Error> {
        self.order.min_qty = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_oca_type(&mut self) -> Result<(), Error> {
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

    fn read_parent_id(&mut self) -> Result<(), Error> {
        self.order.parent_id = self.message.next_int()?;
        Ok(())
    }

    fn read_trigger_method(&mut self) -> Result<(), Error> {
        self.order.trigger_method = self.message.next_int()?;
        Ok(())
    }

    fn read_volatility_order_params(&mut self, read_open_order_attributes: bool) -> Result<(), Error> {
        self.order.volatility = self.message.next_optional_double()?;
        self.order.volatility_type = self.message.next_optional_int()?;
        self.order.delta_neutral_order_type = self.message.next_string()?;
        self.order.delta_neutral_aux_price = self.message.next_optional_double()?;

        if self.order.is_delta_neutral() {
            self.order.delta_neutral_con_id = self.message.next_int()?;
            if read_open_order_attributes {
                self.order.delta_neutral_settling_firm = self.message.next_string()?;
                self.order.delta_neutral_clearing_account = self.message.next_string()?;
                self.order.delta_neutral_clearing_intent = self.message.next_string()?;
                self.order.delta_neutral_open_close = self.message.next_string()?;
            }
            self.order.delta_neutral_short_sale = self.message.next_bool()?;
            self.order.delta_neutral_short_sale_slot = self.message.next_int()?;
            self.order.delta_neutral_designated_location = self.message.next_string()?;
        }

        self.order.continuous_update = self.message.next_bool()?;
        self.order.reference_price_type = self.message.next_optional_int()?;

        Ok(())
    }

    fn read_trail_params(&mut self) -> Result<(), Error> {
        self.order.trail_stop_price = self.message.next_optional_double()?;
        self.order.trailing_percent = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_basis_points(&mut self) -> Result<(), Error> {
        self.order.basis_points = self.message.next_optional_double()?;
        self.order.basis_points_type = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_combo_legs(&mut self) -> Result<(), Error> {
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

    fn read_smart_combo_routing_params(&mut self) -> Result<(), Error> {
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

    fn read_scale_order_params(&mut self) -> Result<(), Error> {
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

    fn read_hedge_params(&mut self) -> Result<(), Error> {
        self.order.hedge_type = self.message.next_string()?;
        if !self.order.hedge_type.is_empty() {
            self.order.hedge_param = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_opt_out_smart_routing(&mut self) -> Result<(), Error> {
        self.order.opt_out_smart_routing = self.message.next_bool()?;
        Ok(())
    }

    fn read_clearing_params(&mut self) -> Result<(), Error> {
        self.order.clearing_account = self.message.next_string()?;
        self.order.clearing_intent = self.message.next_string()?;
        Ok(())
    }

    fn read_not_held(&mut self) -> Result<(), Error> {
        self.order.not_held = self.message.next_bool()?;
        Ok(())
    }

    fn read_delta_neutral(&mut self) -> Result<(), Error> {
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

    fn read_algo_params(&mut self) -> Result<(), Error> {
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

    fn read_solicited(&mut self) -> Result<(), Error> {
        self.order.solicited = self.message.next_bool()?;
        Ok(())
    }

    fn read_what_if_info_and_commission(&mut self) -> Result<(), Error> {
        self.order.what_if = self.message.next_bool()?;
        self.read_order_status()?;

        if self.server_version >= server_versions::WHAT_IF_EXT_FIELDS {
            self.order_state.initial_margin_before = self.message.next_optional_double()?;
            self.order_state.maintenance_margin_before = self.message.next_optional_double()?;
            self.order_state.equity_with_loan_before = self.message.next_optional_double()?;
            self.order_state.initial_margin_change = self.message.next_optional_double()?;
            self.order_state.maintenance_margin_change = self.message.next_optional_double()?;
            self.order_state.equity_with_loan_change = self.message.next_optional_double()?;
        }

        self.order_state.initial_margin_after = self.message.next_optional_double()?;
        self.order_state.maintenance_margin_after = self.message.next_optional_double()?;
        self.order_state.equity_with_loan_after = self.message.next_optional_double()?;
        self.order_state.commission = self.message.next_optional_double()?;
        self.order_state.minimum_commission = self.message.next_optional_double()?;
        self.order_state.maximum_commission = self.message.next_optional_double()?;
        self.order_state.commission_currency = self.message.next_string()?;
        self.order_state.warning_text = self.message.next_string()?;

        Ok(())
    }

    fn read_vol_randomize_flags(&mut self) -> Result<(), Error> {
        self.order.randomize_size = self.message.next_bool()?;
        self.order.randomize_price = self.message.next_bool()?;
        Ok(())
    }

    fn read_peg_to_bench_params(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::PEGGED_TO_BENCHMARK && self.order.order_type == "PEG BENCH" {
            self.order.reference_contract_id = self.message.next_int()?;
            self.order.is_pegged_change_amount_decrease = self.message.next_bool()?;
            self.order.pegged_change_amount = self.message.next_optional_double()?;
            self.order.reference_change_amount = self.message.next_optional_double()?;
            self.order.reference_exchange = self.message.next_string()?;
        }
        Ok(())
    }

    fn read_conditions(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::PEGGED_TO_BENCHMARK {
            let conditions_count = self.message.next_int()?;
            for _ in 0..conditions_count {
                let order_condition = self.message.next_int()?;
                self.order.conditions.push(OrderCondition::from(order_condition));
            }
            if conditions_count > 0 {
                self.order.conditions_ignore_rth = self.message.next_bool()?;
                self.order.conditions_cancel_order = self.message.next_bool()?;
            }
        }
        Ok(())
    }

    fn read_adjusted_order_params(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::PEGGED_TO_BENCHMARK {
            self.order.adjusted_order_type = self.message.next_string()?;
            self.order.trigger_price = self.message.next_optional_double()?;
            self.order.trail_stop_price = self.message.next_optional_double()?;
            self.order.limit_price_offset = self.message.next_optional_double()?;
            self.order.adjusted_stop_price = self.message.next_optional_double()?;
            self.order.adjusted_stop_limit_price = self.message.next_optional_double()?;
            self.order.adjusted_trailing_amount = self.message.next_optional_double()?;
            self.order.adjustable_trailing_unit = self.message.next_int()?;
        }
        Ok(())
    }

    fn read_soft_dollar_tier(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::SOFT_DOLLAR_TIER {
            self.order.soft_dollar_tier = SoftDollarTier {
                name: self.message.next_string()?,
                value: self.message.next_string()?,
                display_name: self.message.next_string()?,
            };
        }
        Ok(())
    }

    fn read_cash_qty(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::CASH_QTY {
            self.order.cash_qty = self.message.next_optional_double()?;
        }
        Ok(())
    }

    fn read_dont_use_auto_price_for_hedge(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::AUTO_PRICE_FOR_HEDGE {
            self.order.dont_use_auto_price_for_hedge = self.message.next_bool()?;
        }
        Ok(())
    }

    fn read_is_oms_container(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::ORDER_CONTAINER {
            self.order.is_oms_container = self.message.next_bool()?;
        }
        Ok(())
    }

    fn read_discretionary_up_to_limit_price(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::D_PEG_ORDERS {
            self.order.discretionary_up_to_limit_price = self.message.next_bool()?;
        }
        Ok(())
    }

    fn read_use_price_mgmt_algo(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::PRICE_MGMT_ALGO {
            self.order.use_price_mgmt_algo = self.message.next_bool()?;
        }
        Ok(())
    }

    fn read_duration(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::DURATION {
            self.order.duration = self.message.next_optional_int()?;
        }
        Ok(())
    }

    fn read_post_to_ats(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::POST_TO_ATS {
            self.order.post_to_ats = self.message.next_optional_int()?;
        }
        Ok(())
    }

    fn read_auto_cancel_parent(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::AUTO_CANCEL_PARENT {
            self.order.auto_cancel_parent = self.message.next_bool()?;
        }
        Ok(())
    }

    fn read_peg_best_peg_mid_order_attributes(&mut self) -> Result<(), Error> {
        if self.server_version >= server_versions::PEGBEST_PEGMID_OFFSETS {
            self.order.min_trade_qty = self.message.next_optional_int()?;
            self.order.min_compete_size = self.message.next_optional_int()?;
            self.order.compete_against_best_offset = self.message.next_optional_double()?;
            self.order.mid_offset_at_whole = self.message.next_optional_double()?;
            self.order.mid_offset_at_half = self.message.next_optional_double()?;
        }
        Ok(())
    }

    fn read_order_status(&mut self) -> Result<(), Error> {
        self.order_state.status = self.message.next_string()?;
        Ok(())
    }

    fn read_stop_price_and_limit_price_offset(&mut self) -> Result<(), Error> {
        self.order.trail_stop_price = self.message.next_optional_double()?;
        self.order.limit_price_offset = self.message.next_optional_double()?;
        Ok(())
    }

    fn read_auto_cancel_date(&mut self) -> Result<(), Error> {
        self.order.auto_cancel_date = self.message.next_string()?;
        Ok(())
    }

    fn read_filled_quantity(&mut self) -> Result<(), Error> {
        self.order.filled_quantity = self.message.next_double()?;
        Ok(())
    }

    fn read_ref_futures_contract_id(&mut self) -> Result<(), Error> {
        self.order.ref_futures_con_id = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_shareholder(&mut self) -> Result<(), Error> {
        self.order.shareholder = self.message.next_string()?;
        Ok(())
    }

    fn read_imbalance_only(&mut self) -> Result<(), Error> {
        self.order.imbalance_only = self.message.next_bool()?;
        Ok(())
    }

    fn read_route_marketable_to_bbo(&mut self) -> Result<(), Error> {
        self.order.route_marketable_to_bbo = self.message.next_bool()?;
        Ok(())
    }

    fn read_parent_perm_id(&mut self) -> Result<(), Error> {
        self.order.parent_perm_id = self.message.next_optional_long()?;
        Ok(())
    }

    fn read_completed_time(&mut self) -> Result<(), Error> {
        self.order_state.completed_time = self.message.next_string()?;
        Ok(())
    }

    fn read_completed_status(&mut self) -> Result<(), Error> {
        self.order_state.completed_status = self.message.next_string()?;
        Ok(())
    }

    fn into_order_data(self) -> OrderData {
        OrderData {
            order_id: self.order_id,
            contract: self.contract,
            order: self.order,
            order_state: self.order_state,
        }
    }
}

pub(crate) fn decode_open_order(server_version: i32, message: ResponseMessage) -> Result<OrderData, Error> {
    let mut decoder = OrderDecoder::new(server_version, message);

    // read order id
    decoder.read_order_id()?;

    // read contract fields
    decoder.read_contract_fields()?;

    // read order fields
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
    decoder.read_rule_80_a()?;
    decoder.read_percent_offset()?;
    decoder.read_settling_firm()?;
    decoder.read_short_sale_params()?;
    decoder.read_auction_strategy()?;
    decoder.read_box_order_params()?;
    decoder.read_peg_to_stock_or_vol_order_params()?;
    decoder.read_display_size()?;
    decoder.read_block_order()?;
    decoder.read_sweep_to_fill()?;
    decoder.read_all_or_none()?;
    decoder.read_min_qty()?;
    decoder.read_oca_type()?;
    decoder.skip_etrade_only();
    decoder.skip_firm_quote_only();
    decoder.skip_nbbo_price_cap();
    decoder.read_parent_id()?;
    decoder.read_trigger_method()?;
    decoder.read_volatility_order_params(true)?;
    decoder.read_trail_params()?;
    decoder.read_basis_points()?;
    decoder.read_combo_legs()?;
    decoder.read_smart_combo_routing_params()?;
    decoder.read_scale_order_params()?;
    decoder.read_hedge_params()?;
    decoder.read_opt_out_smart_routing()?;
    decoder.read_clearing_params()?;
    decoder.read_not_held()?;
    decoder.read_delta_neutral()?;
    decoder.read_algo_params()?;
    decoder.read_solicited()?;
    decoder.read_what_if_info_and_commission()?;
    decoder.read_vol_randomize_flags()?;
    decoder.read_peg_to_bench_params()?;
    decoder.read_conditions()?;
    decoder.read_adjusted_order_params()?;
    decoder.read_soft_dollar_tier()?;
    decoder.read_cash_qty()?;
    decoder.read_dont_use_auto_price_for_hedge()?;
    decoder.read_is_oms_container()?;
    decoder.read_discretionary_up_to_limit_price()?;
    decoder.read_use_price_mgmt_algo()?;
    decoder.read_duration()?;
    decoder.read_post_to_ats()?;
    decoder.read_auto_cancel_parent()?;
    decoder.read_peg_best_peg_mid_order_attributes()?;

    Ok(decoder.into_order_data())
}

pub(crate) fn decode_order_status(server_version: i32, message: &mut ResponseMessage) -> Result<OrderStatus, Error> {
    message.skip(); // message type

    if server_version < server_versions::MARKET_CAP_PRICE {
        message.skip(); // message version
    };

    let mut order_status = OrderStatus {
        order_id: message.next_int()?,
        status: message.next_string()?,
        filled: message.next_double()?,
        remaining: message.next_double()?,
        average_fill_price: message.next_double()?,
        perm_id: message.next_int()?,
        parent_id: message.next_int()?,
        last_fill_price: message.next_double()?,
        client_id: message.next_int()?,
        why_held: message.next_string()?,
        ..Default::default()
    };

    if server_version >= server_versions::MARKET_CAP_PRICE {
        order_status.market_cap_price = message.next_double()?;
    }

    Ok(order_status)
}

pub(crate) fn decode_execution_data(server_version: i32, message: &mut ResponseMessage) -> Result<ExecutionData, Error> {
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

pub(crate) fn decode_commission_report(_server_version: i32, message: &mut ResponseMessage) -> Result<CommissionReport, Error> {
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

pub(crate) fn decode_completed_order(server_version: i32, message: ResponseMessage) -> Result<OrderData, Error> {
    let mut decoder = OrderDecoder::new(server_version, message);

    // read contract fields
    decoder.read_contract_fields()?;

    // read order fields
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
    decoder.read_perm_id()?;
    decoder.read_outside_rth()?;
    decoder.read_hidden()?;
    decoder.read_discretionary_amt()?;
    decoder.read_good_after_time()?;
    decoder.read_fa_params()?;
    decoder.read_model_code()?;
    decoder.read_good_till_date()?;
    decoder.read_rule_80_a()?;
    decoder.read_percent_offset()?;
    decoder.read_settling_firm()?;
    decoder.read_short_sale_params()?;
    decoder.read_box_order_params()?;
    decoder.read_peg_to_stock_or_vol_order_params()?;
    decoder.read_display_size()?;
    decoder.read_sweep_to_fill()?;
    decoder.read_all_or_none()?;
    decoder.read_min_qty()?;
    decoder.read_oca_type()?;
    decoder.read_trigger_method()?;
    decoder.read_volatility_order_params(false)?;
    decoder.read_trail_params()?;
    decoder.read_combo_legs()?;
    decoder.read_smart_combo_routing_params()?;
    decoder.read_scale_order_params()?;
    decoder.read_hedge_params()?;
    decoder.read_clearing_params()?;
    decoder.read_not_held()?;
    decoder.read_delta_neutral()?;
    decoder.read_algo_params()?;
    decoder.read_solicited()?;
    decoder.read_order_status()?;
    decoder.read_vol_randomize_flags()?;
    decoder.read_peg_to_bench_params()?;
    decoder.read_conditions()?;
    decoder.read_stop_price_and_limit_price_offset()?;
    decoder.read_cash_qty()?;
    decoder.read_dont_use_auto_price_for_hedge()?;
    decoder.read_is_oms_container()?;
    decoder.read_auto_cancel_date()?;
    decoder.read_filled_quantity()?;
    decoder.read_ref_futures_contract_id()?;
    decoder.read_auto_cancel_parent()?;
    decoder.read_shareholder()?;
    decoder.read_imbalance_only()?;
    decoder.read_route_marketable_to_bbo()?;
    decoder.read_parent_perm_id()?;
    decoder.read_completed_time()?;
    decoder.read_completed_status()?;
    decoder.read_peg_best_peg_mid_order_attributes()?;

    Ok(decoder.into_order_data())
}
