//! Asynchronous subscription implementation

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, warn};
use tokio::sync::mpsc;

use super::common::{process_decode_result, ProcessingResult};
use super::{ResponseContext, StreamDecoder};
use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
use crate::transport::{AsyncInternalSubscription, AsyncMessageBus};
use crate::Error;

// Type aliases to reduce complexity
type CancelFn = Box<dyn Fn(i32, Option<i32>, Option<&ResponseContext>) -> Result<RequestMessage, Error> + Send + Sync>;
type DecoderFn<T> = Arc<dyn Fn(i32, &mut ResponseMessage) -> Result<T, Error> + Send + Sync>;

/// Asynchronous subscription for streaming data
pub struct Subscription<T> {
    inner: SubscriptionInner<T>,
    /// Metadata for cancellation
    request_id: Option<i32>,
    order_id: Option<i32>,
    _message_type: Option<OutgoingMessages>,
    response_context: ResponseContext,
    cancelled: Arc<AtomicBool>,
    server_version: i32,
    message_bus: Option<Arc<dyn AsyncMessageBus>>,
    /// Cancel message generator
    cancel_fn: Option<Arc<CancelFn>>,
}

enum SubscriptionInner<T> {
    /// Subscription with decoder - receives ResponseMessage and decodes to T
    WithDecoder {
        subscription: AsyncInternalSubscription,
        decoder: DecoderFn<T>,
        server_version: i32,
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
                server_version,
            } => SubscriptionInner::WithDecoder {
                subscription: subscription.clone(),
                decoder: decoder.clone(),
                server_version: *server_version,
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
            response_context: self.response_context.clone(),
            cancelled: self.cancelled.clone(),
            server_version: self.server_version,
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
        server_version: i32,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: D,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        response_context: ResponseContext,
    ) -> Self
    where
        D: Fn(i32, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self {
            inner: SubscriptionInner::WithDecoder {
                subscription: internal,
                decoder: Arc::new(decoder),
                server_version,
            },
            request_id,
            order_id,
            _message_type: message_type,
            response_context,
            cancelled: Arc::new(AtomicBool::new(false)),
            server_version,
            message_bus: Some(message_bus),
            cancel_fn: None,
        }
    }

    /// Create a subscription from an internal subscription with a decoder function
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_decoder<F>(
        internal: AsyncInternalSubscription,
        server_version: i32,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: F,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        response_context: ResponseContext,
    ) -> Self
    where
        F: Fn(i32, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self::with_decoder(
            internal,
            server_version,
            message_bus,
            decoder,
            request_id,
            order_id,
            message_type,
            response_context,
        )
    }

    /// Create a subscription from components and a decoder (alias for with_decoder)
    #[allow(clippy::too_many_arguments)]
    pub fn with_decoder_components<D>(
        internal: AsyncInternalSubscription,
        server_version: i32,
        message_bus: Arc<dyn AsyncMessageBus>,
        decoder: D,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        response_context: ResponseContext,
    ) -> Self
    where
        D: Fn(i32, &mut ResponseMessage) -> Result<T, Error> + Send + Sync + 'static,
    {
        Self::with_decoder(
            internal,
            server_version,
            message_bus,
            decoder,
            request_id,
            order_id,
            message_type,
            response_context,
        )
    }

