//! Client implementation with sync/async support

pub(crate) mod builders;
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
    pub use crate::client::builders::client_builder::sync_impl::ClientBuilder;
    pub use crate::subscriptions::notice_stream::sync_impl::{NoticeStream, NoticeStreamIter};
    pub use crate::subscriptions::sync::{
        SharesChannel, Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter,
    };
}

// Re-export the appropriate Client based on feature selection
#[cfg(feature = "async")]
pub use r#async::Client;
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Client;

// Top-level ClientBuilder alias prefers async (matches the Client alias).
#[cfg(feature = "async")]
pub use crate::client::builders::client_builder::async_impl::ClientBuilder;
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use crate::client::builders::client_builder::sync_impl::ClientBuilder;

#[cfg(feature = "sync")]
pub(crate) use crate::subscriptions::StreamDecoder;

#[cfg(feature = "sync")]
pub use crate::subscriptions::sync::SharesChannel;

#[cfg(feature = "async")]
pub(crate) use builders::r#async::{ClientRequestBuilders, SubscriptionBuilderExt};
