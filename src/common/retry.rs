//! Retry logic patterns for handling connection resets on one-shot operations
//!
//! These utilities provide retry functionality for operations that can be safely retried
//! without losing server-side state (e.g., managed_accounts, server_time).
//! Do NOT use for subscriptions or stateful operations.

use crate::Error;

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

// Sync implementations
#[cfg(feature = "sync")]
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
mod tests {
    use super::*;

    #[cfg(feature = "sync")]
    mod sync_tests {
        use super::*;
        use std::cell::RefCell;

        use crate::common::retry::blocking;

        #[test]
        fn test_retry_on_connection_reset_succeeds_first_try() {
            let mut call_count = 0;
            let result = blocking::retry_on_connection_reset(|| {
                call_count += 1;
                Ok::<i32, Error>(42)
            });

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 42);
            assert_eq!(call_count, 1);
        }

        #[test]
        fn test_retry_on_connection_reset_succeeds_after_retry() {
            let call_count = RefCell::new(0);
            let result = blocking::retry_on_connection_reset(|| {
                *call_count.borrow_mut() += 1;
                if *call_count.borrow() < 3 {
                    Err(Error::ConnectionReset)
                } else {
                    Ok(42)
                }
            });

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 42);
            assert_eq!(*call_count.borrow(), 3);
        }

        #[test]
        fn test_retry_on_connection_reset_exceeds_max_retries() {
            let mut call_count = 0;
            let result = blocking::retry_on_connection_reset(|| {
                call_count += 1;
                Err::<i32, Error>(Error::ConnectionReset)
            });

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
            assert_eq!(call_count, DEFAULT_MAX_RETRIES + 1);
        }

        #[test]
        fn test_retry_on_connection_reset_other_error() {
            let mut call_count = 0;
            let result = blocking::retry_on_connection_reset(|| {
                call_count += 1;
                Err::<i32, Error>(Error::Simple("Other error".to_string()))
            });

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Other error"));
            assert_eq!(call_count, 1); // Should not retry on non-ConnectionReset errors
        }

        #[test]
        fn test_retry_with_custom_limit() {
            let call_count = RefCell::new(0);
            let custom_limit = 5;
            let result = blocking::retry_on_connection_reset_with_limit(
                || {
                    *call_count.borrow_mut() += 1;
                    Err::<i32, Error>(Error::ConnectionReset)
                },
                custom_limit,
            );

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
            assert_eq!(*call_count.borrow(), custom_limit + 1);
        }
    }

    #[cfg(feature = "async")]
    mod async_tests {
        use super::*;
        use std::sync::{Arc, Mutex};

        #[tokio::test]
        async fn test_retry_on_connection_reset_succeeds_first_try() {
            let call_count = Arc::new(Mutex::new(0));
            let count_clone = call_count.clone();

            let result = retry_on_connection_reset(|| {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Ok::<i32, Error>(42)
                }
            })
            .await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 42);
            assert_eq!(*call_count.lock().unwrap(), 1);
        }

        #[tokio::test]
        async fn test_retry_on_connection_reset_succeeds_after_retry() {
            let call_count = Arc::new(Mutex::new(0));
            let count_clone = call_count.clone();

            let result = retry_on_connection_reset(|| {
                let count = count_clone.clone();
                async move {
                    let mut guard = count.lock().unwrap();
                    *guard += 1;
                    if *guard < 3 {
                        Err(Error::ConnectionReset)
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 42);
            assert_eq!(*call_count.lock().unwrap(), 3);
        }

        #[tokio::test]
        async fn test_retry_on_connection_reset_exceeds_max_retries() {
            let call_count = Arc::new(Mutex::new(0));
            let count_clone = call_count.clone();

            let result = retry_on_connection_reset(|| {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, Error>(Error::ConnectionReset)
                }
            })
            .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
            assert_eq!(*call_count.lock().unwrap(), DEFAULT_MAX_RETRIES + 1);
        }

        #[tokio::test]
        async fn test_retry_on_connection_reset_other_error() {
            let call_count = Arc::new(Mutex::new(0));
            let count_clone = call_count.clone();

            let result = retry_on_connection_reset(|| {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, Error>(Error::Simple("Other error".to_string()))
                }
            })
            .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Other error"));
            assert_eq!(*call_count.lock().unwrap(), 1);
        }

        #[tokio::test]
        async fn test_retry_with_custom_limit() {
            let call_count = Arc::new(Mutex::new(0));
            let count_clone = call_count.clone();
            let custom_limit = 5;

            let result = retry_on_connection_reset_with_limit(
                || {
                    let count = count_clone.clone();
                    async move {
                        *count.lock().unwrap() += 1;
                        Err::<i32, Error>(Error::ConnectionReset)
                    }
                },
                custom_limit,
            )
            .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
            assert_eq!(*call_count.lock().unwrap(), custom_limit + 1);
        }
    }
}
