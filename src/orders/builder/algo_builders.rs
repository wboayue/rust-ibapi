//! Builder structs for IB algorithmic order strategies.
//!
//! This module provides type-safe builders for common IB algo strategies:
//! VWAP, TWAP, Percentage of Volume, and Arrival Price.

use super::types::ValidationError;
use crate::contracts::TagValue;

/// Convert a boolean to IB's string representation ("1" or "0").
fn bool_param(v: bool) -> String {
    if v { "1" } else { "0" }.to_string()
}

/// Minimum allowed participation rate (10%).
pub const MIN_PCT_VOL: f64 = 0.1;
/// Maximum allowed participation rate (50%).
pub const MAX_PCT_VOL: f64 = 0.5;

/// Validate percentage is within IB's allowed range.
fn validate_pct_vol(field: &'static str, value: f64) -> Result<(), ValidationError> {
    if !(MIN_PCT_VOL..=MAX_PCT_VOL).contains(&value) {
        Err(ValidationError::InvalidPercentage {
            field,
            value,
            min: MIN_PCT_VOL,
            max: MAX_PCT_VOL,
        })
    } else {
        Ok(())
    }
}

/// Parameters for an algorithmic order strategy.
#[derive(Debug, Clone, Default)]
pub struct AlgoParams {
    /// The algorithm strategy name (e.g., "Vwap", "Twap")
    pub strategy: String,
    /// The algorithm parameters as tag-value pairs
    pub params: Vec<TagValue>,
}

impl From<String> for AlgoParams {
    fn from(strategy: String) -> Self {
        Self {
            strategy,
            params: Vec::new(),
        }
    }
}

impl From<&str> for AlgoParams {
    fn from(strategy: &str) -> Self {
        Self {
            strategy: strategy.to_string(),
            params: Vec::new(),
        }
    }
}

// === VWAP Builder ===

/// Builder for VWAP (Volume Weighted Average Price) algorithmic orders.
///
/// VWAP seeks to achieve the volume-weighted average price from order
/// submission to market close.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::vwap;
///
/// let algo = vwap()
///     .max_pct_vol(0.2)
///     .start_time("09:00:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct VwapBuilder {
    max_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    allow_past_end_time: Option<bool>,
    no_take_liq: Option<bool>,
    speed_up: Option<bool>,
}

impl VwapBuilder {
    /// Create a new VWAP builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum participation rate (must be 10-50% per IB requirements).
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "09:00:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Set end time (format: "HH:MM:SS TZ", e.g., "16:00:00 US/Eastern").
    pub fn end_time(mut self, time: impl Into<String>) -> Self {
        self.end_time = Some(time.into());
        self
    }

    /// Allow trading past the end time.
    pub fn allow_past_end_time(mut self, allow: bool) -> Self {
        self.allow_past_end_time = Some(allow);
        self
    }

    /// Passive only - do not take liquidity.
    pub fn no_take_liq(mut self, no_take: bool) -> Self {
        self.no_take_liq = Some(no_take);
        self
    }

