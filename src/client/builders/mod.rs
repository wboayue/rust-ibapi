//! Builder patterns for request and subscription creation
//!
//! This module provides unified builder patterns that work with both sync and async modes.

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

mod common;

// Re-export builders based on feature
#[cfg(feature = "sync")]
pub use sync::{ClientRequestBuilders, SubscriptionBuilderExt};

#[cfg(feature = "async")]
pub use r#async::{ClientRequestBuilders, SubscriptionBuilderExt};
