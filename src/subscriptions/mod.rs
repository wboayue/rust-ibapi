//! Subscription types for sync/async streaming data

mod common;
pub(crate) use common::{ResponseContext, StreamDecoder};

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate subscription types based on feature
#[cfg(feature = "sync")]
pub use sync::{SharesChannel, Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter};

#[cfg(feature = "async")]
pub use r#async::Subscription;
