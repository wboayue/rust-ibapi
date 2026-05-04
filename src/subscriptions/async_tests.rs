use super::*;
use crate::market_data::realtime::Bar;
use crate::messages::{Notice, OutgoingMessages};
use crate::stubs::MessageBusStub;
use crate::subscriptions::common::RoutedItem;
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
        message_bus,
        |_context, _msg| {
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
        DecoderContext::default(),
    );

    // Send a test message
    let msg = ResponseMessage::from("1\09000\020241231 12:00:00\0100.5\0101.0\0100.0\0100.25\01000\0100.2\05\00");
    tx.send(msg.into()).unwrap();

    // Test that we can receive the decoded message
    let mut sub = subscription;
    let result = sub.next_data().await;
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
        message_bus,
        |_context, _msg| Ok("decoded".to_string()),
        Some(1),
        None,
        Some(OutgoingMessages::RequestMarketData),
        DecoderContext::default(),
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
        message_bus,
        |_context, _msg| Ok(42),
        Some(100),
        Some(200),
        Some(OutgoingMessages::RequestPositions),
        DecoderContext::default(),
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

    let result = subscription.next_data().await;
    assert!(result.is_some());
    assert_eq!(result.unwrap().unwrap(), "test");
}

#[tokio::test]
async fn test_pre_decoded_error_terminates_stream() {
    // Regression: the PreDecoded arm of `next()` previously did not flip
    // `stream_ended` when surfacing an error, so subsequent calls would
    // re-poll the receiver instead of returning `None` deterministically.
    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let mut subscription = Subscription::new(rx);

    tx.send(Err(Error::ConnectionReset)).unwrap();
    tx.send(Ok("should-not-be-yielded".to_string())).unwrap();

    let first = subscription.next().await;
    assert!(matches!(first, Some(Err(Error::ConnectionReset))));

    let second = subscription.next().await;
    assert!(second.is_none(), "stream must terminate after a terminal error");
}

#[tokio::test]
async fn test_routed_item_error_surfaces_through_async_subscription() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    // Decoder would succeed if it ran — but the channel emits a terminal
    // RoutedItem::Error that the consumer should surface directly without
    // ever invoking the decoder.
    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("should-not-be-called".to_string()),
        None,
        None,
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let result = subscription.next_data().await;
    assert!(matches!(result, Some(Err(Error::ConnectionReset))));
}

#[tokio::test]
async fn test_routed_item_notice_skipped_then_response_delivered() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("data".to_string()),
        None,
        None,
        None,
        DecoderContext::default(),
    );

    // PR 2a never emits Notice from the dispatcher, but the receiver-side
    // contract is that notices are silently consumed and the next item is
    // delivered. Lock that contract before PR 3 starts emitting them.
    tx.send(RoutedItem::Notice(Notice {
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("payload\0"))).unwrap();

    let result = subscription.next_data().await;
    assert_eq!(result.unwrap().unwrap(), "data");
}

#[tokio::test]
async fn test_subscription_next_with_error() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Err(Error::Simple("decode error".into())),
        None,
        None,
        None,
        DecoderContext::default(),
    );

    // Send a message that will trigger the error
    let msg = ResponseMessage::from("test\0");
    tx.send(msg.into()).unwrap();

    let result = subscription.next_data().await;
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
        message_bus,
        |_context, _msg| Err(Error::EndOfStream),
        None,
        None,
        None,
        DecoderContext::default(),
    );

    // Send a message that will trigger end of stream
    let msg = ResponseMessage::from("test\0");
    tx.send(msg.into()).unwrap();

    let result = subscription.next_data().await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_subscription_no_retries_after_end_of_stream() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        move |_context, _msg| {
            let n = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if n == 0 {
                Err(Error::EndOfStream)
            } else {
                Err(Error::UnexpectedResponse(ResponseMessage::from("stray\0")))
            }
        },
        None,
        None,
        None,
        DecoderContext::default(),
    );

    // First message triggers EndOfStream
    tx.send(ResponseMessage::from("end\0").into()).unwrap();
    let result = subscription.next_data().await;
    assert!(result.is_none());

    // Send stray messages after stream ended
    tx.send(ResponseMessage::from("stray1\0").into()).unwrap();
    tx.send(ResponseMessage::from("stray2\0").into()).unwrap();

    // Subsequent calls should return None immediately without invoking decoder
    let result = subscription.next_data().await;
    assert!(result.is_none());

    // Decoder should have been called only once (for the EndOfStream message)
    assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_subscription_skips_unexpected_messages_without_retry_limit() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    // Decoder: returns UnexpectedResponse for the first 20 messages (more than
    // MAX_DECODE_RETRIES=10), then returns a success value. If UnexpectedResponse
    // counted toward the retry limit, the subscription would give up after 10.
    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        move |_context, _msg| {
            let n = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if n < 20 {
                Err(Error::UnexpectedResponse(ResponseMessage::from("stray\0")))
            } else {
                Ok("success".to_string())
            }
        },
        None,
        None,
        None,
        DecoderContext::default(),
    );

    // Send 21 messages — 20 will be "unexpected" (skipped), 1 will succeed
    for _ in 0..21 {
        tx.send(ResponseMessage::from("msg\0").into()).unwrap();
    }

    let result = subscription.next_data().await;
    assert!(
        result.is_some(),
        "subscription should not have stopped after skipping unexpected messages"
    );
    assert_eq!(result.unwrap().unwrap(), "success");
    // All 21 messages should have been processed (20 skipped + 1 success)
    assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 21);
}

