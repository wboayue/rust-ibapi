//! Protocol version checking and constants for TWS API features.
//!
//! This module provides utilities for checking server version compatibility
//! and centralizes all version-related constants used throughout the library.

use crate::errors::Error;
use crate::server_versions;

/// Represents a protocol feature that requires a minimum server version.
#[derive(Debug, Clone, Copy)]
pub struct ProtocolFeature {
    /// The name of the feature for error messages
    pub name: &'static str,
    /// The minimum server version required
    pub min_version: i32,
}

impl ProtocolFeature {
    /// Creates a new protocol feature definition
    pub const fn new(name: &'static str, min_version: i32) -> Self {
        Self { name, min_version }
    }
}

/// Common protocol features used throughout the API
pub struct Features;

impl Features {
    // Account features
    pub const POSITIONS: ProtocolFeature = ProtocolFeature::new("positions", server_versions::POSITIONS);
    pub const ACCOUNT_SUMMARY: ProtocolFeature = ProtocolFeature::new("account summary", server_versions::ACCOUNT_SUMMARY);
    pub const FAMILY_CODES: ProtocolFeature = ProtocolFeature::new("family codes", server_versions::REQ_FAMILY_CODES);
    pub const PNL: ProtocolFeature = ProtocolFeature::new("profit and loss", server_versions::PNL);
    pub const UNREALIZED_PNL: ProtocolFeature = ProtocolFeature::new("unrealized PnL", server_versions::UNREALIZED_PNL);
    pub const REALIZED_PNL: ProtocolFeature = ProtocolFeature::new("realized PnL", server_versions::REALIZED_PNL);
    pub const MODELS_SUPPORT: ProtocolFeature = ProtocolFeature::new("models support", server_versions::MODELS_SUPPORT);

    // Market data features
    pub const REAL_TIME_BARS: ProtocolFeature = ProtocolFeature::new("real-time bars", server_versions::REAL_TIME_BARS);
    pub const MARKET_DATA_TYPE: ProtocolFeature = ProtocolFeature::new("market data type", server_versions::REQ_MARKET_DATA_TYPE);
    pub const TICK_BY_TICK: ProtocolFeature = ProtocolFeature::new("tick-by-tick data", server_versions::TICK_BY_TICK);
    pub const TICK_BY_TICK_IGNORE_SIZE: ProtocolFeature =
        ProtocolFeature::new("tick-by-tick ignore size parameter", server_versions::TICK_BY_TICK_IGNORE_SIZE);
    pub const HISTOGRAM: ProtocolFeature = ProtocolFeature::new("histogram data", server_versions::REQ_HISTOGRAM);
    pub const HISTORICAL_TICKS: ProtocolFeature = ProtocolFeature::new("historical ticks", server_versions::HISTORICAL_TICKS);
    pub const HEAD_TIMESTAMP: ProtocolFeature = ProtocolFeature::new("head timestamp", server_versions::REQ_HEAD_TIMESTAMP);
    pub const SYNT_REALTIME_BARS: ProtocolFeature = ProtocolFeature::new("synthetic real-time bars", server_versions::SYNT_REALTIME_BARS);
    pub const HISTORICAL_SCHEDULE: ProtocolFeature = ProtocolFeature::new("historical schedule", server_versions::HISTORICAL_SCHEDULE);
    pub const SMART_DEPTH: ProtocolFeature = ProtocolFeature::new("SMART depth", server_versions::SMART_DEPTH);
    pub const MKT_DEPTH_PRIM_EXCHANGE: ProtocolFeature =
        ProtocolFeature::new("market depth primary exchange", server_versions::MKT_DEPTH_PRIM_EXCHANGE);
    pub const REQ_MKT_DEPTH_EXCHANGES: ProtocolFeature = ProtocolFeature::new("market depth exchanges", server_versions::REQ_MKT_DEPTH_EXCHANGES);

