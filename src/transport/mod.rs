//! Transport layer for TWS communication with sync/async support

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

#[cfg(feature = "sync")]
pub use sync::TcpMessageBus;

#[cfg(feature = "sync")]
pub(crate) use sync::{Connection, ConnectionMetadata, InternalSubscription, MessageBus, Response, TcpSocket};

// These are used in tests
#[cfg(all(feature = "sync", test))]
pub(crate) use sync::{read_message, Io, Reconnect, Stream, SubscriptionBuilder, MAX_RETRIES};

#[cfg(feature = "async")]
pub use r#async::{AsyncInternalSubscription, AsyncMessageBus};

pub mod connection;
pub mod recorder;

// Re-export message types from crate::messages
pub(crate) use crate::messages::{RequestMessage, ResponseMessage};
