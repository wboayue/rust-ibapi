//! Retry logic patterns for handling connection resets on one-shot operations
//!
//! These utilities provide retry functionality for operations that can be safely retried
//! without losing server-side state (e.g., managed_accounts, server_time).
//! Do NOT use for subscriptions or stateful operations.
//!
//! A retry waits for the reconnect the reset announces before its next
//! attempt (the bus refuses sends while the session is down, so retrying
//! straight away would spend every attempt on a request that never reaches
//! TWS).

use crate::Error;

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

// Sync implementations
#[cfg(feature = "sync")]
mod sync_retry {
    use super::*;

    /// What a retry waits on between attempts.
    pub trait ReconnectWaiter {
        /// Block until the session is connected again. `Err` means it never
        /// will be (a reconnect that gave up, or during shutdown).
        fn wait_connected(&self) -> Result<(), Error>;
    }

    impl ReconnectWaiter for crate::client::sync::Client {
        fn wait_connected(&self) -> Result<(), Error> {
            self.message_bus.wait_connected()
        }
    }

    /// Retry logic for sync one-shot operations with configurable retry limit
    pub fn retry_on_connection_reset_with_limit<T, F>(waiter: &impl ReconnectWaiter, mut operation: F, max_retries: u32) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        let mut attempts = 0;
        loop {
            match operation() {
                Err(Error::ConnectionReset) if attempts < max_retries => {
                    attempts += 1;
                    // The reset came from a connection that is being replaced.
                    // Retrying before the replacement is ready is blocked by
                    // the send gate, so give up with the reset the caller would
                    // have seen anyway if it never comes.
                    if waiter.wait_connected().is_err() {
                        return Err(Error::ConnectionReset);
                    }
                    continue;
                }
                other => return other,
            }
        }
    }

    /// Retry logic for sync one-shot operations with default retry limit
    pub fn retry_on_connection_reset<T, F>(waiter: &impl ReconnectWaiter, operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        retry_on_connection_reset_with_limit(waiter, operation, DEFAULT_MAX_RETRIES)
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_retry {
    use super::*;
    use async_trait::async_trait;
    use futures::Future;

    /// What a retry waits on between attempts.
    #[async_trait]
    pub trait ReconnectWaiter: Sync {
        /// Resolve once the session is connected again. `Err` means it never
        /// will be (a reconnect that gave up, or during shutdown).
        async fn wait_connected(&self) -> Result<(), Error>;
    }

    #[async_trait]
    impl ReconnectWaiter for crate::client::r#async::Client {
        async fn wait_connected(&self) -> Result<(), Error> {
            self.message_bus.wait_connected().await
        }
    }

    /// Retry logic for async one-shot operations with configurable retry limit
    pub async fn retry_on_connection_reset_with_limit<T, F, Fut>(
        waiter: &impl ReconnectWaiter,
        mut operation: F,
        max_retries: u32,
    ) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let mut attempts = 0;
        loop {
            match operation().await {
                Err(Error::ConnectionReset) if attempts < max_retries => {
                    attempts += 1;
                    // The reset came from a connection that is being replaced.
                    // Retrying before the replacement is ready is blocked by
                    // the send gate, so give up with the reset the caller would
                    // have seen anyway if it never comes.
                    if waiter.wait_connected().await.is_err() {
                        return Err(Error::ConnectionReset);
                    }
                    continue;
                }
                other => return other,
            }
        }
    }

    /// Retry logic for async one-shot operations with default retry limit
    pub async fn retry_on_connection_reset<T, F, Fut>(waiter: &impl ReconnectWaiter, operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        retry_on_connection_reset_with_limit(waiter, operation, DEFAULT_MAX_RETRIES).await
    }
}

// Re-export based on feature flags
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync_retry::*;
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync_retry::*;

#[cfg(feature = "async")]
pub use async_retry::*;

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
