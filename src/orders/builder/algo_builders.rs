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

/// Build params for a "windowed PctVol" strategy: a base/end participation
/// rate plus optional active-window times. Shared by `PctVolSizeBuilder`
/// (`"PctVolSz"`) and `PctVolTimeBuilder` (`"PctVolTm"`), which differ only
/// in which dimension drives the rate ramp - the on-wire encoding is the
/// same shape.
fn build_windowed_pct_vol(
    strategy: &'static str,
    start_pct_vol: Option<f64>,
    end_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
) -> Result<AlgoParams, ValidationError> {
    let mut params = Vec::new();

    if let Some(v) = start_pct_vol {
        validate_pct_vol("start_pct_vol", v)?;
        params.push(TagValue {
            tag: "startPctVol".to_string(),
            value: v.to_string(),
        });
    }
    if let Some(v) = end_pct_vol {
        validate_pct_vol("end_pct_vol", v)?;
        params.push(TagValue {
            tag: "endPctVol".to_string(),
            value: v.to_string(),
        });
    }
    if let Some(v) = start_time {
        params.push(TagValue {
            tag: "startTime".to_string(),
            value: v,
        });
    }
    if let Some(v) = end_time {
        params.push(TagValue {
            tag: "endTime".to_string(),
            value: v,
        });
    }
    if let Some(v) = no_take_liq {
        params.push(TagValue {
            tag: "noTakeLiq".to_string(),
            value: bool_param(v),
        });
    }

    Ok(AlgoParams {
        strategy: strategy.to_string(),
        params,
    })
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
// === Accumulate/Distribute Builder ===

/// Builder for Accumulate/Distribute (AD) algorithmic orders.
///
/// Slices an order into random increments at random intervals to disguise
/// trading intent.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::accumulate_distribute;
///
/// let algo = accumulate_distribute()
///     .component_size(100)
///     .time_between_orders(60)
///     .randomize_time_20(true)
///     .randomize_size_55(true)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct AccumulateDistributeBuilder {
    component_size: Option<i32>,
    time_between_orders: Option<i32>,
    randomize_time_20: Option<bool>,
    randomize_size_55: Option<bool>,
    give_up: Option<i32>,
    catch_up: Option<bool>,
    wait_for_fill: Option<bool>,
    active_time_start: Option<String>,
    active_time_end: Option<String>,
}

impl AccumulateDistributeBuilder {
    /// Create a new Accumulate/Distribute builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the size of each component slice.
    pub fn component_size(mut self, size: i32) -> Self {
        self.component_size = Some(size);
        self
    }

    /// Set the seconds between component orders.
    pub fn time_between_orders(mut self, seconds: i32) -> Self {
        self.time_between_orders = Some(seconds);
        self
    }

    /// Randomize the time interval by ±20%.
    pub fn randomize_time_20(mut self, randomize: bool) -> Self {
        self.randomize_time_20 = Some(randomize);
        self
    }

    /// Randomize the component size by ±55%.
    pub fn randomize_size_55(mut self, randomize: bool) -> Self {
        self.randomize_size_55 = Some(randomize);
        self
    }

    /// Set the give-up account.
    pub fn give_up(mut self, give_up: i32) -> Self {
        self.give_up = Some(give_up);
        self
    }

    /// Catch up in time if the algo falls behind.
    pub fn catch_up(mut self, catch_up: bool) -> Self {
        self.catch_up = Some(catch_up);
        self
    }

    /// Wait for the previous component to fill before submitting the next.
    pub fn wait_for_fill(mut self, wait: bool) -> Self {
        self.wait_for_fill = Some(wait);
        self
    }

    /// Set the active period start time (format: "YYYYMMDD-HH:MM:SS TZ").
    pub fn active_time_start(mut self, time: impl Into<String>) -> Self {
        self.active_time_start = Some(time.into());
        self
    }

