//! Builder types for order conditions.
//!
//! This module provides fluent builder APIs for constructing order conditions
//! with type safety and validation.

use serde::{Deserialize, Serialize};

// ============================================================================
// Condition Structs (to be created by Unit 1.1)
// ============================================================================

/// Price-based condition that activates an order when a contract reaches a specified price.
///
/// This condition monitors the price of a specific contract and triggers when the price
/// crosses the specified threshold. The trigger method determines which price feed to use
/// (last, bid/ask, mid-point, etc.).
///
/// # TWS Behavior
///
/// - The contract must be specified by its contract ID, which can be obtained via
///   `contract_details()` API call
/// - Different exchanges may have different price feeds available
/// - The condition continuously monitors the price during market hours
/// - When `conditions_ignore_rth` is true on the order, monitoring extends to
///   after-hours trading
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::PriceCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger when AAPL (contract ID 265598) goes above $150 on SMART
/// let condition = PriceCondition::builder(265598, "SMART", 150.0)
///     .trigger_above()
///     .trigger_method(2)  // Use last price
///     .build();
///
/// let order_condition = OrderCondition::Price(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PriceCondition {
    /// Contract identifier for the instrument to monitor.
    /// Use contract_details() to obtain the contract_id for a symbol.
    pub contract_id: i32,
    /// Exchange where the price is monitored (e.g., "SMART", "NASDAQ", "NYSE").
    pub exchange: String,
    /// Trigger price threshold.
    pub price: f64,
    /// Method for price evaluation:
    /// - 0: Default (last for most securities, double bid/ask for OTC and options)
    /// - 1: Double bid/ask (two consecutive bid or ask prices)
    /// - 2: Last price
    /// - 3: Double last (two consecutive last prices)
    /// - 4: Bid/Ask
    /// - 7: Last or bid/ask
    /// - 8: Mid-point
    pub trigger_method: i32,
    /// True to trigger when price goes above threshold, false for below.
    pub is_more: bool,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

/// Time-based condition that activates an order at a specific date and time.
///
/// This condition triggers when the current time passes (or is before) the specified
/// time threshold. Useful for scheduling orders to activate at specific times.
///
/// # TWS Behavior
///
/// - Time is evaluated based on the timezone specified in the time string
/// - The condition checks continuously and triggers once the time threshold is crossed
/// - Common use case: activate orders at market open, before close, or at specific times
/// - Unlike `good_after_time`/`good_till_date` on the order itself, this can be combined
///   with other conditions using AND/OR logic
///
/// # Time Format
///
/// Format: "YYYYMMDD HH:MM:SS TZ"
/// - YYYYMMDD: Year, month, day (e.g., 20251230)
/// - HH:MM:SS: Hour, minute, second in 24-hour format
/// - TZ: Timezone (e.g., "UTC", "US/Eastern", "America/New_York")
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::TimeCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger after 2:30 PM Eastern Time on December 30, 2025
/// let condition = TimeCondition::builder("20251230 14:30:00 US/Eastern")
///     .trigger_after()
///     .build();
///
/// let order_condition = OrderCondition::Time(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimeCondition {
    /// Time in format "YYYYMMDD HH:MM:SS TZ".
    /// Example: "20251230 14:30:00 US/Eastern"
    pub time: String,
    /// True to trigger after the time, false for before.
    pub is_more: bool,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

/// Margin cushion condition that activates an order based on account margin levels.
///
/// The margin cushion is a measure of account health, calculated as:
/// (Equity with Loan Value - Maintenance Margin) / Net Liquidation Value
///
/// This condition monitors your account's margin cushion and triggers when it crosses
/// the specified percentage threshold. Useful for risk management and protecting against
/// margin calls.
///
/// # TWS Behavior
///
/// - Margin cushion is updated in real-time as positions and prices change
/// - The percentage is specified as an integer (e.g., 30 for 30%)
/// - Only applies to margin accounts; cash accounts will not trigger this condition
/// - Common use: Submit protective orders when margin cushion falls below safe levels
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::MarginCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger when margin cushion falls below 30%
/// let condition = MarginCondition::builder(30)
///     .trigger_below()
///     .build();
///
/// let order_condition = OrderCondition::Margin(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MarginCondition {
    /// Margin cushion percentage threshold (0-100).
    /// Example: 30 represents 30% margin cushion.
    pub percent: i32,
    /// True to trigger when margin cushion goes above threshold, false for below.
    pub is_more: bool,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

