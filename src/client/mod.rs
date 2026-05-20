//! Client implementation with sync/async support.
//!
//! ## Canonical paths
//!
//! Two canonical spellings for the `Client` type, depending on which client
//! you want and which features are enabled:
//!
//! - **Async client** — `ibapi::Client`. The crate-root `Client` re-export
//!   resolves to the async client whenever the `async` feature is on (which
//!   is the default).
//! - **Blocking (sync) client** — `ibapi::client::blocking::Client`. The
//!   labelled `blocking` submodule is the canonical sync-explicit path. Use
//!   it whenever both `sync` and `async` features are enabled, since the root
//!   `ibapi::Client` resolves to async in that configuration. When only `sync`
//!   is enabled, `ibapi::Client` also resolves to the blocking client.
//!
//! The `client::sync` and `client::r#async` submodules where the impls live
//! are `#[doc(hidden)]`: still reachable as paths for crate-internal use, but
//! intentionally absent from the docs.rs navigation. Prefer the root `Client`
//! / `client::blocking::Client` spellings in user code, examples, and docs.
//! Raw-identifier syntax (`client::r#async::Client`) is the giveaway that the
//! spelling is non-canonical.

pub(crate) mod builders;
pub(crate) mod error_handler;
pub(crate) mod id_generator;

#[doc(hidden)]
#[cfg(feature = "sync")]
pub mod sync;

#[doc(hidden)]
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

#[cfg(feature = "async")]
pub(crate) use builders::r#async::{ClientRequestBuilders, SubscriptionBuilderExt};
