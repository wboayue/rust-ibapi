pub mod algo_builders;
pub mod algo_helpers;
pub mod condition_helpers;
mod order_builder;
mod types;
mod validation;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync_impl;

#[cfg(feature = "async")]
mod async_impl;

#[cfg(test)]
mod tests;

pub use algo_builders::{AlgoParams, ArrivalPriceBuilder, PctVolBuilder, RiskAversion, TwapBuilder, TwapStrategyType, VwapBuilder};
pub use algo_helpers::{arrival_price, pct_vol, twap, vwap};
pub use condition_helpers::{execution, margin, percent_change, price, time, volume};
pub use order_builder::{BracketOrderBuilder, OrderBuilder};
pub use types::{AuctionType, BracketOrderIds, OrderAnalysis, OrderId, OrderType, Price, Quantity, TimeInForce, ValidationError};
