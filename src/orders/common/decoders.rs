use crate::messages::ResponseMessage;
use crate::orders::conditions::TriggerMethod;
use crate::orders::{
    Action, Execution, ExecutionSide, Liquidity, OcaType, Order, OrderAllocation, OrderCondition, OrderOpenClose, OrderOrigin, OrderState,
    ReferencePriceType, Rule80A, ShortSaleSlot, TimeInForce, VolatilityType, WhatIfType,
};
use crate::orders::{CommissionReport, ExecutionData, OrderBound, OrderData, OrderStatus};
use crate::proto;
use crate::proto::decoders::{
    decode_soft_dollar_tier, optional_f64, parse_decimal_or_zero, parse_optional, parse_optional_decimal, parse_required, required, s, tag_values,
};
use crate::Error;

// All originating outgoing-request gates for OpenOrder, CompletedOrder,
// OrderStatus, ExecutionData, and CommissionReport are <= the connection floor.
// The server always emits proto framing for these messages; text-framed arrival
// is rejected via `ResponseMessage::require_proto`, which raises
// `Error::UnexpectedWireFormat` (docs/rules/wire/proto-only-decoding.md).

pub(crate) fn decode_open_order(message: &ResponseMessage) -> Result<OrderData, Error> {
    decode_open_order_proto(message.proto()?)
}

pub(crate) fn decode_order_status(message: &ResponseMessage) -> Result<OrderStatus, Error> {
    decode_order_status_proto(message.proto()?)
}

pub(crate) fn decode_order_bound(message: &ResponseMessage) -> Result<OrderBound, Error> {
    let p: crate::proto::OrderBound = message.proto()?;
    Ok(OrderBound {
        perm_id: required(p.perm_id, "perm_id", "OrderBound")?,
        client_id: required(p.client_id, "client_id", "OrderBound")?,
        order_id: required(p.order_id, "order_id", "OrderBound")?,
    })
}

pub(crate) fn decode_execution_data(message: &ResponseMessage) -> Result<ExecutionData, Error> {
    decode_execution_data_proto(message.proto()?)
}

pub(crate) fn decode_commission_report(message: &ResponseMessage) -> Result<CommissionReport, Error> {
    decode_commission_report_proto(message.proto()?)
}

pub(crate) fn decode_completed_order(message: &ResponseMessage) -> Result<OrderData, Error> {
    decode_completed_order_proto(message.proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_open_order_proto(p: crate::proto::OpenOrder) -> Result<OrderData, Error> {
    Ok(OrderData {
        order_id: p.order_id.unwrap_or_default(),
        contract: crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "OpenOrder")?)?,
        order: decode_order(required(p.order.as_ref(), "order", "OpenOrder")?)?,
        order_state: decode_order_state(required(p.order_state.as_ref(), "order_state", "OpenOrder")?)?,
    })
}

pub(crate) fn decode_order_status_proto(p: crate::proto::OrderStatus) -> Result<OrderStatus, Error> {
    Ok(OrderStatus {
        order_id: p.order_id.unwrap_or_default(),
        status: crate::proto::decoders::parse_required(p.status.as_deref(), "OrderStatus")?,
        filled: crate::proto::decoders::parse_decimal_or_zero(p.filled.as_deref())?,
        remaining: crate::proto::decoders::parse_decimal_or_zero(p.remaining.as_deref())?,
        average_fill_price: p.avg_fill_price,
        perm_id: p.perm_id.unwrap_or_default(),
        parent_id: p.parent_id.unwrap_or_default(),
        last_fill_price: p.last_fill_price,
        client_id: p.client_id.unwrap_or_default(),
        why_held: p.why_held.unwrap_or_default(),
        market_cap_price: p.mkt_cap_price,
    })
}

pub(crate) fn decode_execution_data_proto(p: crate::proto::ExecutionDetails) -> Result<ExecutionData, Error> {
    Ok(ExecutionData {
        request_id: p.req_id.unwrap_or_default(),
        contract: crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "ExecutionDetails")?)?,
        execution: decode_execution(required(p.execution.as_ref(), "execution", "ExecutionDetails")?)?,
    })
}

