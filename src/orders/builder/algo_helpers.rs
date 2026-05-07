//! Helper functions for creating algorithmic order strategies with a fluent API.
//!
//! This module provides ergonomic helper functions that return algo builders,
//! enabling a fluent API for configuring algo orders.
//!
//! # Example
//!
//! ```no_run
//! use ibapi::orders::builder::{vwap, twap, pct_vol, arrival_price};
//!
//! // Create algo params using helper functions
//! let vwap_algo = vwap()
//!     .max_pct_vol(0.2)
//!     .start_time("09:00:00 US/Eastern")
//!     .end_time("16:00:00 US/Eastern");
//!
//! let twap_algo = twap()
//!     .start_time("09:00:00 US/Eastern")
//!     .end_time("16:00:00 US/Eastern");
//! ```

use crate::orders::builder::algo_builders::{
    AccuDistrBuilder, AccumulateDistributeBuilder, AdaptiveBuilder, ArrivalPriceBuilder, BalanceImpactRiskBuilder, ClosePriceBuilder, DarkIceBuilder,
    MinimiseImpactBuilder, PctVolBuilder, PctVolPriceBuilder, PctVolSizeBuilder, PctVolTimeBuilder, TwapBuilder, VwapBuilder,
};

/// Create a VWAP (Volume Weighted Average Price) algo builder.
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
pub fn vwap() -> VwapBuilder {
    VwapBuilder::new()
}

/// Create a TWAP (Time Weighted Average Price) algo builder.
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
pub fn twap() -> TwapBuilder {
    TwapBuilder::new()
}

/// Create a Percentage of Volume (PctVol) algo builder.
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
pub fn pct_vol() -> PctVolBuilder {
    PctVolBuilder::new()
}

/// Create an Arrival Price algo builder.
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
pub fn arrival_price() -> ArrivalPriceBuilder {
    ArrivalPriceBuilder::new()
}

/// Create an Adaptive algo builder.
///
/// Combines IB's Smart Routing with user-defined urgency.
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
pub fn adaptive() -> AdaptiveBuilder {
    AdaptiveBuilder::new()
}

/// Create a Close Price (ClosePx) algo builder.
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
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn close_price() -> ClosePriceBuilder {
    ClosePriceBuilder::new()
}

/// Create a Dark Ice algo builder.
///
/// Hidden order with randomized display sizes.
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
pub fn dark_ice() -> DarkIceBuilder {
    DarkIceBuilder::new()
}

/// Create an Accumulate/Distribute (AD) algo builder.
///
/// Slices an order into random increments at random intervals.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::accumulate_distribute;
///
/// let algo = accumulate_distribute()
///     .component_size(100)
///     .time_between_orders(60)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn accumulate_distribute() -> AccumulateDistributeBuilder {
    AccumulateDistributeBuilder::new()
}

/// Create a Balance Impact Risk algo builder.
///
/// Balances market impact against adverse-price-movement risk.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::{balance_impact_risk, RiskAversion};
///
/// let algo = balance_impact_risk()
///     .max_pct_vol(0.2)
///     .risk_aversion(RiskAversion::Neutral)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn balance_impact_risk() -> BalanceImpactRiskBuilder {
    BalanceImpactRiskBuilder::new()
}

/// Create a Minimise Impact (MinImpact) algo builder.
///
/// Slices the order to match market average while minimizing impact.
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
pub fn minimise_impact() -> MinimiseImpactBuilder {
    MinimiseImpactBuilder::new()
}

/// Create a Price Variant Percentage of Volume (PctVolPx) algo builder.
///
/// Participation rate varies with the market price.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_price;
///
/// let algo = pct_vol_price()
///     .pct_vol(0.1)
///     .delta_pct_vol(0.05)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn pct_vol_price() -> PctVolPriceBuilder {
    PctVolPriceBuilder::new()
}

