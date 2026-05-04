//! Synchronous subscription implementation

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, error, warn};

use super::common::{process_decode_result, DecoderContext, ProcessingResult, SubscriptionItem};
use super::StreamDecoder;
use crate::errors::Error;
use crate::messages::{OutgoingMessages, ResponseMessage};
use crate::transport::{InternalSubscription, MessageBus};

/// A [Subscription] is a stream of responses returned from TWS. A [Subscription] is normally returned when invoking an API that can return more than one value.
///
/// Each call to [next](Subscription::next), [try_next](Subscription::try_next), or
/// [next_timeout](Subscription::next_timeout) returns
/// `Option<Result<SubscriptionItem<T>, Error>>`:
///
/// * `None` — the stream has ended.
/// * `Some(Ok(SubscriptionItem::Data(t)))` — a decoded value.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice; the stream stays open.
/// * `Some(Err(e))` — terminal error; subsequent calls return `None`.
///
/// When you only care about data, use [`iter_data`](Subscription::iter_data) (or
/// [`next_data`](Subscription::next_data)) which filters notices for you.
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
    stream_ended: AtomicBool,
    subscription: InternalSubscription,
}

enum NextAction<T> {
    Return(Option<T>),
    Skip,
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
            stream_ended: AtomicBool::new(false),
        }
    }

    /// Cancel the subscription
    pub fn cancel(&self) {
        // Skip on snapshot subscriptions whose data already arrived.
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

    /// Returns the next item, blocking until one is available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233"])
    ///     .subscribe()
    ///     .expect("market data request failed");
    ///
    /// while let Some(result) = subscription.next() {
    ///     match result {
    ///         Ok(SubscriptionItem::Data(tick))   => println!("tick: {tick:?}"),
    ///         Ok(SubscriptionItem::Notice(n))    => eprintln!("notice: {n}"),
    ///         Err(e)                             => { eprintln!("error: {e}"); break; }
    ///     }
    /// }
    /// ```
    pub fn next(&self) -> Option<Result<SubscriptionItem<T>, Error>> {
        if self.stream_ended.load(Ordering::Relaxed) {
            return None;
        }

        loop {
            match self.handle_response(self.subscription.next()) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    fn handle_response(&self, response: Option<Result<ResponseMessage, Error>>) -> NextAction<Result<SubscriptionItem<T>, Error>> {
        match response {
            Some(Ok(mut message)) => match process_decode_result(T::decode(&self.context, &mut message)) {
                ProcessingResult::Success(val) => {
                    if val.is_snapshot_end() {
                        self.snapshot_ended.store(true, Ordering::Relaxed);
                    }
                    NextAction::Return(Some(Ok(SubscriptionItem::Data(val))))
                }
                ProcessingResult::Skip => {
                    log::trace!("skipping unexpected message on shared channel");
                    NextAction::Skip
                }
                ProcessingResult::EndOfStream => {
                    self.stream_ended.store(true, Ordering::Relaxed);
                    NextAction::Return(None)
                }
                ProcessingResult::Error(err) => {
                    match &err {
                        Error::Message(code, msg) => warn!("subscription terminated by TWS error [{code}] {msg}"),
                        _ => error!("error decoding message: {err}"),
                    }
                    self.stream_ended.store(true, Ordering::Relaxed);
                    NextAction::Return(Some(Err(err)))
                }
            },
            Some(Err(Error::EndOfStream)) => {
                self.stream_ended.store(true, Ordering::Relaxed);
                NextAction::Return(None)
            }
            Some(Err(e)) => {
                self.stream_ended.store(true, Ordering::Relaxed);
                NextAction::Return(Some(Err(e)))
            }
            None => NextAction::Return(None),
        }
    }

    /// Returns the next item without blocking.
    ///
    /// Returns `None` if no item is available *right now*; check the surrounding
    /// loop or stream state to distinguish from end-of-stream.
    pub fn try_next(&self) -> Option<Result<SubscriptionItem<T>, Error>> {
        if self.stream_ended.load(Ordering::Relaxed) {
            return None;
        }
        loop {
            match self.handle_response(self.subscription.try_next()) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    /// Returns the next item, blocking up to `timeout`.
    pub fn next_timeout(&self, timeout: Duration) -> Option<Result<SubscriptionItem<T>, Error>> {
        if self.stream_ended.load(Ordering::Relaxed) {
            return None;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.handle_response(self.subscription.next_timeout(remaining)) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    /// Convenience: blocking `next` that filters out notices and yields just data.
    /// Equivalent to `iter_data().next()`.
    pub fn next_data(&self) -> Option<Result<T, Error>> {
        loop {
            match self.next()? {
                Ok(SubscriptionItem::Data(t)) => return Some(Ok(t)),
                Ok(SubscriptionItem::Notice(n)) => {
                    log::warn!("ib notice on subscription: {n}");
                    continue;
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }

    /// Blocking iterator yielding `Result<SubscriptionItem<T>, Error>`. Use
    /// [`iter_data`](Subscription::iter_data) when you only want data.
    pub fn iter(&self) -> SubscriptionIter<'_, T> {
        SubscriptionIter { subscription: self }
    }

    /// Non-blocking iterator. Returns `None` immediately when nothing is queued.
    pub fn try_iter(&self) -> SubscriptionTryIter<'_, T> {
        SubscriptionTryIter { subscription: self }
    }

    /// Iterator that waits up to `timeout` for each item.
    pub fn timeout_iter(&self, timeout: Duration) -> SubscriptionTimeoutIter<'_, T> {
        SubscriptionTimeoutIter { subscription: self, timeout }
    }

    /// Blocking iterator that filters notices and yields `Result<T, Error>`.
    /// Notices are logged at `warn!` level.
    pub fn iter_data(&self) -> SubscriptionDataIter<'_, T> {
        SubscriptionDataIter { subscription: self }
    }

    /// Non-blocking data iterator (notices filtered).
    pub fn try_iter_data(&self) -> SubscriptionTryDataIter<'_, T> {
        SubscriptionTryDataIter { subscription: self }
    }

    /// Timeout-bounded data iterator (notices filtered).
    pub fn timeout_iter_data(&self, timeout: Duration) -> SubscriptionTimeoutDataIter<'_, T> {
        SubscriptionTimeoutDataIter { subscription: self, timeout }
    }
}

impl<T: StreamDecoder<T>> Drop for Subscription<T> {
    /// Cancel subscription on drop
    fn drop(&mut self) {
        debug!("dropping subscription");
        self.cancel();
    }
}

/// Convert a `Result<SubscriptionItem<T>, Error>` to `Option<Result<T, Error>>`,
/// dropping (and logging) `SubscriptionItem::Notice` items.
fn filter_data<T>(item: Result<SubscriptionItem<T>, Error>) -> Option<Result<T, Error>> {
    match item {
        Ok(SubscriptionItem::Data(t)) => Some(Ok(t)),
        Ok(SubscriptionItem::Notice(n)) => {
            log::warn!("ib notice on subscription: {n}");
            None
        }
        Err(e) => Some(Err(e)),
    }
}

/// Blocking iterator over `Result<SubscriptionItem<T>, Error>`.
#[allow(private_bounds)]
pub struct SubscriptionIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<'a, T: StreamDecoder<T>> IntoIterator for &'a Subscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;
    type IntoIter = SubscriptionIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Owned blocking iterator over `Result<SubscriptionItem<T>, Error>`.
#[allow(private_bounds)]
pub struct SubscriptionOwnedIter<T: StreamDecoder<T>> {
    subscription: Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionOwnedIter<T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next()
    }
}

impl<T: StreamDecoder<T>> IntoIterator for Subscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;
    type IntoIter = SubscriptionOwnedIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SubscriptionOwnedIter { subscription: self }
    }
}

