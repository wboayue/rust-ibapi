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

pub use order_builder::{BracketOrderBuilder, OrderBuilder};
pub use types::{AuctionType, BracketOrderIds, OrderAnalysis, OrderId, OrderType, Price, Quantity, TimeInForce, ValidationError};
