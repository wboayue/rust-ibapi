//! Subscription types for sync/async streaming data

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate subscription types based on feature
#[cfg(feature = "sync")]
pub use sync::Subscription;

#[cfg(feature = "async")]
pub use r#async::AsyncSubscription;