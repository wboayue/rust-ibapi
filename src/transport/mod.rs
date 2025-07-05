//! Transport layer for TWS communication with sync/async support

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

#[cfg(feature = "sync")]
pub use sync::TcpMessageBus;

#[cfg(feature = "sync")]
pub(crate) use sync::{MessageBus, InternalSubscription, Response, Connection, ConnectionMetadata, TcpSocket};

// These are used in tests
#[cfg(all(feature = "sync", test))]
pub(crate) use sync::{Stream, Io, Reconnect, SubscriptionBuilder, read_message, MAX_RETRIES};

#[cfg(feature = "async")]
pub use r#async::{AsyncMessageBus, AsyncInternalSubscription};

pub mod connection;
pub mod recorder;

// Re-export message types from crate::messages
pub(crate) use crate::messages::{RequestMessage, ResponseMessage};