    /// Set the active period end time (format: "YYYYMMDD-HH:MM:SS TZ").
    pub fn active_time_end(mut self, time: impl Into<String>) -> Self {
        self.active_time_end = Some(time.into());
        self
    }

    /// Build the algo parameters.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.component_size {
            params.push(TagValue {
                tag: "componentSize".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.time_between_orders {
            params.push(TagValue {
                tag: "timeBetweenOrders".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.randomize_time_20 {
            params.push(TagValue {
                tag: "randomizeTime20".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.randomize_size_55 {
            params.push(TagValue {
                tag: "randomizeSize55".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.give_up {
            params.push(TagValue {
                tag: "giveUp".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.catch_up {
            params.push(TagValue {
                tag: "catchUp".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.wait_for_fill {
            params.push(TagValue {
                tag: "waitForFill".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = self.active_time_start {
            params.push(TagValue {
                tag: "activeTimeStart".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.active_time_end {
            params.push(TagValue {
                tag: "activeTimeEnd".to_string(),
                value: v,
            });
        }

        Ok(AlgoParams {
            strategy: "AD".to_string(),
            params,
        })
    }
}
// === Balance Impact Risk Builder ===

/// Builder for Balance Impact Risk algorithmic orders.
///
/// Balances market impact against the risk of adverse price movement.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::{balance_impact_risk, RiskAversion};
///
/// let algo = balance_impact_risk()
///     .max_pct_vol(0.2)
///     .risk_aversion(RiskAversion::Neutral)
///     .force_completion(true)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct BalanceImpactRiskBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    force_completion: Option<bool>,
}

impl BalanceImpactRiskBuilder {
    /// Create a new Balance Impact Risk builder.
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

    /// Force completion of the order.
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
        if let Some(v) = self.force_completion {
            params.push(TagValue {
                tag: "forceCompletion".to_string(),
                value: bool_param(v),
            });
        }

        Ok(AlgoParams {
            strategy: "BalanceImpactRisk".to_string(),
            params,
        })
    }
}
// === Minimise Impact Builder ===

/// Builder for Minimise Impact (MinImpact) algorithmic orders.
///
/// Slices the order to achieve the market average with minimal impact.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::minimise_impact;
///
/// let algo = minimise_impact()
///     .max_pct_vol(0.2)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct MinimiseImpactBuilder {
    max_pct_vol: Option<f64>,
}

impl MinimiseImpactBuilder {
    /// Create a new Minimise Impact builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum participation rate (must be 10-50% per IB requirements).
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct);
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

        Ok(AlgoParams {
            strategy: "MinImpact".to_string(),
            params,
        })
    }
}
// === Price Variant Percentage of Volume Builder ===

/// Builder for Price Variant Percentage of Volume (PctVolPx) algorithmic orders.
///
/// Participation rate varies with the market price: tracks at `pct_vol`,
/// adjusts by `delta_pct_vol` as price moves, and stays bounded by
/// `min_pct_vol_4_px` / `max_pct_vol_4_px`. All four percentage fields are
/// validated to IB's 10-50% range.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_price;
///
/// let algo = pct_vol_price()
///     .pct_vol(0.2)
///     .delta_pct_vol(0.1)
///     .min_pct_vol_4_px(0.1)
///     .max_pct_vol_4_px(0.4)
///     .start_time("09:30:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct PctVolPriceBuilder {
    pct_vol: Option<f64>,
    delta_pct_vol: Option<f64>,
    min_pct_vol_4_px: Option<f64>,
    max_pct_vol_4_px: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}

impl PctVolPriceBuilder {
    /// Create a new PctVolPx builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base participation rate (must be 10-50% per IB requirements).
    pub fn pct_vol(mut self, pct: f64) -> Self {
        self.pct_vol = Some(pct);
        self
    }

    /// Set the rate delta applied as price moves (must be 10-50%).
    pub fn delta_pct_vol(mut self, delta: f64) -> Self {
        self.delta_pct_vol = Some(delta);
        self
    }

    /// Set the minimum participation rate bound (must be 10-50%).
    pub fn min_pct_vol_4_px(mut self, pct: f64) -> Self {
        self.min_pct_vol_4_px = Some(pct);
        self
    }

    /// Set the maximum participation rate bound (must be 10-50%).
    pub fn max_pct_vol_4_px(mut self, pct: f64) -> Self {
        self.max_pct_vol_4_px = Some(pct);
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

    /// Passive only - do not take liquidity.
    pub fn no_take_liq(mut self, no_take: bool) -> Self {
        self.no_take_liq = Some(no_take);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if any of `pct_vol`, `delta_pct_vol`,
    /// `min_pct_vol_4_px`, or `max_pct_vol_4_px` is set but outside IB's
    /// 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.pct_vol {
            validate_pct_vol("pct_vol", v)?;
            params.push(TagValue {
                tag: "pctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.delta_pct_vol {
            validate_pct_vol("delta_pct_vol", v)?;
            params.push(TagValue {
                tag: "deltaPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.min_pct_vol_4_px {
            validate_pct_vol("min_pct_vol_4_px", v)?;
            params.push(TagValue {
                tag: "minPctVol4Px".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.max_pct_vol_4_px {
            validate_pct_vol("max_pct_vol_4_px", v)?;
            params.push(TagValue {
                tag: "maxPctVol4Px".to_string(),
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
            strategy: "PctVolPx".to_string(),
            params,
        })
    }
}
// === Size Variant Percentage of Volume Builder ===

/// Builder for Size Variant Percentage of Volume (PctVolSz) algorithmic orders.
///
/// Participation rate varies linearly from `start_pct_vol` to `end_pct_vol`
/// as the order is filled.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_size;
///
/// let algo = pct_vol_size()
///     .start_pct_vol(0.1)
///     .end_pct_vol(0.4)
///     .start_time("09:30:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct PctVolSizeBuilder {
    start_pct_vol: Option<f64>,
    end_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}

impl PctVolSizeBuilder {
    /// Create a new PctVolSz builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the participation rate at the start (must be 10-50%).
    pub fn start_pct_vol(mut self, pct: f64) -> Self {
        self.start_pct_vol = Some(pct);
        self
    }

    /// Set the participation rate at the end (must be 10-50%).
    pub fn end_pct_vol(mut self, pct: f64) -> Self {
        self.end_pct_vol = Some(pct);
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

    /// Passive only - do not take liquidity.
    pub fn no_take_liq(mut self, no_take: bool) -> Self {
        self.no_take_liq = Some(no_take);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `start_pct_vol` or `end_pct_vol` is set but
    /// outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        build_windowed_pct_vol(
            "PctVolSz",
            self.start_pct_vol,
            self.end_pct_vol,
            self.start_time,
            self.end_time,
            self.no_take_liq,
        )
    }
}

// === Time Variant Percentage of Volume Builder ===

/// Builder for Time Variant Percentage of Volume (PctVolTm) algorithmic orders.
///
/// Participation rate varies linearly from `start_pct_vol` to `end_pct_vol`
/// over the active time window.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_time;
///
/// let algo = pct_vol_time()
///     .start_pct_vol(0.1)
///     .end_pct_vol(0.4)
///     .start_time("09:30:00 US/Eastern")
///     .end_time("16:00:00 US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct PctVolTimeBuilder {
    start_pct_vol: Option<f64>,
    end_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}

impl PctVolTimeBuilder {
    /// Create a new PctVolTm builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the participation rate at the start (must be 10-50%).
    pub fn start_pct_vol(mut self, pct: f64) -> Self {
        self.start_pct_vol = Some(pct);
        self
    }

    /// Set the participation rate at the end (must be 10-50%).
    pub fn end_pct_vol(mut self, pct: f64) -> Self {
        self.end_pct_vol = Some(pct);
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

    /// Passive only - do not take liquidity.
    pub fn no_take_liq(mut self, no_take: bool) -> Self {
        self.no_take_liq = Some(no_take);
        self
    }

    /// Build the algo parameters.
    ///
    /// Returns an error if `start_pct_vol` or `end_pct_vol` is set but
    /// outside the 10-50% range.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        build_windowed_pct_vol(
            "PctVolTm",
            self.start_pct_vol,
            self.end_pct_vol,
            self.start_time,
            self.end_time,
            self.no_take_liq,
        )
    }
}

// === AccuDistr Builder ===

/// Builder for AccuDistr algorithmic orders.
///
/// AccuDistr is a separate IBKR algorithm from `AD` (Accumulate/Distribute).
/// It exposes a smaller set of parameters and adds an explicit time zone
/// (`active_time_tz`) and a `route_order_type` knob. Strategy string on the
/// wire is `"AccuDistr"`.
///
/// `route_order_type` is a free-form string per IBKR's sample code (typical
/// values: `"LMT"`, `"MKT"`, `"REL"`). The IBKR algo docs page does not
/// enumerate the parameters of this algo; the parameter shape is taken from
/// the canonical Testbed sample (`samples/CSharp/Testbed/AvailableAlgoParams.cs`).
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::accu_distr;
///
/// let algo = accu_distr()
///     .time_between_orders(60)
///     .route_order_type("LMT")
///     .component_size(100)
///     .active_time_start("20260101-09:30:00")
///     .active_time_end("20260101-16:00:00")
///     .active_time_tz("US/Eastern")
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct AccuDistrBuilder {
    time_between_orders: Option<i32>,
    route_order_type: Option<String>,
    component_size: Option<i32>,
    active_time_start: Option<String>,
    active_time_end: Option<String>,
    active_time_tz: Option<String>,
}

impl AccuDistrBuilder {
    /// Create a new AccuDistr builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the seconds between component orders.
    pub fn time_between_orders(mut self, seconds: i32) -> Self {
        self.time_between_orders = Some(seconds);
        self
    }

    /// Set the route order type (e.g., "LMT", "MKT", "REL").
    pub fn route_order_type(mut self, order_type: impl Into<String>) -> Self {
        self.route_order_type = Some(order_type.into());
        self
    }

    /// Set the size of each component slice.
    pub fn component_size(mut self, size: i32) -> Self {
        self.component_size = Some(size);
        self
    }

    /// Set the active period start time (format: "YYYYMMDD-HH:MM:SS").
    pub fn active_time_start(mut self, time: impl Into<String>) -> Self {
        self.active_time_start = Some(time.into());
        self
    }

    /// Set the active period end time (format: "YYYYMMDD-HH:MM:SS").
    pub fn active_time_end(mut self, time: impl Into<String>) -> Self {
        self.active_time_end = Some(time.into());
        self
    }

    /// Set the time zone for `active_time_start` / `active_time_end`
    /// (e.g., "US/Eastern", "GMT").
    pub fn active_time_tz(mut self, tz: impl Into<String>) -> Self {
        self.active_time_tz = Some(tz.into());
        self
    }

    /// Build the algo parameters.
    pub fn build(self) -> Result<AlgoParams, ValidationError> {
        let mut params = Vec::new();

        if let Some(v) = self.time_between_orders {
            params.push(TagValue {
                tag: "timeBetweenOrders".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.route_order_type {
            params.push(TagValue {
                tag: "routeOrderType".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.component_size {
            params.push(TagValue {
                tag: "componentSize".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = self.active_time_start {
            params.push(TagValue {
                tag: "activeTimeStart".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.active_time_end {
            params.push(TagValue {
                tag: "activeTimeEnd".to_string(),
                value: v,
            });
        }
        if let Some(v) = self.active_time_tz {
            params.push(TagValue {
                tag: "activeTimeTz".to_string(),
                value: v,
            });
        }

        Ok(AlgoParams {
            strategy: "AccuDistr".to_string(),
            params,
        })
    }
}
#[cfg(test)]
#[path = "algo_builders_tests.rs"]
mod tests;
