//! Synchronous subscription implementation

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, error, warn};

use super::common::{
    debug_assert_request_id_routable, drain_outcome, filter_notice, is_undeclared, notice_item, DecoderContext, Drained, RoutedItem, SubscriptionItem,
};
use super::{log_cancel_error, StreamDecoder};
use crate::client::ids::RequestId;
use crate::errors::Error;
use crate::transport::{InternalSubscription, MessageBus, SharedTicket};

/// A [Subscription] is a stream of responses returned from TWS. A [Subscription] is normally returned when invoking an API that can return more than one value.
///
/// Each call to [next](Subscription::next), [try_next](Subscription::try_next), or
/// [next_timeout](Subscription::next_timeout) returns
/// `Option<Result<SubscriptionItem<T>, Error>>`:
///
/// * `None` — the stream has ended.
/// * `Some(Ok(SubscriptionItem::Data(t)))` — a decoded value.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice (a warning
///   code in [`WARNING_CODE_RANGE`](crate::messages::WARNING_CODE_RANGE) or
///   order-cancel code 202) carried on this subscription's `request_id`; the
///   stream stays open.
/// * `Some(Err(e))` — terminal error; subsequent calls return `None`.
///
/// When you only care about data, use [`iter_data`](Subscription::iter_data) (or
/// [`next_data`](Subscription::next_data)) which filters notices for you.
///
/// Notices that are *not* tied to a specific subscription — connectivity codes
/// 1100/1101/1102, farm-status 2104/2105/2106/2107/2108, etc. — are not delivered
/// here. Subscribe to them via [`Client::notice_stream`](crate::client::blocking::Client::notice_stream)
/// instead.
#[allow(private_bounds)]
#[must_use = "Subscription must be iterated (via .next(), .iter_data(), or .into_iter()) to receive data; dropping it cancels the request"]
pub struct Subscription<T: StreamDecoder<T>> {
    context: DecoderContext,
    message_bus: Arc<dyn MessageBus>,
    request_id: Option<i32>,
    shared: Option<SharedTicket>,
    phantom: PhantomData<T>,
    cancelled: AtomicBool,
    snapshot_ended: AtomicBool,
    stream_ended: AtomicBool,
    /// Set only by the native end marker, not by errors: TWS has finished the
    /// request, so there is nothing left to cancel.
    ended_natively: AtomicBool,
    subscription: InternalSubscription,
}

