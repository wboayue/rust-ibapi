//! Order condition detail structures for encoding condition-specific parameters.
//!
//! According to the IB API, after each condition type is encoded, we need to encode
//! condition-specific fields. This module provides structures to represent these details.

use serde::{Deserialize, Serialize};

/// Details for a price condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriceConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// Whether the condition triggers when price is greater than threshold.
    pub is_more: bool,
    /// The price threshold.
    pub price: f64,
    /// The contract ID for the instrument to monitor.
    pub contract_id: i32,
    /// The exchange where the contract trades.
    pub exchange: String,
    /// The trigger method (0=default, 1=double bid/ask, 2=last, etc.).
    pub trigger_method: i32,
}

/// Details for a time condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// Whether the condition triggers when time is greater than threshold.
    pub is_more: bool,
    /// The time threshold (format: YYYYMMDD-HH:MM:SS or YYYYMMDD HH:MM:SS TZ).
    pub time: String,
}

/// Details for a margin condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// Whether the condition triggers when margin is greater than threshold.
    pub is_more: bool,
    /// The margin cushion percentage threshold.
    pub percent: i32,
}

/// Details for an execution condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// The security type (STK, OPT, etc.).
    pub security_type: String,
    /// The exchange where the contract trades.
    pub exchange: String,
    /// The symbol to monitor for executions.
    pub symbol: String,
}

/// Details for a volume condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// Whether the condition triggers when volume is greater than threshold.
    pub is_more: bool,
    /// The volume threshold.
    pub volume: i32,
    /// The contract ID for the instrument to monitor.
    pub contract_id: i32,
    /// The exchange where the contract trades.
    pub exchange: String,
}

/// Details for a percent change condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PercentChangeConditionDetails {
    /// Whether this condition uses AND (true) or OR (false) conjunction.
    pub is_conjunction: bool,
    /// Whether the condition triggers when change is greater than threshold.
    pub is_more: bool,
    /// The percent change threshold.
    pub change_percent: f64,
    /// The contract ID for the instrument to monitor.
    pub contract_id: i32,
    /// The exchange where the contract trades.
    pub exchange: String,
}

/// Enum representing condition details for different condition types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConditionDetails {
    /// Price condition details.
    Price(PriceConditionDetails),
    /// Time condition details.
    Time(TimeConditionDetails),
    /// Margin condition details.
    Margin(MarginConditionDetails),
    /// Execution condition details.
    Execution(ExecutionConditionDetails),
    /// Volume condition details.
    Volume(VolumeConditionDetails),
    /// Percent change condition details.
    PercentChange(PercentChangeConditionDetails),
}

impl ConditionDetails {
    /// Get the conjunction flag from the condition details.
    pub fn is_conjunction(&self) -> bool {
        match self {
            ConditionDetails::Price(d) => d.is_conjunction,
            ConditionDetails::Time(d) => d.is_conjunction,
            ConditionDetails::Margin(d) => d.is_conjunction,
            ConditionDetails::Execution(d) => d.is_conjunction,
            ConditionDetails::Volume(d) => d.is_conjunction,
            ConditionDetails::PercentChange(d) => d.is_conjunction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_condition_details() {
        let details = PriceConditionDetails {
            is_conjunction: true,
            is_more: true,
            price: 250.0,
            contract_id: 265598,
            exchange: "SMART".to_string(),
            trigger_method: 0,
        };

        assert!(details.is_conjunction);
        assert_eq!(details.price, 250.0);
        assert_eq!(details.contract_id, 265598);
    }

    #[test]
    fn test_time_condition_details() {
        let details = TimeConditionDetails {
            is_conjunction: true,
            is_more: true,
            time: "20250315-09:30:00".to_string(),
        };

        assert!(details.is_conjunction);
        assert_eq!(details.time, "20250315-09:30:00");
    }

    #[test]
    fn test_condition_details_is_conjunction() {
        let price_details = ConditionDetails::Price(PriceConditionDetails {
            is_conjunction: true,
            is_more: true,
            price: 250.0,
            contract_id: 265598,
            exchange: "SMART".to_string(),
            trigger_method: 0,
        });

        let time_details = ConditionDetails::Time(TimeConditionDetails {
            is_conjunction: false,
            is_more: true,
            time: "20250315-09:30:00".to_string(),
        });

        assert!(price_details.is_conjunction());
        assert!(!time_details.is_conjunction());
    }
}
