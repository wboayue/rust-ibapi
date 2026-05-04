//! Asynchronous subscription implementation

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, warn};
use tokio::sync::mpsc;

use super::common::{process_decode_result, DecoderContext, ProcessingResult, SubscriptionItem};
use super::StreamDecoder;
use crate::messages::{OutgoingMessages, ResponseMessage};
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};
use crate::Error;

// Type aliases to reduce complexity
type CancelFn = Box<dyn Fn(i32, Option<i32>, Option<&DecoderContext>) -> Result<Vec<u8>, Error> + Send + Sync>;
type DecoderFn<T> = Arc<dyn Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync>;

/// Asynchronous subscription for streaming data
pub struct Subscription<T> {
    inner: SubscriptionInner<T>,
    /// Metadata for cancellation
    request_id: Option<i32>,
    order_id: Option<i32>,
    _message_type: Option<OutgoingMessages>,
    context: DecoderContext,
    cancelled: Arc<AtomicBool>,
    stream_ended: Arc<AtomicBool>,
    message_bus: Option<Arc<dyn AsyncMessageBus>>,
    /// Cancel message generator
    cancel_fn: Option<Arc<CancelFn>>,
}

enum SubscriptionInner<T> {
    /// Subscription with decoder - receives ResponseMessage and decodes to T
    WithDecoder {
        subscription: AsyncInternalSubscription,
        decoder: DecoderFn<T>,
        context: DecoderContext,
    },
    /// Pre-decoded subscription - receives T directly
    PreDecoded { receiver: mpsc::UnboundedReceiver<Result<T, Error>> },
}

