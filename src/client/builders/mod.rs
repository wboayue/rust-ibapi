//! Builder patterns for request and subscription creation
//!
//! This module provides unified builder patterns that work with both sync and async modes.

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

mod common;

// Re-export builders based on feature selection
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync::{ClientRequestBuilders, SubscriptionBuilderExt};
}
