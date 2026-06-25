use super::*;
use crate::market_data::realtime::Bar;
use crate::messages::{Notice, OutgoingMessages};
use crate::stubs::MessageBusStub;
use crate::subscriptions::common::RoutedItem;
use crate::subscriptions::SubscriptionItem;
use crate::subscriptions::SubscriptionItemStreamExt;
use futures::StreamExt;
use std::sync::RwLock;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn test_subscription_with_decoder() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["1|9000|20241231 12:00:00|100.5|101.0|100.0|100.25|1000|100.2|5|0".to_string()],
        ordered_responses: vec![],
    });

    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

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
        DecoderContext::default(),
    );

    // Send a test message
    let msg = ResponseMessage::from("1\09000\020241231 12:00:00\0100.5\0101.0\0100.0\0100.25\01000\0100.2\05\00");
    tx.send(msg.into()).unwrap();

    // Test that we can receive the decoded message
    let mut sub = subscription;
    let Some(Ok(SubscriptionItem::Data(bar))) = sub.next().await else {
        panic!("expected Data");
    };
    assert_eq!(bar.open, 100.5);
    assert_eq!(bar.high, 101.0);
}

#[tokio::test]
async fn test_subscription_new_from_receiver() {
    let (tx, rx) = mpsc::unbounded_channel();

    let mut subscription = Subscription::new(rx);

    // Send test data
    tx.send(Ok("test".to_string())).unwrap();

    assert!(matches!(
        subscription.next().await,
        Some(Ok(SubscriptionItem::Data(ref s))) if s == "test"
    ));
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
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let result = subscription.next().await;
    assert!(matches!(result, Some(Err(Error::ConnectionReset))));
}

#[tokio::test]
async fn test_routed_item_notice_skipped_then_response_delivered() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("data".to_string()),
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
        advanced_order_reject_json: String::new(),
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("payload\0"))).unwrap();

    // First item via the raw Stream is the Notice (passes through);
    // filter_data() drops it and yields the Data.
    let mut data = subscription.filter_data();
    assert!(matches!(data.next().await, Some(Ok(ref s)) if s == "data"));
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
        DecoderContext::default(),
    );

    // Send a message that will trigger the error
    let msg = ResponseMessage::from("test\0");
    tx.send(msg.into()).unwrap();

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
        message_bus,
        |_context, _msg| Err(Error::EndOfStream),
        None,
        None,
        DecoderContext::default(),
    );

    // Send a message that will trigger end of stream
    let msg = ResponseMessage::from("test\0");
    tx.send(msg.into()).unwrap();

    let result = subscription.next().await;
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
                Err(Error::unexpected_response(&ResponseMessage::from("stray\0")))
            }
        },
        None,
        None,
        DecoderContext::default(),
    );

    // First message triggers EndOfStream
    tx.send(ResponseMessage::from("end\0").into()).unwrap();
    let result = subscription.next().await;
    assert!(result.is_none());

    // Send stray messages after stream ended
    tx.send(ResponseMessage::from("stray1\0").into()).unwrap();
    tx.send(ResponseMessage::from("stray2\0").into()).unwrap();

    // Subsequent calls should return None immediately without invoking decoder
    let result = subscription.next().await;
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
                Err(Error::unexpected_response(&ResponseMessage::from("stray\0")))
            } else {
                Ok("success".to_string())
            }
        },
        None,
        None,
        DecoderContext::default(),
    );

    // Send 21 messages — 20 will be "unexpected" (skipped), 1 will succeed
    for _ in 0..21 {
        tx.send(ResponseMessage::from("msg\0").into()).unwrap();
    }

    assert!(
        matches!(
            subscription.next().await,
            Some(Ok(SubscriptionItem::Data(ref s))) if s == "success"
        ),
        "subscription should not have stopped after skipping unexpected messages"
    );
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
        DecoderContext::default()
            .with_smart_depth(true)
            .with_request_type(OutgoingMessages::RequestPositions),
    );

    let cloned = subscription.clone();
    assert_eq!(cloned.request_id, Some(456));
    assert_eq!(cloned.order_id, Some(789));
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

    let subscription: Subscription<String> = Subscription::new_from_internal_simple::<TestDecoder>(internal, message_bus, DecoderContext::default());

    assert!(subscription.cancel_fn.is_some());
}