enum NextAction<T> {
    Return(Option<T>),
    Skip,
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T>> Subscription<T> {
    pub(crate) fn new(message_bus: Arc<dyn MessageBus>, subscription: InternalSubscription, context: DecoderContext) -> Self {
        // Raw from here on: the public accessor and the decoders' cancel
        // messages speak `i32`.
        let request_id = subscription.request_id.map(RequestId::raw);
        let shared = subscription.shared;

        debug_assert_request_id_routable::<T, T>(request_id);

        Subscription {
            context,
            message_bus,
            request_id,
            shared,
            subscription,
            phantom: PhantomData,
            cancelled: AtomicBool::new(false),
            snapshot_ended: AtomicBool::new(false),
            stream_ended: AtomicBool::new(false),
            ended_natively: AtomicBool::new(false),
        }
    }

    /// Cancel the subscription.
    ///
    /// Writes TWS's cancel unless the request has already finished (end
    /// marker or snapshot end) or its type has none. Either way the
    /// subscription stops receiving: it yields what was already queued, then
    /// `Err(Error::Cancelled)`, then ends. Idempotent; also called on drop.
    pub fn cancel(&self) {
        // One atomic swap, not a load then a store: two threads cancelling the
        // same handle would otherwise both reach the bus, and on a shared
        // stream that releases the count twice.
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        if let Some(ticket) = self.shared {
            // The count is released whether or not the type has a cancel message.
            let message = T::cancel_message(self.context.server_version, self.request_id, Some(&self.context)).ok();
            if let Err(e) = self.message_bus.cancel_shared_subscription(ticket, message.as_deref()) {
                log_cancel_error("shared subscription", &e);
            }
        } else if let Some(message) = self.pending_cancel() {
            // Id-routed, while TWS may still be running the request. The write
            // goes whether or not it reaches TWS.
            if let Err(e) = self.message_bus.send_message(&message) {
                log_cancel_error("subscription", &e);
            }
        }
        // Released either way (`order_update_stream`, which has no id, included),
        // so a handle kept after `cancel()` stops collecting frames.
        self.subscription.cancel();
    }

    /// Returns the request ID associated with this subscription.
    pub fn request_id(&self) -> Option<i32> {
        self.request_id
    }

    /// Whether the stream ended with TWS's end marker, as opposed to an
    /// error or a closed channel (both of which also end iteration).
    pub(crate) fn ended_natively(&self) -> bool {
        self.ended_natively.load(Ordering::Relaxed)
    }

    /// The cancel a subscription that isn't shared writes, if any. `None` when the
    /// type has none, or once the stream has seen its end marker or snapshot
    /// end: the cancel would name a request TWS already finished.
    fn pending_cancel(&self) -> Option<Vec<u8>> {
        if self.ended_natively() || self.snapshot_ended.load(Ordering::Relaxed) {
            return None;
        }
        T::cancel_message(self.context.server_version, self.request_id, Some(&self.context)).ok()
    }

    /// Cancel the request and wait, up to `deadline`, for TWS to confirm it
    /// is over, discarding anything that arrives meanwhile.
    ///
    /// Use it when the request id must be known finished before it's reused,
    /// or before the next request under strict pacing. Dropping the
    /// subscription also cancels, but doesn't wait for TWS.
    ///
    /// | Outcome | Meaning |
    /// | --- | --- |
    /// | [`Drained::Ended`] | TWS sent the end marker, before or after the cancel. Nothing is written if it (or a snapshot's end) had already arrived. |
    /// | [`Drained::Rejected`] | TWS answered with an error for the request. |
    /// | [`Drained::Unconfirmed`] | The deadline passed, or the stream had already ended with an error (such as [`Error::BufferLimitExceeded`]). Treat the id as possibly live. |
    /// | `Err` | The connection reset or the client shut down meanwhile. |
    ///
    /// The cancel is TWS's native one, where the request type has one (contract
    /// details: server 215+); otherwise nothing is written and the drain waits
    /// for a natural end. Observed live (server 225), TWS kept sending contract
    /// details after the cancel: the rest of a prepared 714-row result, then
    /// the end marker; and 8,000+ rows over 150 s of an unfiltered option
    /// query with no end marker yet. So the drain usually waits out the whole
    /// result: size `deadline` for it, and prefer narrow queries. A stream with
    /// no end marker (market data) always ends `Unconfirmed`. Subscriptions
    /// without a request id (shared, order and order-update streams) are cancelled as by
    /// [`cancel`](Self::cancel) and return `Unconfirmed` immediately.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::Drained;
    /// use std::time::{Duration, Instant};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.contract_details_stream(&contract).subscribe().expect("request failed");
    /// let first = subscription.next_data();
    /// println!("first row: {first:?}");
    ///
    /// match subscription.cancel_and_drain(Instant::now() + Duration::from_secs(10)) {
    ///     Ok(Drained::Ended) | Ok(Drained::Rejected(_)) => println!("request finished at TWS"),
    ///     Ok(Drained::Unconfirmed) => println!("not confirmed; don't reuse the request slot yet"),
    ///     Err(e) => eprintln!("session error: {e}"),
    /// }
    /// ```
    pub fn cancel_and_drain(self, deadline: Instant) -> Result<Drained, Error> {
        // A finished snapshot is complete at TWS too; `cancel()` writes nothing for it.
        if self.ended_natively() || self.snapshot_ended.load(Ordering::Relaxed) {
            return Ok(Drained::Ended);
        }
        // Ended with an error already: nothing more can arrive to confirm
        // TWS's state. Drop writes the cancel, as for any errored stream.
        if self.stream_ended.load(Ordering::Relaxed) {
            return Ok(Drained::Unconfirmed);
        }
        if self.request_id.is_none() {
            self.cancel();
            return Ok(Drained::Unconfirmed);
        }

        // Write the cancel but keep the route: `cancel()` would unregister it
        // (`InternalSubscription::cancel`), and then TWS's end marker could
        // not reach us. The route goes when the subscription drops.
        if !self.cancelled.swap(true, Ordering::Relaxed) {
            if let Some(message) = self.pending_cancel() {
                if let Err(e) = self.message_bus.send_message(&message) {
                    log_cancel_error("subscription", &e);
                }
            }
        }

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(Drained::Unconfirmed);
            }
            match self.next_timeout(remaining) {
                Some(item) => {
                    if let Some(outcome) = drain_outcome(item) {
                        return outcome;
                    }
                }
                None if self.ended_natively() => return Ok(Drained::Ended),
                // The deadline passed; the loop returns `Unconfirmed`. The
                // channel can't close under us: the subscription holds a sender.
                None => {}
            }
        }
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
            match self.handle_response(self.subscription.next_routed()) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    fn handle_response(&self, response: Option<RoutedItem>) -> NextAction<Result<SubscriptionItem<T>, Error>> {
        match response {
            Some(RoutedItem::Response(message)) if is_undeclared(T::RESPONSE_MESSAGE_IDS, &message) => {
                log::trace!("skipping {:?} — not declared by this subscription's decoder", message.message_type());
                NextAction::Skip
            }
            Some(RoutedItem::Response(message)) => match T::decode(&self.context, &message) {
                Ok(val) => {
                    if val.is_snapshot_end() {
                        self.snapshot_ended.store(true, Ordering::Relaxed);
                    }
                    NextAction::Return(Some(Ok(SubscriptionItem::Data(val))))
                }
                Err(Error::EndOfStream) => {
                    self.stream_ended.store(true, Ordering::Relaxed);
                    self.ended_natively.store(true, Ordering::Relaxed);
                    NextAction::Return(None)
                }
                Err(err) => {
                    match &err {
                        Error::Notice(n) => warn!("subscription terminated by TWS error {n}"),
                        _ => error!("error decoding message: {err}"),
                    }
                    self.stream_ended.store(true, Ordering::Relaxed);
                    NextAction::Return(Some(Err(err)))
                }
            },
            Some(RoutedItem::Notice(notice)) => NextAction::Return(Some(Ok(notice_item(notice)))),
            Some(RoutedItem::Error(Error::Notice(notice))) if T::is_nonterminal_notice(&notice) => {
                NextAction::Return(Some(Ok(SubscriptionItem::Notice(notice))))
            }
            Some(RoutedItem::Error(Error::EndOfStream)) => {
                self.stream_ended.store(true, Ordering::Relaxed);
                self.ended_natively.store(true, Ordering::Relaxed);
                NextAction::Return(None)
            }
            Some(RoutedItem::Error(e)) => {
                self.stream_ended.store(true, Ordering::Relaxed);
                NextAction::Return(Some(Err(e)))
            }
            None => NextAction::Return(None),
        }
    }

