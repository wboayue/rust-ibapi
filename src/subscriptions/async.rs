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

use super::common::{filter_notice, is_undeclared, DecoderContext, RoutedItem, SubscriptionItem};
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
/// Drop the subscription inside a Tokio runtime: the cancel is sent from a
/// task spawned by `Drop`. A drop on a thread with no runtime logs a warning
/// and skips it; for a shared stream (`positions`, `account_updates`,
/// `news_bulletins`, ...) that also leaves the stream's live count one too
/// high until the next reconnect, so no later subscription of that type can
/// cancel it.
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
    /// Shared across clones — set once a snapshot-end sentinel is observed, so drop/cancel
    /// skips the redundant cancel for an already-completed snapshot (mirrors the sync side).
    snapshot_ended: Arc<AtomicBool>,
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
            snapshot_ended: self.snapshot_ended.clone(),
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
            snapshot_ended: Arc::new(AtomicBool::new(false)),
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

        let Subscription {
            subscription,
            context,
            stream_ended,
            snapshot_ended,
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
                            return Poll::Ready(None);
                        }
                        Err(err) => {
                            stream_ended.store(true, Ordering::Relaxed);
                            return Poll::Ready(Some(Err(err)));
                        }
                    }
                }
                RoutedItem::Notice(notice) => return Poll::Ready(Some(Ok(SubscriptionItem::Notice(notice)))),
                RoutedItem::Error(Error::EndOfStream) => {
                    stream_ended.store(true, Ordering::Relaxed);
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
    /// Cancel the subscription
    pub async fn cancel(&self) {
        // Snapshot subscriptions self-terminate after the snapshot-end sentinel;
        // their request is already complete, so skip the redundant cancel.
        if self.snapshot_ended.load(Ordering::Relaxed) {
            return;
        }

        // One atomic swap, not a load then a store: clones share this flag,
        // and two tasks cancelling at once would otherwise both reach the
        // bus, releasing a shared stream's count twice.
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        let id = self.request_id.or(self.order_id);
        let message = T::cancel_message(self.context.server_version, id, Some(&self.context)).ok();
        if let Err(e) = send_cancel(&self.message_bus, id, self.shared, message).await {
            log_cancel_error("subscription", &e);
        }
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

        // A completed snapshot needs no cancel — mirror the sync drop behavior.
        if self.snapshot_ended.load(Ordering::Relaxed) {
            return;
        }

        // Already cancelled, or being cancelled by a clone right now.
        if self.cancelled.swap(true, Ordering::Relaxed) {
            return;
        }

        // Decoders without a cancel message (the `StreamDecoder` default) return
        // `Err(NotImplemented)`; a shared subscription still has its count to
        // release, an id-routed one has nothing to do.
        let id = self.request_id.or(self.order_id);
        let shared = self.shared;
        let message = T::cancel_message(self.context.server_version, id, Some(&self.context)).ok();
        // Nothing to send and no count to release: nothing to spawn.
        if message.is_none() && shared.is_none() {
            return;
        }
        // Drop can't be async; the cancel send is spawned so it actually goes
        // out. With no runtime on this thread there is nowhere to spawn it, so
        // the cancel and the count release are skipped rather than panicking
        // in Drop. If the bus is still live (the drop happened on a plain
        // thread, not at runtime teardown), the type's count stays one too
        // high until the next reconnect, so no later subscription of the type
        // can write its cancel.
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            warn!("async subscription dropped outside a Tokio runtime: cancel not sent, shared count not released");
            return;
        };
        let message_bus = self.message_bus.clone();
        runtime.spawn(async move {
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