#[tokio::test]
async fn test_data_stream_collects_data_items() {
    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let subscription = Subscription::new(rx);

    tx.send(Ok("a".to_string())).unwrap();
    tx.send(Ok("b".to_string())).unwrap();
    drop(tx);

    let collected: Vec<_> = subscription.filter_data().collect().await;
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].as_ref().unwrap(), "a");
    assert_eq!(collected[1].as_ref().unwrap(), "b");
}

#[tokio::test]
async fn test_data_stream_yields_error_then_ends() {
    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let subscription = Subscription::new(rx);

    tx.send(Ok("first".to_string())).unwrap();
    tx.send(Err(Error::ConnectionReset)).unwrap();
    tx.send(Ok("should-not-be-yielded".to_string())).unwrap();

    let mut stream = subscription.filter_data();

    let first = stream.next().await;
    assert_eq!(first.unwrap().unwrap(), "first");

    let second = stream.next().await;
    assert!(matches!(second, Some(Err(Error::ConnectionReset))));

    let third = stream.next().await;
    assert!(third.is_none(), "stream must end after a terminal error");
}

#[tokio::test]
async fn test_data_stream_filters_notices() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("data".to_string()),
        None,
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Notice(Notice {
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("payload\0"))).unwrap();
    drop(tx);

    let collected: Vec<_> = subscription.filter_data().collect().await;
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].as_ref().unwrap(), "data");
}

/// PR 3: dispatcher emits `RoutedItem::Notice`; `Subscription<T>::next()`
/// surfaces it as `SubscriptionItem::Notice` without terminating the stream.
#[tokio::test]
async fn test_routed_item_notice_surfaces_as_subscription_item() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<String> = Subscription::with_decoder(
        internal,
        message_bus,
        |_context, _msg| Ok("data".to_string()),
        None,
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Notice(Notice {
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("payload\0"))).unwrap();

    match subscription.next().await {
        Some(Ok(SubscriptionItem::Notice(n))) => {
            assert_eq!(n.code, 2104);
            assert_eq!(n.message, "Market data farm OK");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(_)))));
}

#[tokio::test]
async fn test_stream_yields_error_then_ends() {
    let (tx, rx) = mpsc::unbounded_channel::<Result<String, Error>>();
    let mut subscription = Subscription::new(rx);

    tx.send(Ok("first".to_string())).unwrap();
    tx.send(Err(Error::ConnectionReset)).unwrap();
    tx.send(Ok("should-not-be-yielded".to_string())).unwrap();

    let stream = &mut subscription;

    let first = stream.next().await;
    assert!(matches!(first, Some(Ok(SubscriptionItem::Data(ref s))) if s == "first"));

    let second = stream.next().await;
    assert!(matches!(second, Some(Err(Error::ConnectionReset))));

    let third = stream.next().await;
    assert!(third.is_none(), "stream must end after a terminal error");
}

// ---- Stream-impl / SubscriptionItemStreamExt regression tests (Commit 1) ----

/// Exercises `impl Stream for Subscription<T>` end-to-end via `StreamExt`:
/// one-shot `next().await` and combinator chaining on `&mut subscription`.
#[tokio::test]
async fn subscription_impls_stream() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel::<RoutedItem>(16);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<i32> = Subscription::with_decoder(
        internal,
        message_bus,
        |_ctx, msg| Ok(msg.peek_int(0).unwrap_or_default()),
        Some(1),
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Response(ResponseMessage::from("10\0"))).unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("20\0"))).unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("30\0"))).unwrap();

    // One-shot read via StreamExt::next.
    let first = subscription.next().await;
    assert!(matches!(first, Some(Ok(SubscriptionItem::Data(10)))));

    // Combinator chain on &mut subscription (subscription stays usable after).
    let next_two: Vec<_> = (&mut subscription).take(2).collect().await;
    assert_eq!(next_two.len(), 2);
    assert!(matches!(next_two[0], Ok(SubscriptionItem::Data(20))));
    assert!(matches!(next_two[1], Ok(SubscriptionItem::Data(30))));
}