    /// Returns the next item without blocking.
    ///
    /// Same `SubscriptionItem<T>` shape as [`next`](Self::next): `Data`, `Notice`,
    /// or terminal error. Use [`try_iter_data`](Self::try_iter_data) when notices
    /// should be filtered.
    ///
    /// Returns `None` if no item is available *right now*; check the surrounding
    /// loop or stream state to distinguish from end-of-stream.
    pub fn try_next(&self) -> Option<Result<SubscriptionItem<T>, Error>> {
        if self.stream_ended.load(Ordering::Relaxed) {
            return None;
        }
        loop {
            match self.handle_response(self.subscription.try_next_routed()) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    /// Returns the next item, blocking up to `timeout`.
    ///
    /// Same `SubscriptionItem<T>` shape as [`next`](Self::next): `Data`, `Notice`,
    /// or terminal error. Use [`timeout_iter_data`](Self::timeout_iter_data) when
    /// you want notices filtered.
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
            match self.handle_response(self.subscription.next_timeout_routed(remaining)) {
                NextAction::Return(val) => return val,
                NextAction::Skip => continue,
            }
        }
    }

    /// Convenience: blocking `next` that filters out notices and yields just data.
    /// Equivalent to `iter_data().next()`. Filtered notices are logged at `warn!`.
    /// Use [`next`](Self::next) instead if you want to observe `Notice` items.
    pub fn next_data(&self) -> Option<Result<T, Error>> {
        self.iter_data().next()
    }

