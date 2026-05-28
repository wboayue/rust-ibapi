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
