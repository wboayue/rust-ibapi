//! Asynchronous subscription implementation

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::stream::Stream;
use log::{debug, warn};
use tokio::sync::mpsc;

use super::common::{filter_notice, process_decode_result, DecoderContext, ProcessingResult, RoutedItem, SubscriptionItem};
use super::StreamDecoder;
use crate::messages::ResponseMessage;
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};
use crate::Error;

// Type aliases to reduce complexity
type CancelFn = Box<dyn Fn(i32, Option<i32>, Option<&DecoderContext>) -> Result<Vec<u8>, Error> + Send + Sync>;
type DecoderFn<T> = Arc<dyn Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync>;

/// Asynchronous subscription for streaming data.
///
/// `Subscription<T>` implements [`futures::Stream`] with
/// `Item = Result<SubscriptionItem<T>, Error>`:
///
/// * `None` — the stream has ended.
/// * `Some(Ok(SubscriptionItem::Data(t)))` — a decoded value.
/// * `Some(Ok(SubscriptionItem::Notice(n)))` — a non-fatal IB notice (warning code
///   2100..=2169 or order-cancel code 202) carried on this subscription's
///   `request_id`; the stream stays open.
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
/// When you only care about data, use the [`SubscriptionItemStreamExt::filter_data`]
/// adapter to filter notices (logged at `warn!`):
///
/// ```no_run
/// # use ibapi::subscriptions::SubscriptionItemStreamExt;
/// # use futures::StreamExt;
/// # async fn run(mut subscription: ibapi::subscriptions::Subscription<i32>) {
/// let mut data = (&mut subscription).filter_data();
/// while let Some(result) = data.next().await { /* ... */ }
/// # }
/// ```
///
/// Notices that are *not* tied to a specific subscription — connectivity codes
/// 1100/1101/1102, farm-status 2104/2105/2106/2107/2108, etc. — are not delivered
/// here. Subscribe to them via [`Client::notice_stream`](crate::Client::notice_stream)
/// instead.
pub struct Subscription<T> {
    inner: SubscriptionInner<T>,
    /// Metadata for cancellation
    request_id: Option<i32>,
    order_id: Option<i32>,
    context: DecoderContext,
    /// Shared across clones — one `cancel()` call disables future cancel sends from any clone.
    cancelled: Arc<AtomicBool>,
    /// Per-clone — each clone has its own `BroadcastStream` position, so a terminal event
    /// on one clone must not short-circuit other clones' polls.
    stream_ended: AtomicBool,
    message_bus: Option<Arc<dyn AsyncMessageBus>>,
    /// Cancel message generator
    cancel_fn: Option<Arc<CancelFn>>,
}

enum SubscriptionInner<T> {
    /// Subscription with decoder - receives ResponseMessage and decodes to T.
    /// The `context` for decode lives on the outer `Subscription<T>`.
    WithDecoder {
        subscription: AsyncInternalSubscription,
        decoder: DecoderFn<T>,
    },
    /// Pre-decoded subscription - receives T directly
    PreDecoded { receiver: mpsc::UnboundedReceiver<Result<T, Error>> },
}

impl<T> Clone for SubscriptionInner<T> {
    fn clone(&self) -> Self {
        match self {
            SubscriptionInner::WithDecoder { subscription, decoder } => SubscriptionInner::WithDecoder {
                subscription: subscription.clone(),
                decoder: decoder.clone(),
            },
            SubscriptionInner::PreDecoded { .. } => {
                // Can't clone mpsc receivers
                panic!("Cannot clone pre-decoded subscriptions");
            }
        }
    }
}

impl<T> Clone for Subscription<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            request_id: self.request_id,
            order_id: self.order_id,
            context: self.context.clone(),
            cancelled: self.cancelled.clone(),
            // Clone gets a fresh stream_ended — independent BroadcastStream position.
            stream_ended: AtomicBool::new(false),
            message_bus: self.message_bus.clone(),
            cancel_fn: self.cancel_fn.clone(),
        }
    }
}

