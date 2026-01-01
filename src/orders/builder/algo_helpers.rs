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

use crate::orders::builder::algo_builders::{ArrivalPriceBuilder, PctVolBuilder, TwapBuilder, VwapBuilder};

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
///     .build();
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
///     .build();
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
///     .build();
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
///     .build();
/// ```
pub fn arrival_price() -> ArrivalPriceBuilder {
    ArrivalPriceBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::builder::algo_builders::AlgoParams;

    #[test]
    fn test_vwap_helper() {
        let algo: AlgoParams = vwap().max_pct_vol(0.2).build();
        assert_eq!(algo.strategy, "Vwap");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_twap_helper() {
        let algo: AlgoParams = twap().start_time("09:00:00 US/Eastern").build();
        assert_eq!(algo.strategy, "Twap");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_pct_vol_helper() {
        let algo: AlgoParams = pct_vol().pct_vol(0.15).build();
        assert_eq!(algo.strategy, "PctVol");
        assert_eq!(algo.params.len(), 1);
    }

    #[test]
    fn test_arrival_price_helper() {
        let algo: AlgoParams = arrival_price().max_pct_vol(0.1).build();
        assert_eq!(algo.strategy, "ArrivalPx");
        assert_eq!(algo.params.len(), 1);
    }
}
