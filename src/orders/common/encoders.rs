use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::orders::{ExecutionFilter, ExerciseAction, Order, OrderCondition, COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID};
use crate::{server_versions, Error};

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
                encode_condition(&mut message, condition);
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
        if contract.exchange.as_str() == "IBKRATS" {
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

    message.push_field(&OutgoingMessages::RequestCompletedOrders);
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

    if server_version >= server_versions::PARAMETRIZED_DAYS_OF_EXECUTIONS {
        message.push_field(&filter.last_n_days);
        message.push_field(&(filter.specific_dates.len() as i32));
        for date in &filter.specific_dates {
            message.push_field(date);
        }
    }

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

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_exercise_options(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 2;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::ExerciseOptions);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&contract.contract_id);
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    message.push_field(&contract.trading_class);
    message.push_field(&(exercise_action as i32));
    message.push_field(&exercise_quantity);
    message.push_field(&account);
    message.push_field(&ovrd);

    if server_version >= server_versions::MANUAL_ORDER_TIME {
        message.push_field(&manual_order_time);
    }

    Ok(message)
}

/// Encodes a single order condition according to the TWS API protocol.
///
/// Each condition is encoded as:
/// - condition_type (i32): Type discriminator (1=Price, 3=Time, etc.)
/// - is_conjunction (bool): Whether this is an AND condition (true) or OR (false)
/// - condition-specific fields...
pub(crate) fn encode_condition(message: &mut RequestMessage, condition: &OrderCondition) {
    message.push_field(&condition.condition_type());
    message.push_field(&condition.is_conjunction());

    match condition {
        OrderCondition::Price(c) => encode_price_condition(message, c),
        OrderCondition::Time(c) => encode_time_condition(message, c),
        OrderCondition::Margin(c) => encode_margin_condition(message, c),
        OrderCondition::Execution(c) => encode_execution_condition(message, c),
        OrderCondition::Volume(c) => encode_volume_condition(message, c),
        OrderCondition::PercentChange(c) => encode_percent_change_condition(message, c),
    }
}

/// Encodes a PriceCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. price (f64)
/// 5. trigger_method (i32)
fn encode_price_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::PriceCondition) {
    message.push_field(&condition.contract_id);
    message.push_field(&condition.exchange);
    message.push_field(&condition.is_more);
    message.push_field(&condition.price);
    message.push_field(&condition.trigger_method);
}

/// Encodes a TimeCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. is_more (bool)
/// 2. time (String in format "YYYYMMDD HH:MM:SS TZ")
fn encode_time_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::TimeCondition) {
    message.push_field(&condition.is_more);
    message.push_field(&condition.time);
}

/// Encodes a MarginCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. is_more (bool)
/// 2. percent (i32)
fn encode_margin_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::MarginCondition) {
    message.push_field(&condition.is_more);
    message.push_field(&condition.percent);
}

/// Encodes an ExecutionCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. symbol (String)
/// 2. security_type (String)
/// 3. exchange (String)
fn encode_execution_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::ExecutionCondition) {
    message.push_field(&condition.symbol);
    message.push_field(&condition.security_type);
    message.push_field(&condition.exchange);
}

/// Encodes a VolumeCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. volume (i32)
fn encode_volume_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::VolumeCondition) {
    message.push_field(&condition.contract_id);
    message.push_field(&condition.exchange);
    message.push_field(&condition.is_more);
    message.push_field(&condition.volume);
}

/// Encodes a PercentChangeCondition according to the TWS API protocol.
///
/// Fields (in order):
/// 1. contract_id (i32)
/// 2. exchange (String)
/// 3. is_more (bool)
/// 4. percent (f64)
fn encode_percent_change_condition(message: &mut RequestMessage, condition: &crate::orders::conditions::PercentChangeCondition) {
    message.push_field(&condition.contract_id);
    message.push_field(&condition.exchange);
    message.push_field(&condition.is_more);
    message.push_field(&condition.percent);
}

