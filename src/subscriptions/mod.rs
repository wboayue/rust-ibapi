//! Subscription types for sync/async streaming data

mod common;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate subscription types based on feature
#[cfg(feature = "sync")]
pub use sync::{SharesChannel, Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter};

#[cfg(feature = "sync")]
pub(crate) use sync::{DataStream, ResponseContext};

#[cfg(feature = "async")]
pub use r#async::Subscription;