#[tokio::test]
async fn test_subscription_cancel() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (_tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    // Mock cancel function
    let cancel_fn: CancelFn =
        Box::new(|_version, _id, _ctx| Ok(crate::messages::encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[])));

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus.clone(),
        |_context, _msg| Ok("test".to_string()),
        Some(123),
        None,
        Some(OutgoingMessages::RequestMarketData),
        DecoderContext::default(),
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
        message_bus,
        |_context, _msg| Ok("test".to_string()),
        Some(456),
        Some(789),
        Some(OutgoingMessages::RequestPositions),
        DecoderContext::default()
            .with_smart_depth(true)
            .with_request_type(OutgoingMessages::RequestPositions),
    );

    let cloned = subscription.clone();
    assert_eq!(cloned.request_id, Some(456));
    assert_eq!(cloned.order_id, Some(789));
    assert_eq!(cloned._message_type, Some(OutgoingMessages::RequestPositions));
    assert!(cloned.context.is_smart_depth);
}

#[tokio::test]
async fn test_subscription_drop_with_cancel() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (_tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    // Mock cancel function
    let cancel_fn: CancelFn =
        Box::new(|_version, _id, _ctx| Ok(crate::messages::encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[])));

    {
        let mut subscription: Subscription<String> = Subscription::with_decoder(
            internal,
            message_bus.clone(),
            |_context, _msg| Ok("test".to_string()),
            Some(999),
            None,
            Some(OutgoingMessages::RequestMarketData),
            DecoderContext::default(),
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

    let context = DecoderContext::default()
        .with_smart_depth(true)
        .with_request_type(OutgoingMessages::RequestMarketDepth);

    let subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("test".to_string()),
        None,
        None,
        None,
        context.clone(),
    );

    assert_eq!(subscription.context, context);
}

#[tokio::test]
async fn test_subscription_new_from_internal_simple() {
    // Define a simple decoder type
    struct TestDecoder;

    impl StreamDecoder<String> for TestDecoder {
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<String, Error> {
            Ok("decoded".to_string())
        }

        fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
            Ok(crate::messages::encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[]))
        }
    }

    let message_bus = Arc::new(MessageBusStub::default());
    let (_tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<String> = Subscription::new_from_internal_simple::<TestDecoder>(internal, DecoderContext::default(), message_bus);

    assert!(subscription.cancel_fn.is_some());
}

#[tokio::test]
async fn test_data_stream_collects_data_items() {
    use futures::StreamExt;

    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let mut subscription = Subscription::new(rx);

    tx.send(Ok("a".to_string())).unwrap();
    tx.send(Ok("b".to_string())).unwrap();
    drop(tx);

    let collected: Vec<_> = subscription.data_stream().collect().await;
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].as_ref().unwrap(), "a");
    assert_eq!(collected[1].as_ref().unwrap(), "b");
}

#[tokio::test]
async fn test_data_stream_yields_error_then_ends() {
    use futures::StreamExt;

    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let mut subscription = Subscription::new(rx);

    tx.send(Ok("first".to_string())).unwrap();
    tx.send(Err(Error::ConnectionReset)).unwrap();
    tx.send(Ok("should-not-be-yielded".to_string())).unwrap();

    let mut stream = subscription.data_stream();

    let first = stream.next().await;
    assert_eq!(first.unwrap().unwrap(), "first");

    let second = stream.next().await;
    assert!(matches!(second, Some(Err(Error::ConnectionReset))));

    let third = stream.next().await;
    assert!(third.is_none(), "stream must end after a terminal error");
}

#[tokio::test]
async fn test_data_stream_filters_notices() {
    use futures::StreamExt;

    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("data".to_string()),
        None,
        None,
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Notice(Notice {
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("payload\0"))).unwrap();
    drop(tx);

    let collected: Vec<_> = subscription.data_stream().collect().await;
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].as_ref().unwrap(), "data");
}
