//! Asynchronous subscription implementation

use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::stream::Stream;
use futures::StreamExt;
use log::{debug, warn};

use super::common::{drain_outcome, filter_notice, is_undeclared, notice_item, DecoderContext, Drained, RoutedItem, SubscriptionItem};
use super::{log_cancel_error, StreamDecoder};
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus, SharedTicket};
use crate::Error;

/// Asynchronous subscription for streaming data.
///
/// `Subscription<T>` implements [`futures::Stream`] with
/// `Item = Result<SubscriptionItem<T>, Error>`:
///
/// * `None` — the stream has ended.
/// * `Some(Ok(SubscriptionItem::Data(t)))` — a decoded value.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice (a warning
///   code in [`WARNING_CODE_RANGE`](crate::messages::WARNING_CODE_RANGE) or
///   order-cancel code 202) carried on this subscription's `request_id`; the
///   stream stays open.
/// * `Some(Err(e))` — terminal error; subsequent calls return `None`.
///
/// Consume via [`StreamExt`](futures::StreamExt):
///
/// ```no_run
/// # use ibapi::Client;
/// # use ibapi::contracts::Contract;
/// # use ibapi::subscriptions::SubscriptionItem;
/// # use futures::StreamExt;
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::connect("127.0.0.1:4002", 100).await?;
/// let contract = Contract::stock("AAPL").build();
/// let mut subscription = client.market_data(&contract).subscribe().await?;
///
/// while let Some(item) = subscription.next().await {
///     match item {
///         Ok(SubscriptionItem::Data(tick))   => println!("tick: {tick:?}"),
///         Ok(SubscriptionItem::Notice(n))    => eprintln!("notice: {n}"),
///         Err(e)                             => { eprintln!("error: {e}"); break; }
///     }
/// }
/// # Ok(()) }
/// ```
///
/// Dropping sends the cancel from a task spawned on the client's runtime, so a
/// subscription may be dropped on any thread, including one with no Tokio
/// runtime. If the client's runtime has shut down, nothing is sent: the
/// connection went with it.
///
/// Clones share one cancel: dropping or cancelling any clone ends the request
/// for every clone. For a shared stream the clones count as one subscription,
/// so the stream is cancelled at TWS even while another clone is still polling.
///
/// When you only care about data, use the [`SubscriptionItemStreamExt::filter_data`]
/// adapter to filter notices (logged at `warn!`):
///
/// ```no_run
/// # use ibapi::subscriptions::SubscriptionItemStreamExt;
/// # use futures::StreamExt;
/// # use ibapi::market_data::realtime::TickTypes;
/// # async fn run(subscription: ibapi::subscriptions::Subscription<TickTypes>) {
/// let mut data = subscription.filter_data();
/// while let Some(result) = data.next().await { /* result: Result<TickTypes, _> */ }
/// # }
/// ```
///
/// Notices that are *not* tied to a specific subscription — connectivity codes
/// 1100/1101/1102, farm-status 2104/2105/2106/2107/2108, etc. — are not delivered
/// here. Subscribe to them via [`Client::notice_stream`](crate::Client::notice_stream)
/// instead.
#[allow(private_bounds)]
#[must_use = "Subscription must be polled (via .next().await or .filter_data()) to receive data; dropping it cancels the request"]
pub struct Subscription<T: StreamDecoder<T>> {
    subscription: AsyncInternalSubscription,
    /// Metadata for cancellation
    request_id: Option<i32>,
    order_id: Option<i32>,
    /// Set for shared-channel subscriptions (no request or order id), so the
    /// cancel is routed through the bus's per-type count. Derived from the
    /// internal subscription's cleanup signal, never set by a caller.
    shared: Option<SharedTicket>,
    context: DecoderContext,
    /// Shared across clones — one `cancel()` call disables future cancel sends from any clone.
    cancelled: Arc<AtomicBool>,
    /// Shared across clones — set by `cancel()` only: each clone yields
    /// `Err(Cancelled)` at its next poll, then ends. Not `cancelled`, which
    /// `cancel_and_drain` also sets while it keeps reading.
    stopped: Arc<AtomicBool>,
    /// Shared across clones — set once a snapshot-end sentinel is observed, so drop/cancel
    /// skips the redundant cancel for an already-completed snapshot (mirrors the sync side).
    snapshot_ended: Arc<AtomicBool>,
    /// Shared across clones — set only by the native end marker, not by errors:
    /// once any clone has seen it, TWS has finished the request and there is
    /// nothing left to cancel.
    ended_natively: Arc<AtomicBool>,
    /// Per-clone — each clone has its own `BroadcastStream` position, so a terminal event
    /// on one clone must not short-circuit other clones' polls.
    stream_ended: AtomicBool,
    message_bus: Arc<dyn AsyncMessageBus>,
    /// `fn() -> T` rather than `T`: keeps the struct `Unpin`, `Send`, and `Sync`
    /// regardless of `T`, which `poll_next` relies on to project to `&mut Self`.
    phantom: PhantomData<fn() -> T>,
}

