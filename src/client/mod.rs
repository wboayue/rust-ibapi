//! Client implementation with sync/async support

pub(crate) mod builders;
pub(crate) mod error_handler;
pub(crate) mod id_generator;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate Client based on feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Client;

#[cfg(feature = "async")]
pub use r#async::Client;

// Re-export subscription types from subscriptions module
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use crate::subscriptions::{SharesChannel, Subscription};

#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) use crate::subscriptions::{ResponseContext, StreamDecoder};

#[cfg(feature = "async")]
pub use crate::subscriptions::Subscription;

// Re-export builder traits (internal use only)
pub(crate) use builders::{ClientRequestBuilders, SubscriptionBuilderExt};