    /// Blocking iterator yielding `Result<SubscriptionItem<T>, Error>` — both
    /// `Data` and `Notice` arms surface to the caller. Use
    /// [`iter_data`](Subscription::iter_data) when you only want data.
    pub fn iter(&self) -> SubscriptionIter<'_, T> {
        SubscriptionIter { subscription: self }
    }

    /// Non-blocking iterator. Same `SubscriptionItem<T>` shape as [`iter`](Self::iter)
    /// (see [`try_iter_data`](Self::try_iter_data) for the data-only variant).
    /// Returns `None` immediately when nothing is queued.
    pub fn try_iter(&self) -> SubscriptionTryIter<'_, T> {
        SubscriptionTryIter { subscription: self }
    }

    /// Iterator that waits up to `timeout` for each item. Same
    /// `SubscriptionItem<T>` shape as [`iter`](Self::iter); see
    /// [`timeout_iter_data`](Self::timeout_iter_data) for the data-only variant.
    pub fn timeout_iter(&self, timeout: Duration) -> SubscriptionTimeoutIter<'_, T> {
        SubscriptionTimeoutIter { subscription: self, timeout }
    }

    /// Blocking iterator that filters notices and yields `Result<T, Error>`.
    /// Notices are logged at `warn!` level.
    pub fn iter_data(&self) -> FilterData<SubscriptionIter<'_, T>> {
        self.iter().filter_data()
    }

    /// Non-blocking data iterator (notices filtered).
    pub fn try_iter_data(&self) -> FilterData<SubscriptionTryIter<'_, T>> {
        self.try_iter().filter_data()
    }

    /// Timeout-bounded data iterator (notices filtered).
    pub fn timeout_iter_data(&self, timeout: Duration) -> FilterData<SubscriptionTimeoutIter<'_, T>> {
        self.timeout_iter(timeout).filter_data()
    }

    /// Collects data items into a `Vec`, bounded by a total wall-clock `timeout`.
    ///
    /// Drives the subscription until the first of: the `timeout` elapses, the
    /// stream ends, a snapshot-end sentinel arrives (e.g.
    /// [`TickTypes::SnapshotEnd`](crate::market_data::realtime::TickTypes::SnapshotEnd)),
    /// or a terminal error occurs. Notices are filtered (logged at `warn!`); the
    /// snapshot-end sentinel is not included in the returned `Vec`. On a terminal
    /// error the items collected so far are returned (the error is logged at
    /// `warn!`).
    ///
    /// This is the one-shot snapshot terminal: combined with
    /// [`MarketDataBuilder::snapshot`](crate::market_data::realtime::MarketDataBuilder::snapshot),
    /// the request returns one round of data ending in a snapshot sentinel, so
    /// `timeout` acts only as a safety bound. Equivalent to
    /// [`collect_until`](Self::collect_until) with a predicate that never fires.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract).snapshot().subscribe().expect("request failed");
    ///
    /// let ticks = subscription.collect_for(Duration::from_secs(5));
    /// println!("collected {} ticks", ticks.len());
    /// ```
    pub fn collect_for(&self, timeout: Duration) -> Vec<T> {
        self.collect_until(timeout, |_| false)
    }

    /// Collects data items into a `Vec`, stopping early once `stop` is satisfied.
    ///
    /// Like [`collect_for`](Self::collect_for), but after each item is appended
    /// the `stop` predicate is called with the full accumulated slice; returning
    /// `true` ends collection (the triggering item is included). Use it to stop
    /// as soon as the fields of interest are populated, rather than waiting out
    /// the whole `timeout`. The same timeout / stream-end / snapshot-end /
    /// terminal-error bounds as `collect_for` still apply.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::TickTypes;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client.market_data(&contract).snapshot().subscribe().expect("request failed");
    ///
    /// // Stop as soon as a price tick has arrived.
    /// let ticks = subscription.collect_until(Duration::from_secs(5), |ticks| {
    ///     ticks.iter().any(|t| matches!(t, TickTypes::Price(_) | TickTypes::PriceSize(_)))
    /// });
    /// println!("collected {} ticks", ticks.len());
    /// ```
    pub fn collect_until(&self, timeout: Duration, mut stop: impl FnMut(&[T]) -> bool) -> Vec<T> {
        let deadline = Instant::now() + timeout;
        let mut collected = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self.next_timeout(remaining) {
                Some(Ok(SubscriptionItem::Data(value))) => {
                    if value.is_snapshot_end() {
                        break;
                    }
                    collected.push(value);
                    if stop(&collected) {
                        break;
                    }
                }
                Some(Ok(SubscriptionItem::Notice(notice))) => warn!("ib notice on subscription: {notice}"),
                Some(Err(e)) => {
                    warn!("subscription error during collect: {e}");
                    break;
                }
                // Per-item timeout (total deadline reached) or end of stream.
                None => break,
            }
        }
        collected
    }

    /// Collects every data item until TWS's end marker. Notices are logged at
    /// `warn!`. A terminal error is returned as is; a stream that ends without
    /// the end marker (a closed channel) is `Error::UnexpectedEndOfStream`.
    ///
    /// For request-scoped streams that end, such as contract details. Blocks
    /// until the end, so not for open-ended subscriptions like market data.
    pub(crate) fn collect_to_end(&self) -> Result<Vec<T>, Error> {
        let mut collected = Vec::new();
        while let Some(item) = self.next() {
            match item? {
                SubscriptionItem::Data(value) => collected.push(value),
                SubscriptionItem::Notice(notice) => warn!("ib notice on subscription: {notice}"),
            }
        }
        if !self.ended_natively() {
            return Err(Error::UnexpectedEndOfStream);
        }
        Ok(collected)
    }
}

