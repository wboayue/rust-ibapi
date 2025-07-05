//! Client implementation with sync/async support

pub(crate) mod error_handler;
pub(crate) mod id_generator;
pub(crate) mod request_builder;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate Client based on feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::Client;

#[cfg(feature = "async")]
pub use r#async::Client;

// Re-export subscription types from subscriptions module
#[cfg(feature = "sync")]
pub use crate::subscriptions::{SharesChannel, Subscription};

#[cfg(feature = "sync")]
pub(crate) use crate::subscriptions::{DataStream, ResponseContext};

// Re-export request builder traits
pub(crate) use request_builder::ClientRequestBuilders;
