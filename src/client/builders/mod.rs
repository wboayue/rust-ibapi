//! Builder patterns for request and subscription creation
//!
//! This module provides unified builder patterns that work with both sync and async modes.

#[cfg(all(feature = "sync", not(feature = "async")))]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

mod common;

// Re-export builders based on feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{ClientRequestBuilders, SubscriptionBuilderExt};

#[cfg(feature = "async")]
pub use r#async::{ClientRequestBuilders, SubscriptionBuilderExt};

// Re-export ResponseContext from common module
pub use common::ResponseContext;
