//! TWS/Gateway configuration snapshot.
//!
//! [`Client::config`](crate::Client::config) reads the API, precautions,
//! orders, and lock-and-exit settings the running gateway is configured with.
//! This is a read-only view; the write path (`updateConfig`) is not yet
//! implemented.
//!
//! Every field mirrors the wire message and is optional — a `None` means the
//! gateway did not report that setting, not that it is disabled.

use serde::{Deserialize, Serialize};

mod common;

// Re-export common functionality
use common::encoders;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// A snapshot of the TWS/Gateway configuration.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Lock-and-exit (auto-logoff) settings.
    pub lock_and_exit: Option<LockAndExit>,
    /// Configurable API message prompts and their default actions.
    pub messages: Vec<MessageSetting>,
    /// API-level configuration (precautions and settings).
    pub api: Option<ApiConfig>,
    /// Order-handling configuration.
    pub orders: Option<OrdersConfig>,
}

/// Auto-logoff / lock-and-exit configuration.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockAndExit {
    /// Time of day at which the gateway auto-logs off.
    pub auto_logoff_time: Option<String>,
    /// Auto-logoff period.
    pub auto_logoff_period: Option<String>,
    /// Auto-logoff type.
    pub auto_logoff_type: Option<String>,
}

/// A single configurable API message prompt.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSetting {
    /// Message identifier.
    pub id: Option<i32>,
    /// Message title.
    pub title: Option<String>,
    /// Message body.
    pub message: Option<String>,
    /// The default action taken for this prompt.
    pub default_action: Option<String>,
    /// Whether this prompt is enabled.
    pub enabled: Option<bool>,
}

/// API-level configuration.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Order-precaution bypass flags.
    pub precautions: Option<ApiPrecautions>,
    /// General API settings.
    pub settings: Option<ApiSettings>,
}

/// Order-precaution bypass flags. Each `Some(true)` means the corresponding
/// safety confirmation is bypassed.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiPrecautions {
    /// Bypass order precautions.
    pub bypass_order_precautions: Option<bool>,
    /// Bypass bond warning.
    pub bypass_bond_warning: Option<bool>,
    /// Bypass negative-yield confirmation.
    pub bypass_negative_yield_confirmation: Option<bool>,
    /// Bypass called-bond warning.
    pub bypass_called_bond_warning: Option<bool>,
    /// Bypass same-action pair-trade warning.
    pub bypass_same_action_pair_trade_warning: Option<bool>,
    /// Bypass flagged-accounts warning.
    pub bypass_flagged_accounts_warning: Option<bool>,
    /// Bypass price-based volatility warning.
    pub bypass_price_based_volatility_warning: Option<bool>,
    /// Bypass redirect-order warning.
    pub bypass_redirect_order_warning: Option<bool>,
    /// Bypass no-overfill-protection warning.
    pub bypass_no_overfill_protection: Option<bool>,
    /// Bypass route-marketable-to-BBO warning.
    pub bypass_route_marketable_to_bbo: Option<bool>,
}

/// General API settings reported by the gateway.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSettings {
    /// Read-only API mode.
    pub read_only_api: Option<bool>,
    /// Report total quantity for mutual funds.
    pub total_quantity_for_mutual_funds: Option<bool>,
    /// Download open orders on connection.
    pub download_open_orders_on_connection: Option<bool>,
    /// Include virtual FX positions.
    pub include_virtual_fx_positions: Option<bool>,
    /// Prepare daily PnL.
    pub prepare_daily_pnl: Option<bool>,
    /// Send status updates for volatility orders.
    pub send_status_updates_for_volatility_orders: Option<bool>,
    /// API message encoding.
    pub encode_api_messages: Option<String>,
    /// Socket port the gateway listens on.
    pub socket_port: Option<i32>,
    /// Use negative auto-range.
    pub use_negative_auto_range: Option<bool>,
    /// Create an API message log file.
    pub create_api_message_log_file: Option<bool>,
    /// Include market data in the log file.
    pub include_market_data_in_log_file: Option<bool>,
    /// Expose the trading schedule to the API.
    pub expose_trading_schedule_to_api: Option<bool>,
    /// Split insured deposit from cash balance.
    pub split_insured_deposit_from_cash_balance: Option<bool>,
    /// Send zero positions for today only.
    pub send_zero_positions_for_today_only: Option<bool>,
    /// Let API account requests switch subscription.
    pub let_api_account_requests_switch_subscription: Option<bool>,
    /// Use account groups with allocation methods.
    pub use_account_groups_with_allocation_methods: Option<bool>,
    /// Logging level.
    pub logging_level: Option<String>,
    /// Master client id.
    pub master_client_id: Option<i32>,
    /// Bulk data timeout.
    pub bulk_data_timeout: Option<i32>,
    /// Component-exchange separator.
    pub component_exch_separator: Option<String>,
    /// Show forex data in 1/10 pips.
    pub show_forex_data_in_1_10_pips: Option<bool>,
    /// Allow forex trading in 1/10 pips.
    pub allow_forex_trading_in_1_10_pips: Option<bool>,
    /// Round account values to the nearest whole number.
    pub round_account_values_to_nearest_whole_number: Option<bool>,
    /// Send market data in lots for US stocks.
    pub send_market_data_in_lots_for_us_stocks: Option<bool>,
    /// Show advanced order reject in UI.
    pub show_advanced_order_reject_in_ui: Option<bool>,
    /// Reject messages above max rate.
    pub reject_messages_above_max_rate: Option<bool>,
    /// Maintain connection on incorrect fields.
    pub maintain_connection_on_incorrect_fields: Option<bool>,
    /// Compatibility mode for NASDAQ stocks.
    pub compatibility_mode_nasdaq_stocks: Option<bool>,
    /// Send instrument timezone.
    pub send_instrument_timezone: Option<String>,
    /// Send forex data in compatibility mode.
    pub send_forex_data_in_compatibility_mode: Option<bool>,
    /// Maintain and resubmit orders on reconnect.
    pub maintain_and_resubmit_orders_on_reconnect: Option<bool>,
    /// Historical data max size.
    pub historical_data_max_size: Option<i32>,
    /// Auto-report netting-event contract trades.
    pub auto_report_netting_event_contract_trades: Option<bool>,
    /// Option-exercise request type.
    pub option_exercise_request_type: Option<String>,
    /// Allow localhost connections only.
    pub allow_localhost_only: Option<bool>,
    /// Trusted IP addresses.
    pub trusted_ips: Vec<String>,
}

/// Order-handling configuration.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrdersConfig {
    /// Smart-routing configuration.
    pub smart_routing: Option<OrdersSmartRouting>,
}

/// Smart-routing configuration.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrdersSmartRouting {
    /// Seek price improvement.
    pub seek_price_improvement: Option<bool>,
    /// Pre-open reroute.
    pub pre_open_reroute: Option<bool>,
    /// Do not route to dark pools.
    pub do_not_route_to_dark_pools: Option<bool>,
    /// Default algorithm.
    pub default_algorithm: Option<String>,
}
