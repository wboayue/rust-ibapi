//! Client implementation with sync/async support

pub(crate) mod builders;
pub(crate) mod common;
pub(crate) mod error_handler;
pub(crate) mod id_generator;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

/// Blocking client bindings for synchronous workflows.
#[cfg(feature = "sync")]
pub mod blocking {
    pub use super::sync::Client;
    pub(crate) use crate::client::builders::blocking::{ClientRequestBuilders, SubscriptionBuilderExt};
    pub use crate::subscriptions::sync::{
        SharesChannel, Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter,
    };
}

// Re-export the appropriate Client based on feature selection
#[cfg(feature = "async")]
pub use r#async::Client;
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Client;

#[cfg(feature = "sync")]
pub(crate) use crate::subscriptions::StreamDecoder;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use crate::subscriptions::sync::Subscription;

#[cfg(feature = "async")]
pub use crate::subscriptions::r#async::Subscription;

#[cfg(feature = "sync")]
pub use crate::subscriptions::sync::SharesChannel;

#[cfg(all(feature = "sync", feature = "async"))]
pub(crate) use builders::r#async::{ClientRequestBuilders, SubscriptionBuilderExt};

#[cfg(all(not(feature = "sync"), feature = "async"))]
pub(crate) use builders::r#async::{ClientRequestBuilders, SubscriptionBuilderExt};