impl<T> Clone for SubscriptionInner<T> {
    fn clone(&self) -> Self {
        match self {
            SubscriptionInner::WithDecoder {
                subscription,
                decoder,
                context,
            } => SubscriptionInner::WithDecoder {
                subscription: subscription.clone(),
                decoder: decoder.clone(),
                context: context.clone(),
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
            _message_type: self._message_type,
            context: self.context.clone(),
            cancelled: self.cancelled.clone(),
            stream_ended: self.stream_ended.clone(),
            message_bus: self.message_bus.clone(),
            cancel_fn: self.cancel_fn.clone(),
        }
    }
}

impl<T> Subscription<T> {
    /// Create a subscription from an internal subscription and a decoder
    #[allow(clippy::too_many_arguments)]
    pub fn with_decoder<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: D,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        context: DecoderContext,
    ) -> Self
    where
        D: Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self {
            inner: SubscriptionInner::WithDecoder {
                subscription: internal,
                decoder: Arc::new(decoder),
                context: context.clone(),
            },
            request_id,
            order_id,
            _message_type: message_type,
            context,
            cancelled: Arc::new(AtomicBool::new(false)),
            stream_ended: Arc::new(AtomicBool::new(false)),
            message_bus: Some(message_bus),
            cancel_fn: None,
        }
    }

    /// Create a subscription from an internal subscription with a decoder function
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_decoder<F>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: F,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        context: DecoderContext,
    ) -> Self
    where
        F: Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self::with_decoder(internal, message_bus, decoder, request_id, order_id, message_type, context)
    }

    /// Create a subscription from components and a decoder (alias for with_decoder)
    #[allow(clippy::too_many_arguments)]
    pub fn with_decoder_components<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: D,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        context: DecoderContext,
    ) -> Self
    where
        D: Fn(&DecoderContext, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self::with_decoder(internal, message_bus, decoder, request_id, order_id, message_type, context)
    }

    /// Create a subscription from an internal subscription using the DataStream decoder
    pub(crate) fn new_from_internal<D>(
        internal: AsyncInternalSubscription,
        message_bus: Arc<dyn AsyncMessageBus>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        context: DecoderContext,
    ) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        let mut sub = Self::with_decoder_components(internal, message_bus, D::decode, request_id, order_id, message_type, context);
        // Store the cancel function
        sub.cancel_fn = Some(Arc::new(Box::new(D::cancel_message)));
        sub
    }

    /// Create a subscription from internal subscription without explicit metadata
    pub(crate) fn new_from_internal_simple<D>(
        internal: AsyncInternalSubscription,
        context: DecoderContext,
        message_bus: Arc<dyn AsyncMessageBus>,
    ) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        // The AsyncInternalSubscription already has cleanup logic, so we don't need cancel metadata
        Self::new_from_internal::<D>(internal, message_bus, None, None, None, context)
    }

    /// Create subscription from existing receiver (for backward compatibility)
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        // This creates a subscription that expects pre-decoded messages
        // Used for compatibility with existing code that manually decodes
        Self {
            inner: SubscriptionInner::PreDecoded { receiver },
            request_id: None,
            order_id: None,
            _message_type: None,
            context: DecoderContext::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
            stream_ended: Arc::new(AtomicBool::new(false)),
            message_bus: None,
            cancel_fn: None,
        }
    }

    /// Get the next item from the subscription.
    ///
    /// Returns `Option<Result<SubscriptionItem<T>, Error>>`:
    /// - `None` — stream ended.
    /// - `Some(Ok(SubscriptionItem::Data(t)))` — decoded payload.
    /// - `Some(Ok(SubscriptionItem::Notice(n)))` — non-fatal IB notice; stream stays open.
    /// - `Some(Err(e))` — terminal error; subsequent calls return `None`.
    ///
    /// The `Notice` arm is part of the public contract but not yet emitted: the
    /// dispatcher routes IB warnings (codes 2100..=2169) globally today and
    /// will surface them per-subscription in a future release. Match it
    /// defensively now to avoid a churned migration later.
    ///
    /// When you only care about data, use [`next_data`](Self::next_data) which
    /// filters notices.
    pub async fn next(&mut self) -> Option<Result<SubscriptionItem<T>, Error>>
    where
        T: 'static,
    {
        if self.stream_ended.load(Ordering::Relaxed) {
            return None;
        }

        match &mut self.inner {
            SubscriptionInner::WithDecoder {
                subscription,
                decoder,
                context,
            } => loop {
                match subscription.next().await {
                    Some(Ok(mut message)) => {
                        let result = decoder(context, &mut message);
                        match process_decode_result(result) {
                            ProcessingResult::Success(val) => return Some(Ok(SubscriptionItem::Data(val))),
                            ProcessingResult::EndOfStream => {
                                self.stream_ended.store(true, Ordering::Relaxed);
                                return None;
                            }
                            ProcessingResult::Skip => {
                                log::trace!("skipping unexpected message on shared channel");
                                continue;
                            }
                            ProcessingResult::Error(err) => {
                                self.stream_ended.store(true, Ordering::Relaxed);
                                return Some(Err(err));
                            }
                        }
                    }
                    Some(Err(Error::EndOfStream)) => {
                        self.stream_ended.store(true, Ordering::Relaxed);
                        return None;
                    }
                    Some(Err(e)) => {
                        self.stream_ended.store(true, Ordering::Relaxed);
                        return Some(Err(e));
                    }
                    None => return None,
                }
            },
            SubscriptionInner::PreDecoded { receiver } => match receiver.recv().await? {
                Ok(t) => Some(Ok(SubscriptionItem::Data(t))),
                Err(e) => {
                    self.stream_ended.store(true, Ordering::Relaxed);
                    Some(Err(e))
                }
            },
        }
    }

    /// Convenience: blocking `next` that filters notices and yields just data.
    pub async fn next_data(&mut self) -> Option<Result<T, Error>>
    where
        T: 'static,
    {
        loop {
            match self.next().await? {
                Ok(SubscriptionItem::Data(t)) => return Some(Ok(t)),
                Ok(SubscriptionItem::Notice(n)) => {
                    log::warn!("ib notice on subscription: {n}");
                    continue;
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }

    /// Get the request ID associated with this subscription
    pub fn request_id(&self) -> Option<i32> {
        self.request_id
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

        // The AsyncInternalSubscription's Drop will handle cleanup
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

            // Clone the cancel function for use in the spawned task
            if let Ok(message) = cancel_fn(context.server_version, id, Some(&context)) {
                // Spawn a task to send the cancel message since drop can't be async
                tokio::spawn(async move {
                    if let Err(e) = message_bus.send_message(message).await {
                        warn!("error sending cancel message in drop: {e}");
                    }
                });
            }
        }

        // The AsyncInternalSubscription's Drop will handle channel cleanup
    }
}

// Note: Stream trait implementation removed because tokio's broadcast::Receiver
// doesn't provide poll_recv. Users should use the async next() method instead.
// If Stream is needed, users can convert using futures::stream::unfold.

#[cfg(all(test, feature = "async"))]
#[path = "async_tests.rs"]
mod tests;
