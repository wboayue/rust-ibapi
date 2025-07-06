//! Client implementation with sync/async support

pub(crate) mod builders;
pub(crate) mod error_handler;
pub(crate) mod id_generator;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate Client based on feature
#[cfg(feature = "sync")]
pub use sync::Client;

#[cfg(feature = "async")]
pub use r#async::Client;

// Re-export subscription types from subscriptions module
#[cfg(feature = "sync")]
pub use crate::subscriptions::{SharesChannel, Subscription};

#[cfg(feature = "sync")]
pub(crate) use crate::subscriptions::sync::{DataStream, ResponseContext};

#[cfg(feature = "async")]
pub use crate::subscriptions::Subscription;

// Re-export builder traits (internal use only)
pub(crate) use builders::{ClientRequestBuilders, SubscriptionBuilderExt};