    /// Create a subscription from an internal subscription using the DataStream decoder
    pub(crate) fn new_from_internal<D>(
        internal: AsyncInternalSubscription,
        server_version: i32,
        message_bus: Arc<dyn AsyncMessageBus>,
        request_id: Option<i32>,
        order_id: Option<i32>,
        message_type: Option<OutgoingMessages>,
        response_context: ResponseContext,
    ) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        let mut sub = Self::with_decoder_components(
            internal,
            server_version,
            message_bus,
            D::decode,
            request_id,
            order_id,
            message_type,
            response_context,
        );
        // Store the cancel function
        sub.cancel_fn = Some(Arc::new(Box::new(D::cancel_message)));
        sub
    }

    /// Create a subscription from internal subscription without explicit metadata
    pub(crate) fn new_from_internal_simple<D>(internal: AsyncInternalSubscription, server_version: i32, message_bus: Arc<dyn AsyncMessageBus>) -> Self
    where
        D: StreamDecoder<T> + 'static,
        T: 'static,
    {
        // The AsyncInternalSubscription already has cleanup logic, so we don't need cancel metadata
        Self::new_from_internal::<D>(internal, server_version, message_bus, None, None, None, ResponseContext::default())
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
            response_context: ResponseContext::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
            server_version: 0, // Default value for backward compatibility
            message_bus: None,
            cancel_fn: None,
        }
    }

    /// Get the next value from the subscription
    pub async fn next(&mut self) -> Option<Result<T, Error>>
    where
        T: 'static,
    {
        match &mut self.inner {
            SubscriptionInner::WithDecoder {
                subscription,
                decoder,
                server_version,
            } => loop {
                match subscription.next().await {
                    Some(Ok(mut message)) => {
                        let result = decoder(*server_version, &mut message);
                        match process_decode_result(result) {
                            ProcessingResult::Success(val) => return Some(Ok(val)),
                            ProcessingResult::EndOfStream => return None,
                            ProcessingResult::Retry => continue,
                            ProcessingResult::Error(err) => return Some(Err(err)),
                        }
                    }
                    Some(Err(e)) => return Some(Err(e)),
                    None => return None,
                }
            },
            SubscriptionInner::PreDecoded { receiver } => receiver.recv().await,
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
            if let Ok(message) = cancel_fn(self.server_version, id, Some(&self.response_context)) {
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
            let response_context = self.response_context.clone();
            let server_version = self.server_version;

            // Clone the cancel function for use in the spawned task
            if let Ok(message) = cancel_fn(server_version, id, Some(&response_context)) {
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
mod tests {
    use super::*;
    use crate::market_data::realtime::Bar;
    use crate::messages::OutgoingMessages;
    use crate::stubs::MessageBusStub;
    use std::sync::RwLock;
    use time::OffsetDateTime;
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn test_subscription_with_decoder() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["1|9000|20241231 12:00:00|100.5|101.0|100.0|100.25|1000|100.2|5|0".to_string()],
        });

        let (tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx.resubscribe());

        let subscription: Subscription<Bar> = Subscription::with_decoder(
            internal,
            176,
            message_bus,
            |_server_version, _msg| {
                let bar = Bar {
                    date: OffsetDateTime::now_utc(),
                    open: 100.5,
                    high: 101.0,
                    low: 100.0,
                    close: 100.25,
                    volume: 1000.0,
                    wap: 100.2,
                    count: 5,
                };
                Ok(bar)
            },
            Some(9000),
            None,
            Some(OutgoingMessages::RequestRealTimeBars),
            ResponseContext::default(),
        );

        // Send a test message
        let msg = ResponseMessage::from("1\09000\020241231 12:00:00\0100.5\0101.0\0100.0\0100.25\01000\0100.2\05\00");
        tx.send(msg).unwrap();

        // Test that we can receive the decoded message
        let mut sub = subscription;
        let result = sub.next().await;
        assert!(result.is_some());
        let bar = result.unwrap().unwrap();
        assert_eq!(bar.open, 100.5);
        assert_eq!(bar.high, 101.0);
    }

    #[tokio::test]
    async fn test_subscription_new_with_decoder() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let subscription: Subscription<String> = Subscription::new_with_decoder(
            internal,
            176,
            message_bus,
            |_version, _msg| Ok("decoded".to_string()),
            Some(1),
            None,
            Some(OutgoingMessages::RequestMarketData),
            ResponseContext::default(),
        );

        assert_eq!(subscription.request_id, Some(1));
        assert_eq!(subscription._message_type, Some(OutgoingMessages::RequestMarketData));
    }

    #[tokio::test]
    async fn test_subscription_with_decoder_components() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let subscription: Subscription<i32> = Subscription::with_decoder_components(
            internal,
            176,
            message_bus,
            |_version, _msg| Ok(42),
            Some(100),
            Some(200),
            Some(OutgoingMessages::RequestPositions),
            ResponseContext::default(),
        );

        assert_eq!(subscription.request_id, Some(100));
        assert_eq!(subscription.order_id, Some(200));
    }

    #[tokio::test]
    async fn test_subscription_new_from_receiver() {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut subscription = Subscription::new(rx);

        // Send test data
        tx.send(Ok("test".to_string())).unwrap();

        let result = subscription.next().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "test");
    }

    #[tokio::test]
    async fn test_subscription_next_with_error() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let mut subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            176,
            message_bus,
            |_version, _msg| Err(Error::Simple("decode error".into())),
            None,
            None,
            None,
            ResponseContext::default(),
        );

        // Send a message that will trigger the error
        let msg = ResponseMessage::from("test\0");
        tx.send(msg).unwrap();

        let result = subscription.next().await;
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_subscription_next_end_of_stream() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let mut subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            176,
            message_bus,
            |_version, _msg| Err(Error::EndOfStream),
            None,
            None,
            None,
            ResponseContext::default(),
        );

        // Send a message that will trigger end of stream
        let msg = ResponseMessage::from("test\0");
        tx.send(msg).unwrap();

        let result = subscription.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_subscription_cancel() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        // Mock cancel function
        let cancel_fn: CancelFn = Box::new(|_version, _id, _ctx| {
            let mut msg = RequestMessage::new();
            msg.push_field(&OutgoingMessages::CancelMarketData);
            Ok(msg)
        });

        let mut subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            176,
            message_bus.clone(),
            |_version, _msg| Ok("test".to_string()),
            Some(123),
            None,
            Some(OutgoingMessages::RequestMarketData),
            ResponseContext::default(),
        );
        subscription.cancel_fn = Some(Arc::new(cancel_fn));

        // Cancel the subscription
        subscription.cancel().await;

        // Verify cancelled flag is set
        assert!(subscription.cancelled.load(Ordering::Relaxed));

        // Cancel again should be a no-op
        subscription.cancel().await;
    }

    #[tokio::test]
    async fn test_subscription_clone() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            176,
            message_bus,
            |_version, _msg| Ok("test".to_string()),
            Some(456),
            Some(789),
            Some(OutgoingMessages::RequestPositions),
            ResponseContext {
                is_smart_depth: true,
                request_type: Some(OutgoingMessages::RequestPositions),
            },
        );

        let cloned = subscription.clone();
        assert_eq!(cloned.request_id, Some(456));
        assert_eq!(cloned.order_id, Some(789));
        assert_eq!(cloned._message_type, Some(OutgoingMessages::RequestPositions));
        assert!(cloned.response_context.is_smart_depth);
    }

    #[tokio::test]
    async fn test_subscription_drop_with_cancel() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        // Mock cancel function
        let cancel_fn: CancelFn = Box::new(|_version, _id, _ctx| {
            let mut msg = RequestMessage::new();
            msg.push_field(&OutgoingMessages::CancelMarketData);
            Ok(msg)
        });

        {
            let mut subscription: Subscription<String> = Subscription::with_decoder(
                internal,
                176,
                message_bus.clone(),
                |_version, _msg| Ok("test".to_string()),
                Some(999),
                None,
                Some(OutgoingMessages::RequestMarketData),
                ResponseContext::default(),
            );
            subscription.cancel_fn = Some(Arc::new(cancel_fn));
            // Subscription will be dropped here and should send cancel message
        }

        // Give async task time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot clone pre-decoded subscriptions")]
    async fn test_subscription_inner_clone_panic() {
        let (_tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
        let subscription = Subscription::new(rx);

        // This should panic because PreDecoded subscriptions can't be cloned
        let _ = subscription.inner.clone();
    }

    #[tokio::test]
    async fn test_subscription_with_context() {
        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let context = ResponseContext {
            is_smart_depth: true,
            request_type: Some(OutgoingMessages::RequestMarketDepth),
        };

        let subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            176,
            message_bus,
            |_version, _msg| Ok("test".to_string()),
            None,
            None,
            None,
            context.clone(),
        );

        assert_eq!(subscription.response_context, context);
    }

    #[tokio::test]
    async fn test_subscription_new_from_internal_simple() {
        // Define a simple decoder type
        struct TestDecoder;

        impl StreamDecoder<String> for TestDecoder {
            fn decode(_server_version: i32, _msg: &mut ResponseMessage) -> Result<String, Error> {
                Ok("decoded".to_string())
            }

            fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
                let mut msg = RequestMessage::new();
                msg.push_field(&OutgoingMessages::CancelMarketData);
                Ok(msg)
            }
        }

        let message_bus = Arc::new(MessageBusStub::default());
        let (_tx, rx) = broadcast::channel(100);
        let internal = AsyncInternalSubscription::new(rx);

        let subscription: Subscription<String> = Subscription::new_from_internal_simple::<TestDecoder>(internal, 176, message_bus);

        assert!(subscription.cancel_fn.is_some());
    }
}
