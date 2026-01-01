//! Builder structs for IB algorithmic order strategies.
//!
//! This module provides type-safe builders for common IB algo strategies:
//! VWAP, TWAP, Percentage of Volume, and Arrival Price.

use crate::contracts::TagValue;

/// Convert a boolean to IB's string representation ("1" or "0").
fn bool_param(v: bool) -> String {
    if v { "1" } else { "0" }.to_string()
}

/// Minimum allowed participation rate (10%).
const MIN_PCT_VOL: f64 = 0.1;
/// Maximum allowed participation rate (50%).
const MAX_PCT_VOL: f64 = 0.5;

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
///     .build();
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

    /// Set maximum participation rate, clamped to 10-50% per IB requirements.
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct.clamp(MIN_PCT_VOL, MAX_PCT_VOL));
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
    pub fn build(self) -> AlgoParams {
        self.into()
    }
}

impl From<VwapBuilder> for AlgoParams {
    fn from(builder: VwapBuilder) -> Self {
        let mut params = Vec::new();

        if let Some(v) = builder.max_pct_vol {
            params.push(TagValue {
                tag: "maxPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = builder.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = builder.no_take_liq {
            params.push(TagValue {
                tag: "noTakeLiq".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = builder.speed_up {
            params.push(TagValue {
                tag: "speedUp".to_string(),
                value: bool_param(v),
            });
        }

        AlgoParams {
            strategy: "Vwap".to_string(),
            params,
        }
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
///     .build();
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
    pub fn build(self) -> AlgoParams {
        self.into()
    }
}

impl From<TwapBuilder> for AlgoParams {
    fn from(builder: TwapBuilder) -> Self {
        let mut params = Vec::new();

        if let Some(v) = builder.strategy_type {
            params.push(TagValue {
                tag: "strategyType".to_string(),
                value: v.as_str().to_string(),
            });
        }
        if let Some(v) = builder.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }

        AlgoParams {
            strategy: "Twap".to_string(),
            params,
        }
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
///     .build();
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

    /// Set target participation rate, clamped to 10-50% per IB requirements.
    pub fn pct_vol(mut self, pct: f64) -> Self {
        self.pct_vol = Some(pct.clamp(MIN_PCT_VOL, MAX_PCT_VOL));
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
    pub fn build(self) -> AlgoParams {
        self.into()
    }
}

impl From<PctVolBuilder> for AlgoParams {
    fn from(builder: PctVolBuilder) -> Self {
        let mut params = Vec::new();

        if let Some(v) = builder.pct_vol {
            params.push(TagValue {
                tag: "pctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = builder.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.no_take_liq {
            params.push(TagValue {
                tag: "noTakeLiq".to_string(),
                value: bool_param(v),
            });
        }

        AlgoParams {
            strategy: "PctVol".to_string(),
            params,
        }
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
///     .build();
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

    /// Set maximum participation rate, clamped to 10-50% per IB requirements.
    pub fn max_pct_vol(mut self, pct: f64) -> Self {
        self.max_pct_vol = Some(pct.clamp(MIN_PCT_VOL, MAX_PCT_VOL));
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
    pub fn build(self) -> AlgoParams {
        self.into()
    }
}

impl From<ArrivalPriceBuilder> for AlgoParams {
    fn from(builder: ArrivalPriceBuilder) -> Self {
        let mut params = Vec::new();

        if let Some(v) = builder.max_pct_vol {
            params.push(TagValue {
                tag: "maxPctVol".to_string(),
                value: v.to_string(),
            });
        }
        if let Some(v) = builder.risk_aversion {
            params.push(TagValue {
                tag: "riskAversion".to_string(),
                value: v.as_str().to_string(),
            });
        }
        if let Some(v) = builder.start_time {
            params.push(TagValue {
                tag: "startTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.end_time {
            params.push(TagValue {
                tag: "endTime".to_string(),
                value: v,
            });
        }
        if let Some(v) = builder.force_completion {
            params.push(TagValue {
                tag: "forceCompletion".to_string(),
                value: bool_param(v),
            });
        }
        if let Some(v) = builder.allow_past_end_time {
            params.push(TagValue {
                tag: "allowPastEndTime".to_string(),
                value: bool_param(v),
            });
        }

        AlgoParams {
            strategy: "ArrivalPx".to_string(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algo_params_from_string() {
        let params: AlgoParams = "Vwap".into();
        assert_eq!(params.strategy, "Vwap");
        assert!(params.params.is_empty());
    }

    #[test]
    fn test_vwap_builder() {
        let params: AlgoParams = VwapBuilder::new()
            .max_pct_vol(0.2)
            .start_time("09:00:00 US/Eastern")
            .end_time("16:00:00 US/Eastern")
            .allow_past_end_time(true)
            .no_take_liq(true)
            .speed_up(true)
            .build();

        assert_eq!(params.strategy, "Vwap");
        assert_eq!(params.params.len(), 6);

        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("maxPctVol"), Some(&"0.2".to_string()));
        assert_eq!(find_param("startTime"), Some(&"09:00:00 US/Eastern".to_string()));
        assert_eq!(find_param("endTime"), Some(&"16:00:00 US/Eastern".to_string()));
        assert_eq!(find_param("allowPastEndTime"), Some(&"1".to_string()));
        assert_eq!(find_param("noTakeLiq"), Some(&"1".to_string()));
        assert_eq!(find_param("speedUp"), Some(&"1".to_string()));
    }

    #[test]
    fn test_twap_builder() {
        let params: AlgoParams = TwapBuilder::new()
            .strategy_type(TwapStrategyType::MatchingMidpoint)
            .start_time("09:00:00 US/Eastern")
            .end_time("16:00:00 US/Eastern")
            .allow_past_end_time(false)
            .build();

        assert_eq!(params.strategy, "Twap");
        assert_eq!(params.params.len(), 4);

        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("strategyType"), Some(&"Matching Midpoint".to_string()));
        assert_eq!(find_param("allowPastEndTime"), Some(&"0".to_string()));
    }

    #[test]
    fn test_pct_vol_builder() {
        let params: AlgoParams = PctVolBuilder::new()
            .pct_vol(0.15)
            .start_time("09:30:00 US/Eastern")
            .end_time("15:30:00 US/Eastern")
            .no_take_liq(false)
            .build();

        assert_eq!(params.strategy, "PctVol");
        assert_eq!(params.params.len(), 4);

        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("pctVol"), Some(&"0.15".to_string()));
        assert_eq!(find_param("noTakeLiq"), Some(&"0".to_string()));
    }

    #[test]
    fn test_arrival_price_builder() {
        let params: AlgoParams = ArrivalPriceBuilder::new()
            .max_pct_vol(0.1)
            .risk_aversion(RiskAversion::Aggressive)
            .start_time("09:00:00 US/Eastern")
            .end_time("16:00:00 US/Eastern")
            .force_completion(true)
            .allow_past_end_time(true)
            .build();

        assert_eq!(params.strategy, "ArrivalPx");
        assert_eq!(params.params.len(), 6);

        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("riskAversion"), Some(&"Aggressive".to_string()));
        assert_eq!(find_param("forceCompletion"), Some(&"1".to_string()));
    }

    #[test]
    fn test_builder_minimal() {
        // Test that builders work with no params set
        let vwap: AlgoParams = VwapBuilder::new().build();
        assert_eq!(vwap.strategy, "Vwap");
        assert!(vwap.params.is_empty());

        let twap: AlgoParams = TwapBuilder::new().build();
        assert_eq!(twap.strategy, "Twap");
        assert!(twap.params.is_empty());
    }

    #[test]
    fn test_pct_vol_clamped_to_range() {
        // Values above 0.5 should be capped at max
        let params: AlgoParams = PctVolBuilder::new().pct_vol(0.8).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("pctVol"), Some(&"0.5".to_string()));

        let params: AlgoParams = VwapBuilder::new().max_pct_vol(1.0).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("maxPctVol"), Some(&"0.5".to_string()));

        // Values below 0.1 should be raised to min
        let params: AlgoParams = PctVolBuilder::new().pct_vol(0.05).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("pctVol"), Some(&"0.1".to_string()));

        let params: AlgoParams = ArrivalPriceBuilder::new().max_pct_vol(0.01).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("maxPctVol"), Some(&"0.1".to_string()));
    }

    #[test]
    fn test_pct_vol_valid_values_unchanged() {
        // Values within 0.1-0.5 should pass through unchanged
        let params: AlgoParams = PctVolBuilder::new().pct_vol(0.25).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("pctVol"), Some(&"0.25".to_string()));

        let params: AlgoParams = VwapBuilder::new().max_pct_vol(0.1).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("maxPctVol"), Some(&"0.1".to_string()));

        let params: AlgoParams = VwapBuilder::new().max_pct_vol(0.5).build();
        let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
        assert_eq!(find_param("maxPctVol"), Some(&"0.5".to_string()));
    }
}