/// Create a Size Variant Percentage of Volume (PctVolSz) algo builder.
///
/// Participation rate varies based on remaining order size.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_size;
///
/// let algo = pct_vol_size()
///     .start_pct_vol(0.1)
///     .end_pct_vol(0.4)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn pct_vol_size() -> PctVolSizeBuilder {
    PctVolSizeBuilder::new()
}

/// Create a Time Variant Percentage of Volume (PctVolTm) algo builder.
///
/// Participation rate varies linearly over the active time window.
///
/// # Example
///
/// ```no_run
/// use ibapi::orders::builder::pct_vol_time;
///
/// let algo = pct_vol_time()
///     .start_pct_vol(0.1)
///     .end_pct_vol(0.4)
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn pct_vol_time() -> PctVolTimeBuilder {
    PctVolTimeBuilder::new()
}

/// Create an AccuDistr algo builder.
///
/// AccuDistr is distinct from `AD` (Accumulate/Distribute) and exposes a
/// smaller set of parameters with explicit `route_order_type` and
/// `active_time_tz`. See [`AccuDistrBuilder`] for the parameter shape.
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
///     .build()?;
/// # Ok::<(), ibapi::orders::builder::ValidationError>(())
/// ```
pub fn accu_distr() -> AccuDistrBuilder {
    AccuDistrBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::builder::algo_builders::AlgoParams;

    #[test]
    fn test_vwap_helper() {
        let algo: AlgoParams = vwap().max_pct_vol(0.2).build().unwrap();
        assert_eq!(algo.strategy, "Vwap");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_twap_helper() {
        let algo: AlgoParams = twap().start_time("09:00:00 US/Eastern").build().unwrap();
        assert_eq!(algo.strategy, "Twap");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_pct_vol_helper() {
        let algo: AlgoParams = pct_vol().pct_vol(0.15).build().unwrap();
        assert_eq!(algo.strategy, "PctVol");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_arrival_price_helper() {
        let algo: AlgoParams = arrival_price().max_pct_vol(0.1).build().unwrap();
        assert_eq!(algo.strategy, "ArrivalPx");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_adaptive_helper() {
        use crate::orders::builder::AdaptivePriority;
        let algo: AlgoParams = adaptive().priority(AdaptivePriority::Urgent).build().unwrap();
        assert_eq!(algo.strategy, "Adaptive");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_close_price_helper() {
        let algo: AlgoParams = close_price().max_pct_vol(0.2).build().unwrap();
        assert_eq!(algo.strategy, "ClosePx");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_dark_ice_helper() {
        let algo: AlgoParams = dark_ice().display_size(100).build().unwrap();
        assert_eq!(algo.strategy, "DarkIce");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_accumulate_distribute_helper() {
        let algo: AlgoParams = accumulate_distribute().component_size(100).build().unwrap();
        assert_eq!(algo.strategy, "AD");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_balance_impact_risk_helper() {
        let algo: AlgoParams = balance_impact_risk().max_pct_vol(0.2).build().unwrap();
        assert_eq!(algo.strategy, "BalanceImpactRisk");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_minimise_impact_helper() {
        let algo: AlgoParams = minimise_impact().max_pct_vol(0.2).build().unwrap();
        assert_eq!(algo.strategy, "MinImpact");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_pct_vol_price_helper() {
        let algo: AlgoParams = pct_vol_price().pct_vol(0.15).build().unwrap();
        assert_eq!(algo.strategy, "PctVolPx");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_pct_vol_size_helper() {
        let algo: AlgoParams = pct_vol_size().start_pct_vol(0.1).build().unwrap();
        assert_eq!(algo.strategy, "PctVolSz");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_pct_vol_time_helper() {
        let algo: AlgoParams = pct_vol_time().start_pct_vol(0.1).build().unwrap();
        assert_eq!(algo.strategy, "PctVolTm");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_accu_distr_helper() {
        let algo: AlgoParams = accu_distr().component_size(100).build().unwrap();
        assert_eq!(algo.strategy, "AccuDistr");
        assert_eq!(algo.params.len(), 1);
    }
}