#[cfg(test)]
pub(crate) mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn message_version_for() {
        assert_eq!(super::message_version_for(server_versions::NOT_HELD), 45);
        assert_eq!(super::message_version_for(server_versions::EXECUTION_DATA_CHAIN), 27);
    }

    #[test]
    fn f64_max_to_zero() {
        assert_eq!(super::f64_max_to_zero(Some(f64::MAX)), Some(0.0));
        assert_eq!(super::f64_max_to_zero(Some(0.0)), Some(0.0));
        assert_eq!(super::f64_max_to_zero(Some(50.0)), Some(50.0));
    }

    #[test]
    fn test_encode_price_condition() {
        use crate::orders::conditions::{PriceCondition, TriggerMethod};
        use crate::orders::OrderCondition;

        let condition = OrderCondition::Price(PriceCondition {
            contract_id: 12345,
            exchange: "NASDAQ".to_string(),
            price: 150.0,
            trigger_method: TriggerMethod::DoubleBidAsk,
            is_more: true,
            is_conjunction: false,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        // Verify the encoded fields match the TWS protocol
        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "1"); // condition_type (Price)
        assert_eq!(field_vec[1], "0"); // is_conjunction (false)
        assert_eq!(field_vec[2], "12345"); // contract_id
        assert_eq!(field_vec[3], "NASDAQ"); // exchange
        assert_eq!(field_vec[4], "1"); // is_more (true)
        assert_eq!(field_vec[5], "150"); // price
        assert_eq!(field_vec[6], "1"); // trigger_method
    }

    #[test]
    fn test_encode_time_condition() {
        use crate::orders::conditions::TimeCondition;
        use crate::orders::OrderCondition;

        let condition = OrderCondition::Time(TimeCondition {
            time: "20251230 23:59:59 UTC".to_string(),
            is_more: true,
            is_conjunction: true,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "3"); // condition_type (Time)
        assert_eq!(field_vec[1], "1"); // is_conjunction (true)
        assert_eq!(field_vec[2], "1"); // is_more (true)
        assert_eq!(field_vec[3], "20251230 23:59:59 UTC"); // time
    }

    #[test]
    fn test_encode_margin_condition() {
        use crate::orders::conditions::MarginCondition;
        use crate::orders::OrderCondition;

        let condition = OrderCondition::Margin(MarginCondition {
            percent: 30,
            is_more: false,
            is_conjunction: true,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "4"); // condition_type (Margin)
        assert_eq!(field_vec[1], "1"); // is_conjunction (true)
        assert_eq!(field_vec[2], "0"); // is_more (false)
        assert_eq!(field_vec[3], "30"); // percent
    }

    #[test]
    fn test_encode_execution_condition() {
        use crate::orders::conditions::ExecutionCondition;
        use crate::orders::OrderCondition;

        let condition = OrderCondition::Execution(ExecutionCondition {
            symbol: "AAPL".to_string(),
            security_type: "STK".to_string(),
            exchange: "SMART".to_string(),
            is_conjunction: false,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "5"); // condition_type (Execution)
        assert_eq!(field_vec[1], "0"); // is_conjunction (false)
        assert_eq!(field_vec[2], "AAPL"); // symbol
        assert_eq!(field_vec[3], "STK"); // security_type
        assert_eq!(field_vec[4], "SMART"); // exchange
    }

    #[test]
    fn test_encode_volume_condition() {
        use crate::orders::conditions::VolumeCondition;
        use crate::orders::OrderCondition;

        let condition = OrderCondition::Volume(VolumeCondition {
            contract_id: 54321,
            exchange: "NYSE".to_string(),
            volume: 1000000,
            is_more: true,
            is_conjunction: true,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "6"); // condition_type (Volume)
        assert_eq!(field_vec[1], "1"); // is_conjunction (true)
        assert_eq!(field_vec[2], "54321"); // contract_id
        assert_eq!(field_vec[3], "NYSE"); // exchange
        assert_eq!(field_vec[4], "1"); // is_more (true)
        assert_eq!(field_vec[5], "1000000"); // volume
    }

    #[test]
    fn test_encode_percent_change_condition() {
        use crate::orders::conditions::PercentChangeCondition;
        use crate::orders::OrderCondition;

        let condition = OrderCondition::PercentChange(PercentChangeCondition {
            contract_id: 98765,
            exchange: "NASDAQ".to_string(),
            percent: 5.5,
            is_more: false,
            is_conjunction: false,
        });

        let mut message = RequestMessage::default();
        encode_condition(&mut message, &condition);

        let fields = message.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "7"); // condition_type (PercentChange)
        assert_eq!(field_vec[1], "0"); // is_conjunction (false)
        assert_eq!(field_vec[2], "98765"); // contract_id
        assert_eq!(field_vec[3], "NASDAQ"); // exchange
        assert_eq!(field_vec[4], "0"); // is_more (false)
        assert_eq!(field_vec[5], "5.5"); // percent
    }

    #[test]
    fn test_encode_executions_without_date_filter() {
        let filter = ExecutionFilter {
            client_id: Some(1),
            account_code: "DU123456".to_string(),
            time: "20260101 09:30:00".to_string(),
            symbol: "AAPL".to_string(),
            security_type: "STK".to_string(),
            exchange: "SMART".to_string(),
            side: "BUY".to_string(),
            ..Default::default()
        };

        // Version below PARAMETRIZED_DAYS_OF_EXECUTIONS should not include date fields
        let result = encode_executions(server_versions::WSH_EVENT_DATA_FILTERS_DATE, 9000, &filter).unwrap();
        let fields = result.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "7"); // RequestExecutions
        assert_eq!(field_vec[1], "3"); // VERSION
        assert_eq!(field_vec[2], "9000"); // request_id
        assert_eq!(field_vec[3], "1"); // client_id
        assert_eq!(field_vec[4], "DU123456"); // account_code
        assert_eq!(field_vec[5], "20260101 09:30:00"); // time
        assert_eq!(field_vec[6], "AAPL"); // symbol
        assert_eq!(field_vec[7], "STK"); // security_type
        assert_eq!(field_vec[8], "SMART"); // exchange
        assert_eq!(field_vec[9], "BUY"); // side
        assert_eq!(field_vec.len(), 11); // 10 fields + trailing empty
    }

    #[test]
    fn test_encode_executions_with_date_filter() {
        let filter = ExecutionFilter {
            client_id: Some(1),
            account_code: "DU123456".to_string(),
            time: "".to_string(),
            symbol: "".to_string(),
            security_type: "".to_string(),
            exchange: "".to_string(),
            side: "".to_string(),
            last_n_days: 7,
            specific_dates: vec!["20260125".to_string(), "20260126".to_string()],
        };

        // Version at PARAMETRIZED_DAYS_OF_EXECUTIONS should include date fields
        let result = encode_executions(server_versions::PARAMETRIZED_DAYS_OF_EXECUTIONS, 9000, &filter).unwrap();
        let fields = result.encode();
        let field_vec: Vec<&str> = fields.split('\0').collect();

        assert_eq!(field_vec[0], "7"); // RequestExecutions
        assert_eq!(field_vec[1], "3"); // VERSION
        assert_eq!(field_vec[2], "9000"); // request_id
        assert_eq!(field_vec[3], "1"); // client_id
        assert_eq!(field_vec[4], "DU123456"); // account_code
        assert_eq!(field_vec[5], ""); // time
        assert_eq!(field_vec[6], ""); // symbol
        assert_eq!(field_vec[7], ""); // security_type
        assert_eq!(field_vec[8], ""); // exchange
        assert_eq!(field_vec[9], ""); // side
        assert_eq!(field_vec[10], "7"); // last_n_days
        assert_eq!(field_vec[11], "2"); // specific_dates count
        assert_eq!(field_vec[12], "20260125"); // specific_dates[0]
        assert_eq!(field_vec[13], "20260126"); // specific_dates[1]
        assert_eq!(field_vec.len(), 15); // 14 fields + trailing empty
    }
}
