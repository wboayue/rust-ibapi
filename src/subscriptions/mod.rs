//! Subscription types for sync/async streaming data.
//!
//! ## Canonical paths
//!
//! - **Async `Subscription` / extensions** — `ibapi::Subscription`,
//!   `ibapi::subscriptions::SubscriptionItemStreamExt`. The crate-root and
//!   `subscriptions::*` re-exports resolve to the async implementation
//!   whenever the `async` feature is on (which is the default).
//! - **Blocking (sync) `Subscription` / iterators** — `ibapi::client::blocking::Subscription`
//!   (and `SubscriptionIter`, `SubscriptionOwnedIter`, etc.). The labelled
//!   `blocking` submodule is the canonical sync-explicit path. When only
//!   `sync` is enabled, `ibapi::Subscription` also resolves to the blocking
//!   form.
//!
//! The `subscriptions::sync` and `subscriptions::r#async` submodules where
//! the impls live are `#[doc(hidden)]`: still reachable as paths for
//! crate-internal use, but intentionally absent from the docs.rs navigation.
//! Prefer the canonical spellings above. Raw-identifier syntax
//! (`subscriptions::r#async::Subscription`) is the giveaway that the spelling
//! is non-canonical.

pub(crate) mod common;
pub use common::SubscriptionItem;
pub(crate) use common::{DecoderContext, StreamDecoder};

#[doc(hidden)]
#[cfg(feature = "sync")]
pub mod sync;

#[doc(hidden)]
#[cfg(feature = "async")]
pub mod r#async;

pub(crate) mod notice_stream;
#[cfg(feature = "sync")]
pub use notice_stream::sync_impl::NoticeStreamIter;

// Top-level `NoticeStream` mirrors the `Subscription` policy: prefer the async
// implementation when both features are enabled. The sync version is also
// available at `client::blocking::NoticeStream`.
#[cfg(feature = "async")]
pub use notice_stream::async_impl::NoticeStream;
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use notice_stream::sync_impl::NoticeStream;

// Re-export the appropriate subscription types based on feature
#[cfg(feature = "sync")]
pub use sync::{
    FilterData, SharesChannel, SubscriptionItemIterExt, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter,
};

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Subscription;

#[cfg(feature = "async")]
pub use r#async::{FilterDataStream, Subscription, SubscriptionItemStreamExt};
