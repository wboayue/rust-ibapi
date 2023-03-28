use crate::Error;

use super::*;

pub(crate) fn encode_place_order(server_version: i32, order_id: i32, contract: &Contract, order: &Order) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();
    let message_version = message_version_for(server_version);

    message.push_field(&OutgoingMessages::PlaceOrder);

    if server_version < server_versions::ORDER_CONTAINER {
        message.push_field(&message_version);
    }

    message.push_field(&order_id);

    if server_version >= server_versions::PLACE_ORDER_CONID {
        message.push_field(&contract.contract_id);
    }
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.trading_class);
    }
    if server_version >= server_versions::SEC_ID_TYPE {
        message.push_field(&contract.security_id_type);
        message.push_field(&contract.security_id);
    }

    message.push_field(&order.action);

    if server_version >= server_versions::FRACTIONAL_POSITIONS {
        message.push_field(&order.total_quantity);
    } else {
        message.push_field(&(order.total_quantity as i32));
    }

    message.push_field(&order.order_type);
    if server_version < server_versions::ORDER_COMBO_LEGS_PRICE {
        message.push_field(&f64_max_to_zero(order.limit_price));
    } else {
        message.push_field(&order.limit_price);
    }
    if server_version < server_versions::TRAILING_PERCENT {
        message.push_field(&f64_max_to_zero(order.aux_price));
    } else {
        message.push_field(&order.aux_price);
    }

    // extended order fields
    message.push_field(&order.tif);
    message.push_field(&order.oca_group);
    message.push_field(&order.account);
    message.push_field(&order.open_close);
    message.push_field(&order.origin);
    message.push_field(&order.order_ref);
    message.push_field(&order.transmit);
    message.push_field(&order.parent_id);

    message.push_field(&order.block_order);
    message.push_field(&order.sweep_to_fill);
    message.push_field(&order.display_size);
    message.push_field(&order.trigger_method);
    message.push_field(&order.outside_rth);

    message.push_field(&order.hidden);

    // Contract combo legs for BAG requests
    if contract.is_bag() {
        message.push_field(&contract.combo_legs.len());

        for combo_leg in &contract.combo_legs {
            message.push_field(&combo_leg.contract_id);
            message.push_field(&combo_leg.ratio);
            message.push_field(&combo_leg.action);
            message.push_field(&combo_leg.exchange);
            message.push_field(&combo_leg.open_close);

            if server_version >= server_versions::SSHORT_COMBO_LEGS {
                message.push_field(&combo_leg.short_sale_slot);
                message.push_field(&combo_leg.designated_location);
            }
            if server_version >= server_versions::SSHORTX_OLD {
                message.push_field(&combo_leg.exempt_code);
            }
        }
    }

    // Order combo legs for BAG requests
    if server_version >= server_versions::ORDER_COMBO_LEGS_PRICE && contract.is_bag() {
        message.push_field(&order.order_combo_legs.len());

        for combo_leg in &order.order_combo_legs {
            message.push_field(&combo_leg.price);
        }
    }

    if server_version >= server_versions::SMART_COMBO_ROUTING_PARAMS && contract.is_bag() {
        message.push_field(&order.smart_combo_routing_params.len());

        for tag_value in &order.smart_combo_routing_params {
            message.push_field(&tag_value.tag);
            message.push_field(&tag_value.value);
        }
    }

    message.push_field(&""); // deprecated sharesAllocation field

    message.push_field(&order.discretionary_amt);
    message.push_field(&order.good_after_time);
    message.push_field(&order.good_till_date);

    message.push_field(&order.fa_group);
    message.push_field(&order.fa_method);
    message.push_field(&order.fa_percentage);
    if server_version < server_versions::FA_PROFILE_DESUPPORT {
        message.push_field(&order.fa_profile);
    }

    if server_version >= server_versions::MODELS_SUPPORT {
        message.push_field(&order.model_code);
    }

    message.push_field(&order.short_sale_slot);
    message.push_field(&order.designated_location);

    if server_version >= server_versions::SSHORTX_OLD {
        message.push_field(&order.exempt_code);
    }

    message.push_field(&order.oca_type);
    message.push_field(&order.rule_80_a);
    message.push_field(&order.settling_firm);
    message.push_field(&order.all_or_none);
    message.push_field(&order.min_qty);
    message.push_field(&order.percent_offset);
    message.push_field(&false);
    message.push_field(&false);
    message.push_field(&Option::<f64>::None);
    message.push_field(&order.auction_strategy);
    message.push_field(&order.starting_price);
    message.push_field(&order.stock_ref_price);
    message.push_field(&order.delta);
    message.push_field(&order.stock_range_lower);
    message.push_field(&order.stock_range_upper);

    message.push_field(&order.override_percentage_constraints);

    // Volitility orders
    message.push_field(&order.volatility);
    message.push_field(&order.volatility_type);
    message.push_field(&order.delta_neutral_order_type);
    message.push_field(&order.delta_neutral_aux_price);

    if server_version >= server_versions::DELTA_NEUTRAL_CONID && order.is_delta_neutral() {
        message.push_field(&order.delta_neutral_con_id);
        message.push_field(&order.delta_neutral_settling_firm);
        message.push_field(&order.delta_neutral_clearing_account);
        message.push_field(&order.delta_neutral_clearing_intent);
    }

    if server_version >= server_versions::DELTA_NEUTRAL_OPEN_CLOSE && order.is_delta_neutral() {
        message.push_field(&order.delta_neutral_open_close);
        message.push_field(&order.delta_neutral_short_sale);
        message.push_field(&order.delta_neutral_short_sale_slot);
        message.push_field(&order.delta_neutral_designated_location);
    }

    message.push_field(&order.continuous_update);
    message.push_field(&order.reference_price_type);

    message.push_field(&order.trail_stop_price);
    if server_version >= server_versions::TRAILING_PERCENT {
        message.push_field(&order.trailing_percent);
    }

    if server_version >= server_versions::SCALE_ORDERS {
        if server_version >= server_versions::SCALE_ORDERS2 {
            message.push_field(&order.scale_init_level_size);
            message.push_field(&order.scale_subs_level_size);
        } else {
            message.push_field(&"");
            message.push_field(&order.scale_init_level_size);
        }
        message.push_field(&order.scale_price_increment);
    }

    if server_version >= server_versions::SCALE_ORDERS3 && order.is_scale_order() {
        message.push_field(&order.scale_price_adjust_value);
        message.push_field(&order.scale_price_adjust_interval);
        message.push_field(&order.scale_profit_offset);
        message.push_field(&order.scale_auto_reset);
        message.push_field(&order.scale_init_position);
        message.push_field(&order.scale_init_fill_qty);
        message.push_field(&order.scale_random_percent);
    }

    if server_version >= server_versions::SCALE_TABLE {
        message.push_field(&order.scale_table);
        message.push_field(&order.active_start_time);
        message.push_field(&order.active_stop_time);
    }

    if server_version >= server_versions::HEDGE_ORDERS {
        message.push_field(&order.hedge_type);
        if !order.hedge_type.is_empty() {
            message.push_field(&order.hedge_param);
        }
    }

    if server_version >= server_versions::OPT_OUT_SMART_ROUTING {
        message.push_field(&order.opt_out_smart_routing);
    }

    if server_version >= server_versions::PTA_ORDERS {
        message.push_field(&order.clearing_account);
        message.push_field(&order.clearing_intent);
    }

    if server_version >= server_versions::NOT_HELD {
        message.push_field(&order.not_held);
    }

    if server_version >= server_versions::DELTA_NEUTRAL {
        if let Some(delta_neutral_contract) = &contract.delta_neutral_contract {
            message.push_field(&true);
            message.push_field(&delta_neutral_contract.contract_id);
            message.push_field(&delta_neutral_contract.delta);
            message.push_field(&delta_neutral_contract.price);
        } else {
            message.push_field(&false);
        }
    }

    if server_version >= server_versions::ALGO_ORDERS {
        message.push_field(&order.algo_strategy);
        if !order.algo_strategy.is_empty() {
            message.push_field(&order.algo_params.len());
            for tag_value in &order.algo_params {
                message.push_field(&tag_value.tag);
                message.push_field(&tag_value.value);
            }
        }
    }

    if server_version >= server_versions::ALGO_ID {
        message.push_field(&order.algo_id);
    }

    if server_version >= server_versions::WHAT_IF_ORDERS {
        message.push_field(&order.what_if);
    }

    if server_version >= server_versions::LINKING {
        message.push_field(&order.order_misc_options);
    }

    if server_version >= server_versions::ORDER_SOLICITED {
        message.push_field(&order.solicited);
    }

    if server_version >= server_versions::RANDOMIZE_SIZE_AND_PRICE {
        message.push_field(&order.randomize_size);
        message.push_field(&order.randomize_price);
    }

    if server_version >= server_versions::PEGGED_TO_BENCHMARK {
        if order.order_type == "PEG BENCH" {
            message.push_field(&order.reference_contract_id);
            message.push_field(&order.is_pegged_change_amount_decrease);
            message.push_field(&order.pegged_change_amount);
            message.push_field(&order.reference_change_amount);
            message.push_field(&order.reference_exchange);
        }

        message.push_field(&order.conditions.len());

        if !order.conditions.is_empty() {
            for condition in &order.conditions {
                // verify
                // https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EClient.cs#L1187
                message.push_field(condition);
            }

            message.push_field(&order.conditions_ignore_rth);
            message.push_field(&order.conditions_cancel_order);
        }

        message.push_field(&order.adjusted_order_type);
        message.push_field(&order.trigger_price);
        message.push_field(&order.limit_price_offset);
        message.push_field(&order.adjusted_stop_price);
        message.push_field(&order.adjusted_stop_limit_price);
        message.push_field(&order.adjusted_trailing_amount);
        message.push_field(&order.adjustable_trailing_unit);
    }

    if server_version >= server_versions::EXT_OPERATOR {
        message.push_field(&order.ext_operator);
    }

    if server_version >= server_versions::SOFT_DOLLAR_TIER {
        message.push_field(&order.soft_dollar_tier.name);
        message.push_field(&order.soft_dollar_tier.value);
    }

    if server_version >= server_versions::CASH_QTY {
        message.push_field(&order.cash_qty);
    }

    if server_version >= server_versions::DECISION_MAKER {
        message.push_field(&order.mifid2_decision_maker);
        message.push_field(&order.mifid2_decision_algo);
    }

    if server_version >= server_versions::MIFID_EXECUTION {
        message.push_field(&order.mifid2_execution_trader);
        message.push_field(&order.mifid2_execution_algo);
    }

    if server_version >= server_versions::AUTO_PRICE_FOR_HEDGE {
        message.push_field(&order.dont_use_auto_price_for_hedge);
    }

    if server_version >= server_versions::ORDER_CONTAINER {
        message.push_field(&order.is_oms_container);
    }

    if server_version >= server_versions::D_PEG_ORDERS {
        message.push_field(&order.discretionary_up_to_limit_price);
    }

    if server_version >= server_versions::PRICE_MGMT_ALGO {
        message.push_field(&order.use_price_mgmt_algo);
    }

    if server_version >= server_versions::DURATION {
        message.push_field(&order.duration);
    }

    if server_version >= server_versions::POST_TO_ATS {
        message.push_field(&order.post_to_ats);
    }

    if server_version >= server_versions::AUTO_CANCEL_PARENT {
        message.push_field(&order.auto_cancel_parent);
    }

    if server_version >= server_versions::ADVANCED_ORDER_REJECT {
        message.push_field(&order.advanced_error_override);
    }

    if server_version >= server_versions::MANUAL_ORDER_TIME {
        message.push_field(&order.manual_order_time);
    }

    if server_version >= server_versions::PEGBEST_PEGMID_OFFSETS {
        if contract.exchange == "IBKRATS" {
            message.push_field(&order.min_trade_qty);
        }
        let mut send_mid_offsets = false;
        if order.order_type == "PEG BEST" {
            message.push_field(&order.min_compete_size);
            message.push_field(&order.compete_against_best_offset);
            if order.compete_against_best_offset == COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID {
                send_mid_offsets = true;
            }
        } else if order.order_type == "PEG MID" {
            send_mid_offsets = true;
        }
        if send_mid_offsets {
            message.push_field(&order.mid_offset_at_whole);
            message.push_field(&order.mid_offset_at_half);
        }
    }

    Ok(message)
}

