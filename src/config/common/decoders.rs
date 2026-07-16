//! Decoders for configuration messages. Proto-only; text framing surfaces as
//! `Error::UnexpectedResponse` via `require_proto()`.

use prost::Message;

use crate::config::{ApiConfig, ApiPrecautions, ApiSettings, Config, LockAndExit, MessageSetting, OrdersConfig, OrdersSmartRouting};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::proto;
use crate::Error;

/// Dispatch on the incoming message type and forward to the typed decoder.
/// Routes `Error` frames into `Error::from` and any other variant into
/// `Error::UnexpectedResponse`.
pub(in crate::config) fn decode_config_message(message: &ResponseMessage) -> Result<Config, Error> {
    match message.message_type() {
        IncomingMessages::ConfigResponse => decode_config_proto(message.require_proto()?),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

fn decode_config_proto(bytes: &[u8]) -> Result<Config, Error> {
    let p = proto::ConfigResponse::decode(bytes)?;
    Ok(Config {
        lock_and_exit: p.lock_and_exit.map(convert_lock_and_exit),
        messages: p.messages.into_iter().map(convert_message).collect(),
        api: p.api.map(convert_api),
        orders: p.orders.map(convert_orders),
    })
}

fn convert_lock_and_exit(p: proto::LockAndExitConfig) -> LockAndExit {
    LockAndExit {
        auto_logoff_time: p.auto_logoff_time,
        auto_logoff_period: p.auto_logoff_period,
        auto_logoff_type: p.auto_logoff_type,
    }
}

fn convert_message(p: proto::MessageConfig) -> MessageSetting {
    MessageSetting {
        id: p.id,
        title: p.title,
        message: p.message,
        default_action: p.default_action,
        enabled: p.enabled,
    }
}

fn convert_api(p: proto::ApiConfig) -> ApiConfig {
    ApiConfig {
        precautions: p.precautions.map(convert_precautions),
        settings: p.settings.map(convert_settings),
    }
}

fn convert_precautions(p: proto::ApiPrecautionsConfig) -> ApiPrecautions {
    ApiPrecautions {
        bypass_order_precautions: p.bypass_order_precautions,
        bypass_bond_warning: p.bypass_bond_warning,
        bypass_negative_yield_confirmation: p.bypass_negative_yield_confirmation,
        bypass_called_bond_warning: p.bypass_called_bond_warning,
        bypass_same_action_pair_trade_warning: p.bypass_same_action_pair_trade_warning,
        bypass_flagged_accounts_warning: p.bypass_flagged_accounts_warning,
        bypass_price_based_volatility_warning: p.bypass_price_based_volatility_warning,
        bypass_redirect_order_warning: p.bypass_redirect_order_warning,
        bypass_no_overfill_protection: p.bypass_no_overfill_protection,
        bypass_route_marketable_to_bbo: p.bypass_route_marketable_to_bbo,
    }
}

fn convert_settings(p: proto::ApiSettingsConfig) -> ApiSettings {
    ApiSettings {
        read_only_api: p.read_only_api,
        total_quantity_for_mutual_funds: p.total_quantity_for_mutual_funds,
        download_open_orders_on_connection: p.download_open_orders_on_connection,
        include_virtual_fx_positions: p.include_virtual_fx_positions,
        prepare_daily_pnl: p.prepare_daily_pn_l,
        send_status_updates_for_volatility_orders: p.send_status_updates_for_volatility_orders,
        encode_api_messages: p.encode_api_messages,
        socket_port: p.socket_port,
        use_negative_auto_range: p.use_negative_auto_range,
        create_api_message_log_file: p.create_api_message_log_file,
        include_market_data_in_log_file: p.include_market_data_in_log_file,
        expose_trading_schedule_to_api: p.expose_trading_schedule_to_api,
        split_insured_deposit_from_cash_balance: p.split_insured_deposit_from_cash_balance,
        send_zero_positions_for_today_only: p.send_zero_positions_for_today_only,
        let_api_account_requests_switch_subscription: p.let_api_account_requests_switch_subscription,
        use_account_groups_with_allocation_methods: p.use_account_groups_with_allocation_methods,
        logging_level: p.logging_level,
        master_client_id: p.master_client_id,
        bulk_data_timeout: p.bulk_data_timeout,
        component_exch_separator: p.component_exch_separator,
        show_forex_data_in_1_10_pips: p.show_forex_data_in1_10pips,
        allow_forex_trading_in_1_10_pips: p.allow_forex_trading_in1_10pips,
        round_account_values_to_nearest_whole_number: p.round_account_values_to_nearest_whole_number,
        send_market_data_in_lots_for_us_stocks: p.send_market_data_in_lots_for_us_stocks,
        show_advanced_order_reject_in_ui: p.show_advanced_order_reject_in_ui,
        reject_messages_above_max_rate: p.reject_messages_above_max_rate,
        maintain_connection_on_incorrect_fields: p.maintain_connection_on_incorrect_fields,
        compatibility_mode_nasdaq_stocks: p.compatibility_mode_nasdaq_stocks,
        send_instrument_timezone: p.send_instrument_timezone,
        send_forex_data_in_compatibility_mode: p.send_forex_data_in_compatibility_mode,
        maintain_and_resubmit_orders_on_reconnect: p.maintain_and_resubmit_orders_on_reconnect,
        historical_data_max_size: p.historical_data_max_size,
        auto_report_netting_event_contract_trades: p.auto_report_netting_event_contract_trades,
        option_exercise_request_type: p.option_exercise_request_type,
        allow_localhost_only: p.allow_localhost_only,
        trusted_ips: p.trusted_i_ps,
    }
}

fn convert_orders(p: proto::OrdersConfig) -> OrdersConfig {
    OrdersConfig {
        smart_routing: p.smart_routing.map(convert_smart_routing),
    }
}

fn convert_smart_routing(p: proto::OrdersSmartRoutingConfig) -> OrdersSmartRouting {
    OrdersSmartRouting {
        seek_price_improvement: p.seek_price_improvement,
        pre_open_reroute: p.pre_open_reroute,
        do_not_route_to_dark_pools: p.do_not_route_to_dark_pools,
        default_algorithm: p.default_algorithm,
    }
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
