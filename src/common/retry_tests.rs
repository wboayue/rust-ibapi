use super::*;

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::Instant;

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
        fn wait_connected(&self, _deadline: Option<Instant>) -> Result<(), Error> {
            self.waits.fetch_add(1, Ordering::SeqCst);
            if self.reconnects {
                Ok(())
            } else {
                Err(Error::Shutdown)
            }
        }
    }

    #[test]
    fn retry_on_connection_reset_succeeds_first_try() {
        let mut call_count = 0;
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
            call_count += 1;
            Ok::<i32, Error>(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count, 1);
    }

    #[test]
    fn retry_on_connection_reset_succeeds_after_retry() {
        let call_count = RefCell::new(0);
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
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
    fn retry_on_connection_reset_exceeds_max_retries() {
        let mut call_count = 0;
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
            call_count += 1;
            Err::<i32, Error>(Error::ConnectionReset)
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ConnectionReset));
        assert_eq!(call_count, DEFAULT_MAX_RETRIES + 1);
    }

    #[test]
    fn retry_on_connection_reset_other_error() {
        let mut call_count = 0;
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
            call_count += 1;
            Err::<i32, Error>(Error::InvalidArgument("Other error".to_string()))
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Other error"));
        assert_eq!(call_count, 1); // Should not retry on non-ConnectionReset errors
    }

    #[test]
    fn retry_with_custom_limit() {
        let call_count = RefCell::new(0);
        let custom_limit = 5;
        let waiter = TestWaiter::reconnecting();
        let result = blocking::retry_on_connection_reset_with_limit(
            &waiter,
            None,
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
    fn retry_waits_for_the_reconnect_between_attempts() {
        let waiter = TestWaiter::reconnecting();
        let call_count = RefCell::new(0);
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
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

    /// A session that never comes back ends the retry with the waiter's
    /// error instead of looping.
    #[test]
    fn retry_gives_up_when_the_session_is_gone() {
        let waiter = TestWaiter::default();
        let mut call_count = 0;
        let result = blocking::retry_on_connection_reset(&waiter, None, || {
            call_count += 1;
            Err::<i32, Error>(Error::ConnectionReset)
        });

        assert!(matches!(result.unwrap_err(), Error::Shutdown));
        assert_eq!(call_count, 1, "no attempt after the session was reported gone");
        assert_eq!(waiter.waits(), 1);
    }

    /// A refusal while reconnecting is retried like a reset. When retries run out, the call
    /// reports `Reconnecting` (nothing written) only if no attempt ended in a reset; otherwise
    /// an earlier attempt may have reached TWS, so it reports `ConnectionReset`.
    #[test]
    fn retry_reports_a_last_refusal_by_whether_an_earlier_attempt_was_written() {
        let cases = [
            (
                "refusals retried until the session is back",
                vec![Error::Reconnecting, Error::Reconnecting],
                3,
                Ok(42),
            ),
            (
                "only refusals: nothing was written",
                vec![Error::Reconnecting, Error::Reconnecting],
                1,
                Err("Reconnecting"),
            ),
            (
                "reset, then a refusal: the first attempt was written",
                vec![Error::ConnectionReset, Error::Reconnecting],
                1,
                Err("ConnectionReset"),
            ),
            (
                "refusal, then a reset",
                vec![Error::Reconnecting, Error::ConnectionReset],
                1,
                Err("ConnectionReset"),
            ),
        ];
        for (label, attempts, max_retries, expected) in cases {
            let attempts = RefCell::new(attempts.into_iter());
            let waiter = TestWaiter::reconnecting();
            let result =
                blocking::retry_on_connection_reset_with_limit(&waiter, None, || attempts.borrow_mut().next().map_or(Ok(42), Err), max_retries);
            assert_eq!(result.map_err(|e| e.to_string()), expected.map_err(str::to_string), "{label}");
        }
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::Instant;

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
        async fn wait_connected(&self, _deadline: Option<Instant>) -> Result<(), Error> {
            self.waits.fetch_add(1, Ordering::SeqCst);
            if self.reconnects {
                Ok(())
            } else {
                Err(Error::Shutdown)
            }
        }
    }

    #[tokio::test]
    async fn retry_on_connection_reset_succeeds_first_try() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, None, || {
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
    async fn retry_on_connection_reset_succeeds_after_retry() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, None, || {
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
    async fn retry_on_connection_reset_exceeds_max_retries() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, None, || {
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
    async fn retry_on_connection_reset_other_error() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset(&waiter, None, || {
            let count = count_clone.clone();
            async move {
                *count.lock().unwrap() += 1;
                Err::<i32, Error>(Error::InvalidArgument("Other error".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Other error"));
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn retry_with_custom_limit() {
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();
        let custom_limit = 5;

        let waiter = TestWaiter::reconnecting();
        let result = retry_on_connection_reset_with_limit(
            &waiter,
            None,
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
    async fn retry_waits_for_the_reconnect_between_attempts() {
        let waiter = TestWaiter::reconnecting();
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let result = retry_on_connection_reset(&waiter, None, || {
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

    /// A session that never comes back ends the retry with the waiter's
    /// error instead of looping.
    #[tokio::test]
    async fn retry_gives_up_when_the_session_is_gone() {
        let waiter = TestWaiter::default();
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = call_count.clone();

        let result = retry_on_connection_reset(&waiter, None, || {
            let count = count_clone.clone();
            async move {
                *count.lock().unwrap() += 1;
                Err::<i32, Error>(Error::ConnectionReset)
            }
        })
        .await;

        assert!(matches!(result.unwrap_err(), Error::Shutdown));
        assert_eq!(*call_count.lock().unwrap(), 1, "no attempt after the session was reported gone");
        assert_eq!(waiter.waits(), 1);
    }

    /// Async sibling of the sync test of the same name.
    #[tokio::test]
    async fn retry_reports_a_last_refusal_by_whether_an_earlier_attempt_was_written() {
        let cases = [
            (
                "refusals retried until the session is back",
                vec![Error::Reconnecting, Error::Reconnecting],
                3,
                Ok(42),
            ),
            (
                "only refusals: nothing was written",
                vec![Error::Reconnecting, Error::Reconnecting],
                1,
                Err("Reconnecting"),
            ),
            (
                "reset, then a refusal: the first attempt was written",
                vec![Error::ConnectionReset, Error::Reconnecting],
                1,
                Err("ConnectionReset"),
            ),
            (
                "refusal, then a reset",
                vec![Error::Reconnecting, Error::ConnectionReset],
                1,
                Err("ConnectionReset"),
            ),
        ];
        for (label, attempts, max_retries, expected) in cases {
            let attempts = Mutex::new(attempts.into_iter());
            let waiter = TestWaiter::reconnecting();
            let result = retry_on_connection_reset_with_limit(
                &waiter,
                None,
                || {
                    let next = attempts.lock().unwrap().next();
                    async move { next.map_or(Ok(42), Err) }
                },
                max_retries,
            )
            .await;
            assert_eq!(result.map_err(|e| e.to_string()), expected.map_err(str::to_string), "{label}");
        }
    }
}
