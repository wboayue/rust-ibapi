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
pub(crate) mod id_generator;
pub(crate) mod ids;

#[doc(hidden)]
#[cfg(feature = "sync")]
pub mod sync;

#[doc(hidden)]
#[cfg(feature = "async")]
pub mod r#async;

/// Blocking client bindings for synchronous workflows.
///
/// Also the path for the blocking versions of dual-feature types (subscriptions
/// and their iterators) whose top-level name is the async type when both the
/// `sync` and `async` features are enabled.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::blocking::{Client, DisplayGroupSubscription, TickSubscription};
/// use ibapi::contracts::Contract;
/// use ibapi::market_data::historical::TickLast;
///
/// # fn main() -> Result<(), ibapi::Error> {
/// let client = Client::connect("127.0.0.1:4002", 100)?;
/// let contract = Contract::stock("TSLA").build();
///
/// let ticks: TickSubscription<TickLast> = client.historical_ticks(&contract, 100).trade()?;
/// for tick in ticks.iter_data() {
///     println!("{:?}", tick?);
/// }
///
/// let group: DisplayGroupSubscription = client.subscribe_to_group_events(1)?;
/// for update in group.iter_data() {
///     println!("{:?}", update?);
/// }
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "sync")]
pub mod blocking {
    pub use super::sync::Client;
    pub use crate::accounts::sync::AccountSummarySnapshots;
    pub(crate) use crate::client::builders::blocking::{ClientRequestBuilders, SubscriptionBuilderExt};
    pub use crate::client::builders::client_builder::sync_impl::ClientBuilder;
    pub use crate::display_groups::sync::DisplayGroupSubscription;
    pub use crate::market_data::historical::sync::{
        TickSubscription, TickSubscriptionIter, TickSubscriptionOwnedIter, TickSubscriptionTimeoutIter, TickSubscriptionTryIter,
    };
    pub use crate::subscriptions::notice_stream::sync_impl::{NoticeStream, NoticeStreamIter};
    pub use crate::subscriptions::sync::{Subscription, SubscriptionIter, SubscriptionOwnedIter, SubscriptionTimeoutIter, SubscriptionTryIter};
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
