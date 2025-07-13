//! Retry logic patterns for handling connection resets on one-shot operations
//!
//! These utilities provide retry functionality for operations that can be safely retried
//! without losing server-side state (e.g., managed_accounts, server_time).
//! Do NOT use for subscriptions or stateful operations.

use crate::Error;

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

// Sync implementations
#[cfg(all(feature = "sync", not(feature = "async")))]
mod sync_retry {
    use super::*;

    /// Retry logic for sync one-shot operations with configurable retry limit
    pub fn retry_on_connection_reset_with_limit<T, F>(mut operation: F, max_retries: u32) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        let mut attempts = 0;
        loop {
            match operation() {
                Err(Error::ConnectionReset) if attempts < max_retries => {
                    attempts += 1;
                    continue;
                }
                other => return other,
            }
        }
    }

    /// Retry logic for sync one-shot operations with default retry limit
    pub fn retry_on_connection_reset<T, F>(operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Result<T, Error>,
    {
        retry_on_connection_reset_with_limit(operation, DEFAULT_MAX_RETRIES)
    }
}

// Async implementations
#[cfg(feature = "async")]
mod async_retry {
    use super::*;
    use futures::Future;

    /// Retry logic for async one-shot operations with configurable retry limit
    pub async fn retry_on_connection_reset_with_limit<T, F, Fut>(mut operation: F, max_retries: u32) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let mut attempts = 0;
        loop {
            match operation().await {
                Err(Error::ConnectionReset) if attempts < max_retries => {
                    attempts += 1;
                    continue;
                }
                other => return other,
            }
        }
    }

    /// Retry logic for async one-shot operations with default retry limit
    pub async fn retry_on_connection_reset<T, F, Fut>(operation: F) -> Result<T, Error>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        retry_on_connection_reset_with_limit(operation, DEFAULT_MAX_RETRIES).await
    }
}

// Re-export based on feature flags
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync_retry::*;

#[cfg(feature = "async")]
pub use async_retry::*;
