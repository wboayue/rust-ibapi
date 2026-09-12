use super::*;

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use std::cell::RefCell;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::common::retry::blocking::{self, ReconnectWaiter};

    /// Stands in for the client between attempts: counts the waits, and can
    /// report a session that never comes back.
    #[derive(Default)]
    struct TestWaiter {
        waits: AtomicUsize,
        reconnects: bool,
    }

    impl TestWaiter {
        fn reconnecting() -> Self {
            Self {
                waits: AtomicUsize::new(0),
                reconnects: true,
            }
        }

        fn waits(&self) -> usize {
            self.waits.load(Ordering::SeqCst)
        }
    }

    impl ReconnectWaiter for TestWaiter {
        fn wait_connected(&self) -> Result<(), Error> {
            self.waits.fetch_add(1, Ordering::SeqCst);
            if self.reconnects {
                Ok(())
            } else {
                Err(Error::Shutdown)
            }
        }
    }

    #[test]
    fn test_retry_on_connection_reset_succeeds_first_try() {
        let mut call_count = 0;
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, || {
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
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, || {
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
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, || {
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
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, || {
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
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset_with_limit(
            &waiter,
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

    /// Every retry waits for the reconnect first: a send made before it
    /// completes is refused by the bus, which would spend the attempts on
    /// requests that never reach TWS.
    #[test]
    fn test_retry_waits_for_the_reconnect_between_attempts() {
        let waiter = TestWaiter::reconnecting();
        let call_count = RefCell::new(0);
        let result = blocking::retry_on_connection_reset(&waiter, || {
            *call_count.borrow_mut() += 1;
            if *call_count.borrow() < 3 {
                Err(Error::ConnectionReset)
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(waiter.waits(), 2, "one wait per retry, none before the first attempt");
    }

    /// A session that never comes back ends the retry with the reset
    /// instead of looping.
    #[test]
    fn test_retry_gives_up_when_the_session_is_gone() {
        let waiter = TestWaiter::default();
        let mut call_count = 0;
        let result = blocking::retry_on_connection_reset(&waiter, || {
            call_count += 1;
            Err::<i32, Error>(Error::ConnectionReset)
        });

        assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
        assert_eq!(call_count, 1, "no attempt after the session was reported gone");
        assert_eq!(waiter.waits(), 1);
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::common::retry::ReconnectWaiter;

    /// Stands in for the client between attempts: counts the waits, and can
    /// report a session that never comes back.
    #[derive(Default)]
    struct TestWaiter {
        waits: AtomicUsize,
        reconnects: bool,
    }

    impl TestWaiter {
        fn reconnecting() -> Self {
            Self {
                waits: AtomicUsize::new(0),
                reconnects: true,
            }
        }

        fn waits(&self) -> usize {
            self.waits.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl ReconnectWaiter for TestWaiter {
        async fn wait_connected(&self) -> Result<(), Error> {
            self.waits.fetch_add(1, Ordering::SeqCst);
            if self.reconnects {
                Ok(())
            } else {
                Err(Error::Shutdown)
            }
        }
    }

    #[tokio::test]
    async fn test_retry_on_connection_reset_succeeds_first_try() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, || {
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

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, || {
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

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, || {
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

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, || {
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

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset_with_limit(
            &waiter,
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

    /// Every retry waits for the reconnect first: a send made before it
    /// completes is refused by the bus, which would spend the attempts on
    /// requests that never reach TWS.
    #[tokio::test]
    async fn test_retry_waits_for_the_reconnect_between_attempts() {
        let waiter = TestWaiter::reconnecting();
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let result = retry_on_connection_reset(&waiter, || {
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

        assert_eq!(result.unwrap(), 42);
        assert_eq!(waiter.waits(), 2, "one wait per retry, none before the first attempt");
    }

    /// A session that never comes back ends the retry with the reset
    /// instead of looping.
    #[tokio::test]
    async fn test_retry_gives_up_when_the_session_is_gone() {
        let waiter = TestWaiter::default();
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let result = retry_on_connection_reset(&waiter, || {
            let count = count_clone.clone();
            async move {
                *count.lock().unwrap() += 1;
                Err::<i32, Error>(Error::ConnectionReset)
            }
        })
        .await;

        assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
        assert_eq!(*call_count.lock().unwrap(), 1, "no attempt after the session was reported gone");
        assert_eq!(waiter.waits(), 1);
    }
}
