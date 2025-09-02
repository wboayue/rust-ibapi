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

#[cfg(test)]
mod order_builder_tests;

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod final_coverage_tests;

#[cfg(test)]
mod edge_case_tests;

#[cfg(test)]
mod additional_coverage_tests;

#[cfg(test)]
mod bracket_coverage_tests;

#[cfg(test)]
mod uncovered_lines_tests;

#[cfg(test)]
mod mock_client;

#[cfg(all(test, feature = "async"))]
mod async_mock_client;

#[cfg(all(test, feature = "sync"))]
mod sync_tests;

#[cfg(all(test, feature = "sync"))]
mod sync_impl_tests;

#[cfg(all(test, feature = "async"))]
mod async_impl_tests;

pub use order_builder::{BracketOrderBuilder, OrderBuilder};
pub use types::{AuctionType, BracketOrderIds, OrderAnalysis, OrderId, OrderType, Price, Quantity, TimeInForce, ValidationError};
