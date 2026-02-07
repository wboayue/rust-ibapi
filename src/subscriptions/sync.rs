//! Synchronous subscription implementation

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, error, warn};

use super::common::{check_retry, process_decode_result, should_retry_error, should_store_error, DecoderContext, ProcessingResult, RetryDecision};
use super::StreamDecoder;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, ResponseMessage};
use crate::transport::{InternalSubscription, MessageBus};

/// A [Subscription] is a stream of responses returned from TWS. A [Subscription] is normally returned when invoking an API that can return more than one value.
///
/// You can convert subscriptions into blocking or non-blocking iterators using the [iter](Subscription::iter), [try_iter](Subscription::try_iter) or [timeout_iter](Subscription::timeout_iter) methods.
///
/// Alternatively, you may poll subscriptions in a blocking or non-blocking manner using the [next](Subscription::next), [try_next](Subscription::try_next) or [next_timeout](Subscription::next_timeout) methods.
#[allow(private_bounds)]
pub struct Subscription<T: StreamDecoder<T>> {
    context: DecoderContext,
    message_bus: Arc<dyn MessageBus>,
    request_id: Option<i32>,
    order_id: Option<i32>,
    message_type: Option<OutgoingMessages>,
    phantom: PhantomData<T>,
    cancelled: AtomicBool,
    snapshot_ended: AtomicBool,
    subscription: InternalSubscription,
    error: Mutex<Option<Error>>,
    retry_count: AtomicUsize,
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T>> Subscription<T> {
    pub(crate) fn new(message_bus: Arc<dyn MessageBus>, subscription: InternalSubscription, context: DecoderContext) -> Self {
        let request_id = subscription.request_id;
        let order_id = subscription.order_id;
        let message_type = subscription.message_type;

        Subscription {
            context,
            message_bus,
            request_id,
            order_id,
            message_type,
            subscription,
            phantom: PhantomData,
            cancelled: AtomicBool::new(false),
            snapshot_ended: AtomicBool::new(false),
            error: Mutex::new(None),
            retry_count: AtomicUsize::new(0),
        }
    }

    /// Cancel the subscription
    pub fn cancel(&self) {
        // Only cancel if snapshot hasn't ended (for market data snapshots)
        // For streaming subscriptions, snapshot_ended will remain false
        if self.snapshot_ended.load(Ordering::Relaxed) {
            return;
        }

        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }

        self.cancelled.store(true, Ordering::Relaxed);

        if let Some(request_id) = self.request_id {
            if let Ok(message) = T::cancel_message(self.context.server_version, self.request_id, Some(&self.context)) {
                if let Err(e) = self.message_bus.cancel_subscription(request_id, &message) {
                    warn!("error cancelling subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else if let Some(order_id) = self.order_id {
            if let Ok(message) = T::cancel_message(self.context.server_version, self.request_id, Some(&self.context)) {
                if let Err(e) = self.message_bus.cancel_order_subscription(order_id, &message) {
                    warn!("error cancelling order subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else if let Some(message_type) = self.message_type {
            if let Ok(message) = T::cancel_message(self.context.server_version, self.request_id, Some(&self.context)) {
                if let Err(e) = self.message_bus.cancel_shared_subscription(message_type, &message) {
                    warn!("error cancelling shared subscription: {e}")
                }
                self.subscription.cancel();
            }
        } else {
            debug!("Could not determine cancel method")
        }
    }

    /// Returns the request ID associated with this subscription.
    pub fn request_id(&self) -> Option<i32> {
        self.request_id
    }

    /// Returns the next available value, blocking if necessary until a value becomes available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233"])
    ///     .subscribe()
    ///     .expect("market data request failed");
    ///
    /// // Process data blocking until the next value is available
    /// while let Some(data) = subscription.next() {
    ///     println!("Received data: {data:?}");
    /// }
    ///
    /// // When the loop exits, check if it was due to an error
    /// if let Some(err) = subscription.error() {
    ///     eprintln!("subscription error: {err}");
    /// }
    /// ```
    /// # Returns
    /// * `Some(T)` - The next available item from the subscription
    /// * `None` - If the subscription has ended or encountered an error
    pub fn next(&self) -> Option<T> {
        match self.process_response(self.subscription.next()) {
            Some(val) => {
                self.retry_count.store(0, Ordering::Relaxed);
                Some(val)
            }
            None => match self.error() {
                Some(ref err) if should_retry_error(err) => {
                    let retries = self.retry_count.fetch_add(1, Ordering::Relaxed);
                    if check_retry(retries) == RetryDecision::Continue {
                        self.next()
                    } else {
                        self.retry_count.store(0, Ordering::Relaxed);
                        None
                    }
                }
                _ => {
                    self.retry_count.store(0, Ordering::Relaxed);
                    None
                }
            },
        }
    }

    /// Returns the current error state of the subscription.
    ///
    /// This method allows checking if an error occurred during subscription processing.
    /// Errors are stored internally when they occur during `next()`, `try_next()`, or `next_timeout()` calls.
    ///
    /// # Returns
    /// * `Some(Error)` - If an error has occurred
    /// * `None` - If no error has occurred
    pub fn error(&self) -> Option<Error> {
        let mut error = self.error.lock().unwrap();
        error.take()
    }

    fn clear_error(&self) {
        let mut error = self.error.lock().unwrap();
        *error = None;
    }

    fn process_response(&self, response: Option<Result<ResponseMessage, Error>>) -> Option<T> {
        self.clear_error();

        match response {
            Some(Ok(message)) => self.process_message(message),
            Some(Err(e)) => {
                if should_store_error(&e) {
                    let mut error = self.error.lock().unwrap();
                    *error = Some(e);
                }
                None
            }
            None => None,
        }
    }

    fn process_message(&self, mut message: ResponseMessage) -> Option<T> {
        match process_decode_result(T::decode(&self.context, &mut message)) {
            ProcessingResult::Success(val) => {
                // Check if this decoded value represents the end of a snapshot subscription
                if val.is_snapshot_end() {
                    self.snapshot_ended.store(true, Ordering::Relaxed);
                }
                Some(val)
            }
            ProcessingResult::EndOfStream => None,
            ProcessingResult::Retry => {
                // This case shouldn't happen here since UnexpectedResponse is handled at the next() level
                // but we handle it for completeness
                None
            }
            ProcessingResult::Error(err) => {
                error!("error decoding message: {err}");
                let mut error = self.error.lock().unwrap();
                *error = Some(err);
                None
            }
        }
    }

    /// Tries to return the next available value without blocking.
    ///
    /// Returns immediately with:
    /// - `Some(value)` if a value is available
    /// - `None` if no data is currently available
    ///
    /// Use this method when you want to poll for data without blocking.
    /// Check `error()` to determine if `None` was returned due to an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233"])
    ///     .subscribe()
    ///     .expect("market data request failed");
    ///
    /// // Poll for data without blocking
    /// loop {
    ///     if let Some(data) = subscription.try_next() {
    ///         println!("{data:?}");
    ///     } else if let Some(err) = subscription.error() {
    ///         eprintln!("Error: {err}");
    ///         break;
    ///     } else {
    ///         // No data available, do other work or sleep
    ///         thread::sleep(Duration::from_millis(100));
    ///     }
    /// }
    /// ```
    pub fn try_next(&self) -> Option<T> {
        self.process_response(self.subscription.try_next())
    }

    /// Waits for the next available value up to the specified timeout duration.
    ///
    /// Returns:
    /// - `Some(value)` if a value becomes available within the timeout
    /// - `None` if the timeout expires before data becomes available
    ///
    /// Check `error()` to determine if `None` was returned due to an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233"])
    ///     .subscribe()
    ///     .expect("market data request failed");
    ///
    /// // Wait up to 5 seconds for data
    /// if let Some(data) = subscription.next_timeout(Duration::from_secs(5)) {
    ///     println!("{data:?}");
    /// } else if let Some(err) = subscription.error() {
    ///     eprintln!("Error: {err}");
    /// } else {
    ///     eprintln!("Timeout: no data received within 5 seconds");
    /// }
    /// ```
    pub fn next_timeout(&self, timeout: Duration) -> Option<T> {
        self.process_response(self.subscription.next_timeout(timeout))
    }

    /// Creates a blocking iterator over the subscription data.
    ///
    /// The iterator will block waiting for the next value if none is immediately available.
    /// The iterator ends when the subscription is cancelled or an unrecoverable error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.positions().expect("positions request failed");
    ///
    /// // Process all positions as they arrive
    /// for position in subscription.iter() {
    ///     println!("{position:?}");
    /// }
    ///
    /// // Check if iteration ended due to an error
    /// if let Some(err) = subscription.error() {
    ///     eprintln!("Subscription error: {err}");
    /// }
    /// ```
    pub fn iter(&self) -> SubscriptionIter<'_, T> {
        SubscriptionIter { subscription: self }
    }

    /// Creates a non-blocking iterator over the subscription data.
    ///
    /// The iterator will return immediately with `None` if no data is available.
    /// Use this when you want to process available data without blocking.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.positions().expect("positions request failed");
    ///
    /// // Process available positions without blocking
    /// loop {
    ///     let mut data_received = false;
    ///     for position in subscription.try_iter() {
    ///         data_received = true;
    ///         println!("{position:?}");
    ///     }
    ///     
    ///     if let Some(err) = subscription.error() {
    ///         eprintln!("Error: {err}");
    ///         break;
    ///     }
    ///     
    ///     if !data_received {
    ///         // No data available, do other work or sleep
    ///         thread::sleep(Duration::from_millis(100));
    ///     }
    /// }
    /// ```
    pub fn try_iter(&self) -> SubscriptionTryIter<'_, T> {
        SubscriptionTryIter { subscription: self }
    }

    /// Creates an iterator that waits up to the specified timeout for each value.
    ///
    /// The iterator will wait up to `timeout` duration for each value.
    /// If the timeout expires, the iterator ends.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.positions().expect("positions request failed");
    ///
    /// // Process positions with a 5 second timeout per item
    /// for position in subscription.timeout_iter(Duration::from_secs(5)) {
    ///     println!("{position:?}");
    /// }
    ///
    /// if let Some(err) = subscription.error() {
    ///     eprintln!("Error: {err}");
    /// } else {
    ///     println!("No more positions received within timeout");
    /// }
    /// ```
    pub fn timeout_iter(&self, timeout: Duration) -> SubscriptionTimeoutIter<'_, T> {
        SubscriptionTimeoutIter { subscription: self, timeout }
    }
}

impl<T: StreamDecoder<T>> Drop for Subscription<T> {
    /// Cancel subscription on drop
    fn drop(&mut self) {
        debug!("dropping subscription");
        self.cancel();
    }
}

/// An iterator that yields items as they become available, blocking if necessary.
#[allow(private_bounds)]
pub struct SubscriptionIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: StreamDecoder<T>> IntoIterator for &'a Subscription<T> {
    type Item = T;
    type IntoIter = SubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator that takes ownership and yields items as they become available, blocking if necessary.
#[allow(private_bounds)]
pub struct SubscriptionOwnedIter<T: StreamDecoder<T>> {
    subscription: Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionOwnedIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<T: StreamDecoder<T>> IntoIterator for Subscription<T> {
    type Item = T;
    type IntoIter = SubscriptionOwnedIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SubscriptionOwnedIter { subscription: self }
    }
}

/// An iterator that yields items as they become available without blocking.
#[allow(private_bounds)]
pub struct SubscriptionTryIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTryIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// An iterator that yields items with a timeout.
#[allow(private_bounds)]
pub struct SubscriptionTimeoutIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
    timeout: Duration,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTimeoutIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

/// Marker trait for subscriptions that share a channel based on message type
pub trait SharesChannel {}