/// Non-blocking iterator.
#[allow(private_bounds)]
pub struct SubscriptionTryIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTryIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.try_next()
    }
}

/// Timeout-bounded iterator.
#[allow(private_bounds)]
pub struct SubscriptionTimeoutIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
    timeout: Duration,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTimeoutIter<'_, T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscription.next_timeout(self.timeout)
    }
}

/// Blocking iterator filtering notices; yields `Result<T, Error>`.
#[allow(private_bounds)]
pub struct SubscriptionDataIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionDataIter<'_, T> {
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(out) = filter_data(self.subscription.next()?) {
                return Some(out);
            }
        }
    }
}

/// Non-blocking iterator filtering notices.
#[allow(private_bounds)]
pub struct SubscriptionTryDataIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTryDataIter<'_, T> {
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(out) = filter_data(self.subscription.try_next()?) {
                return Some(out);
            }
        }
    }
}

/// Timeout-bounded iterator filtering notices.
#[allow(private_bounds)]
pub struct SubscriptionTimeoutDataIter<'a, T: StreamDecoder<T>> {
    subscription: &'a Subscription<T>,
    timeout: Duration,
}

impl<T: StreamDecoder<T>> Iterator for SubscriptionTimeoutDataIter<'_, T> {
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(out) = filter_data(self.subscription.next_timeout(self.timeout)?) {
                return Some(out);
            }
        }
    }
}

/// Marker trait for subscriptions that share a channel based on message type
pub trait SharesChannel {}

#[cfg(all(test, feature = "sync"))]
#[path = "sync_tests.rs"]
mod tests;
