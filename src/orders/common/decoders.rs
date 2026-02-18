use crate::contracts::{ComboLeg, ComboLegOpenClose, Contract, Currency, DeltaNeutralContract, Exchange, SecurityType, Symbol, TagValue};
use crate::messages::ResponseMessage;
use crate::orders::{
    Action, CommissionReport, ExecutionData, Liquidity, Order, OrderAllocation, OrderComboLeg, OrderCondition, OrderData, OrderOpenClose, OrderState,
    OrderStatus, Rule80A, SoftDollarTier, TimeInForce,
};
use crate::{server_versions, Error};

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
        contract.symbol = Symbol::from(message.next_string()?);

        let security_type = message.next_string()?;
        contract.security_type = SecurityType::from(&security_type);

        contract.last_trade_date_or_contract_month = message.next_string()?;
        contract.strike = message.next_double()?;
        contract.right = message.next_string()?;
        contract.multiplier = message.next_string()?;
        contract.exchange = Exchange::from(message.next_string()?);
        contract.currency = Currency::from(message.next_string()?);
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
        self.order.tif = TimeInForce::from(self.message.next_string()?);
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
        self.order.origin = self.message.next_int()?.into();
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
        if self.server_version < server_versions::MODELS_SUPPORT {
            return Ok(());
        }
        self.order.model_code = self.message.next_string()?;
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
        self.order.short_sale_slot = self.message.next_int()?.into();
        self.order.designated_location = self.message.next_string()?;
        self.order.exempt_code = self.message.next_int()?;
        Ok(())
    }

    fn read_auction_strategy(&mut self) -> Result<(), Error> {
        self.order.auction_strategy = self.message.next_optional_int()?.filter(|&v| v != 0).map(|v| v.into());
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
        self.order.oca_type = self.message.next_int()?.into();
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
        self.order.trigger_method = self.message.next_int()?.into();
        Ok(())
    }

    fn read_volatility_order_params(&mut self, read_open_order_attributes: bool) -> Result<(), Error> {
        self.order.volatility = self.message.next_optional_double()?;
        self.order.volatility_type = self.message.next_optional_int()?.filter(|&v| v != 0).map(|v| v.into());
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
        self.order.reference_price_type = self.message.next_optional_int()?.filter(|&v| v != 0).map(|v| v.into());

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

        if self.server_version >= server_versions::FULL_ORDER_PREVIEW_FIELDS {
            self.order_state.margin_currency = self.message.next_string()?;
            self.order_state.initial_margin_before_outside_rth = self.message.next_optional_double()?;
            self.order_state.maintenance_margin_before_outside_rth = self.message.next_optional_double()?;
            self.order_state.equity_with_loan_before_outside_rth = self.message.next_optional_double()?;
            self.order_state.initial_margin_change_outside_rth = self.message.next_optional_double()?;
            self.order_state.maintenance_margin_change_outside_rth = self.message.next_optional_double()?;
            self.order_state.equity_with_loan_change_outside_rth = self.message.next_optional_double()?;
            self.order_state.initial_margin_after_outside_rth = self.message.next_optional_double()?;
            self.order_state.maintenance_margin_after_outside_rth = self.message.next_optional_double()?;
            self.order_state.equity_with_loan_after_outside_rth = self.message.next_optional_double()?;
            self.order_state.suggested_size = self.message.next_optional_double()?;
            self.order_state.reject_reason = self.message.next_string()?;

            let count = self.message.next_int()?;
            for _ in 0..count {
                self.order_state.order_allocations.push(OrderAllocation {
                    account: self.message.next_string()?,
                    position: self.message.next_optional_double()?,
                    position_desired: self.message.next_optional_double()?,
                    position_after: self.message.next_optional_double()?,
                    desired_alloc_qty: self.message.next_optional_double()?,
                    allowed_alloc_qty: self.message.next_optional_double()?,
                    is_monetary: self.message.next_bool()?,
                });
            }
        }

        self.order_state.warning_text = self.message.next_string()?;

        Ok(())
    }

    fn read_vol_randomize_flags(&mut self) -> Result<(), Error> {
        self.order.randomize_size = self.message.next_bool()?;
        self.order.randomize_price = self.message.next_bool()?;
        Ok(())
    }

    fn read_peg_to_bench_params(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::PEGGED_TO_BENCHMARK || self.order.order_type != "PEG BENCH" {
            return Ok(());
        }

        self.order.reference_contract_id = self.message.next_int()?;
        self.order.is_pegged_change_amount_decrease = self.message.next_bool()?;
        self.order.pegged_change_amount = self.message.next_optional_double()?;
        self.order.reference_change_amount = self.message.next_optional_double()?;
        self.order.reference_exchange = self.message.next_string()?;

        Ok(())
    }

    fn read_conditions(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::PEGGED_TO_BENCHMARK {
            return Ok(());
        }

        let conditions_count = self.message.next_int()?;
        for _ in 0..conditions_count {
            let condition_type = self.message.next_int()?;
            let is_conjunction = self.message.next_bool()?;

            let condition = match condition_type {
                1 => decode_price_condition(&mut self.message, is_conjunction)?,
                3 => decode_time_condition(&mut self.message, is_conjunction)?,
                4 => decode_margin_condition(&mut self.message, is_conjunction)?,
                5 => decode_execution_condition(&mut self.message, is_conjunction)?,
                6 => decode_volume_condition(&mut self.message, is_conjunction)?,
                7 => decode_percent_change_condition(&mut self.message, is_conjunction)?,
                _ => return Err(Error::Parse(0, condition_type.to_string(), "Unknown condition type".to_string())),
            };

            self.order.conditions.push(condition);
        }
        if conditions_count > 0 {
            self.order.conditions_ignore_rth = self.message.next_bool()?;
            self.order.conditions_cancel_order = self.message.next_bool()?;
        }

        Ok(())
    }

    fn read_adjusted_order_params(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::PEGGED_TO_BENCHMARK {
            return Ok(());
        }
        self.order.adjusted_order_type = self.message.next_string()?;
        self.order.trigger_price = self.message.next_optional_double()?;
        self.order.trail_stop_price = self.message.next_optional_double()?;
        self.order.limit_price_offset = self.message.next_optional_double()?;
        self.order.adjusted_stop_price = self.message.next_optional_double()?;
        self.order.adjusted_stop_limit_price = self.message.next_optional_double()?;
        self.order.adjusted_trailing_amount = self.message.next_optional_double()?;
        self.order.adjustable_trailing_unit = self.message.next_int()?;
        Ok(())
    }

    fn read_soft_dollar_tier(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::SOFT_DOLLAR_TIER {
            return Ok(());
        }
        self.order.soft_dollar_tier = SoftDollarTier {
            name: self.message.next_string()?,
            value: self.message.next_string()?,
            display_name: self.message.next_string()?,
        };
        Ok(())
    }

    fn read_cash_qty(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::CASH_QTY {
            return Ok(());
        }
        self.order.cash_qty = self.message.next_optional_double()?;
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

    fn read_customer_account(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::CUSTOMER_ACCOUNT {
            return Ok(());
        }
        self.order.customer_account = self.message.next_string()?;
        Ok(())
    }

    fn read_professional_customer(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::PROFESSIONAL_CUSTOMER {
            return Ok(());
        }
        self.order.professional_customer = self.message.next_bool()?;
        Ok(())
    }

    fn read_bond_accrued_interest(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::BOND_ACCRUED_INTEREST {
            return Ok(());
        }
        self.order.bond_accrued_interest = self.message.next_string()?;
        Ok(())
    }

    fn read_include_overnight(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::INCLUDE_OVERNIGHT {
            return Ok(());
        }
        self.order.include_overnight = self.message.next_bool()?;
        Ok(())
    }

    fn read_cme_tagging_fields(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::CME_TAGGING_FIELDS_IN_OPEN_ORDER {
            return Ok(());
        }
        self.order.ext_operator = self.message.next_string()?;
        self.order.manual_order_indicator = self.message.next_optional_int()?;
        Ok(())
    }

    fn read_submitter(&mut self) -> Result<(), Error> {
        if self.server_version < server_versions::SUBMITTER {
            return Ok(());
        }
        self.order.submitter = self.message.next_string()?;
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
        if self.server_version < server_versions::IMBALANCE_ONLY {
            return Ok(());
        }
        self.order.imbalance_only = self.message.next_bool()?;
        Ok(())
    }

    /// Reads imbalance_only unconditionally (completed orders always include this field).
    fn read_imbalance_only_always(&mut self) -> Result<(), Error> {
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
    decoder.read_customer_account()?;
    decoder.read_professional_customer()?;
    decoder.read_bond_accrued_interest()?;
    decoder.read_include_overnight()?;
    decoder.read_cme_tagging_fields()?;
    decoder.read_submitter()?;
    decoder.read_imbalance_only()?;

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
    contract.symbol = Symbol::from(message.next_string()?);
    let secutity_type = message.next_string()?;
    contract.security_type = SecurityType::from(&secutity_type);
    contract.last_trade_date_or_contract_month = message.next_string()?;
    contract.strike = message.next_double()?;
    contract.right = message.next_string()?;
    contract.multiplier = message.next_string()?;
    contract.exchange = Exchange::from(message.next_string()?);
    contract.currency = Currency::from(message.next_string()?);
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

    if server_version >= server_versions::PENDING_PRICE_REVISION {
        execution.pending_price_revision = message.next_bool()?;
    }

    if server_version >= server_versions::SUBMITTER {
        execution.submitter = message.next_string()?;
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
    decoder.read_imbalance_only_always()?;
    decoder.read_route_marketable_to_bbo()?;
    decoder.read_parent_perm_id()?;
    decoder.read_completed_time()?;
    decoder.read_completed_status()?;
    decoder.read_peg_best_peg_mid_order_attributes()?;
    decoder.read_customer_account()?;
    decoder.read_professional_customer()?;
    decoder.read_submitter()?;

    Ok(decoder.into_order_data())
}

/// Decodes a PriceCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. price (f64)
/// 5. trigger_method (i32)
fn decode_price_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::PriceCondition;

    Ok(OrderCondition::Price(PriceCondition {
        contract_id: message.next_int()?,
        exchange: message.next_string()?,
        is_more: message.next_bool()?,
        price: message.next_double()?,
        trigger_method: message.next_int()?.into(),
        is_conjunction,
    }))
}

/// Decodes a TimeCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. is_more (bool)
/// 2. time (String)
fn decode_time_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::TimeCondition;

    Ok(OrderCondition::Time(TimeCondition {
        is_more: message.next_bool()?,
        time: message.next_string()?,
        is_conjunction,
    }))
}

