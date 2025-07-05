//! Synchronous transport implementation

pub mod sync_message_bus;

pub use sync_message_bus::TcpMessageBus;
pub(crate) use sync_message_bus::{MessageBus, InternalSubscription, Response, Connection, ConnectionMetadata, TcpSocket};

// These are used in tests and other modules
#[allow(unused_imports)]
pub(crate) use sync_message_bus::{Stream, Io, Reconnect, SubscriptionBuilder, read_message, MAX_RETRIES};