    // Order features
    pub const WHAT_IF_ORDERS: ProtocolFeature = ProtocolFeature::new("what-if orders", server_versions::WHAT_IF_ORDERS);
    pub const ORDER_CONTAINER: ProtocolFeature = ProtocolFeature::new("order container", server_versions::ORDER_CONTAINER);
    pub const AUTO_CANCEL_PARENT: ProtocolFeature = ProtocolFeature::new("auto cancel parent", server_versions::AUTO_CANCEL_PARENT);
    pub const FRACTIONAL_SIZE_SUPPORT: ProtocolFeature = ProtocolFeature::new("fractional size support", server_versions::FRACTIONAL_SIZE_SUPPORT);
    pub const CASH_QTY: ProtocolFeature = ProtocolFeature::new("cash quantity", server_versions::CASH_QTY);
    pub const DECISION_MAKER: ProtocolFeature = ProtocolFeature::new("decision maker", server_versions::DECISION_MAKER);
    pub const MIFID_EXECUTION: ProtocolFeature = ProtocolFeature::new("MiFID execution", server_versions::MIFID_EXECUTION);
    pub const MANUAL_ORDER_TIME: ProtocolFeature = ProtocolFeature::new("manual order time", server_versions::MANUAL_ORDER_TIME);
    pub const COMPLETED_ORDERS: ProtocolFeature = ProtocolFeature::new("completed orders", server_versions::COMPLETED_ORDERS);
    pub const REQ_GLOBAL_CANCEL: ProtocolFeature = ProtocolFeature::new("global cancel", server_versions::REQ_GLOBAL_CANCEL);

    // Contract features
    pub const TRADING_CLASS: ProtocolFeature = ProtocolFeature::new("trading class", server_versions::TRADING_CLASS);
    pub const SIZE_RULES: ProtocolFeature = ProtocolFeature::new("size rules", server_versions::SIZE_RULES);
    pub const BOND_ISSUERID: ProtocolFeature = ProtocolFeature::new("bond issuer ID", server_versions::BOND_ISSUERID);
    pub const SEC_ID_TYPE: ProtocolFeature = ProtocolFeature::new("security ID type", server_versions::SEC_ID_TYPE);
    pub const SMART_COMPONENTS: ProtocolFeature = ProtocolFeature::new("smart components", server_versions::REQ_SMART_COMPONENTS);
    pub const LINKING: ProtocolFeature = ProtocolFeature::new("linking", server_versions::LINKING);

    // News features
    pub const NEWS_PROVIDERS: ProtocolFeature = ProtocolFeature::new("news providers", server_versions::REQ_NEWS_PROVIDERS);
    pub const NEWS_ARTICLE: ProtocolFeature = ProtocolFeature::new("news article", server_versions::REQ_NEWS_ARTICLE);
    pub const HISTORICAL_NEWS: ProtocolFeature = ProtocolFeature::new("historical news", server_versions::REQ_HISTORICAL_NEWS);
    pub const NEWS_QUERY_ORIGINS: ProtocolFeature = ProtocolFeature::new("news query origins", server_versions::NEWS_QUERY_ORIGINS);

    // Scanner features
    pub const SCANNER_GENERIC_OPTS: ProtocolFeature = ProtocolFeature::new("scanner generic options", server_versions::SCANNER_GENERIC_OPTS);

    // WSH features
    pub const WSHE_CALENDAR: ProtocolFeature = ProtocolFeature::new("WSHE Calendar", server_versions::WSHE_CALENDAR);
    pub const WSH_EVENT_DATA_FILTERS: ProtocolFeature = ProtocolFeature::new("WSH event data filters", server_versions::WSH_EVENT_DATA_FILTERS);
    pub const WSH_EVENT_DATA_FILTERS_DATE: ProtocolFeature =
        ProtocolFeature::new("WSH event data filters with date", server_versions::WSH_EVENT_DATA_FILTERS_DATE);