/// `SubscriptionItemStreamExt::filter_data` drops `Notice` items (logged) and
/// yields the underlying `Result<T, Error>`. Errors must still propagate.
#[tokio::test]
async fn filter_data_stream_drops_notices() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel::<RoutedItem>(16);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<i32> = Subscription::with_decoder(
        internal,
        message_bus,
        |_ctx, msg| Ok(msg.peek_int(0).unwrap_or_default()),
        Some(7),
        None,
        DecoderContext::default(),
    );

    tx.send(RoutedItem::Response(ResponseMessage::from("11\0"))).unwrap();
    tx.send(RoutedItem::Notice(Notice {
        code: 2104,
        message: "data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }))
    .unwrap();
    tx.send(RoutedItem::Response(ResponseMessage::from("13\0"))).unwrap();
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let mut data = subscription.filter_data();
    assert!(matches!(data.next().await, Some(Ok(11))));
    // Notice is filtered (logged at warn!) — the next yielded item is the 13 payload.
    assert!(matches!(data.next().await, Some(Ok(13))));
    // Errors still propagate through filter_data.
    assert!(matches!(data.next().await, Some(Err(Error::ConnectionReset))));
    // After a terminal error, the stream is exhausted.
    assert!(data.next().await.is_none());
}

/// The `PreDecoded` arm of `Subscription` is polled via
/// `mpsc::UnboundedReceiver::poll_recv` inside the `Stream` impl — exercise
/// it explicitly so the dispatch arm stays covered.
#[tokio::test]
async fn pre_decoded_subscription_polls() {
    let (tx, rx) = mpsc::unbounded_channel::<Result<u32, Error>>();
    let mut subscription: Subscription<u32> = Subscription::new(rx);

    tx.send(Ok(1)).unwrap();
    tx.send(Ok(2)).unwrap();
    drop(tx); // close the channel so the stream eventually terminates.

    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(1)))));
    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(2)))));
    // After the sender drops, the stream is exhausted.
    assert!(subscription.next().await.is_none());
}

// --- collect_for / collect_until ----------------------------------------

use std::time::Duration;

/// Test decoder for the collect tests: payload is text field 0; the value `-1`
/// marks a snapshot-end sentinel (mirrors `TickTypes::SnapshotEnd`).
#[derive(Debug, PartialEq)]
struct CollectItem(i32);

impl StreamDecoder<CollectItem> for CollectItem {
    fn decode(_context: &DecoderContext, msg: &mut ResponseMessage) -> Result<CollectItem, Error> {
        Ok(CollectItem(msg.peek_int(0)?))
    }

    fn is_snapshot_end(&self) -> bool {
        self.0 == -1
    }
}

fn collect_data(value: i32) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("{value}\0")))
}

/// Build a `Subscription<CollectItem>` pre-loaded with `items`. When `keep_open`
/// the broadcast sender is returned so the channel stays open (lets the timeout
/// branch fire); otherwise it is dropped so the stream ends after draining.
fn collect_subscription(items: Vec<RoutedItem>, keep_open: bool) -> (Subscription<CollectItem>, Option<broadcast::Sender<RoutedItem>>) {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);
    for item in items {
        tx.send(item).unwrap();
    }
    let sub = Subscription::<CollectItem>::with_decoder(internal, message_bus, CollectItem::decode, None, None, DecoderContext::default());
    let keep = if keep_open { Some(tx) } else { None };
    (sub, keep)
}

#[tokio::test]
async fn test_collect_for_stops_at_snapshot_end() {
    let (mut sub, _keep) = collect_subscription(vec![collect_data(10), collect_data(20), collect_data(-1), collect_data(30)], true);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}

#[tokio::test]
async fn test_collect_until_stops_on_predicate() {
    let (mut sub, _keep) = collect_subscription(vec![collect_data(10), collect_data(20), collect_data(30), collect_data(40)], true);

    let collected = sub.collect_until(Duration::from_secs(30), |items| items.len() >= 2).await;

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}

#[tokio::test]
async fn test_collect_for_returns_prefix_on_terminal_error() {
    let (mut sub, _keep) = collect_subscription(vec![collect_data(10), RoutedItem::Error(Error::ConnectionReset), collect_data(20)], true);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![CollectItem(10)]);
}

#[tokio::test]
async fn test_collect_for_returns_empty_on_timeout() {
    // Channel stays open with no data; the total timeout bounds the wait.
    let (mut sub, _keep) = collect_subscription(vec![], true);

    let collected = sub.collect_for(Duration::from_millis(50)).await;

    assert!(collected.is_empty());
}

#[tokio::test]
async fn test_collect_for_drains_to_stream_end() {
    let (mut sub, _keep) = collect_subscription(vec![collect_data(10), collect_data(20), collect_data(30)], false);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20), CollectItem(30)]);
}

#[tokio::test]
async fn test_collect_for_filters_notices() {
    let notice = RoutedItem::Notice(Notice {
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    });
    let (mut sub, _keep) = collect_subscription(vec![collect_data(10), notice, collect_data(20)], false);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}
