use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::{ExecutionFilter, ExerciseAction, Order, OrderCondition, SoftDollarTier};
use crate::proto;
use crate::proto::encoders::{some_bool, some_display, some_f64_ne, some_i32_ne, some_i64_ne, some_str, tag_values_to_map};
use crate::Error;

pub(crate) fn encode_place_order(order_id: i32, contract: &Contract, order: &Order) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::PlaceOrderRequest {
        order_id: Some(order_id),
        contract: Some(crate::proto::encoders::encode_contract_with_order(contract, Some(order))),
        order: Some(encode_order(order)),
        attached_orders: encode_attached_orders(order),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::PlaceOrder as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_order(order_id: i32, manual_order_cancel_time: &str) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::CancelOrderRequest {
        order_id: Some(order_id),
        order_cancel: Some(encode_order_cancel(manual_order_cancel_time)),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::CancelOrder as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_open_orders() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(OpenOrdersRequest, OutgoingMessages::RequestOpenOrders)
}

pub(crate) fn encode_all_open_orders() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(AllOpenOrdersRequest, OutgoingMessages::RequestAllOpenOrders)
}

pub(crate) fn encode_auto_open_orders(auto_bind: bool) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::AutoOpenOrdersRequest {
        auto_bind: some_bool(auto_bind),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestAutoOpenOrders as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_completed_orders(api_only: bool) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::CompletedOrdersRequest {
        api_only: some_bool(api_only),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestCompletedOrders as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_executions(request_id: i32, filter: &ExecutionFilter) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::ExecutionRequest {
        req_id: Some(request_id),
        execution_filter: Some(encode_execution_filter(filter)),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestExecutions as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_global_cancel() -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::GlobalCancelRequest {
        order_cancel: Some(encode_order_cancel("")),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestGlobalCancel as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_next_valid_order_id() -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::IdsRequest { num_ids: Some(0) };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestIds as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_exercise_options(
    order_id: i32,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<Vec<u8>, Error> {
    use prost::Message;
    use time::macros::format_description;
    use time_tz::OffsetDateTimeExt;

    let manual_order_time_str = manual_order_time.map(|dt| {
        let adjusted = dt.to_timezone(time_tz::timezones::db::UTC);
        let fmt = format_description!("[year][month][day] [hour]:[minute]:[second]");
        format!("{} UTC", adjusted.format(fmt).unwrap())
    });

    let request = crate::proto::ExerciseOptionsRequest {
        order_id: Some(order_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        exercise_action: Some(exercise_action as i32),
        exercise_quantity: Some(exercise_quantity),
        account: some_str(account),
        r#override: some_bool(ovrd),
        manual_order_time: manual_order_time_str,
        customer_account: None,
        professional_customer: None,
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::ExerciseOptions as i32,
        &request.encode_to_vec(),
    ))
}

// === Order ===

pub(crate) fn encode_order(order: &Order) -> proto::Order {
    let proto_order = proto::Order {
        client_id: some_i32_ne(order.client_id, 0),
        perm_id: some_i64_ne(order.perm_id, 0),
        parent_id: some_i32_ne(order.parent_id, 0),
        action: some_str(&order.action.to_string()),
        total_quantity: if order.total_quantity == 0.0 {
            None
        } else {
            Some(order.total_quantity.to_string())
        },
        display_size: order.display_size,
        order_type: some_str(&order.order_type),
        lmt_price: order.limit_price,
        aux_price: order.aux_price,
        tif: some_str(&order.tif.to_string()),
        account: some_str(&order.account),
        settling_firm: some_str(&order.settling_firm),
        clearing_account: some_str(&order.clearing_account),
        clearing_intent: some_str(&order.clearing_intent),
        all_or_none: some_bool(order.all_or_none),
        block_order: some_bool(order.block_order),
        hidden: some_bool(order.hidden),
        outside_rth: some_bool(order.outside_rth),
        sweep_to_fill: some_bool(order.sweep_to_fill),
        percent_offset: order.percent_offset,
        trailing_percent: order.trailing_percent,
        trail_stop_price: order.trail_stop_price,
        min_qty: order.min_qty,
        good_after_time: some_str(&order.good_after_time),
        good_till_date: some_str(&order.good_till_date),
        oca_group: some_str(&order.oca_group),
        order_ref: some_str(&order.order_ref),
        rule80_a: order.rule_80_a.as_ref().and_then(|r| some_str(r.as_str())),
        oca_type: some_i32_ne(i32::from(order.oca_type), 0),
        trigger_method: some_i32_ne(i32::from(order.trigger_method), 0),
        active_start_time: some_str(&order.active_start_time),
        active_stop_time: some_str(&order.active_stop_time),
        fa_group: some_str(&order.fa_group),
        fa_method: some_str(&order.fa_method),
        fa_percentage: some_str(&order.fa_percentage),
        volatility: order.volatility,
        volatility_type: order.volatility_type.map(i32::from),
        continuous_update: some_bool(order.continuous_update),
        reference_price_type: order.reference_price_type.map(i32::from),
        delta_neutral_order_type: some_str(&order.delta_neutral_order_type),
        delta_neutral_aux_price: order.delta_neutral_aux_price,
        delta_neutral_con_id: some_i32_ne(order.delta_neutral_con_id, 0),
        delta_neutral_open_close: some_str(&order.delta_neutral_open_close),
        delta_neutral_short_sale: some_bool(order.delta_neutral_short_sale),
        delta_neutral_short_sale_slot: some_i32_ne(order.delta_neutral_short_sale_slot, 0),
        delta_neutral_designated_location: some_str(&order.delta_neutral_designated_location),
        scale_init_level_size: order.scale_init_level_size,
        scale_subs_level_size: order.scale_subs_level_size,
        scale_price_increment: order.scale_price_increment,
        scale_price_adjust_value: order.scale_price_adjust_value,
        scale_price_adjust_interval: order.scale_price_adjust_interval,
        scale_profit_offset: order.scale_profit_offset,
        scale_auto_reset: some_bool(order.scale_auto_reset),
        scale_init_position: order.scale_init_position,
        scale_init_fill_qty: order.scale_init_fill_qty,
        scale_random_percent: some_bool(order.scale_random_percent),
        scale_table: some_str(&order.scale_table),
        hedge_type: some_str(&order.hedge_type),
        hedge_param: some_str(&order.hedge_param),
        hedge_max_size: order.hedge_max_size,
        algo_strategy: some_str(&order.algo_strategy),
        algo_params: tag_values_to_map(&order.algo_params),
        algo_id: some_str(&order.algo_id),
        smart_combo_routing_params: tag_values_to_map(&order.smart_combo_routing_params),
        what_if: some_bool(order.what_if),
        transmit: some_bool(order.transmit),
        override_percentage_constraints: some_bool(order.override_percentage_constraints),
        open_close: order.open_close.as_ref().and_then(|oc| some_str(oc.as_str())),
        origin: some_i32_ne(i32::from(order.origin), 0),
        short_sale_slot: some_i32_ne(i32::from(order.short_sale_slot), 0),
        designated_location: some_str(&order.designated_location),
        exempt_code: some_i32_ne(order.exempt_code, 0),
        delta_neutral_settling_firm: some_str(&order.delta_neutral_settling_firm),
        delta_neutral_clearing_account: some_str(&order.delta_neutral_clearing_account),
        delta_neutral_clearing_intent: some_str(&order.delta_neutral_clearing_intent),
        discretionary_amt: some_f64_ne(order.discretionary_amt, 0.0),
        opt_out_smart_routing: some_bool(order.opt_out_smart_routing),
        starting_price: order.starting_price,
        stock_ref_price: order.stock_ref_price,
        delta: order.delta,
        stock_range_lower: order.stock_range_lower,
        stock_range_upper: order.stock_range_upper,
        not_held: some_bool(order.not_held),
        order_misc_options: tag_values_to_map(&order.order_misc_options),
        solicited: some_bool(order.solicited),
        randomize_size: some_bool(order.randomize_size),
        randomize_price: some_bool(order.randomize_price),
        reference_contract_id: some_i32_ne(order.reference_contract_id, 0),
        pegged_change_amount: order.pegged_change_amount,
        is_pegged_change_amount_decrease: some_bool(order.is_pegged_change_amount_decrease),
        reference_change_amount: order.reference_change_amount,
        reference_exchange_id: some_str(&order.reference_exchange),
        adjusted_order_type: some_str(&order.adjusted_order_type),
        trigger_price: order.trigger_price,
        adjusted_stop_price: order.adjusted_stop_price,
        adjusted_stop_limit_price: order.adjusted_stop_limit_price,
        adjusted_trailing_amount: order.adjusted_trailing_amount,
        adjustable_trailing_unit: some_i32_ne(order.adjustable_trailing_unit, 0),
        lmt_price_offset: order.limit_price_offset,
        conditions: encode_conditions(&order.conditions),
        conditions_cancel_order: some_bool(order.conditions_cancel_order),
        conditions_ignore_rth: some_bool(order.conditions_ignore_rth),
        conditions_include_overnight: some_bool(order.conditions_include_overnight),
        model_code: some_str(&order.model_code),
        ext_operator: some_str(&order.ext_operator),
        soft_dollar_tier: encode_soft_dollar_tier(&order.soft_dollar_tier),
        cash_qty: order.cash_qty,
        mifid2_decision_maker: some_str(&order.mifid2_decision_maker),
        mifid2_decision_algo: some_str(&order.mifid2_decision_algo),
        mifid2_execution_trader: some_str(&order.mifid2_execution_trader),
        mifid2_execution_algo: some_str(&order.mifid2_execution_algo),
        dont_use_auto_price_for_hedge: some_bool(order.dont_use_auto_price_for_hedge),
        is_oms_container: some_bool(order.is_oms_container),
        discretionary_up_to_limit_price: some_bool(order.discretionary_up_to_limit_price),
        use_price_mgmt_algo: if order.use_price_mgmt_algo { Some(1) } else { None },
        duration: order.duration,
        post_to_ats: order.post_to_ats,
        advanced_error_override: some_str(&order.advanced_error_override),
        manual_order_time: some_str(&order.manual_order_time),
        min_trade_qty: order.min_trade_qty,
        min_compete_size: order.min_compete_size,
        compete_against_best_offset: order.compete_against_best_offset,
        mid_offset_at_whole: order.mid_offset_at_whole,
        mid_offset_at_half: order.mid_offset_at_half,
        customer_account: some_str(&order.customer_account),
        professional_customer: some_bool(order.professional_customer),
        bond_accrued_interest: some_str(&order.bond_accrued_interest),
        include_overnight: some_bool(order.include_overnight),
        manual_order_indicator: order.manual_order_indicator,
        submitter: some_str(&order.submitter),
        auto_cancel_parent: some_bool(order.auto_cancel_parent),
        imbalance_only: some_bool(order.imbalance_only),
        // fields not directly mapped from our Order struct
        order_id: None,
        auto_cancel_date: some_str(&order.auto_cancel_date),
        filled_quantity: if order.filled_quantity == 0.0 {
            None
        } else {
            Some(order.filled_quantity.to_string())
        },
        ref_futures_con_id: order.ref_futures_con_id,
        shareholder: some_str(&order.shareholder),
        route_marketable_to_bbo: if order.route_marketable_to_bbo { Some(1) } else { None },
        parent_perm_id: order.parent_perm_id,
        deactivate: some_bool(order.deactivate),
        post_only: some_bool(order.post_only),
        allow_pre_open: some_bool(order.allow_pre_open),
        ignore_open_auction: some_bool(order.ignore_open_auction),
        seek_price_improvement: order.seek_price_improvement.map(i32::from),
        what_if_type: order.what_if_type.map(i32::from),
    };

    proto_order
}

fn encode_conditions(conditions: &[OrderCondition]) -> Vec<proto::OrderCondition> {
    conditions.iter().map(encode_condition).collect()
}

fn encode_condition(condition: &OrderCondition) -> proto::OrderCondition {
    let mut proto_cond = proto::OrderCondition {
        r#type: Some(condition.condition_type()),
        is_conjunction_connection: Some(condition.is_conjunction()),
        ..Default::default()
    };

    match condition {
        OrderCondition::Price(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.price = some_f64_ne(c.price, 0.0);
            // C# PriceCondition.Serialize always writes trigger_method (incl. Default=0);
            // omitting it makes TWS reject with "Invalid value in field # 6127".
            proto_cond.trigger_method = Some(i32::from(c.trigger_method));
        }
        OrderCondition::Time(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.time = some_str(&c.time);
        }
        OrderCondition::Margin(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.percent = some_i32_ne(c.percent, 0);
        }
        OrderCondition::Execution(c) => {
            proto_cond.symbol = some_str(&c.symbol);
            proto_cond.sec_type = some_str(&c.security_type);
            proto_cond.exchange = some_str(&c.exchange);
        }
        OrderCondition::Volume(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.volume = some_i32_ne(c.volume, 0);
        }
        OrderCondition::PercentChange(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.change_percent = some_f64_ne(c.percent, 0.0);
        }
        OrderCondition::Unknown(c) => {
            proto_cond.is_more = c.is_more;
            proto_cond.con_id = c.contract_id;
            proto_cond.exchange = c.exchange.clone();
            proto_cond.symbol = c.symbol.clone();
            proto_cond.sec_type = c.security_type.clone();
            proto_cond.percent = c.percent;
            proto_cond.change_percent = c.change_percent;
            proto_cond.price = c.price;
            proto_cond.trigger_method = c.trigger_method;
            proto_cond.time = c.time.clone();
            proto_cond.volume = c.volume;
        }
    }

    proto_cond
}

fn encode_soft_dollar_tier(tier: &SoftDollarTier) -> Option<proto::SoftDollarTier> {
    if tier.name.is_empty() && tier.value.is_empty() && tier.display_name.is_empty() {
        return None;
    }
    Some(proto::SoftDollarTier {
        name: some_str(&tier.name),
        value: some_str(&tier.value),
        display_name: some_str(&tier.display_name),
    })
}

// === Execution Filter ===

pub(crate) fn encode_execution_filter(filter: &ExecutionFilter) -> proto::ExecutionFilter {
    proto::ExecutionFilter {
        client_id: filter.client_id,
        acct_code: some_str(&filter.account_code),
        time: some_str(&filter.time),
        symbol: some_str(&filter.symbol),
        sec_type: some_str(&filter.security_type),
        exchange: some_str(&filter.exchange),
        side: some_display(filter.side.as_ref()),
        last_n_days: some_i32_ne(filter.last_n_days, 0),
        specific_dates: filter.specific_dates.iter().filter_map(|d| d.parse::<i32>().ok()).collect(),
    }
}

// === Attached orders, OrderCancel ===

/// The only attached-order type TWS accepts (live-verified, #842).
const PRESET_ATTACHED_ORDER_TYPE: &str = "PRESET";

/// `PlaceOrderRequest.attached_orders` for the preset legs on `order`, or `None` when it has none.
pub(crate) fn encode_attached_orders(order: &Order) -> Option<proto::AttachedOrders> {
    let stop_loss = order.preset_stop_loss_order_id;
    let profit_taker = order.preset_profit_taker_order_id;
    if stop_loss.is_none() && profit_taker.is_none() {
        return None;
    }
    let preset = |id: Option<i32>| id.map(|_| PRESET_ATTACHED_ORDER_TYPE.to_string());
    Some(proto::AttachedOrders {
        sl_order_id: stop_loss,
        sl_order_type: preset(stop_loss),
        pt_order_id: profit_taker,
        pt_order_type: preset(profit_taker),
    })
}

pub(crate) fn encode_order_cancel(manual_order_cancel_time: &str) -> proto::OrderCancel {
    proto::OrderCancel {
        manual_order_cancel_time: some_str(manual_order_cancel_time),
        ext_operator: None,
        manual_order_indicator: None,
    }
}

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;
