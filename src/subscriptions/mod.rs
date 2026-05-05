//! Subscription types for sync/async streaming data

pub(crate) mod common;
pub use common::SubscriptionItem;
pub(crate) use common::{DecoderContext, StreamDecoder};

#[cfg(feature = "sync")]
pub mod sync;

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
pub use r#async::Subscription;