    // Other features
    pub const FA_PROFILE_DESUPPORT: ProtocolFeature = ProtocolFeature::new("FA profile desupport", server_versions::FA_PROFILE_DESUPPORT);
    pub const MARKET_RULES: ProtocolFeature = ProtocolFeature::new("market rules", server_versions::MARKET_RULES);
    pub const REQ_MATCHING_SYMBOLS: ProtocolFeature = ProtocolFeature::new("matching symbols", server_versions::REQ_MATCHING_SYMBOLS);
    pub const REQ_CALC_IMPLIED_VOLAT: ProtocolFeature = ProtocolFeature::new("calculate implied volatility", server_versions::REQ_CALC_IMPLIED_VOLAT);
    pub const REQ_CALC_OPTION_PRICE: ProtocolFeature = ProtocolFeature::new("calculate option price", server_versions::REQ_CALC_OPTION_PRICE);
    pub const SEC_DEF_OPT_PARAMS_REQ: ProtocolFeature =
        ProtocolFeature::new("security definition option parameters", server_versions::SEC_DEF_OPT_PARAMS_REQ);
}

/// Checks if the server version supports a given feature.
///
/// # Arguments
/// * `server_version` - The connected server's version
/// * `feature` - The protocol feature to check
///
/// # Returns
/// * `Ok(())` if the feature is supported
/// * `Err(Error)` if the server version is too old
///
/// # Example
/// ```no_run
/// use ibapi::protocol::{check_version, Features};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let server_version = 156;
/// check_version(server_version, Features::TICK_BY_TICK)?;
/// # Ok(())
/// # }
/// ```
pub fn check_version(server_version: i32, feature: ProtocolFeature) -> Result<(), Error> {
    if server_version < feature.min_version {
        Err(Error::ServerVersion(server_version, feature.min_version, feature.name.to_string()))
    } else {
        Ok(())
    }
}

/// Checks if a feature is supported without returning an error.
///
/// # Arguments
/// * `server_version` - The connected server's version
/// * `feature` - The protocol feature to check
///
/// # Returns
/// * `true` if the feature is supported
/// * `false` if the server version is too old
pub fn is_supported(server_version: i32, feature: ProtocolFeature) -> bool {
    server_version >= feature.min_version
}

/// Helper function to conditionally include fields based on server version.
///
/// This is useful when encoding messages that have optional fields depending
/// on the server version.
///
/// # Arguments
/// * `server_version` - The connected server's version
/// * `feature` - The protocol feature that enables the field
/// * `include_fn` - A closure that adds the field(s) to the message
///
/// # Example
/// ```no_run
/// use ibapi::protocol::{include_if_supported, Features};
///
/// let server_version = 156;
/// let cash_qty = 1000.0;
///
/// include_if_supported(server_version, Features::CASH_QTY, || {
///     // Add cash_qty field to message only if server supports it
///     println!("Server supports cash quantity: {cash_qty:?}");
/// });
/// ```
pub fn include_if_supported<F>(server_version: i32, feature: ProtocolFeature, include_fn: F)
where
    F: FnOnce(),
{
    if is_supported(server_version, feature) {
        include_fn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_version_supported() {
        let result = check_version(150, Features::POSITIONS);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_version_unsupported() {
        let result = check_version(50, Features::TICK_BY_TICK);
        assert!(result.is_err());
        match result {
            Err(Error::ServerVersion(server, required, feature)) => {
                assert_eq!(server, 50);
                assert_eq!(required, 137);
                assert_eq!(feature, "tick-by-tick data");
            }
            _ => panic!("Expected ServerVersion error"),
        }
    }

    #[test]
    fn test_is_supported() {
        assert!(is_supported(150, Features::POSITIONS));
        assert!(!is_supported(50, Features::TICK_BY_TICK));
    }

    #[test]
    fn test_include_if_supported() {
        let mut called = false;
        include_if_supported(150, Features::POSITIONS, || {
            called = true;
        });
        assert!(called);

        let mut called = false;
        include_if_supported(50, Features::TICK_BY_TICK, || {
            called = true;
        });
        assert!(!called);
    }
}