    /// Speed up execution in momentum.
    pub fn speed_up(mut self, speed_up: bool) -> Self {
        self.speed_up = Some(speed_up);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `max_pct_vol` is set but outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.max_pct_vol {
            validate_pct_vol("max_pct_vol", v)?;
            params.push(TagValue {
                tag: "maxPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.no_take_liq {
            params.push(TagValue {
                tag: "noTakeLiq".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.speed_up {
            params.push(TagValue {
                tag: "speedUp".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "Vwap".to_string(),
            params,
        })
    }
}

impl TryFrom<VwapBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: VwapBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === TWAP Builder ===

/// Strategy type for TWAP orders.
#[derive(Debug, Clone, Copy, Default)]
pub enum TwapStrategyType {
    /// Default TWAP strategy
    #[default]
    Marketable,
    /// Match midpoint
    MatchingMidpoint,
    /// Match same side
    MatchingSameSide,
    /// Match last
    MatchingLast,
}

impl TwapStrategyType {
    fn as_str(&self) -> &'static str {
        match self {
            TwapStrategyType::Marketable => "Marketable",
            TwapStrategyType::MatchingMidpoint => "Matching Midpoint",
            TwapStrategyType::MatchingSameSide => "Matching Same Side",
            TwapStrategyType::MatchingLast => "Matching Last",
        }
    }
}

/// Builder for TWAP (Time Weighted Average Price) algorithmic orders.
///
/// TWAP seeks to achieve the time-weighted average price.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::twap;
///
/// let algo = twap()
///     .start_time("09:00:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct TwapBuilder {
    strategy_type: Option<TwapStrategyType>,
    start_time: Option<String>,
    end_time: Option<String>,
    allow_past_end_time: Option<bool>,
}

impl TwapBuilder {
    /// Create a new TWAP builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the TWAP strategy type.
    pub fn strategy_type(mut self, strategy: TwapStrategyType) -> Self {
        self.strategy_type = Some(strategy);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "09:00:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Set end time (format: "HH:MM:SS TZ", e.g., "16:00:00 US/Eastern").
    pub fn end_time(mut self, time: impl Into<String>) -> Self {
        self.end_time = Some(time.into());
        self
    }

    /// Allow trading past the end time.
    pub fn allow_past_end_time(mut self, allow: bool) -> Self {
        self.allow_past_end_time = Some(allow);
        self
    }

    /// Build the algo parameters.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.strategy_type {
            params.push(TagValue {
                tag: "strategyType".to_string(),
                value: v.as_str().to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "Twap".to_string(),
            params,
        })
    }
}

impl TryFrom<TwapBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: TwapBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === Percentage of Volume Builder ===

/// Builder for Percentage of Volume (PctVol) algorithmic orders.
///
/// Controls participation rate to minimize market impact.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol;
///
/// let algo = pct_vol()
///     .pct_vol(0.1)
///     .start_time("09:00:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct PctVolBuilder {
    pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}

impl PctVolBuilder {
    /// Create a new PctVol builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set target participation rate (must be 10-50% per IB requirements).
    pub fn pct_vol(mut self, pct: f64) -> Self {
        self.pct_vol = Some(pct);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "09:00:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Set end time (format: "HH:MM:SS TZ", e.g., "16:00:00 US/Eastern").
    pub fn end_time(mut self, time: impl Into<String>) -> Self {
        self.end_time = Some(time.into());
        self
    }

    /// Passive only - do not take liquidity.
    pub fn no_take_liq(mut self, no_take: bool) -> Self {
        self.no_take_liq = Some(no_take);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `pct_vol` is set but outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.pct_vol {
            validate_pct_vol("pct_vol", v)?;
            params.push(TagValue {
                tag: "pctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.no_take_liq {
            params.push(TagValue {
                tag: "noTakeLiq".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "PctVol".to_string(),
            params,
        })
    }
}

impl TryFrom<PctVolBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: PctVolBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === Arrival Price Builder ===

/// Risk aversion level for Arrival Price orders.
#[derive(Debug, Clone, Copy, Default)]
pub enum RiskAversion {
    /// Get Done - complete order quickly
    GetDone,
    /// Aggressive - favor speed over price
    Aggressive,
    /// Neutral - balance speed and price
    #[default]
    Neutral,
    /// Passive - favor price over speed
    Passive,
}

impl RiskAversion {
    fn as_str(&self) -> &'static str {
        match self {
            RiskAversion::GetDone => "Get Done",
            RiskAversion::Aggressive => "Aggressive",
            RiskAversion::Neutral => "Neutral",
            RiskAversion::Passive => "Passive",
        }
    }
}

/// Builder for Arrival Price algorithmic orders.
///
/// Achieves the bid/ask midpoint at order arrival time.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::{arrival_price, RiskAversion};
///
/// let algo = arrival_price()
///     .max_pct_vol(0.1)
///     .risk_aversion(RiskAversion::Neutral)
///     .start_time("09:00:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct ArrivalPriceBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    start_time: Option<String>,
    end_time: Option<String>,
    force_completion: Option<bool>,
    allow_past_end_time: Option<bool>,
}

impl ArrivalPriceBuilder {
    /// Create a new Arrival Price builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum participation rate (must be 10-50% per IB requirements).
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct);
        self
    }

    /// Set risk aversion level.
    pub fn risk_aversion(mut self, risk: RiskAversion) -> Self {
        self.risk_aversion = Some(risk);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "09:00:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Set end time (format: "HH:MM:SS TZ", e.g., "16:00:00 US/Eastern").
    pub fn end_time(mut self, time: impl Into<String>) -> Self {
        self.end_time = Some(time.into());
        self
    }

    /// Force completion by end time.
    pub fn force_completion(mut self, force: bool) -> Self {
        self.force_completion = Some(force);
        self
    }

    /// Allow trading past the end time.
    pub fn allow_past_end_time(mut self, allow: bool) -> Self {
        self.allow_past_end_time = Some(allow);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `max_pct_vol` is set but outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.max_pct_vol {
            validate_pct_vol("max_pct_vol", v)?;
            params.push(TagValue {
                tag: "maxPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.risk_aversion {
            params.push(TagValue {
                tag: "riskAversion".to_string(),
                value: v.as_str().to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.force_completion {
            params.push(TagValue {
                tag: "forceCompletion".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "ArrivalPx".to_string(),
            params,
        })
    }
}

impl TryFrom<ArrivalPriceBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: ArrivalPriceBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === Adaptive Builder ===

/// Urgency priority for Adaptive algorithmic orders.
#[derive(Debug, Clone, Copy, Default)]
pub enum AdaptivePriority {
    /// Urgent - complete quickly, less concerned with price improvement
    Urgent,
    /// Normal - balanced execution speed and price improvement
    #[default]
    Normal,
    /// Patient - prefer price improvement, accept slower execution
    Patient,
}

impl AdaptivePriority {
    fn as_str(&self) -> &'static str {
        match self {
            AdaptivePriority::Urgent => "Urgent",
            AdaptivePriority::Normal => "Normal",
            AdaptivePriority::Patient => "Patient",
        }
    }
}

/// Builder for Adaptive algorithmic orders.
///
/// Combines IB's Smart Routing with user-defined urgency to balance
/// execution speed against price improvement.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::{adaptive, AdaptivePriority};
///
/// let algo = adaptive()
///     .priority(AdaptivePriority::Normal)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct AdaptiveBuilder {
    priority: Option<AdaptivePriority>,
}

impl AdaptiveBuilder {
    /// Create a new Adaptive builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the urgency priority.
    pub fn priority(mut self, priority: AdaptivePriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Build the algo parameters.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.priority {
            params.push(TagValue {
                tag: "adaptivePriority".to_string(),
                value: v.as_str().to_string(),
            });
        }

        Ok(AlgoParams {
            strategy: "Adaptive".to_string(),
            params,
        })
    }
}

impl TryFrom<AdaptiveBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: AdaptiveBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === Close Price Builder ===

/// Builder for Close Price (ClosePx) algorithmic orders.
///
/// Minimizes slippage relative to the closing auction price.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::{close_price, RiskAversion};
///
/// let algo = close_price()
///     .max_pct_vol(0.2)
///     .risk_aversion(RiskAversion::Neutral)
///     .start_time("15:30:00 US/Eastern")
///     .force_completion(true)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClosePriceBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    start_time: Option<String>,
    force_completion: Option<bool>,
}

impl ClosePriceBuilder {
    /// Create a new Close Price builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum participation rate (must be 10-50% per IB requirements).
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct);
        self
    }

    /// Set risk aversion level.
    pub fn risk_aversion(mut self, risk: RiskAversion) -> Self {
        self.risk_aversion = Some(risk);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "15:30:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Force completion by the close.
    pub fn force_completion(mut self, force: bool) -> Self {
        self.force_completion = Some(force);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `max_pct_vol` is set but outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.max_pct_vol {
            validate_pct_vol("max_pct_vol", v)?;
            params.push(TagValue {
                tag: "maxPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.risk_aversion {
            params.push(TagValue {
                tag: "riskAversion".to_string(),
                value: v.as_str().to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.force_completion {
            params.push(TagValue {
                tag: "forceCompletion".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "ClosePx".to_string(),
            params,
        })
    }
}

impl TryFrom<ClosePriceBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: ClosePriceBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

// === Dark Ice Builder ===

/// Builder for Dark Ice algorithmic orders.
///
/// Hidden order with randomized display sizes - the user-supplied display
/// size is randomized in increments to camouflage the actual order size.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::dark_ice;
///
/// let algo = dark_ice()
///     .display_size(100)
///     .start_time("09:30:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct DarkIceBuilder {
    display_size: Option<i32>,
    start_time: Option<String>,
    end_time: Option<String>,
    allow_past_end_time: Option<bool>,
}

impl DarkIceBuilder {
    /// Create a new Dark Ice builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the display size (the visible portion of the order).
    pub fn display_size(mut self, size: i32) -> Self {
        self.display_size = Some(size);
        self
    }

    /// Set start time (format: "HH:MM:SS TZ", e.g., "09:30:00 US/Eastern").
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        self.start_time = Some(time.into());
        self
    }

    /// Set end time (format: "HH:MM:SS TZ", e.g., "16:00:00 US/Eastern").
    pub fn end_time(mut self, time: impl Into<String>) -> Self {
        self.end_time = Some(time.into());
        self
    }

    /// Allow trading past the end time.
    pub fn allow_past_end_time(mut self, allow: bool) -> Self {
        self.allow_past_end_time = Some(allow);
        self
    }

    /// Build the algo parameters.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.display_size {
            params.push(TagValue {
                tag: "displaySize".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "DarkIce".to_string(),
            params,
        })
    }
}

impl TryFrom<DarkIceBuilder> for AlgoParams {
    type Error = ValidationError;

    fn try_from(builder: DarkIceBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

#[cfg(test)]
#[path = "algo_builders_tests.rs"]
mod tests;
