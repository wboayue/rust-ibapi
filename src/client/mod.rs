//! Client implementation with sync/async support

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;


// Re-export the appropriate Client based on feature
#[cfg(feature = "sync")]
pub use sync::{Client, Subscription, SharesChannel};

#[cfg(feature = "sync")]
pub(crate) use sync::{DataStream, ResponseContext};

#[cfg(feature = "async")]
pub use r#async::Client;