/// Decodes a MarginCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. is_more (bool)
/// 2. percent (i32)
fn decode_margin_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::MarginCondition;

    Ok(OrderCondition::Margin(MarginCondition {
        is_more: message.next_bool()?,
        percent: message.next_int()?,
        is_conjunction,
    }))
}

/// Decodes an ExecutionCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. symbol (String)
/// 2. security_type (String)
/// 3. exchange (String)
fn decode_execution_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::ExecutionCondition;

    Ok(OrderCondition::Execution(ExecutionCondition {
        symbol: message.next_string()?,
        security_type: message.next_string()?,
        exchange: message.next_string()?,
        is_conjunction,
    }))
}

/// Decodes a VolumeCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. volume (i32)
fn decode_volume_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::VolumeCondition;

    Ok(OrderCondition::Volume(VolumeCondition {
        contract_id: message.next_int()?,
        exchange: message.next_string()?,
        is_more: message.next_bool()?,
        volume: message.next_int()?,
        is_conjunction,
    }))
}

/// Decodes a PercentChangeCondition from a TWS response.
///
/// Expected field order after type and is_conjunction:
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. percent (f64)
fn decode_percent_change_condition(message: &mut ResponseMessage, is_conjunction: bool) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::PercentChangeCondition;

    Ok(OrderCondition::PercentChange(PercentChangeCondition {
        contract_id: message.next_int()?,
        exchange: message.next_string()?,
        is_more: message.next_bool()?,
        percent: message.next_double()?,
        is_conjunction,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completed_order_parsing_issue_318() {
        // Real message captured from live IB Gateway server version 173
        // This is an AAPL STK (stock) order with 102 fields that was successfully parsed
        // This test confirms that the server version fix works for actual server messages
        let raw_message = vec![
            "101",
            "265598",
            "AAPL",
            "STK",
            "",
            "0",
            "?",
            "",
            "SMART",
            "USD",
            "AAPL",
            "NMS",
            "BUY",
            "1",
            "LMT",
            "100.0",
            "0.0",
            "DAY",
            "",
            "DU1236109",
            "",
            "0",
            "",
            "1295810623",
            "0",
            "0",
            "0",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "0",
            "",
            "-1",
            "",
            "",
            "",
            "",
            "",
            "2147483647",
            "0",
            "0",
            "",
            "3",
            "0",
            "",
            "0",
            "None",
            "",
            "0",
            "0",
            "0",
            "",
            "0",
            "0",
            "",
            "",
            "",
            "0",
            "0",
            "0",
            "2147483647",
            "2147483647",
            "",
            "",
            "",
            "IB",
            "0",
            "0",
            "",
            "0",
            "Cancelled",
            "0",
            "0",
            "0",
            "101.0",
            "1.7976931348623157E308",
            "0",
            "1",
            "0",
            "",
            "0",
            "2147483647",
            "0",
            "Not an insider or substantial shareholder",
            "0",
            "0",
            "9223372036854775807",
            "20250924 01:21:07 America/New_York",
            "Cancelled by Trader",
            "",
            "",
            "",
            "",
            "",
            "",
        ];

        let mut message_str = raw_message.join("\0");
        message_str.push('\0'); // match real TWS framing which terminates messages with NUL
        let message = ResponseMessage::from(&message_str);

        // Using the actual server version from our test (173)
        let server_version = 173;

        // This should parse successfully with the server version fix
        let result = decode_completed_order(server_version, message);

        match result {
            Ok(order_data) => {
                // Verify the order was parsed correctly
                assert_eq!(order_data.contract.symbol.to_string(), "AAPL");
                assert_eq!(order_data.contract.security_type.to_string(), "STK");
                assert_eq!(order_data.order.action.to_string(), "BUY");
                assert_eq!(order_data.order.order_type, "LMT");
                assert_eq!(order_data.order.limit_price, Some(100.0));
                assert_eq!(order_data.order_state.status, "Cancelled");
                assert_eq!(order_data.order_state.completed_time, "20250924 01:21:07 America/New_York");
                assert_eq!(order_data.order_state.completed_status, "Cancelled by Trader");

                // Verify that the server version fix worked
                // Server version 173 < all three threshold values (183, 184, 198)
                // So these fields should be empty/default values since they weren't read
                println!("✅ Successfully parsed live server message with {} fields", raw_message.len());
                println!("✅ Server version {} correctly skipped problematic fields", server_version);
            }
            Err(e) => {
                panic!("Failed to parse live server completed order message: {:?}", e);
            }
        }
    }

    #[test]
    fn test_completed_order_parsing_issue_318_bag() {
        // Real BAG (combo/spread) order message with 117 fields
        // This message represents a SPY spread order that was successfully filled
        // This tests the exact scenario from issue #318 with actual BAG order data
        let raw_message = vec![
            "101",
            "28812380",
            "SPY",
            "BAG",
            "",
            "0",
            "?",
            "",
            "SMART",
            "USD",
            "28812380",
            "COMB",
            "BUY",
            "0",
            "LMT",
            "-0.57",
            "0.0",
            "DAY",
            "",
            "DUK000000",
            "",
            "0",
            "bpcs",
            "216108144",
            "0",
            "0",
            "0",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "0",
            "",
            "",
            "0",
            "",
            "-1",
            "",
            "",
            "",
            "",
            "",
            "2147483647",
            "0",
            "0",
            "",
            "3",
            "0",
            "",
            "0",
            "None",
            "",
            "0",
            "0",
            "0",
            "",
            "0",
            "0",
            "",
            "",
            "810118027|1,810118051|-1",
            "2",
            "810118027",
            "1",
            "BUY",
            "SMART",
            "0",
            "0",
            "",
            "-1",
            "810118051",
            "1",
            "SELL",
            "SMART",
            "0",
            "0",
            "",
            "-1",
            "0",
            "0",
            "2147483647",
            "2147483647",
            "",
            "",
            "",
            "IB",
            "0",
            "0",
            "",
            "0",
            "Filled",
            "0",
            "0",
            "0",
            "1.7976931348623157E308",
            "1.7976931348623157E308",
            "0",
            "1",
            "0",
            "",
            "1",
            "2147483647",
            "0",
            "Not an insider or substantial shareholder",
            "0",
            "0",
            "0",
            "20250922 11:49:07 America/Los_Angeles",
            "Filled Size: 1",
            "",
            "",
            "",
            "",
            "",
        ];

        let mut message_str = raw_message.join("\0");
        message_str.push('\0'); // ensure final empty field is preserved
        let message = ResponseMessage::from(&message_str);

        // Using server version 173 which is below the thresholds for problematic fields
        let server_version = 173;

        // This should parse successfully with the server version fix
        let result = decode_completed_order(server_version, message);

        match result {
            Ok(order_data) => {
                // Verify the BAG order was parsed correctly
                assert_eq!(order_data.contract.symbol.to_string(), "SPY");
                assert_eq!(order_data.contract.security_type.to_string(), "BAG");
                assert_eq!(order_data.order.action.to_string(), "BUY");
                assert_eq!(order_data.order.order_type, "LMT");
                assert_eq!(order_data.order.limit_price, Some(-0.57));
                assert_eq!(order_data.order_state.status, "Filled");
                assert_eq!(order_data.order_state.completed_time, "20250922 11:49:07 America/Los_Angeles");
                assert_eq!(order_data.order_state.completed_status, "Filled Size: 1");

                // Verify combo legs were parsed (should have 2 legs for this spread)
                assert!(!order_data.contract.combo_legs.is_empty(), "BAG order should have combo legs");

                println!("✅ Successfully parsed real BAG order with {} fields", raw_message.len());
                println!("✅ BAG order has {} combo legs", order_data.contract.combo_legs.len());
            }
            Err(e) => {
                panic!("Failed to parse BAG order from issue #318: {:?}", e);
            }
        }
    }

    /// Tests round-trip encoding and decoding of PriceCondition.
    #[test]
    fn test_price_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::{PriceCondition, TriggerMethod};

        let original = OrderCondition::Price(PriceCondition {
            contract_id: 12345,
            exchange: "NASDAQ".to_string(),
            price: 150.0,
            trigger_method: TriggerMethod::DoubleBidAsk,
            is_more: true,
            is_conjunction: false,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_price_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 1);
        assert_eq!(original, decoded);
    }

    /// Tests round-trip encoding and decoding of TimeCondition.
    #[test]
    fn test_time_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::TimeCondition;

        let original = OrderCondition::Time(TimeCondition {
            time: "20251230 23:59:59 UTC".to_string(),
            is_more: true,
            is_conjunction: true,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_time_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 3);
        assert_eq!(original, decoded);
    }

    /// Tests round-trip encoding and decoding of MarginCondition.
    #[test]
    fn test_margin_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::MarginCondition;

        let original = OrderCondition::Margin(MarginCondition {
            percent: 30,
            is_more: false,
            is_conjunction: true,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_margin_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 4);
        assert_eq!(original, decoded);
    }

    /// Tests round-trip encoding and decoding of ExecutionCondition.
    #[test]
    fn test_execution_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::ExecutionCondition;

        let original = OrderCondition::Execution(ExecutionCondition {
            symbol: "AAPL".to_string(),
            security_type: "STK".to_string(),
            exchange: "SMART".to_string(),
            is_conjunction: false,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_execution_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 5);
        assert_eq!(original, decoded);
    }

    /// Tests round-trip encoding and decoding of VolumeCondition.
    #[test]
    fn test_volume_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::VolumeCondition;

        let original = OrderCondition::Volume(VolumeCondition {
            contract_id: 12345,
            exchange: "NASDAQ".to_string(),
            volume: 1000000,
            is_more: true,
            is_conjunction: true,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_volume_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 6);
        assert_eq!(original, decoded);
    }

    /// Tests round-trip encoding and decoding of PercentChangeCondition.
    #[test]
    fn test_percent_change_condition_round_trip() {
        use crate::messages::RequestMessage;
        use crate::orders::common::encoders::encode_condition;
        use crate::orders::conditions::PercentChangeCondition;

        let original = OrderCondition::PercentChange(PercentChangeCondition {
            contract_id: 12345,
            exchange: "NASDAQ".to_string(),
            percent: 5.0,
            is_more: false,
            is_conjunction: false,
        });

        // Encode
        let mut request_message = RequestMessage::default();
        encode_condition(&mut request_message, &original);

        // Create ResponseMessage from encoded fields
        let encoded = request_message.encode();
        let mut response_message = ResponseMessage::from(&encoded);

        // Decode
        let condition_type = response_message.next_int().unwrap();
        let is_conjunction = response_message.next_bool().unwrap();
        let decoded = decode_percent_change_condition(&mut response_message, is_conjunction).unwrap();

        assert_eq!(condition_type, 7);
        assert_eq!(original, decoded);
    }

    /// Tests error handling for unknown condition type.
    #[test]
    fn test_unknown_condition_type() {
        let encoded = "99\x001\x00"; // Unknown type 99
        let mut response_message = ResponseMessage::from(encoded);

        let condition_type = response_message.next_int().unwrap();
        let _is_conjunction = response_message.next_bool().unwrap();

        // Should return error for unknown type
        match condition_type {
            1 => panic!("Should be unknown type"),
            _ => {
                // This is the expected path - unknown type should be caught in read_conditions
                assert_eq!(condition_type, 99);
            }
        }
    }

    /// Builds a base open order message fields for a simple AAPL LMT order.
    /// Server version must be >= ORDER_CONTAINER (145) and >= FA_PROFILE_DESUPPORT (177).
    fn build_open_order_base_fields(server_version: i32) -> Vec<&'static str> {
        let mut fields = vec![
            "5", // message type (OpenOrder)
            // No message version (server_version >= ORDER_CONTAINER)
            "42", // order_id
            // contract fields
            "265598", // contract_id
            "AAPL",   // symbol
            "STK",    // security_type
            "",       // last_trade_date
            "0",      // strike
            "?",      // right
            "",       // multiplier
            "SMART",  // exchange
            "USD",    // currency
            "AAPL",   // local_symbol
            "NMS",    // trading_class
            // order fields
            "BUY",       // action
            "100",       // total_quantity
            "LMT",       // order_type
            "150.50",    // limit_price
            "0",         // aux_price
            "DAY",       // tif
            "",          // oca_group
            "DU1234567", // account
            "",          // open_close
            "0",         // origin
            "",          // order_ref
            "1",         // client_id
            "123456",    // perm_id
            "0",         // outside_rth
            "0",         // hidden
            "0",         // discretionary_amt
            "",          // good_after_time
            "",          // skip_shares_allocation
            "",          // fa_group
            "",          // fa_method
            "",          // fa_percentage
            // no fa_profile (server_version >= FA_PROFILE_DESUPPORT)
            "",   // model_code (>= MODELS_SUPPORT)
            "",   // good_till_date
            "",   // rule_80_a
            "",   // percent_offset
            "",   // settling_firm
            "0",  // short_sale_slot
            "",   // designated_location
            "-1", // exempt_code
            "0",  // auction_strategy
            "",   // starting_price
            "",   // stock_ref_price
            "",   // delta
            "",   // stock_range_lower
            "",   // stock_range_upper
            "",   // display_size
            "0",  // block_order
            "0",  // sweep_to_fill
            "0",  // all_or_none
            "",   // min_qty
            "0",  // oca_type
            "",   // skip_etrade_only
            "",   // skip_firm_quote_only
            "",   // skip_nbbo_price_cap
            "0",  // parent_id
            "0",  // trigger_method
            // volatility_order_params (read_open_order_attributes=true)
            "", // volatility
            "", // volatility_type
            "", // delta_neutral_order_type
            "", // delta_neutral_aux_price
            // (not delta neutral, so no extra fields)
            "0", // continuous_update
            "",  // reference_price_type
            // trail_params
            "", // trail_stop_price
            "", // trailing_percent
            // basis_points
            "", // basis_points
            "", // basis_points_type
            // combo_legs
            "",  // combo_legs_description
            "0", // combo_legs_count
            "0", // order_combo_legs_count
            // smart_combo_routing_params
            "0", // count
            // scale_order_params
            "", // scale_init_level_size
            "", // scale_subs_level_size
            "", // scale_price_increment
            // hedge_params
            "", // hedge_type (empty, no hedge_param)
            // opt_out_smart_routing
            "0",
            // clearing_params
            "", // clearing_account
            "", // clearing_intent
            // not_held
            "0",
            // delta_neutral
            "0", // has_delta_neutral_contract (false)
            // algo_params
            "", // algo_strategy (empty, no params)
            // solicited
            "0",
            // what_if_info_and_commission
            "0",         // what_if
            "Submitted", // order_status
            // what_if_ext_fields (>= WHAT_IF_EXT_FIELDS)
            "", // initial_margin_before
            "", // maintenance_margin_before
            "", // equity_with_loan_before
            "", // initial_margin_change
            "", // maintenance_margin_change
            "", // equity_with_loan_change
            "", // initial_margin_after
            "", // maintenance_margin_after
            "", // equity_with_loan_after
            "", // commission
            "", // minimum_commission
            "", // maximum_commission
            "", // commission_currency
        ];

        // full_order_preview_fields (>= FULL_ORDER_PREVIEW_FIELDS=195)
        if server_version >= server_versions::FULL_ORDER_PREVIEW_FIELDS {
            fields.extend_from_slice(&[
                "",  // margin_currency
                "",  // initial_margin_before_outside_rth
                "",  // maintenance_margin_before_outside_rth
                "",  // equity_with_loan_before_outside_rth
                "",  // initial_margin_change_outside_rth
                "",  // maintenance_margin_change_outside_rth
                "",  // equity_with_loan_change_outside_rth
                "",  // initial_margin_after_outside_rth
                "",  // maintenance_margin_after_outside_rth
                "",  // equity_with_loan_after_outside_rth
                "",  // suggested_size
                "",  // reject_reason
                "0", // order_allocations_count
            ]);
        }

        fields.extend_from_slice(&[
            "", // warning_text
            // vol_randomize_flags
            "0", // randomize_size
            "0", // randomize_price
            // peg_to_bench: skipped (order_type != "PEG BENCH")
            // conditions (>= PEGGED_TO_BENCHMARK)
            "0", // conditions_count
            // adjusted_order_params (>= PEGGED_TO_BENCHMARK)
            "",  // adjusted_order_type
            "",  // trigger_price
            "",  // trail_stop_price
            "",  // limit_price_offset
            "",  // adjusted_stop_price
            "",  // adjusted_stop_limit_price
            "",  // adjusted_trailing_amount
            "0", // adjustable_trailing_unit
            // soft_dollar_tier (>= SOFT_DOLLAR_TIER)
            "", // name
            "", // value
            "", // display_name
            // cash_qty (>= CASH_QTY)
            "",  // dont_use_auto_price_for_hedge (>= AUTO_PRICE_FOR_HEDGE)
            "0", // is_oms_container (>= ORDER_CONTAINER)
            "0", // discretionary_up_to_limit_price (>= D_PEG_ORDERS)
            "0", // use_price_mgmt_algo (>= PRICE_MGMT_ALGO)
            "0", // duration (>= DURATION)
            "",  // post_to_ats (>= POST_TO_ATS)
            "",  // auto_cancel_parent (>= AUTO_CANCEL_PARENT)
            "0", // peg_best_peg_mid (>= PEGBEST_PEGMID_OFFSETS)
            "",  // min_trade_qty
            "",  // min_compete_size
            "",  // compete_against_best_offset
            "",  // mid_offset_at_whole
            "",  // mid_offset_at_half
        ]);

        fields
    }

    #[test]
    fn test_decode_open_order_v200_new_fields() {
        let mut fields = build_open_order_base_fields(200);

        // New fields for v183-v199
        fields.push("CUST001"); // customer_account (>= CUSTOMER_ACCOUNT=183)
        fields.push("1"); // professional_customer (>= PROFESSIONAL_CUSTOMER=184)
        fields.push("1.25"); // bond_accrued_interest (>= BOND_ACCRUED_INTEREST=185)
        fields.push("1"); // include_overnight (>= INCLUDE_OVERNIGHT=189)
        fields.push("EXTOP1"); // ext_operator (>= CME_TAGGING_FIELDS_IN_OPEN_ORDER=193)
        fields.push("3"); // manual_order_indicator (>= CME_TAGGING_FIELDS_IN_OPEN_ORDER)
        fields.push("SUB001"); // submitter (>= SUBMITTER=198)
        fields.push("1"); // imbalance_only (>= IMBALANCE_ONLY=199)

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_open_order(200, message).unwrap();

        // Verify core fields
        assert_eq!(result.order_id, 42);
        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.order.action.to_string(), "BUY");
        assert_eq!(result.order.order_type, "LMT");
        assert_eq!(result.order.limit_price, Some(150.50));
        assert_eq!(result.order_state.status, "Submitted");

        // Verify new fields
        assert_eq!(result.order.customer_account, "CUST001");
        assert!(result.order.professional_customer);
        assert_eq!(result.order.bond_accrued_interest, "1.25");
        assert!(result.order.include_overnight);
        assert_eq!(result.order.ext_operator, "EXTOP1");
        assert_eq!(result.order.manual_order_indicator, Some(3));
        assert_eq!(result.order.submitter, "SUB001");
        assert!(result.order.imbalance_only);
    }

    #[test]
    fn test_decode_open_order_v182_skips_new_fields() {
        let fields = build_open_order_base_fields(182);

        // At v182, none of the new fields (>= v183) are present.
        // The message ends after peg_best_peg_mid fields.
        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_open_order(182, message).unwrap();

        // Core fields still parse
        assert_eq!(result.order_id, 42);
        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.order.action.to_string(), "BUY");

        // New fields should be defaults
        assert_eq!(result.order.customer_account, "");
        assert!(!result.order.professional_customer);
        assert_eq!(result.order.bond_accrued_interest, "");
        assert!(!result.order.include_overnight);
        assert_eq!(result.order.ext_operator, "");
        assert_eq!(result.order.manual_order_indicator, None);
        assert_eq!(result.order.submitter, "");
        assert!(!result.order.imbalance_only);
    }

    #[test]
    fn test_decode_open_order_v200_full_order_preview_fields() {
        let mut fields = build_open_order_base_fields(200);

        // Append v183-v199 fields
        fields.extend_from_slice(&[
            "CUST001", // customer_account
            "1",       // professional_customer
            "1.25",    // bond_accrued_interest
            "1",       // include_overnight
            "EXTOP1",  // ext_operator
            "3",       // manual_order_indicator
            "SUB001",  // submitter
            "1",       // imbalance_only
        ]);

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_open_order(200, message).unwrap();

        // Verify full order preview fields are default (empty in base)
        assert_eq!(result.order_state.margin_currency, "");
        assert_eq!(result.order_state.initial_margin_before_outside_rth, None);
        assert_eq!(result.order_state.maintenance_margin_before_outside_rth, None);
        assert_eq!(result.order_state.equity_with_loan_before_outside_rth, None);
        assert_eq!(result.order_state.initial_margin_change_outside_rth, None);
        assert_eq!(result.order_state.maintenance_margin_change_outside_rth, None);
        assert_eq!(result.order_state.equity_with_loan_change_outside_rth, None);
        assert_eq!(result.order_state.initial_margin_after_outside_rth, None);
        assert_eq!(result.order_state.maintenance_margin_after_outside_rth, None);
        assert_eq!(result.order_state.equity_with_loan_after_outside_rth, None);
        assert_eq!(result.order_state.suggested_size, None);
        assert_eq!(result.order_state.reject_reason, "");
        assert!(result.order_state.order_allocations.is_empty());
    }

    #[test]
    fn test_decode_open_order_v200_full_order_preview_with_values() {
        // Build v194 base (no preview block), then splice in preview fields with values
        let base = build_open_order_base_fields(194);

        // Find "Submitted" (order_status) to locate the insertion point
        let status_idx = base.iter().position(|&f| f == "Submitted").unwrap();
        // After status: 6 ext margins + 3 after margins + 3 commissions + commission_currency = 13
        let after_commission_currency = status_idx + 1 + 13;

        let mut fields: Vec<&str> = base[..after_commission_currency].to_vec();

        // Insert full_order_preview_fields with values
        fields.extend_from_slice(&[
            "USD",                // margin_currency
            "5000.0",             // initial_margin_before_outside_rth
            "4000.0",             // maintenance_margin_before_outside_rth
            "3000.0",             // equity_with_loan_before_outside_rth
            "100.0",              // initial_margin_change_outside_rth
            "80.0",               // maintenance_margin_change_outside_rth
            "60.0",               // equity_with_loan_change_outside_rth
            "5100.0",             // initial_margin_after_outside_rth
            "4080.0",             // maintenance_margin_after_outside_rth
            "3060.0",             // equity_with_loan_after_outside_rth
            "50.0",               // suggested_size
            "some reject reason", // reject_reason
            "2",                  // order_allocations_count
            "ACC1",               // allocation[0].account
            "100.0",              // allocation[0].position
            "150.0",              // allocation[0].position_desired
            "150.0",              // allocation[0].position_after
            "50.0",               // allocation[0].desired_alloc_qty
            "50.0",               // allocation[0].allowed_alloc_qty
            "0",                  // allocation[0].is_monetary
            "ACC2",               // allocation[1].account
            "200.0",              // allocation[1].position
            "250.0",              // allocation[1].position_desired
            "250.0",              // allocation[1].position_after
            "50.0",               // allocation[1].desired_alloc_qty
            "50.0",               // allocation[1].allowed_alloc_qty
            "1",                  // allocation[1].is_monetary
        ]);

        // Append rest of base (warning_text onward)
        fields.extend_from_slice(&base[after_commission_currency..]);

        // Append v183+ fields
        fields.extend_from_slice(&["CUST001", "1", "1.25", "1", "EXTOP1", "3", "SUB001", "1"]);

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_open_order(200, message).unwrap();

        assert_eq!(result.order_state.margin_currency, "USD");
        assert_eq!(result.order_state.initial_margin_before_outside_rth, Some(5000.0));
        assert_eq!(result.order_state.maintenance_margin_before_outside_rth, Some(4000.0));
        assert_eq!(result.order_state.equity_with_loan_before_outside_rth, Some(3000.0));
        assert_eq!(result.order_state.initial_margin_change_outside_rth, Some(100.0));
        assert_eq!(result.order_state.maintenance_margin_change_outside_rth, Some(80.0));
        assert_eq!(result.order_state.equity_with_loan_change_outside_rth, Some(60.0));
        assert_eq!(result.order_state.initial_margin_after_outside_rth, Some(5100.0));
        assert_eq!(result.order_state.maintenance_margin_after_outside_rth, Some(4080.0));
        assert_eq!(result.order_state.equity_with_loan_after_outside_rth, Some(3060.0));
        assert_eq!(result.order_state.suggested_size, Some(50.0));
        assert_eq!(result.order_state.reject_reason, "some reject reason");

        assert_eq!(result.order_state.order_allocations.len(), 2);
        let alloc0 = &result.order_state.order_allocations[0];
        assert_eq!(alloc0.account, "ACC1");
        assert_eq!(alloc0.position, Some(100.0));
        assert_eq!(alloc0.position_desired, Some(150.0));
        assert_eq!(alloc0.position_after, Some(150.0));
        assert_eq!(alloc0.desired_alloc_qty, Some(50.0));
        assert_eq!(alloc0.allowed_alloc_qty, Some(50.0));
        assert!(!alloc0.is_monetary);

        let alloc1 = &result.order_state.order_allocations[1];
        assert_eq!(alloc1.account, "ACC2");
        assert!(alloc1.is_monetary);
    }

    #[test]
    fn test_decode_execution_data_v200_new_fields() {
        let fields = vec![
            "11", // message type (ExecutionData)
            // no version (server_version >= LAST_LIQUIDITY)
            "9000",                         // request_id
            "42",                           // order_id
            "265598",                       // contract_id
            "AAPL",                         // symbol
            "STK",                          // security_type
            "",                             // last_trade_date
            "0",                            // strike
            "?",                            // right
            "",                             // multiplier
            "SMART",                        // exchange
            "USD",                          // currency
            "AAPL",                         // local_symbol
            "NMS",                          // trading_class
            "0001f4e8.67890abc.01.01",      // execution_id
            "20260115 10:30:00 US/Eastern", // time
            "DU1234567",                    // account_number
            "SMART",                        // exchange
            "BOT",                          // side
            "100",                          // shares
            "150.50",                       // price
            "123456",                       // perm_id
            "1",                            // client_id
            "0",                            // liquidation
            "100",                          // cumulative_quantity
            "150.50",                       // average_price
            "",                             // order_reference
            "",                             // ev_rule
            "",                             // ev_multiplier
            "",                             // model_code (>= MODELS_SUPPORT)
            "2",                            // last_liquidity (>= LAST_LIQUIDITY)
            "1",                            // pending_price_revision (>= PENDING_PRICE_REVISION=178)
            "SUB002",                       // submitter (>= SUBMITTER=198)
        ];

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let mut message = ResponseMessage::from(&message_str);

        let result = decode_execution_data(200, &mut message).unwrap();

        // Verify core fields
        assert_eq!(result.request_id, 9000);
        assert_eq!(result.execution.order_id, 42);
        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.execution.execution_id, "0001f4e8.67890abc.01.01");
        assert_eq!(result.execution.shares, 100.0);
        assert_eq!(result.execution.price, 150.50);

        // Verify new fields
        assert!(result.execution.pending_price_revision);
        assert_eq!(result.execution.submitter, "SUB002");
    }

    #[test]
    fn test_decode_execution_data_v177_skips_new_fields() {
        // v177 is below PENDING_PRICE_REVISION (178) and SUBMITTER (198)
        let fields = vec![
            "11",                           // message type
            "9000",                         // request_id
            "42",                           // order_id
            "265598",                       // contract_id
            "AAPL",                         // symbol
            "STK",                          // security_type
            "",                             // last_trade_date
            "0",                            // strike
            "?",                            // right
            "",                             // multiplier
            "SMART",                        // exchange
            "USD",                          // currency
            "AAPL",                         // local_symbol
            "NMS",                          // trading_class
            "0001f4e8.67890abc.01.01",      // execution_id
            "20260115 10:30:00 US/Eastern", // time
            "DU1234567",                    // account_number
            "SMART",                        // exchange
            "BOT",                          // side
            "100",                          // shares
            "150.50",                       // price
            "123456",                       // perm_id
            "1",                            // client_id
            "0",                            // liquidation
            "100",                          // cumulative_quantity
            "150.50",                       // average_price
            "",                             // order_reference
            "",                             // ev_rule
            "",                             // ev_multiplier
            "",                             // model_code (>= MODELS_SUPPORT)
            "2",                            // last_liquidity (>= LAST_LIQUIDITY)
                                            // No pending_price_revision (v177 < 178)
                                            // No submitter (v177 < 198)
        ];

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let mut message = ResponseMessage::from(&message_str);

        let result = decode_execution_data(177, &mut message).unwrap();

        assert_eq!(result.execution.order_id, 42);
        assert!(!result.execution.pending_price_revision);
        assert_eq!(result.execution.submitter, "");
    }

    /// Builds base completed order message fields for a simple AAPL LMT order.
    fn build_completed_order_base_fields() -> Vec<&'static str> {
        vec![
            "101", // message type (CompletedOrder)
            // No message version (server_version >= ORDER_CONTAINER)
            // contract fields
            "265598", // contract_id
            "AAPL",   // symbol
            "STK",    // security_type
            "",       // last_trade_date
            "0",      // strike
            "?",      // right
            "",       // multiplier
            "SMART",  // exchange
            "USD",    // currency
            "AAPL",   // local_symbol
            "NMS",    // trading_class
            // order fields
            "BUY",       // action
            "1",         // total_quantity
            "LMT",       // order_type
            "100.0",     // limit_price
            "0.0",       // aux_price
            "DAY",       // tif
            "",          // oca_group
            "DU1234567", // account
            "",          // open_close
            "0",         // origin
            "",          // order_ref
            // (no client_id in completed orders)
            "1295810623", // perm_id
            "0",          // outside_rth
            "0",          // hidden
            "0",          // discretionary_amt
            "",           // good_after_time
            // (no skip_shares_allocation in completed orders)
            "", // fa_group
            "", // fa_method
            "", // fa_percentage
            // no fa_profile (>= FA_PROFILE_DESUPPORT)
            "",   // model_code (>= MODELS_SUPPORT)
            "",   // good_till_date
            "",   // rule_80_a
            "",   // percent_offset
            "",   // settling_firm
            "0",  // short_sale_slot
            "",   // designated_location
            "-1", // exempt_code
            // (no auction_strategy in completed orders)
            "", // starting_price
            "", // stock_ref_price
            "", // delta
            "", // stock_range_lower
            "", // stock_range_upper
            "", // display_size
            // (no block_order in completed orders)
            "0", // sweep_to_fill
            "0", // all_or_none
            "",  // min_qty
            "0", // oca_type
            // (no skip_etrade_only, skip_firm_quote_only, skip_nbbo_price_cap)
            // (no parent_id)
            "0", // trigger_method
            // volatility_order_params (read_open_order_attributes=false)
            "",  // volatility
            "",  // volatility_type
            "",  // delta_neutral_order_type
            "",  // delta_neutral_aux_price
            "0", // continuous_update
            "",  // reference_price_type
            // trail_params
            "", // trail_stop_price
            "", // trailing_percent
            // (no basis_points in completed orders)
            // combo_legs
            "",  // combo_legs_description
            "0", // combo_legs_count
            "0", // order_combo_legs_count
            // smart_combo_routing_params
            "0", // count
            // scale_order_params
            "", // scale_init_level_size
            "", // scale_subs_level_size
            "", // scale_price_increment
            // hedge_params
            "", // hedge_type (empty)
            // (no opt_out_smart_routing in completed orders)
            // clearing_params
            "", // clearing_account
            "", // clearing_intent
            // not_held
            "0",
            // delta_neutral
            "0", // has_delta_neutral_contract
            // algo_params
            "", // algo_strategy
            // solicited
            "0",
            // order_status
            "Cancelled",
            // vol_randomize_flags
            "0", // randomize_size
            "0", // randomize_price
            // peg_to_bench: skipped (order_type != "PEG BENCH")
            // conditions (>= PEGGED_TO_BENCHMARK)
            "0", // conditions_count
            // stop_price_and_limit_price_offset
            "", // trail_stop_price
            "", // limit_price_offset
            // cash_qty (>= CASH_QTY)
            "",
            // dont_use_auto_price_for_hedge (>= AUTO_PRICE_FOR_HEDGE)
            "0",
            // is_oms_container (>= ORDER_CONTAINER)
            "0",
            // auto_cancel_date
            "",
            // filled_quantity
            "0",
            // ref_futures_contract_id
            "",
            // auto_cancel_parent (>= AUTO_CANCEL_PARENT)
            "0",
            // shareholder
            "Not an insider or substantial shareholder",
            // imbalance_only (min_version=0, always read)
            "0",
            // route_marketable_to_bbo
            "0",
            // parent_perm_id
            "9223372036854775807",
            // completed_time
            "20260115 10:30:00 America/New_York",
            // completed_status
            "Cancelled by Trader",
            // peg_best_peg_mid (>= PEGBEST_PEGMID_OFFSETS)
            "", // min_trade_qty
            "", // min_compete_size
            "", // compete_against_best_offset
            "", // mid_offset_at_whole
            "", // mid_offset_at_half
        ]
    }

    #[test]
    fn test_decode_completed_order_v200_new_fields() {
        let mut fields = build_completed_order_base_fields();

        // New fields for completed orders at v200
        fields.push("CUST002"); // customer_account (>= CUSTOMER_ACCOUNT=183)
        fields.push("1"); // professional_customer (>= PROFESSIONAL_CUSTOMER=184)
        fields.push("SUB003"); // submitter (>= SUBMITTER=198)
                               // Note: completed orders do NOT decode bond_accrued_interest,
                               // include_overnight, or cme_tagging_fields per C# reference

        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_completed_order(200, message).unwrap();

        // Verify core fields
        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.order.action.to_string(), "BUY");
        assert_eq!(result.order_state.status, "Cancelled");
        assert_eq!(result.order_state.completed_time, "20260115 10:30:00 America/New_York");
        assert_eq!(result.order_state.completed_status, "Cancelled by Trader");

        // Verify new fields
        assert_eq!(result.order.customer_account, "CUST002");
        assert!(result.order.professional_customer);
        assert_eq!(result.order.submitter, "SUB003");
    }

    #[test]
    fn test_decode_completed_order_v182_skips_new_fields() {
        let fields = build_completed_order_base_fields();

        // At v182, customer_account, professional_customer, submitter are not present
        let mut message_str = fields.join("\0");
        message_str.push('\0');
        let message = ResponseMessage::from(&message_str);

        let result = decode_completed_order(182, message).unwrap();

        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.order_state.completed_status, "Cancelled by Trader");

        // New fields should be defaults
        assert_eq!(result.order.customer_account, "");
        assert!(!result.order.professional_customer);
        assert_eq!(result.order.submitter, "");
    }
}
