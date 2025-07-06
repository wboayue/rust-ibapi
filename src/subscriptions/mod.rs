//! Subscription types for sync/async streaming data

mod common;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate subscription types based on feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{SharesChannel, Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter};

#[cfg(feature = "async")]
pub use r#async::Subscription;
