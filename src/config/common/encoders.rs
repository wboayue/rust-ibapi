//! Encoders for configuration request messages.

use crate::config::{ApiConfig, ApiPrecautions, ApiSettings, ConfigWarning, LockAndExit, MessageSetting, OrdersConfig, OrdersSmartRouting};
use crate::messages::{encode_protobuf_message, OutgoingMessages};
use crate::proto;
use crate::Error;
use prost::Message;

pub(in crate::config) fn encode_request_config(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, ConfigRequest, OutgoingMessages::ReqConfig)
}

/// Frame a fully-assembled `UpdateConfigRequest` proto onto the wire.
pub(in crate::config) fn encode_update_config(request: &proto::UpdateConfigRequest) -> Result<Vec<u8>, Error> {
    Ok(encode_protobuf_message(OutgoingMessages::UpdateConfig as i32, &request.encode_to_vec()))
}

// Domain → proto converters (mirror of the `convert_*` decoders). Each borrows
// the domain value so the builder can re-encode on connection-reset retries.

pub(in crate::config) fn to_proto_lock_and_exit(d: &LockAndExit) -> proto::LockAndExitConfig {
    proto::LockAndExitConfig {
        auto_logoff_time: d.auto_logoff_time.clone(),
        auto_logoff_period: d.auto_logoff_period.clone(),
        auto_logoff_type: d.auto_logoff_type.clone(),
    }
}

pub(in crate::config) fn to_proto_message(d: &MessageSetting) -> proto::MessageConfig {
    proto::MessageConfig {
        id: d.id,
        title: d.title.clone(),
        message: d.message.clone(),
        default_action: d.default_action.clone(),
        enabled: d.enabled,
    }
}

pub(in crate::config) fn to_proto_warning(d: &ConfigWarning) -> proto::UpdateConfigWarning {
    proto::UpdateConfigWarning {
        message_id: d.message_id,
        title: d.title.clone(),
        message: d.message.clone(),
    }
}

pub(in crate::config) fn to_proto_api(d: &ApiConfig) -> proto::ApiConfig {
    proto::ApiConfig {
        precautions: d.precautions.as_ref().map(to_proto_precautions),
        settings: d.settings.as_ref().map(to_proto_settings),
    }
}

fn to_proto_precautions(d: &ApiPrecautions) -> proto::ApiPrecautionsConfig {
    proto::ApiPrecautionsConfig {
        bypass_order_precautions: d.bypass_order_precautions,
        bypass_bond_warning: d.bypass_bond_warning,
        bypass_negative_yield_confirmation: d.bypass_negative_yield_confirmation,
        bypass_called_bond_warning: d.bypass_called_bond_warning,
        bypass_same_action_pair_trade_warning: d.bypass_same_action_pair_trade_warning,
        bypass_flagged_accounts_warning: d.bypass_flagged_accounts_warning,
        bypass_price_based_volatility_warning: d.bypass_price_based_volatility_warning,
        bypass_redirect_order_warning: d.bypass_redirect_order_warning,
        bypass_no_overfill_protection: d.bypass_no_overfill_protection,
        bypass_route_marketable_to_bbo: d.bypass_route_marketable_to_bbo,
    }
}

fn to_proto_settings(d: &ApiSettings) -> proto::ApiSettingsConfig {
    proto::ApiSettingsConfig {
        read_only_api: d.read_only_api,
        total_quantity_for_mutual_funds: d.total_quantity_for_mutual_funds,
        download_open_orders_on_connection: d.download_open_orders_on_connection,
        include_virtual_fx_positions: d.include_virtual_fx_positions,
        prepare_daily_pn_l: d.prepare_daily_pnl,
        send_status_updates_for_volatility_orders: d.send_status_updates_for_volatility_orders,
        encode_api_messages: d.encode_api_messages.clone(),
        socket_port: d.socket_port,
        use_negative_auto_range: d.use_negative_auto_range,
        create_api_message_log_file: d.create_api_message_log_file,
        include_market_data_in_log_file: d.include_market_data_in_log_file,
        expose_trading_schedule_to_api: d.expose_trading_schedule_to_api,
        split_insured_deposit_from_cash_balance: d.split_insured_deposit_from_cash_balance,
        send_zero_positions_for_today_only: d.send_zero_positions_for_today_only,
        let_api_account_requests_switch_subscription: d.let_api_account_requests_switch_subscription,
        use_account_groups_with_allocation_methods: d.use_account_groups_with_allocation_methods,
        logging_level: d.logging_level.clone(),
        master_client_id: d.master_client_id,
        bulk_data_timeout: d.bulk_data_timeout,
        component_exch_separator: d.component_exch_separator.clone(),
        show_forex_data_in1_10pips: d.show_forex_data_in_1_10_pips,
        allow_forex_trading_in1_10pips: d.allow_forex_trading_in_1_10_pips,
        round_account_values_to_nearest_whole_number: d.round_account_values_to_nearest_whole_number,
        send_market_data_in_lots_for_us_stocks: d.send_market_data_in_lots_for_us_stocks,
        show_advanced_order_reject_in_ui: d.show_advanced_order_reject_in_ui,
        reject_messages_above_max_rate: d.reject_messages_above_max_rate,
        maintain_connection_on_incorrect_fields: d.maintain_connection_on_incorrect_fields,
        compatibility_mode_nasdaq_stocks: d.compatibility_mode_nasdaq_stocks,
        send_instrument_timezone: d.send_instrument_timezone.clone(),
        send_forex_data_in_compatibility_mode: d.send_forex_data_in_compatibility_mode,
        maintain_and_resubmit_orders_on_reconnect: d.maintain_and_resubmit_orders_on_reconnect,
        historical_data_max_size: d.historical_data_max_size,
        auto_report_netting_event_contract_trades: d.auto_report_netting_event_contract_trades,
        option_exercise_request_type: d.option_exercise_request_type.clone(),
        allow_localhost_only: d.allow_localhost_only,
        trusted_i_ps: d.trusted_ips.clone(),
    }
}

pub(in crate::config) fn to_proto_orders(d: &OrdersConfig) -> proto::OrdersConfig {
    proto::OrdersConfig {
        smart_routing: d.smart_routing.as_ref().map(to_proto_smart_routing),
    }
}

fn to_proto_smart_routing(d: &OrdersSmartRouting) -> proto::OrdersSmartRoutingConfig {
    proto::OrdersSmartRoutingConfig {
        seek_price_improvement: d.seek_price_improvement,
        pre_open_reroute: d.pre_open_reroute,
        do_not_route_to_dark_pools: d.do_not_route_to_dark_pools,
        default_algorithm: d.default_algorithm.clone(),
    }
}

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;

// Request-config encoder body assertions live in the sync/async tests via
// `assert_request<B>(builder)`.