/// Execution-based condition that activates an order when a trade of a specific security executes.
///
/// This condition monitors executions in your account and triggers when any trade of the
/// specified contract executes. The condition checks for executions matching the symbol,
/// security type, and exchange.
///
/// # TWS Behavior
///
/// - The condition triggers on ANY execution of the specified contract, regardless of side or quantity
/// - Only monitors executions in the current account
/// - The execution can be from any order type (market, limit, stop, etc.)
/// - Common use case: Place a hedge order immediately after an initial position is filled
/// - The symbol must match exactly (case-sensitive in most cases)
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::ExecutionCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger when MSFT stock executes on SMART exchange
/// let condition = ExecutionCondition::builder("MSFT", "STK", "SMART")
///     .build();
///
/// let order_condition = OrderCondition::Execution(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCondition {
    /// Symbol of the contract to monitor for executions.
    pub symbol: String,
    /// Security type: "STK" (stock), "OPT" (option), "FUT" (future), "FOP" (future option), etc.
    pub security_type: String,
    /// Exchange where execution is monitored (e.g., "SMART", "NASDAQ", "NYSE").
    pub exchange: String,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

/// Volume-based condition that activates an order when cumulative volume reaches a threshold.
///
/// This condition monitors the cumulative trading volume for a specific contract throughout
/// the trading day and triggers when the volume crosses the specified threshold.
///
/// # TWS Behavior
///
/// - Volume is cumulative from market open (resets daily)
/// - The contract must be specified by its contract ID
/// - Volume tracking is exchange-specific (different exchanges may show different volumes)
/// - When `conditions_ignore_rth` is true on the order, includes after-hours volume
/// - Common use case: Enter positions after sufficient liquidity is established
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::VolumeCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger when TSLA volume exceeds 50 million shares
/// let condition = VolumeCondition::builder(76792991, "SMART", 50_000_000)
///     .trigger_above()
///     .build();
///
/// let order_condition = OrderCondition::Volume(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VolumeCondition {
    /// Contract identifier for the instrument to monitor.
    /// Use contract_details() to obtain the contract_id for a symbol.
    pub contract_id: i32,
    /// Exchange where volume is monitored (e.g., "SMART", "NASDAQ", "NYSE").
    pub exchange: String,
    /// Volume threshold (number of shares/contracts traded).
    pub volume: i32,
    /// True to trigger when volume goes above threshold, false for below.
    pub is_more: bool,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

/// Percent change condition that activates an order based on price movement percentage.
///
/// This condition monitors the percentage change in a contract's price from its value at
/// the start of the trading day and triggers when the change crosses the specified threshold.
/// The percentage can be positive (gain) or negative (loss).
///
/// # TWS Behavior
///
/// - Percent change is calculated from the session's opening price
/// - The contract must be specified by its contract ID
/// - The percentage is specified as a decimal (e.g., 2.0 for 2%, not 0.02)
/// - When `is_more` is true, triggers on upward moves; when false, on downward moves
/// - Resets at the start of each trading session
/// - Common use case: Momentum trading or volatility-based order activation
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::PercentChangeCondition;
/// use ibapi::orders::OrderCondition;
///
/// // Trigger when SPY moves more than 2% in either direction from open
/// let condition = PercentChangeCondition::builder(756733, "SMART", 2.0)
///     .trigger_above()
///     .build();
///
/// let order_condition = OrderCondition::PercentChange(condition);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PercentChangeCondition {
    /// Contract identifier for the instrument to monitor.
    /// Use contract_details() to obtain the contract_id for a symbol.
    pub contract_id: i32,
    /// Exchange where price change is monitored (e.g., "SMART", "NASDAQ", "NYSE").
    pub exchange: String,
    /// Percentage change threshold (e.g., 2.0 for 2%, 5.5 for 5.5%).
    pub percent: f64,
    /// True to trigger when percent change goes above threshold (gains), false for below (losses).
    pub is_more: bool,
    /// True for AND condition (all conditions must be met), false for OR condition (any condition triggers).
    pub is_conjunction: bool,
}