impl<T: StreamDecoder<T>> Clone for Subscription<T> {
    fn clone(&self) -> Self {
        Self {
            subscription: self.subscription.clone(),
            request_id: self.request_id,
            order_id: self.order_id,
            shared: self.shared,
            context: self.context.clone(),
            cancelled: self.cancelled.clone(),
            stopped: self.stopped.clone(),
            snapshot_ended: self.snapshot_ended.clone(),
            ended_natively: self.ended_natively.clone(),
            // Clone gets a fresh stream_ended — independent BroadcastStream position.
            stream_ended: AtomicBool::new(false),
            message_bus: self.message_bus.clone(),
            phantom: PhantomData,
        }
    }
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T>> Subscription<T> {
    /// Create a subscription from an internal subscription. `T` is the decoder:
    /// `poll_next` calls `T::decode` directly, as the sync side does.
    ///
    /// `pub(crate)` because the parameter types (`AsyncInternalSubscription`,
    /// `DecoderContext`) are not part of the public API. External callers
    /// reach subscriptions via the typed builders on `Client`.
    pub(crate) fn new_from_internal(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        context: DecoderContext,
    ) -> Self {
        super::common::debug_assert_request_id_routable::<T, T>(request_id);

        let shared = internal.shared_ticket();
        Self {
            subscription: internal,
            request_id,
            order_id,
            shared,
            context,
            cancelled: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
            snapshot_ended: Arc::new(AtomicBool::new(false)),
            ended_natively: Arc::new(AtomicBool::new(false)),
            stream_ended: AtomicBool::new(false),
            message_bus,
            phantom: PhantomData,
        }
    }

    /// Create a subscription from internal subscription without explicit metadata.
    /// AsyncInternalSubscription's Drop carries the cancel signal, so no id metadata.
    pub(crate) fn new_from_internal_simple(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        context: DecoderContext,
    ) -> Self {
        Self::new_from_internal(internal, message_bus, None, None, context)
    }

    /// Get the request ID associated with this subscription
    pub fn request_id(&self) -> Option<i32> {
        self.request_id
    }
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T> + Send + 'static> Subscription<T> {
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
    /// use ibapi::prelude::*;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let mut subscription = client.market_data(&contract).snapshot().subscribe().await.expect("request failed");
    ///
    ///     let ticks = subscription.collect_for(Duration::from_secs(5)).await;
    ///     println!("collected {} ticks", ticks.len());
    /// }
    /// ```
    pub async fn collect_for(&mut self, timeout: Duration) -> Vec<T> {
        self.collect_until(timeout, |_| false).await
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
    /// use ibapi::market_data::realtime::TickTypes;
    /// use ibapi::prelude::*;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let mut subscription = client.market_data(&contract).snapshot().subscribe().await.expect("request failed");
    ///
    ///     // Stop as soon as a price tick has arrived.
    ///     let ticks = subscription
    ///         .collect_until(Duration::from_secs(5), |ticks| {
    ///             ticks.iter().any(|t| matches!(t, TickTypes::Price(_) | TickTypes::PriceSize(_)))
    ///         })
    ///         .await;
    ///     println!("collected {} ticks", ticks.len());
    /// }
    /// ```
    pub async fn collect_until(&mut self, timeout: Duration, mut stop: impl FnMut(&[T]) -> bool) -> Vec<T> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut collected = Vec::new();
        loop {
            match tokio::time::timeout_at(deadline, self.next()).await {
                // Total deadline reached.
                Err(_elapsed) => break,
                // End of stream.
                Ok(None) => break,
                Ok(Some(Ok(SubscriptionItem::Data(value)))) => {
                    if value.is_snapshot_end() {
                        break;
                    }
                    collected.push(value);
                    if stop(&collected) {
                        break;
                    }
                }
                Ok(Some(Ok(SubscriptionItem::Notice(notice)))) => warn!("ib notice on subscription: {notice}"),
                Ok(Some(Err(e))) => {
                    warn!("subscription error during collect: {e}");
                    break;
                }
            }
        }
        collected
    }

    /// Collects every data item until TWS's end marker. Notices are logged at
    /// `warn!`. A terminal error is returned as is; a stream that ends without
    /// the end marker (a closed channel) is `Error::UnexpectedEndOfStream`.
    ///
    /// For request-scoped streams that end, such as contract details. Waits
    /// until the end, so not for open-ended subscriptions like market data.
    pub(crate) async fn collect_to_end(&mut self) -> Result<Vec<T>, Error> {
        let mut collected = Vec::new();
        while let Some(item) = self.next().await {
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
    /// Dropping the returned future mid-drain is safe: the cancel write runs in
    /// its own task and completes, the subscription drops, and nothing is
    /// written twice.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use tokio::time::{Duration, Instant};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL").build();
    ///     let mut subscription = client.contract_details_stream(&contract).subscribe().await.expect("request failed");
    ///     let first = subscription.next().await;
    ///     println!("first row: {first:?}");
    ///
    ///     match subscription.cancel_and_drain(Instant::now() + Duration::from_secs(10)).await {
    ///         Ok(Drained::Ended) | Ok(Drained::Rejected(_)) => println!("request finished at TWS"),
    ///         Ok(Drained::Unconfirmed) => println!("not confirmed; don't reuse the request slot yet"),
    ///         Err(e) => eprintln!("session error: {e}"),
    ///     }
    /// }
    /// ```
    pub async fn cancel_and_drain(mut self, deadline: tokio::time::Instant) -> Result<Drained, Error> {
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
            self.cancel().await;
            return Ok(Drained::Unconfirmed);
        }

        // Write the cancel and keep reading: the route stays until the
        // subscription drops, so TWS's end marker can still reach us. The
        // write runs in its own task, as `Drop`'s does, so dropping this
        // future mid-write can't lose it: `cancelled` is already set.
        if !self.cancelled.swap(true, Ordering::Relaxed) {
            if let Some(message) = self.pending_cancel(self.request_id) {
                let message_bus = self.message_bus.clone();
                let write = self.message_bus.runtime_handle().spawn(async move {
                    if let Err(e) = message_bus.send_message(message).await {
                        log_cancel_error("subscription", &e);
                    }
                });
                let _ = write.await;
            }
        }

        loop {
            match tokio::time::timeout_at(deadline, self.next()).await {
                Err(_elapsed) => return Ok(Drained::Unconfirmed),
                Ok(Some(item)) => {
                    if let Some(outcome) = drain_outcome(item) {
                        return outcome;
                    }
                }
                Ok(None) if self.ended_natively() => return Ok(Drained::Ended),
                // The channel closed without an end marker.
                Ok(None) => return Ok(Drained::Unconfirmed),
            }
        }
    }
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T> + Send + 'static> Stream for Subscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Subscription<T> is auto-Unpin: BroadcastStream uses ReusableBoxFuture
        // (boxed → Unpin externally), the phantom is `fn() -> T`, and every
        // other field is Unpin. Safe to project to &mut Self.
        let this = self.get_mut();

        if this.stream_ended.load(Ordering::Relaxed) {
            return Poll::Ready(None);
        }

        // `cancel()` ends the stream locally, reporting it once per clone.
        if this.stopped.load(Ordering::Relaxed) {
            this.stream_ended.store(true, Ordering::Relaxed);
            return Poll::Ready(Some(Err(Error::Cancelled)));
        }

        let Subscription {
            subscription,
            context,
            stream_ended,
            snapshot_ended,
            ended_natively,
            ..
        } = this;
        // Drain the BroadcastStream synchronously while items are ready, so
        // skipped frames don't re-yield to the executor between
        // immediately-available items.
        // Lag is converted to an in-band gap notice inside `poll_next_routed`
        // (#779); the Notice arm below delivers it.
        loop {
            let routed = match subscription.poll_next_routed(cx) {
                Poll::Ready(Some(item)) => item,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            match routed {
                RoutedItem::Response(message) => {
                    if is_undeclared(T::RESPONSE_MESSAGE_IDS, &message) {
                        log::trace!("skipping {:?} — not declared by this subscription's decoder", message.message_type());
                        continue;
                    }
                    match T::decode(context, &message) {
                        Ok(val) => {
                            if val.is_snapshot_end() {
                                snapshot_ended.store(true, Ordering::Relaxed);
                            }
                            return Poll::Ready(Some(Ok(SubscriptionItem::Data(val))));
                        }
                        Err(Error::EndOfStream) => {
                            stream_ended.store(true, Ordering::Relaxed);
                            ended_natively.store(true, Ordering::Relaxed);
                            return Poll::Ready(None);
                        }
                        Err(err) => {
                            stream_ended.store(true, Ordering::Relaxed);
                            return Poll::Ready(Some(Err(err)));
                        }
                    }
                }
                RoutedItem::Notice(notice) => return Poll::Ready(Some(Ok(notice_item(notice)))),
                RoutedItem::Error(Error::Notice(notice)) if T::is_nonterminal_notice(&notice) => {
                    return Poll::Ready(Some(Ok(SubscriptionItem::Notice(notice))));
                }
                RoutedItem::Error(Error::EndOfStream) => {
                    stream_ended.store(true, Ordering::Relaxed);
                    ended_natively.store(true, Ordering::Relaxed);
                    return Poll::Ready(None);
                }
                RoutedItem::Error(e) => {
                    stream_ended.store(true, Ordering::Relaxed);
                    return Poll::Ready(Some(Err(e)));
                }
            }
        }
    }
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T>> Subscription<T> {
    /// Cancel the subscription.
    ///
    /// Writes TWS's cancel unless the request has already finished (end
    /// marker or snapshot end) or its type has none. Either way the stream
    /// stops, for every clone: the next poll yields `Err(Error::Cancelled)`,
    /// frames not yet read are skipped, and the stream ends. Idempotent.
    pub async fn cancel(&self) {
        // The route itself is released at drop, by liveness.
        self.stopped.store(true, Ordering::Relaxed);

        // One atomic swap, not a load then a store: clones share this flag,
        // and two tasks cancelling at once would otherwise both reach the
        // bus, releasing a shared stream's count twice.
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        let id = self.request_id.or(self.order_id);
        let message = self.pending_cancel(id);
        if let Err(e) = send_cancel(&self.message_bus, id, self.shared, message).await {
            log_cancel_error("subscription", &e);
        }
    }
}

#[allow(private_bounds)]
impl<T: StreamDecoder<T>> Subscription<T> {
    /// The cancel to write, if any. `None` when the type has none, or once a
    /// request-id stream has seen its end marker or a snapshot its end: the
    /// cancel would name a request TWS already finished.
    fn pending_cancel(&self, id: Option<i32>) -> Option<Vec<u8>> {
        if self.request_id.is_some() && self.ended_natively.load(Ordering::Relaxed) || self.snapshot_ended.load(Ordering::Relaxed) {
            return None;
        }
        T::cancel_message(self.context.server_version, id, Some(&self.context)).ok()
    }

    /// Whether the stream ended with TWS's end marker, as opposed to an
    /// error or a closed channel (both of which also end the stream).
    pub(crate) fn ended_natively(&self) -> bool {
        self.ended_natively.load(Ordering::Relaxed)
    }
}

/// A shared-channel subscription (no id) ends through the bus's per-type
/// count, with or without a cancel message; everything else writes its
/// cancel directly, and has nothing to do without one.
///
/// Four arguments, over the budget: they are exactly what `Drop` moves into
/// its task, and a private helper gains nothing from a struct for them.
async fn send_cancel(
    message_bus: &Arc<dyn AsyncMessageBus>,
    id: Option<i32>,
    shared: Option<SharedTicket>,
    message: Option<Vec<u8>>,
) -> Result<(), Error> {
    match (id, shared, message) {
        (None, Some(ticket), message) => message_bus.cancel_shared_subscription(ticket, message).await,
        (_, _, Some(message)) => message_bus.send_message(message).await,
        (_, _, None) => Ok(()),
    }
}

impl<T: StreamDecoder<T>> Drop for Subscription<T> {
    fn drop(&mut self) {
        debug!("dropping async subscription");

        // Already cancelled, or being cancelled by a clone right now.
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        // Decoders without a cancel message (the `StreamDecoder` default) return
        // `Err(NotImplemented)`; a shared subscription still has its count to
        // release, an id-routed one has nothing to do.
        let id = self.request_id.or(self.order_id);
        let shared = self.shared;
        let message = self.pending_cancel(id);
        // Nothing to send and no count to release: nothing to spawn.
        if message.is_none() && shared.is_none() {
            return;
        }
        // Drop can't be async; the cancel is spawned on the bus's runtime,
        // which works from any thread, runtime or not.
        let message_bus = self.message_bus.clone();
        self.message_bus.runtime_handle().spawn(async move {
            if let Err(e) = send_cancel(&message_bus, id, shared, message).await {
                log_cancel_error("subscription", &e);
            }
        });
    }
}

/// Stream adapter that filters `SubscriptionItem::Notice` items (logging them
/// at `warn!`) from any `Stream<Item = Result<SubscriptionItem<T>, Error>>` and
/// yields the underlying `Result<T, Error>` to the caller.
///
/// Returned by [`SubscriptionItemStreamExt::filter_data`]. Async mirror of the
/// sync `FilterData` iterator adapter.
#[must_use = "streams are lazy and do nothing unless polled"]
pub struct FilterDataStream<S> {
    inner: S,
}

impl<S, T> Stream for FilterDataStream<S>
where
    S: Stream<Item = Result<SubscriptionItem<T>, Error>> + Unpin,
{
    type Item = Result<T, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    if let Some(out) = filter_notice(item) {
                        return Poll::Ready(Some(out));
                    }
                    // Filtered Notice; loop and poll again.
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Extension trait that adds [`filter_data`](SubscriptionItemStreamExt::filter_data)
/// to any stream yielding `Result<SubscriptionItem<T>, Error>`. Async mirror of
/// the sync `SubscriptionItemIterExt`.
///
/// Use it for the data-only flow when consuming a [`Subscription`]:
///
/// ```no_run
/// # use ibapi::subscriptions::{Subscription, SubscriptionItemStreamExt};
/// # use futures::StreamExt;
/// # use ibapi::market_data::realtime::TickTypes;
/// # async fn run(subscription: Subscription<TickTypes>) {
/// let mut data = subscription.filter_data();
/// while let Some(result) = data.next().await { /* result: Result<TickTypes, _> */ }
/// # }
/// ```
pub trait SubscriptionItemStreamExt: Stream + Sized {
    /// Wrap `self` in a [`FilterDataStream`] adapter that drops
    /// `SubscriptionItem::Notice` items (logging them) and yields the
    /// underlying `Result<T, Error>`.
    fn filter_data<T>(self) -> FilterDataStream<Self>
    where
        Self: Stream<Item = Result<SubscriptionItem<T>, Error>>,
    {
        FilterDataStream { inner: self }
    }
}

impl<S: Stream + Sized> SubscriptionItemStreamExt for S {}

#[cfg(all(test, feature = "async"))]
#[path = "async_tests.rs"]
mod tests;
