//! Connection management for TWS communication

use time::OffsetDateTime;
use time_tz::Tz;

pub mod common;

// Re-export StartupMessageCallback for lib.rs to re-export publicly
pub use common::StartupMessageCallback;

/// Metadata about the connection to TWS
#[derive(Default, Clone, Debug)]
pub struct ConnectionMetadata {
    /// Next order ID to use for placing orders
    pub next_order_id: i32,
    /// Client ID for this connection
    pub client_id: i32,
    /// Server version (TWS version)
    pub server_version: i32,
    /// Comma-separated list of managed accounts
    pub managed_accounts: String,
    /// Connection time
    pub connection_time: Option<OffsetDateTime>,
    /// Server time zone
    pub time_zone: Option<&'static Tz>,
}

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;