impl<T> Subscription<T> {
    /// Create a subscription from an internal subscription and a decoder.
    ///
    /// `pub(crate)` because the parameter types (`AsyncInternalSubscription`,
    /// `DecoderContext`) are not part of the public API. External callers
    /// reach subscriptions via the typed builders on `Client`.
    pub(crate) fn with_decoder<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: D,
        request_id: Option<i32>,
        order_id: Option<i32>,
        context: DecoderContext,
    ) -> Self
    where
        D: Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self {
            inner: SubscriptionInner::WithDecoder {
                subscription: internal,
                decoder: Arc::new(decoder),
            },
            request_id,
            order_id,
            context,
            cancelled: Arc::new(AtomicBool::new(false)),
            stream_ended: AtomicBool::new(false),
            message_bus: Some(message_bus),
            cancel_fn: None,
        }
    }

    /// Create a subscription from an internal subscription using the DataStream decoder
    pub(crate) fn new_from_internal<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        context: DecoderContext,
    ) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        let mut sub = Self::with_decoder(internal, message_bus, D::decode, request_id, order_id, context);
        sub.cancel_fn = Some(Arc::new(Box::new(D::cancel_message)));
        sub
    }

    /// Create a subscription from internal subscription without explicit metadata.
    /// AsyncInternalSubscription's Drop carries the cancel signal, so no cancel-fn metadata.
    pub(crate) fn new_from_internal_simple<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        context: DecoderContext,
    ) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        Self::new_from_internal::<D>(internal, message_bus, None, None, context)
    }

    /// Create subscription from existing receiver (for backward compatibility)
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        // This creates a subscription that expects pre-decoded messages
        // Used for compatibility with existing code that manually decodes
        Self {
            inner: SubscriptionInner::PreDecoded { receiver },
            request_id: None,
            order_id: None,
            context: DecoderContext::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
            stream_ended: AtomicBool::new(false),
            message_bus: None,
            cancel_fn: None,
        }
    }

    /// Get the request ID associated with this subscription
    pub fn request_id(&self) -> Option<i32> {
        self.request_id
    }
}

impl<T: Send + 'static> Stream for Subscription<T> {
    type Item = Result<SubscriptionItem<T>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Subscription<T> is auto-Unpin: BroadcastStream uses ReusableBoxFuture
        // (boxed → Unpin externally), mpsc::UnboundedReceiver is Unpin, and
        // every other field is Unpin. Safe to project to &mut Self.
        let this = self.get_mut();

        if this.stream_ended.load(Ordering::Relaxed) {
            return Poll::Ready(None);
        }

        let Subscription {
            inner,
            context,
            stream_ended,
            ..
        } = this;
        loop {
            match inner {
                SubscriptionInner::WithDecoder { subscription, decoder } => {
                    // Drain the BroadcastStream synchronously while items are
                    // ready, so we can apply Skip without re-yielding to the
                    // executor between immediately-available items.
                    let routed = match Pin::new(&mut subscription.stream).poll_next(cx) {
                        Poll::Ready(Some(Ok(item))) => item,
                        Poll::Ready(Some(Err(_lagged))) => continue, // skip BroadcastStream lag
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => return Poll::Pending,
                    };

                    match routed {
                        RoutedItem::Response(mut message) => {
                            let result = decoder(context, &mut message);
                            match process_decode_result(result) {
                                ProcessingResult::Success(val) => return Poll::Ready(Some(Ok(SubscriptionItem::Data(val)))),
                                ProcessingResult::EndOfStream => {
                                    stream_ended.store(true, Ordering::Relaxed);
                                    return Poll::Ready(None);
                                }
                                ProcessingResult::Skip => {
                                    log::trace!("skipping unexpected message on shared channel");
                                    continue;
                                }
                                ProcessingResult::Error(err) => {
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
                SubscriptionInner::PreDecoded { receiver } => {
                    return match receiver.poll_recv(cx) {
                        Poll::Ready(Some(Ok(t))) => Poll::Ready(Some(Ok(SubscriptionItem::Data(t)))),
                        Poll::Ready(Some(Err(e))) => {
                            stream_ended.store(true, Ordering::Relaxed);
                            Poll::Ready(Some(Err(e)))
                        }
                        Poll::Ready(None) => Poll::Ready(None),
                        Poll::Pending => Poll::Pending,
                    };
                }
            }
        }
    }
}

impl<T> Subscription<T> {
    /// Cancel the subscription
    pub async fn cancel(&self) {
        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }

        self.cancelled.store(true, Ordering::Relaxed);

        if let (Some(message_bus), Some(cancel_fn)) = (&self.message_bus, &self.cancel_fn) {
            let id = self.request_id.or(self.order_id);
            if let Ok(message) = cancel_fn(self.context.server_version, id, Some(&self.context)) {
                if let Err(e) = message_bus.send_message(message).await {
                    warn!("error sending cancel message: {e}")
                }
            }
        }
    }
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        debug!("dropping async subscription");

        // Check if already cancelled
        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }

        self.cancelled.store(true, Ordering::Relaxed);

        // Try to send cancel message if we have the necessary components
        if let (Some(message_bus), Some(cancel_fn)) = (&self.message_bus, &self.cancel_fn) {
            let message_bus = message_bus.clone();
            let id = self.request_id.or(self.order_id);
            let context = self.context.clone();

            if let Ok(message) = cancel_fn(context.server_version, id, Some(&context)) {
                // Drop can't be async; spawn the cancel send so it actually goes out.
                tokio::spawn(async move {
                    if let Err(e) = message_bus.send_message(message).await {
                        warn!("error sending cancel message in drop: {e}");
                    }
                });
            }
        }
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
/// # async fn run(mut subscription: Subscription<i32>) {
/// let mut data = (&mut subscription).filter_data();
/// while let Some(result) = data.next().await { /* ... */ }
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