impl<T: StreamDecoder<T>> Drop for Subscription<T> {
    /// Cancel subscription on drop
    fn drop(&mut self) {
        debug!("dropping subscription");
        self.cancel();
    }
}

/// Adapter that filters `SubscriptionItem::Notice` items (logging them at `warn!`)
/// from any `Iterator<Item = Result<SubscriptionItem<T>, Error>>` and yields the
/// underlying `Result<T, Error>` to the caller.
///
/// Returned by [`SubscriptionItemIterExt::filter_data`].
#[must_use = "iterator adapters are lazy and do nothing unless consumed"]
pub struct FilterData<I> {
    inner: I,
}

impl<I, T> Iterator for FilterData<I>
where
    I: Iterator<Item = Result<SubscriptionItem<T>, Error>>,
{
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(out) = filter_notice(self.inner.next()?) {
                return Some(out);
            }
        }
    }
}

/// Extension trait that adds [`filter_data`](SubscriptionItemIterExt::filter_data)
/// to any iterator yielding `Result<SubscriptionItem<T>, Error>`. Use it to compose
/// the data-only flow with iterator combinators that the built-in
/// [`iter_data`](Subscription::iter_data) family doesn't already cover, e.g.
/// `subscription.iter().take(10).filter_data()`.
pub trait SubscriptionItemIterExt: Iterator + Sized {
    /// Wrap `self` in a [`FilterData`] adapter that drops `SubscriptionItem::Notice`
    /// items (logging them) and yields the underlying `Result<T, Error>`.
    fn filter_data<T>(self) -> FilterData<Self>
    where
        Self: Iterator<Item = Result<SubscriptionItem<T>, Error>>,
    {
        FilterData { inner: self }
    }
}

impl<I: Iterator> SubscriptionItemIterExt for I {}

/// Blocking iterator over `Result<SubscriptionItem<T>, Error>`.
#[allow(private_bounds)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
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
#[must_use = "iterators are lazy and do nothing unless consumed"]
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
#[must_use = "iterators are lazy and do nothing unless consumed"]
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
#[must_use = "iterators are lazy and do nothing unless consumed"]
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

#[cfg(all(test, feature = "sync"))]
#[path = "sync_tests.rs"]
mod tests;