// ============================================================================
// Builder Types
// ============================================================================

/// Builder for [`PriceCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::PriceCondition;
///
/// let condition = PriceCondition::builder(12345, "NASDAQ", 150.0)
///     .trigger_above()
///     .trigger_method(1)
///     .conjunction(false)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct PriceConditionBuilder {
    contract_id: i32,
    exchange: String,
    price: f64,
    trigger_method: i32,
    is_more: bool,
    is_conjunction: bool,
}

impl PriceCondition {
    /// Create a builder for a price condition.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where the price is monitored
    /// - `price`: Trigger price threshold
    pub fn builder(contract_id: i32, exchange: impl Into<String>, price: f64) -> PriceConditionBuilder {
        PriceConditionBuilder::new(contract_id, exchange, price)
    }
}

impl PriceConditionBuilder {
    /// Create a new price condition builder.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where the price is monitored
    /// - `price`: Trigger price threshold
    pub fn new(contract_id: i32, exchange: impl Into<String>, price: f64) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            price,
            trigger_method: 0,    // Default: last price
            is_more: true,        // Default: trigger when price goes above
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Set the trigger method for price evaluation.
    ///
    /// # Parameters
    ///
    /// - `0`: Default (last price)
    /// - `1`: Double bid/ask
    /// - `2`: Last price
    /// - `3`: Double last price
    /// - `4`: Bid/Ask
    /// - `7`: Last or bid/ask
    /// - `8`: Mid-point
    pub fn trigger_method(mut self, method: i32) -> Self {
        self.trigger_method = method;
        self
    }

    /// Trigger when price goes above the threshold (default).
    pub fn trigger_above(mut self) -> Self {
        self.is_more = true;
        self
    }

