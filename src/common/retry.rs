//! Retry logic patterns for handling connection resets on one-shot operations
//!
//! These utilities provide retry functionality for operations that can be safely retried
//! without losing server-side state (e.g., managed_accounts, server_time).
//! Do NOT use for subscriptions or stateful operations.
//!
//! A retry waits for the reconnect the reset announces, up to the caller's
//! deadline, before its next attempt (the bus refuses sends while the
//! session is down, with `Error::Reconnecting`, so retrying straight away
//! would spend every attempt on a request that never reaches TWS).

use crate::Error;

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Whether `error` ends an attempt the retry tries again: a reset (`ConnectionReset`, the
/// attempt may have been written) or a refusal while reconnecting (`Reconnecting`, it wasn't).
fn is_retryable(error: &Error) -> bool {
    matches!(error, Error::ConnectionReset | Error::Reconnecting)
}

/// The error a retry ends with. A last attempt refused while reconnecting wrote nothing, but an
/// earlier attempt that ended in a reset may have reached TWS, so the call as a whole reports
/// [`Error::ConnectionReset`] rather than claim nothing was sent.
fn final_error(error: Error, earlier_written: bool) -> Error {
    match error {
        Error::Reconnecting if earlier_written => Error::ConnectionReset,
        other => other,
    }
}

// Sync implementations
#[cfg(feature = "sync")]
mod sync_impl {
    use super::*;
    use std::time::Instant;

    /// What a retry waits on between attempts.
    pub trait ReconnectWaiter {
        /// Block until the session is connected again. Returns
        /// `Error::ConnectionFailed` (a reconnect that gave up) or
        /// `Error::Shutdown` (during shutdown) if it never will be; the retry
        /// passes that on as-is, since a
        /// `ConnectionReset` would invite the caller to retry a client that is gone.
        /// Returns `Error::Timeout` if `deadline` passes first.
        fn wait_connected(&self, deadline: Option<Instant>) -> Result<(), Error>;
    }

    impl ReconnectWaiter for crate::client::sync::Client {
        fn wait_connected(&self, deadline: Option<Instant>) -> Result<(), Error> {
            self.message_bus.wait_connected(deadline)
        }
    }

    /// Retry logic for sync one-shot operations with configurable retry limit.
    /// `deadline` bounds each wait for the reconnect.
    pub fn retry_on_connection_reset_with_limit<T, F>(
        waiter: &impl ReconnectWaiter,
        deadline: Option<Instant>,
        mut operation: F,
        max_retries: u32,
    ) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        let mut attempts = 0;
        let mut earlier_written = false;
        loop {
            match operation() {
                Err(error) if is_retryable(&error) && attempts < max_retries => {
                    attempts += 1;
                    earlier_written |= matches!(error, Error::ConnectionReset);
                    // The send gate refuses a retry until the replacement
                    // connection is ready, so wait for it first.
                    waiter.wait_connected(deadline)?;
                    continue;
                }
                Err(error) => return Err(final_error(error, earlier_written)),
                ok => return ok,
            }
        }
    }

    /// Retry logic for sync one-shot operations with default retry limit.
    /// `deadline` bounds each wait for the reconnect.
    pub fn retry_on_connection_reset<T, F>(waiter: &impl ReconnectWaiter, deadline: Option<Instant>, operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        retry_on_connection_reset_with_limit(waiter, deadline, operation, DEFAULT_MAX_RETRIES)
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_impl {
    use super::*;
    use async_trait::async_trait;
    use futures::Future;
    use tokio::time::Instant;

    /// What a retry waits on between attempts.
    #[async_trait]
    pub trait ReconnectWaiter: Sync {
        /// Resolve once the session is connected again. Returns
        /// `Error::ConnectionFailed` (a reconnect that gave up) or
        /// `Error::Shutdown` (during shutdown) if it never will be; the retry
        /// passes that on as-is, since a
        /// `ConnectionReset` would invite the caller to retry a client that is gone.
        /// Returns `Error::Timeout` if `deadline` passes first.
        async fn wait_connected(&self, deadline: Option<Instant>) -> Result<(), Error>;
    }

    #[async_trait]
    impl ReconnectWaiter for crate::client::r#async::Client {
        async fn wait_connected(&self, deadline: Option<Instant>) -> Result<(), Error> {
            self.message_bus.wait_connected(deadline).await
        }
    }

    /// Retry logic for async one-shot operations with configurable retry limit.
    /// `deadline` bounds each wait for the reconnect.
    pub async fn retry_on_connection_reset_with_limit<T, F, Fut>(
        waiter: &impl ReconnectWaiter,
        deadline: Option<Instant>,
        mut operation: F,
        max_retries: u32,
    ) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let mut attempts = 0;
        let mut earlier_written = false;
        loop {
            match operation().await {
                Err(error) if is_retryable(&error) && attempts < max_retries => {
                    attempts += 1;
                    earlier_written |= matches!(error, Error::ConnectionReset);
                    // The send gate refuses a retry until the replacement
                    // connection is ready, so wait for it first.
                    waiter.wait_connected(deadline).await?;
                    continue;
                }
                Err(error) => return Err(final_error(error, earlier_written)),
                ok => return ok,
            }
        }
    }

    /// Retry logic for async one-shot operations with default retry limit.
    /// `deadline` bounds each wait for the reconnect.
    pub async fn retry_on_connection_reset<T, F, Fut>(waiter: &impl ReconnectWaiter, deadline: Option<Instant>, operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        retry_on_connection_reset_with_limit(waiter, deadline, operation, DEFAULT_MAX_RETRIES).await
    }
}

// Re-export based on feature flags
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync_impl::*;
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub use sync_impl::*;

#[cfg(feature = "async")]
pub use async_impl::*;

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
