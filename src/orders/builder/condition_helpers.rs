//! Helper functions for creating order conditions with a fluent API.
//!
//! This module provides ergonomic helper functions that return partially-built
//! condition builders, enabling a fluent API for adding conditions to orders.
//!
//! # Example
//!
//! ```no_run
//! use ibapi::orders::builder::{price, time, margin};
//!
//! // Create conditions using helper functions
//! let price_cond = price(265598, "SMART").greater_than(150.0);
//! let time_cond = time().greater_than("20251230 14:30:00 US/Eastern");
//! let margin_cond = margin().less_than(30);
//! ```

use crate::orders::conditions::*;
use crate::orders::OrderCondition;

/// Create a price condition builder.
///
/// # Parameters
///
/// - `contract_id`: Contract identifier for the instrument to monitor
/// - `exchange`: Exchange where the price is monitored
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::price;
///
/// let condition = price(265598, "SMART")
///     .greater_than(150.0)
///     .build();
/// ```
pub fn price(contract_id: impl Into<i32>, exchange: impl Into<String>) -> PriceConditionBuilder {
    PriceCondition::builder(contract_id.into(), exchange)
}

/// Create a time condition builder.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::time;
///
/// let condition = time()
///     .greater_than("20251230 14:30:00 US/Eastern")
///     .build();
/// ```
pub fn time() -> TimeConditionBuilder {
    TimeCondition::builder()
}

/// Create a margin condition builder.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::margin;
///
/// let condition = margin()
///     .less_than(30)
///     .build();
/// ```
pub fn margin() -> MarginConditionBuilder {
    MarginCondition::builder()
}

/// Create a volume condition builder.
///
/// # Parameters
///
/// - `contract_id`: Contract identifier for the instrument to monitor
/// - `exchange`: Exchange where volume is monitored
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::volume;
///
/// let condition = volume(76792991, "SMART")
///     .greater_than(50_000_000)
///     .build();
/// ```
pub fn volume(contract_id: i32, exchange: impl Into<String>) -> VolumeConditionBuilder {
    VolumeCondition::builder(contract_id, exchange)
}

/// Create an execution condition directly.
///
/// Unlike other condition types, execution conditions don't have a threshold,
/// so this function returns an `OrderCondition` directly rather than a builder.
///
/// # Parameters
///
/// - `symbol`: Symbol of the contract
/// - `security_type`: Security type (e.g., "STK", "OPT")
/// - `exchange`: Exchange where execution is monitored
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::execution;
///
/// let condition = execution("MSFT", "STK", "SMART");
/// ```
pub fn execution(symbol: impl Into<String>, security_type: impl Into<String>, exchange: impl Into<String>) -> OrderCondition {
    OrderCondition::Execution(ExecutionCondition {
        symbol: symbol.into(),
        security_type: security_type.into(),
        exchange: exchange.into(),
        is_conjunction: true,
    })
}

/// Create a percent change condition builder.
///
/// # Parameters
///
/// - `contract_id`: Contract identifier for the instrument to monitor
/// - `exchange`: Exchange where price change is monitored
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::percent_change;
///
/// let condition = percent_change(756733, "SMART")
///     .greater_than(2.0)
///     .build();
/// ```
pub fn percent_change(contract_id: i32, exchange: impl Into<String>) -> PercentChangeConditionBuilder {
    PercentChangeCondition::builder(contract_id, exchange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_helper() {
        let condition = price(12345, "NASDAQ").greater_than(150.0).build();
        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "NASDAQ");
        assert_eq!(condition.price, 150.0);
        assert!(condition.is_more);
    }

    #[test]
    fn test_time_helper() {
        let condition = time().greater_than("20251230 14:30:00 UTC").build();
        assert_eq!(condition.time, "20251230 14:30:00 UTC");
        assert!(condition.is_more);
    }

    #[test]
    fn test_margin_helper() {
        let condition = margin().less_than(30).build();
        assert_eq!(condition.percent, 30);
        assert!(!condition.is_more);
    }

    #[test]
    fn test_volume_helper() {
        let condition = volume(12345, "SMART").greater_than(1000000).build();
        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "SMART");
        assert_eq!(condition.volume, 1000000);
        assert!(condition.is_more);
    }

    #[test]
    fn test_execution_helper() {
        let condition = execution("AAPL", "STK", "NASDAQ");
        match condition {
            OrderCondition::Execution(exec) => {
                assert_eq!(exec.symbol, "AAPL");
                assert_eq!(exec.security_type, "STK");
                assert_eq!(exec.exchange, "NASDAQ");
                assert!(exec.is_conjunction);
            }
            _ => panic!("Expected Execution condition"),
        }
    }

    #[test]
    fn test_percent_change_helper() {
        let condition = percent_change(12345, "SMART").greater_than(5.0).build();
        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "SMART");
        assert_eq!(condition.percent, 5.0);
        assert!(condition.is_more);
    }
}