    /// Trigger when price goes below the threshold.
    pub fn trigger_below(mut self) -> Self {
        self.is_more = false;
        self
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the price condition.
    pub fn build(self) -> PriceCondition {
        PriceCondition {
            contract_id: self.contract_id,
            exchange: self.exchange,
            price: self.price,
            trigger_method: self.trigger_method,
            is_more: self.is_more,
            is_conjunction: self.is_conjunction,
        }
    }
}

/// Builder for [`TimeCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::TimeCondition;
///
/// let condition = TimeCondition::builder("20251230 23:59:59 UTC")
///     .trigger_after()
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct TimeConditionBuilder {
    time: String,
    is_more: bool,
    is_conjunction: bool,
}

impl TimeCondition {
    /// Create a builder for a time condition.
    ///
    /// # Parameters
    ///
    /// - `time`: Time in format "YYYYMMDD HH:MM:SS TZ"
    pub fn builder(time: impl Into<String>) -> TimeConditionBuilder {
        TimeConditionBuilder::new(time)
    }
}

impl TimeConditionBuilder {
    /// Create a new time condition builder.
    ///
    /// # Parameters
    ///
    /// - `time`: Time in format "YYYYMMDD HH:MM:SS TZ"
    pub fn new(time: impl Into<String>) -> Self {
        Self {
            time: time.into(),
            is_more: true,        // Default: trigger after time
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Trigger after the specified time (default).
    pub fn trigger_after(mut self) -> Self {
        self.is_more = true;
        self
    }

    /// Trigger before the specified time.
    pub fn trigger_before(mut self) -> Self {
        self.is_more = false;
        self
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the time condition.
    pub fn build(self) -> TimeCondition {
        TimeCondition {
            time: self.time,
            is_more: self.is_more,
            is_conjunction: self.is_conjunction,
        }
    }
}

/// Builder for [`MarginCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::MarginCondition;
///
/// let condition = MarginCondition::builder(30)
///     .trigger_below()
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MarginConditionBuilder {
    percent: i32,
    is_more: bool,
    is_conjunction: bool,
}

impl MarginCondition {
    /// Create a builder for a margin cushion condition.
    ///
    /// # Parameters
    ///
    /// - `percent`: Margin cushion percentage threshold
    pub fn builder(percent: i32) -> MarginConditionBuilder {
        MarginConditionBuilder::new(percent)
    }
}

impl MarginConditionBuilder {
    /// Create a new margin condition builder.
    ///
    /// # Parameters
    ///
    /// - `percent`: Margin cushion percentage threshold
    pub fn new(percent: i32) -> Self {
        Self {
            percent,
            is_more: true,        // Default: trigger when above threshold
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Trigger when margin cushion goes above the threshold (default).
    pub fn trigger_above(mut self) -> Self {
        self.is_more = true;
        self
    }

    /// Trigger when margin cushion goes below the threshold.
    pub fn trigger_below(mut self) -> Self {
        self.is_more = false;
        self
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the margin condition.
    pub fn build(self) -> MarginCondition {
        MarginCondition {
            percent: self.percent,
            is_more: self.is_more,
            is_conjunction: self.is_conjunction,
        }
    }
}

/// Builder for [`ExecutionCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::ExecutionCondition;
///
/// let condition = ExecutionCondition::builder("AAPL", "STK", "SMART")
///     .conjunction(false)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ExecutionConditionBuilder {
    symbol: String,
    security_type: String,
    exchange: String,
    is_conjunction: bool,
}

impl ExecutionCondition {
    /// Create a builder for an execution condition.
    ///
    /// # Parameters
    ///
    /// - `symbol`: Symbol of the contract
    /// - `security_type`: Security type (e.g., "STK", "OPT")
    /// - `exchange`: Exchange where execution is monitored
    pub fn builder(symbol: impl Into<String>, security_type: impl Into<String>, exchange: impl Into<String>) -> ExecutionConditionBuilder {
        ExecutionConditionBuilder::new(symbol, security_type, exchange)
    }
}

impl ExecutionConditionBuilder {
    /// Create a new execution condition builder.
    ///
    /// # Parameters
    ///
    /// - `symbol`: Symbol of the contract
    /// - `security_type`: Security type (e.g., "STK", "OPT")
    /// - `exchange`: Exchange where execution is monitored
    pub fn new(symbol: impl Into<String>, security_type: impl Into<String>, exchange: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            security_type: security_type.into(),
            exchange: exchange.into(),
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the execution condition.
    pub fn build(self) -> ExecutionCondition {
        ExecutionCondition {
            symbol: self.symbol,
            security_type: self.security_type,
            exchange: self.exchange,
            is_conjunction: self.is_conjunction,
        }
    }
}

/// Builder for [`VolumeCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::VolumeCondition;
///
/// let condition = VolumeCondition::builder(12345, "NASDAQ", 1000000)
///     .trigger_above()
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct VolumeConditionBuilder {
    contract_id: i32,
    exchange: String,
    volume: i32,
    is_more: bool,
    is_conjunction: bool,
}

impl VolumeCondition {
    /// Create a builder for a volume condition.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where volume is monitored
    /// - `volume`: Volume threshold
    pub fn builder(contract_id: i32, exchange: impl Into<String>, volume: i32) -> VolumeConditionBuilder {
        VolumeConditionBuilder::new(contract_id, exchange, volume)
    }
}

impl VolumeConditionBuilder {
    /// Create a new volume condition builder.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where volume is monitored
    /// - `volume`: Volume threshold
    pub fn new(contract_id: i32, exchange: impl Into<String>, volume: i32) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            volume,
            is_more: true,        // Default: trigger when above threshold
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Trigger when volume goes above the threshold (default).
    pub fn trigger_above(mut self) -> Self {
        self.is_more = true;
        self
    }

    /// Trigger when volume goes below the threshold.
    pub fn trigger_below(mut self) -> Self {
        self.is_more = false;
        self
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the volume condition.
    pub fn build(self) -> VolumeCondition {
        VolumeCondition {
            contract_id: self.contract_id,
            exchange: self.exchange,
            volume: self.volume,
            is_more: self.is_more,
            is_conjunction: self.is_conjunction,
        }
    }
}

/// Builder for [`PercentChangeCondition`].
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::conditions::PercentChangeCondition;
///
/// let condition = PercentChangeCondition::builder(12345, "NASDAQ", 5.0)
///     .trigger_above()
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct PercentChangeConditionBuilder {
    contract_id: i32,
    exchange: String,
    percent: f64,
    is_more: bool,
    is_conjunction: bool,
}

impl PercentChangeCondition {
    /// Create a builder for a percent change condition.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where price change is monitored
    /// - `percent`: Percentage change threshold
    pub fn builder(contract_id: i32, exchange: impl Into<String>, percent: f64) -> PercentChangeConditionBuilder {
        PercentChangeConditionBuilder::new(contract_id, exchange, percent)
    }
}

impl PercentChangeConditionBuilder {
    /// Create a new percent change condition builder.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: Contract identifier for the instrument to monitor
    /// - `exchange`: Exchange where price change is monitored
    /// - `percent`: Percentage change threshold
    pub fn new(contract_id: i32, exchange: impl Into<String>, percent: f64) -> Self {
        Self {
            contract_id,
            exchange: exchange.into(),
            percent,
            is_more: true,        // Default: trigger when above threshold
            is_conjunction: true, // Default: AND condition
        }
    }

    /// Trigger when percent change goes above the threshold (default).
    pub fn trigger_above(mut self) -> Self {
        self.is_more = true;
        self
    }

    /// Trigger when percent change goes below the threshold.
    pub fn trigger_below(mut self) -> Self {
        self.is_more = false;
        self
    }

    /// Set whether this is an AND (conjunction) or OR (disjunction) condition.
    ///
    /// Default is `true` (AND).
    pub fn conjunction(mut self, is_conjunction: bool) -> Self {
        self.is_conjunction = is_conjunction;
        self
    }

    /// Build the percent change condition.
    pub fn build(self) -> PercentChangeCondition {
        PercentChangeCondition {
            contract_id: self.contract_id,
            exchange: self.exchange,
            percent: self.percent,
            is_more: self.is_more,
            is_conjunction: self.is_conjunction,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_condition_builder() {
        let condition = PriceCondition::builder(12345, "NASDAQ", 150.0)
            .trigger_above()
            .trigger_method(1)
            .conjunction(false)
            .build();

        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "NASDAQ");
        assert_eq!(condition.price, 150.0);
        assert_eq!(condition.trigger_method, 1);
        assert!(condition.is_more);
        assert!(!condition.is_conjunction);
    }

    #[test]
    fn test_time_condition_builder() {
        let condition = TimeCondition::builder("20251230 23:59:59 UTC").trigger_before().build();

        assert_eq!(condition.time, "20251230 23:59:59 UTC");
        assert!(!condition.is_more);
        assert!(condition.is_conjunction);
    }

    #[test]
    fn test_margin_condition_builder() {
        let condition = MarginCondition::builder(30).trigger_below().conjunction(false).build();

        assert_eq!(condition.percent, 30);
        assert!(!condition.is_more);
        assert!(!condition.is_conjunction);
    }

    #[test]
    fn test_execution_condition_builder() {
        let condition = ExecutionCondition::builder("AAPL", "STK", "SMART").conjunction(false).build();

        assert_eq!(condition.symbol, "AAPL");
        assert_eq!(condition.security_type, "STK");
        assert_eq!(condition.exchange, "SMART");
        assert!(!condition.is_conjunction);
    }

    #[test]
    fn test_volume_condition_builder() {
        let condition = VolumeCondition::builder(12345, "NASDAQ", 1000000).trigger_below().build();

        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "NASDAQ");
        assert_eq!(condition.volume, 1000000);
        assert!(!condition.is_more);
        assert!(condition.is_conjunction);
    }

    #[test]
    fn test_percent_change_condition_builder() {
        let condition = PercentChangeCondition::builder(12345, "NASDAQ", 5.0)
            .trigger_above()
            .conjunction(false)
            .build();

        assert_eq!(condition.contract_id, 12345);
        assert_eq!(condition.exchange, "NASDAQ");
        assert_eq!(condition.percent, 5.0);
        assert!(condition.is_more);
        assert!(!condition.is_conjunction);
    }

    #[test]
    fn test_default_values() {
        let condition = PriceCondition::builder(12345, "NASDAQ", 150.0).build();

        assert_eq!(condition.trigger_method, 0);
        assert!(condition.is_more);
        assert!(condition.is_conjunction);
    }
}