pub(crate) fn decode_completed_order_proto(p: crate::proto::CompletedOrder) -> Result<OrderData, Error> {
    let contract = crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "CompletedOrder")?)?;
    let order = decode_order(required(p.order.as_ref(), "order", "CompletedOrder")?)?;
    let order_state = decode_order_state(required(p.order_state.as_ref(), "order_state", "CompletedOrder")?)?;

    Ok(OrderData {
        // Completed orders carry no live order_id; preserve the legacy text-decoder
        // sentinel so v2 → v3 consumers see the same value.
        order_id: -1,
        contract,
        order,
        order_state,
    })
}

pub(crate) fn decode_commission_report_proto(p: crate::proto::CommissionAndFeesReport) -> Result<CommissionReport, Error> {
    Ok(CommissionReport {
        execution_id: p.exec_id.unwrap_or_default(),
        commission: p.commission_and_fees.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
        realized_pnl: crate::proto::decoders::optional_f64(p.realized_pnl),
        yields: crate::proto::decoders::optional_f64(p.bond_yield),
        yield_redemption_date: p.yield_redemption_date.unwrap_or_default(),
    })
}

pub(crate) fn decode_next_valid_id_proto(p: crate::proto::NextValidId) -> Result<i32, Error> {
    Ok(p.order_id.unwrap_or_default())
}

// === Proto → domain converters ===

