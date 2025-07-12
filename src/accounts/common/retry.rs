//! Retry logic patterns for handling connection resets on one-shot operations

use crate::Error;

/// Default maximum number of retry attempts
pub(in crate::accounts) const DEFAULT_MAX_RETRIES: u32 = 3;

/// Retry logic for sync one-shot operations with configurable retry limit
///
/// Note: This should only be used for operations that can be safely retried
/// without losing server-side state (e.g., managed_accounts, server_time).
/// Do NOT use for subscriptions or stateful operations.
#[allow(dead_code)] // Available for future use with custom retry limits
pub(in crate::accounts) fn retry_on_connection_reset_with_limit<T, F>(mut operation: F, max_retries: u32) -> Result<T, Error>
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
#[allow(dead_code)] // Used by sync helpers
pub(in crate::accounts) fn retry_on_connection_reset<T, F>(operation: F) -> Result<T, Error>
where
    F: FnMut() -> Result<T, Error>,
{
    retry_on_connection_reset_with_limit(operation, DEFAULT_MAX_RETRIES)
}

#[cfg(feature = "async")]
use futures::Future;

/// Retry logic for async one-shot operations with configurable retry limit
///
/// Note: This should only be used for operations that can be safely retried
/// without losing server-side state (e.g., managed_accounts, server_time).
/// Do NOT use for subscriptions or stateful operations.
#[cfg(feature = "async")]
#[allow(dead_code)] // Available for future use with custom retry limits
pub(in crate::accounts) async fn retry_on_connection_reset_with_limit_async<T, F, Fut>(mut operation: F, max_retries: u32) -> Result<T, Error>
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
#[cfg(feature = "async")]
pub(in crate::accounts) async fn retry_on_connection_reset_async<T, F, Fut>(operation: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    retry_on_connection_reset_with_limit_async(operation, DEFAULT_MAX_RETRIES).await
}
