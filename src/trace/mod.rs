//! Server interaction tracing for debugging and monitoring
//!
//! This module provides functionality to capture and retrieve server interactions
//! globally across the application. It supports both sync and async modes.

// Common types and storage
mod common;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags
pub use common::Interaction;

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
/// Blocking tracing helpers that wrap the synchronous client.
pub mod blocking {
    pub use super::sync::{last_interaction, record_request, record_response};
}

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{last_interaction, record_request, record_response};

#[cfg(feature = "async")]
pub use r#async::{last_interaction, record_request, record_response};