// `Order` has ~200 fields; one assignment per line keeps each wire field's conversion
// on its own line.
#[allow(clippy::field_reassign_with_default)]
pub(crate) fn decode_order(proto: &proto::Order) -> Result<Order, Error> {
    let mut order = Order::default();

    order.client_id = proto.client_id.unwrap_or_default();
    order.order_id = proto.order_id.unwrap_or_default();
    order.perm_id = proto.perm_id.unwrap_or_default();
    order.parent_id = proto.parent_id.unwrap_or_default();

    order.action = parse_required::<Action>(proto.action.as_deref(), "action")?;
    // Absent means 0 upstream (Order.cs leaves TotalQuantity at decimal's default).
    order.total_quantity = parse_decimal_or_zero(proto.total_quantity.as_deref())?;
    order.display_size = proto.display_size.map(Some).unwrap_or(Some(0));
    order.order_type = s(&proto.order_type);
    order.limit_price = optional_f64(proto.lmt_price);
    order.aux_price = optional_f64(proto.aux_price);
    // Absent means the field is unset upstream, not GTZ-style unknown:
    // EClientUtils omits an empty Tif and EDecoderUtils only assigns when
    // HasTif, so "" and absent both collapse to the Order::default() TIF.
    // Anything else we do not model is preserved as Unknown(raw) rather than
    // coerced to DAY.
    order.tif = TimeInForce::from(proto.tif.as_deref().filter(|tif| !tif.is_empty()).unwrap_or("DAY"));

    // clearing info
    order.account = s(&proto.account);
    order.settling_firm = s(&proto.settling_firm);
    order.clearing_account = s(&proto.clearing_account);
    order.clearing_intent = s(&proto.clearing_intent);

    // secondary attributes
    order.all_or_none = proto.all_or_none.unwrap_or_default();
    order.block_order = proto.block_order.unwrap_or_default();
    order.hidden = proto.hidden.unwrap_or_default();
    order.outside_rth = proto.outside_rth.unwrap_or_default();
    order.sweep_to_fill = proto.sweep_to_fill.unwrap_or_default();
    order.percent_offset = optional_f64(proto.percent_offset);
    order.trailing_percent = optional_f64(proto.trailing_percent);
    order.trail_stop_price = optional_f64(proto.trail_stop_price);
    order.min_qty = proto.min_qty;
    order.good_after_time = s(&proto.good_after_time);
    order.good_till_date = s(&proto.good_till_date);
    order.oca_group = s(&proto.oca_group);
    order.order_ref = s(&proto.order_ref);
    order.rule_80_a = parse_optional::<Rule80A>(proto.rule80_a.as_deref())?;
    order.oca_type = OcaType::from(proto.oca_type.unwrap_or_default());
    order.trigger_method = TriggerMethod::from(proto.trigger_method.unwrap_or_default());

    // extended order fields
    order.active_start_time = s(&proto.active_start_time);
    order.active_stop_time = s(&proto.active_stop_time);

    // advisor allocation
    order.fa_group = s(&proto.fa_group);
    order.fa_method = s(&proto.fa_method);
    order.fa_percentage = s(&proto.fa_percentage);

    // volatility orders
    order.volatility = optional_f64(proto.volatility);
    order.volatility_type = proto.volatility_type.map(VolatilityType::from);
    order.continuous_update = proto.continuous_update.unwrap_or_default();
    order.reference_price_type = proto.reference_price_type.map(ReferencePriceType::from);
    order.delta_neutral_order_type = s(&proto.delta_neutral_order_type);
    order.delta_neutral_aux_price = optional_f64(proto.delta_neutral_aux_price);
    order.delta_neutral_con_id = proto.delta_neutral_con_id.unwrap_or_default();
    order.delta_neutral_open_close = s(&proto.delta_neutral_open_close);
    order.delta_neutral_short_sale = proto.delta_neutral_short_sale.unwrap_or_default();
    order.delta_neutral_short_sale_slot = proto.delta_neutral_short_sale_slot.unwrap_or_default();
    order.delta_neutral_designated_location = s(&proto.delta_neutral_designated_location);

    // scale orders
    order.scale_init_level_size = proto.scale_init_level_size;
    order.scale_subs_level_size = proto.scale_subs_level_size;
    order.scale_price_increment = optional_f64(proto.scale_price_increment);
    order.scale_price_adjust_value = optional_f64(proto.scale_price_adjust_value);
    order.scale_price_adjust_interval = proto.scale_price_adjust_interval;
    order.scale_profit_offset = optional_f64(proto.scale_profit_offset);
    order.scale_auto_reset = proto.scale_auto_reset.unwrap_or_default();
    order.scale_init_position = proto.scale_init_position;
    order.scale_init_fill_qty = proto.scale_init_fill_qty;
    order.scale_random_percent = proto.scale_random_percent.unwrap_or_default();
    order.scale_table = s(&proto.scale_table);

    // hedge orders
    order.hedge_type = s(&proto.hedge_type);
    order.hedge_param = s(&proto.hedge_param);
    order.hedge_max_size = proto.hedge_max_size;

    // algo orders
    order.algo_strategy = s(&proto.algo_strategy);
    order.algo_params = tag_values(&proto.algo_params);
    order.algo_id = s(&proto.algo_id);

    // combo orders
    order.smart_combo_routing_params = tag_values(&proto.smart_combo_routing_params);

    // processing control
    order.what_if = proto.what_if.unwrap_or_default();
    order.what_if_type = proto.what_if_type.map(WhatIfType::from);
    order.transmit = proto.transmit.unwrap_or(true);
    order.deactivate = proto.deactivate.unwrap_or_default();
    order.post_only = proto.post_only.unwrap_or_default();
    order.allow_pre_open = proto.allow_pre_open.unwrap_or_default();
    order.ignore_open_auction = proto.ignore_open_auction.unwrap_or_default();
    order.override_percentage_constraints = proto.override_percentage_constraints.unwrap_or_default();

    // institutional orders
    order.open_close = parse_optional::<OrderOpenClose>(proto.open_close.as_deref())?;
    order.origin = OrderOrigin::from(proto.origin.unwrap_or_default());
    order.short_sale_slot = ShortSaleSlot::from(proto.short_sale_slot.unwrap_or_default());
    order.designated_location = s(&proto.designated_location);
    order.exempt_code = proto.exempt_code.unwrap_or(-1);
    order.delta_neutral_settling_firm = s(&proto.delta_neutral_settling_firm);
    order.delta_neutral_clearing_account = s(&proto.delta_neutral_clearing_account);
    order.delta_neutral_clearing_intent = s(&proto.delta_neutral_clearing_intent);

    // SMART routing
    order.discretionary_amt = proto.discretionary_amt.unwrap_or_default();
    order.opt_out_smart_routing = proto.opt_out_smart_routing.unwrap_or_default();

    // BOX orders
    order.starting_price = optional_f64(proto.starting_price);
    order.stock_ref_price = optional_f64(proto.stock_ref_price);
    order.delta = optional_f64(proto.delta);

    // pegged orders
    order.stock_range_lower = optional_f64(proto.stock_range_lower);
    order.stock_range_upper = optional_f64(proto.stock_range_upper);

    // not held
    order.not_held = proto.not_held.unwrap_or_default();

    // order misc options
    order.order_misc_options = tag_values(&proto.order_misc_options);

    // solicited / randomize
    order.solicited = proto.solicited.unwrap_or_default();
    order.randomize_size = proto.randomize_size.unwrap_or_default();
    order.randomize_price = proto.randomize_price.unwrap_or_default();

    // PEG2BENCH fields
    order.reference_contract_id = proto.reference_contract_id.unwrap_or_default();
    order.pegged_change_amount = optional_f64(proto.pegged_change_amount);
    order.is_pegged_change_amount_decrease = proto.is_pegged_change_amount_decrease.unwrap_or_default();
    order.reference_change_amount = optional_f64(proto.reference_change_amount);
    order.reference_exchange = s(&proto.reference_exchange_id);
    order.adjusted_order_type = s(&proto.adjusted_order_type);
    order.trigger_price = optional_f64(proto.trigger_price);
    order.adjusted_stop_price = optional_f64(proto.adjusted_stop_price);
    order.adjusted_stop_limit_price = optional_f64(proto.adjusted_stop_limit_price);
    order.adjusted_trailing_amount = optional_f64(proto.adjusted_trailing_amount);
    order.adjustable_trailing_unit = proto.adjustable_trailing_unit.unwrap_or_default();
    order.limit_price_offset = optional_f64(proto.lmt_price_offset);

    // conditions
    order.conditions = proto.conditions.iter().map(decode_order_condition).collect::<Result<_, _>>()?;
    order.conditions_cancel_order = proto.conditions_cancel_order.unwrap_or_default();
    order.conditions_ignore_rth = proto.conditions_ignore_rth.unwrap_or_default();
    order.conditions_include_overnight = proto.conditions_include_overnight.unwrap_or_default();

    // models
    order.model_code = s(&proto.model_code);
    order.ext_operator = s(&proto.ext_operator);
    order.soft_dollar_tier = proto.soft_dollar_tier.as_ref().map(decode_soft_dollar_tier).unwrap_or_default();

    // native cash quantity
    order.cash_qty = optional_f64(proto.cash_qty);

    // MIFID2
    order.mifid2_decision_maker = s(&proto.mifid2_decision_maker);
    order.mifid2_decision_algo = s(&proto.mifid2_decision_algo);
    order.mifid2_execution_trader = s(&proto.mifid2_execution_trader);
    order.mifid2_execution_algo = s(&proto.mifid2_execution_algo);

    // additional fields
    order.dont_use_auto_price_for_hedge = proto.dont_use_auto_price_for_hedge.unwrap_or_default();
    order.is_oms_container = proto.is_oms_container.unwrap_or_default();
    order.discretionary_up_to_limit_price = proto.discretionary_up_to_limit_price.unwrap_or_default();
    order.auto_cancel_date = s(&proto.auto_cancel_date);
    // Absent means unset upstream (Order.cs ctor sets decimal.MaxValue).
    order.filled_quantity = parse_decimal_or_zero(proto.filled_quantity.as_deref())?;
    order.ref_futures_con_id = proto.ref_futures_con_id.map(Some).unwrap_or(Some(0));
    order.auto_cancel_parent = proto.auto_cancel_parent.unwrap_or_default();
    order.shareholder = s(&proto.shareholder);
    order.imbalance_only = proto.imbalance_only.unwrap_or_default();
    order.route_marketable_to_bbo = proto.route_marketable_to_bbo.unwrap_or_default() != 0;
    order.seek_price_improvement = proto.seek_price_improvement.map(|v| v != 0);
    order.parent_perm_id = proto.parent_perm_id;
    order.use_price_mgmt_algo = proto.use_price_mgmt_algo.unwrap_or_default() != 0;
    order.duration = proto.duration;
    order.post_to_ats = proto.post_to_ats;
    order.advanced_error_override = s(&proto.advanced_error_override);
    order.manual_order_time = s(&proto.manual_order_time);
    order.min_trade_qty = proto.min_trade_qty;
    order.min_compete_size = proto.min_compete_size;
    order.compete_against_best_offset = optional_f64(proto.compete_against_best_offset);
    order.mid_offset_at_whole = optional_f64(proto.mid_offset_at_whole);
    order.mid_offset_at_half = optional_f64(proto.mid_offset_at_half);
    order.customer_account = s(&proto.customer_account);
    order.professional_customer = proto.professional_customer.unwrap_or_default();
    order.bond_accrued_interest = s(&proto.bond_accrued_interest);
    order.include_overnight = proto.include_overnight.unwrap_or_default();
    order.manual_order_indicator = proto.manual_order_indicator;
    order.submitter = s(&proto.submitter);

    Ok(order)
}

