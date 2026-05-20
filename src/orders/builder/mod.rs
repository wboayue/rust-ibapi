pub mod algo_builders;
pub mod algo_helpers;
pub mod condition_helpers;
pub(crate) mod order_builder;
pub(crate) mod types;
mod validation;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync_impl;

#[cfg(feature = "async")]
mod async_impl;

#[cfg(test)]
mod tests;

pub use algo_builders::{
    AccuDistrBuilder, AccumulateDistributeBuilder, AdaptiveBuilder, AdaptivePriority, AlgoParams, ArrivalPriceBuilder, BalanceImpactRiskBuilder,
    ClosePriceBuilder, DarkIceBuilder, MinimiseImpactBuilder, PctVolBuilder, PctVolPriceBuilder, PctVolSizeBuilder, PctVolTimeBuilder, RiskAversion,
    TwapBuilder, TwapStrategyType, VwapBuilder,
};
pub use algo_helpers::{
    accu_distr, accumulate_distribute, adaptive, arrival_price, balance_impact_risk, close_price, dark_ice, minimise_impact, pct_vol, pct_vol_price,
    pct_vol_size, pct_vol_time, twap, vwap,
};
pub use condition_helpers::{execution, margin, percent_change, price, time, volume};
pub use types::{AuctionType, OrderAnalysis, OrderType, Price, Quantity, TimeInForce, ValidationError};