pub(crate) fn encode_cancel_order(server_version: i32, order_id: i32, manual_order_cancel_time: &str) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::CancelOrder);
    message.push_field(&VERSION);
    message.push_field(&order_id);

    if server_version >= server_versions::MANUAL_ORDER_TIME {
        message.push_field(&manual_order_cancel_time);
    }

    Ok(message)
}

pub(crate) fn encode_global_cancel() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestGlobalCancel);
    message.push_field(&VERSION);

    Ok(message)
}

pub(crate) fn encode_next_valid_order_id() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestIds);
    message.push_field(&VERSION);
    message.push_field(&0);

    Ok(message)
}

pub(crate) fn encode_completed_orders(api_only: bool) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::ReqCompletedOrders);
    message.push_field(&api_only);

    Ok(message)
}

pub(crate) fn encode_open_orders() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestOpenOrders);
    message.push_field(&VERSION);

    Ok(message)
}

pub(crate) fn encode_all_open_orders() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestAllOpenOrders);
    message.push_field(&VERSION);

    Ok(message)
}

pub(crate) fn encode_auto_open_orders(auto_bind: bool) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestAutoOpenOrders);
    message.push_field(&VERSION);
    message.push_field(&auto_bind);

    Ok(message)
}

pub(crate) fn encode_executions(server_version: i32, request_id: i32, filter: &ExecutionFilter) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 3;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestExecutions);
    message.push_field(&VERSION);

    if server_version >= server_versions::EXECUTION_DATA_CHAIN {
        message.push_field(&request_id);
    }

    message.push_field(&filter.client_id);
    message.push_field(&filter.account_code);
    message.push_field(&filter.time); // "yyyyMMdd-HH:mm:ss" (UTC) or "yyyyMMdd HH:mm:ss timezone"
    message.push_field(&filter.symbol);
    message.push_field(&filter.security_type);
    message.push_field(&filter.exchange);
    message.push_field(&filter.side);

    Ok(message)
}

fn f64_max_to_zero(num: Option<f64>) -> Option<f64> {
    if num == Some(f64::MAX) {
        Some(0.0)
    } else {
        num
    }
}

fn message_version_for(server_version: i32) -> i32 {
    if server_version < server_versions::NOT_HELD {
        27
    } else {
        45
    }
}

#[cfg(test)]
mod tests;