fn decode_order_condition(proto: &proto::OrderCondition) -> Result<OrderCondition, Error> {
    use crate::orders::conditions::*;

    // The reference client always sets `type`, so an absent one is a malformed frame.
    let Some(condition_type) = proto.r#type else {
        return Err(Error::parse_proto("type", "missing OrderCondition type"));
    };
    let is_conjunction = proto.is_conjunction_connection.unwrap_or(true);
    let is_more = proto.is_more.unwrap_or_default();

    let condition = match condition_type {
        1 => OrderCondition::Price(PriceCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            price: proto.price.unwrap_or_default(),
            is_more,
            is_conjunction,
            trigger_method: TriggerMethod::from(proto.trigger_method.unwrap_or_default()),
        }),
        3 => OrderCondition::Time(TimeCondition {
            time: s(&proto.time),
            is_more,
            is_conjunction,
        }),
        4 => OrderCondition::Margin(MarginCondition {
            percent: proto.percent.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        5 => OrderCondition::Execution(ExecutionCondition {
            symbol: s(&proto.symbol),
            security_type: s(&proto.sec_type),
            exchange: s(&proto.exchange),
            is_conjunction,
        }),
        6 => OrderCondition::Volume(VolumeCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            volume: proto.volume.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        7 => OrderCondition::PercentChange(PercentChangeCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            percent: proto.change_percent.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        // Preserved whole, so re-placing the order sends it back unchanged.
        _ => OrderCondition::Unknown(UnknownCondition {
            condition_type,
            is_conjunction,
            is_more: proto.is_more,
            contract_id: proto.con_id,
            exchange: proto.exchange.clone(),
            symbol: proto.symbol.clone(),
            security_type: proto.sec_type.clone(),
            percent: proto.percent,
            change_percent: proto.change_percent,
            price: proto.price,
            trigger_method: proto.trigger_method,
            time: proto.time.clone(),
            volume: proto.volume,
        }),
    };

    Ok(condition)
}

pub(crate) fn decode_order_state(proto: &proto::OrderState) -> Result<OrderState, Error> {
    Ok(OrderState {
        status: parse_required(proto.status.as_deref(), "OrderStatus")?,
        initial_margin_before: optional_f64(proto.init_margin_before),
        maintenance_margin_before: optional_f64(proto.maint_margin_before),
        equity_with_loan_before: optional_f64(proto.equity_with_loan_before),
        initial_margin_change: optional_f64(proto.init_margin_change),
        maintenance_margin_change: optional_f64(proto.maint_margin_change),
        equity_with_loan_change: optional_f64(proto.equity_with_loan_change),
        initial_margin_after: optional_f64(proto.init_margin_after),
        maintenance_margin_after: optional_f64(proto.maint_margin_after),
        equity_with_loan_after: optional_f64(proto.equity_with_loan_after),
        commission: optional_f64(proto.commission_and_fees),
        minimum_commission: optional_f64(proto.min_commission_and_fees),
        maximum_commission: optional_f64(proto.max_commission_and_fees),
        commission_currency: s(&proto.commission_and_fees_currency),
        margin_currency: s(&proto.margin_currency),
        initial_margin_before_outside_rth: optional_f64(proto.init_margin_before_outside_rth),
        maintenance_margin_before_outside_rth: optional_f64(proto.maint_margin_before_outside_rth),
        equity_with_loan_before_outside_rth: optional_f64(proto.equity_with_loan_before_outside_rth),
        initial_margin_change_outside_rth: optional_f64(proto.init_margin_change_outside_rth),
        maintenance_margin_change_outside_rth: optional_f64(proto.maint_margin_change_outside_rth),
        equity_with_loan_change_outside_rth: optional_f64(proto.equity_with_loan_change_outside_rth),
        initial_margin_after_outside_rth: optional_f64(proto.init_margin_after_outside_rth),
        maintenance_margin_after_outside_rth: optional_f64(proto.maint_margin_after_outside_rth),
        equity_with_loan_after_outside_rth: optional_f64(proto.equity_with_loan_after_outside_rth),
        suggested_size: parse_optional_decimal(proto.suggested_size.as_deref())?,
        reject_reason: s(&proto.reject_reason),
        order_allocations: proto
            .order_allocations
            .iter()
            .map(decode_order_allocation)
            .collect::<Result<Vec<_>, Error>>()?,
        warning_text: s(&proto.warning_text),
        completed_time: s(&proto.completed_time),
        completed_status: s(&proto.completed_status),
    })
}

fn decode_order_allocation(proto: &proto::OrderAllocation) -> Result<OrderAllocation, Error> {
    Ok(OrderAllocation {
        account: s(&proto.account),
        position: parse_optional_decimal(proto.position.as_deref())?,
        position_desired: parse_optional_decimal(proto.position_desired.as_deref())?,
        position_after: parse_optional_decimal(proto.position_after.as_deref())?,
        desired_alloc_qty: parse_optional_decimal(proto.desired_alloc_qty.as_deref())?,
        allowed_alloc_qty: parse_optional_decimal(proto.allowed_alloc_qty.as_deref())?,
        is_monetary: proto.is_monetary.unwrap_or_default(),
    })
}

pub(crate) fn decode_execution(proto: &proto::Execution) -> Result<Execution, Error> {
    Ok(Execution {
        order_id: proto.order_id.unwrap_or_default(),
        client_id: proto.client_id.unwrap_or_default(),
        execution_id: s(&proto.exec_id),
        time: s(&proto.time),
        account_number: s(&proto.acct_number),
        exchange: s(&proto.exchange),
        side: parse_required::<ExecutionSide>(proto.side.as_deref(), "Execution.side")?,
        // Absent means 0 upstream (Execution.cs ctor sets Shares/CumQty to 0).
        shares: parse_decimal_or_zero(proto.shares.as_deref())?,
        price: proto.price.unwrap_or_default(),
        perm_id: proto.perm_id.unwrap_or_default(),
        liquidation: if proto.is_liquidation.unwrap_or_default() { 1 } else { 0 },
        cumulative_quantity: parse_decimal_or_zero(proto.cum_qty.as_deref())?,
        average_price: proto.avg_price.unwrap_or_default(),
        order_reference: s(&proto.order_ref),
        ev_rule: s(&proto.ev_rule),
        ev_multiplier: optional_f64(proto.ev_multiplier),
        model_code: s(&proto.model_code),
        last_liquidity: Liquidity::from(proto.last_liquidity.unwrap_or_default()),
        pending_price_revision: proto.is_price_revision_pending.unwrap_or_default(),
        submitter: s(&proto.submitter),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
