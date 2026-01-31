//! Subscription types for sync/async streaming data

mod common;
pub(crate) use common::{DecoderContext, StreamDecoder};

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate subscription types based on feature
#[cfg(feature = "sync")]
pub use sync::{SharesChannel, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter};

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Subscription;

#[cfg(feature = "async")]
pub use r#async::Subscription;
