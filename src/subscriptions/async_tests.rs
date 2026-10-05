use super::*;
use crate::common::test_utils::helpers::test_notice;
use crate::messages::{encode_protobuf_message, IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::stubs::MessageBusStub;
use crate::subscriptions::common::RoutedItem;
use crate::subscriptions::SubscriptionItemStreamExt;
use crate::subscriptions::{Drained, SubscriptionItem};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::broadcast;

// ---- Test decoders --------------------------------------------------------
//
// `Subscription<T>` dispatches statically on `T: StreamDecoder<T>`, so each
// decode behavior under test is a type. All declare `TickPrice` (message id 1);
// `int_frame` builds a frame the declared-id filter lets through.

/// Decodes the integer at field 1. The value `-1` marks a snapshot-end
/// sentinel (mirrors `TickTypes::SnapshotEnd`).
#[derive(Debug, PartialEq)]
struct IntItem(i32);

impl StreamDecoder<IntItem> for IntItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<IntItem, Error> {
        Ok(IntItem(msg.peek_int(1)?))
    }

    fn is_snapshot_end(&self) -> bool {
        self.0 == -1
    }
}

/// Like `IntItem`, with a cancel message, so cancel and drop have something to send.
#[derive(Debug)]
struct CancellableItem;

impl StreamDecoder<CancellableItem> for CancellableItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<CancellableItem, Error> {
        Ok(CancellableItem)
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(cancel_frame())
    }
}

/// Panics if `decode` runs: for tests whose frames must never reach the decoder.
#[derive(Debug)]
struct NeverDecodes;

impl StreamDecoder<NeverDecodes> for NeverDecodes {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<NeverDecodes, Error> {
        unreachable!("no Response frames should reach this decoder")
    }
}

/// Every frame fails to decode.
#[derive(Debug)]
struct DecodeError;

impl StreamDecoder<DecodeError> for DecodeError {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<DecodeError, Error> {
        Err(Error::Simple("decode error".into()))
    }
}

/// Every frame ends the stream.
#[derive(Debug)]
struct EndOfStreamItem;

impl StreamDecoder<EndOfStreamItem> for EndOfStreamItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<EndOfStreamItem, Error> {
        Err(Error::EndOfStream)
    }
}

/// Both a snapshot-end sentinel (`-1`, like `IntItem`) and a cancel message
/// (like `CancellableItem`), so a test can drive a subscription to snapshot-end
/// and then observe that no cancel goes out.
#[derive(Debug)]
struct CancellableSnapshotItem(i32);

impl StreamDecoder<CancellableSnapshotItem> for CancellableSnapshotItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<CancellableSnapshotItem, Error> {
        Ok(CancellableSnapshotItem(msg.peek_int(1)?))
    }

    fn is_snapshot_end(&self) -> bool {
        self.0 == -1
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(cancel_frame())
    }
}

/// Ends the stream on `-1` (the native end marker, like `ContractDataEnd`) and
/// has a cancel message, so a test can observe that no cancel follows the end.
#[derive(Debug)]
struct CancellableEndItem(i32);

impl StreamDecoder<CancellableEndItem> for CancellableEndItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<CancellableEndItem, Error> {
        match msg.peek_int(1)? {
            -1 => Err(Error::EndOfStream),
            value => Ok(CancellableEndItem(value)),
        }
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(cancel_frame())
    }
}

fn cancel_frame() -> Vec<u8> {
    encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[])
}

/// Field 0 is the message id (`TickPrice`, matching every decoder above);
/// the payload `IntItem` reads sits at field 1.
fn int_frame(value: i32) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("1\0{value}\0")))
}

/// What a test built by [`subscription_with`] gets back: the subscription, the
/// broadcast sender so it can feed frames, and the stub bus so it can assert on
/// what the subscription sent (cancel frames).
struct Fixture<T: StreamDecoder<T>> {
    subscription: Subscription<T>,
    tx: broadcast::Sender<RoutedItem>,
    bus: Arc<MessageBusStub>,
}

/// Build a `Subscription<T>` over a fresh broadcast channel and a fresh stub bus.
fn subscription_with<T: StreamDecoder<T>>(request_id: Option<i32>, order_id: Option<i32>, context: DecoderContext) -> Fixture<T> {
    let bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);
    Fixture {
        subscription: Subscription::new_from_internal(internal, bus.clone(), request_id, order_id, context),
        tx,
        bus,
    }
}

/// [`subscription_with`] with no request or order id and a default context —
/// what most tests here need.
fn subscription<T: StreamDecoder<T>>() -> (Subscription<T>, broadcast::Sender<RoutedItem>) {
    let f = subscription_with(None, None, DecoderContext::default());
    (f.subscription, f.tx)
}

/// `Drop` spawns the cancel on the bus's runtime, so a drop on a thread with
/// no runtime still sends it (#848).
#[tokio::test]
async fn test_drop_outside_runtime_sends_cancel() {
    let f = subscription_with::<CancellableItem>(Some(123), None, DecoderContext::default());
    let subscription = f.subscription;
    std::thread::spawn(move || drop(subscription))
        .join()
        .expect("drop outside a runtime panicked");

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while f.bus.request_messages().is_empty() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()]);
}

/// Once the bus's runtime has shut down there is nowhere for the cancel to
/// run; `Drop` must still not panic.
#[test]
fn test_drop_after_runtime_shutdown_does_not_panic() {
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let f = runtime.block_on(async { subscription_with::<CancellableItem>(Some(123), None, DecoderContext::default()) });
    drop(runtime);

    let subscription = f.subscription;
    std::thread::spawn(move || drop(subscription))
        .join()
        .expect("drop after runtime shutdown panicked");
    assert!(f.bus.request_messages().is_empty());
}

// ---- Stream contract --------------------------------------------------------

#[tokio::test]
async fn test_subscription_decodes_through_stream_decoder() {
    let (mut sub, tx) = subscription::<IntItem>();

    tx.send(int_frame(42)).unwrap();

    match sub.next().await {
        Some(Ok(SubscriptionItem::Data(item))) => assert_eq!(item, IntItem(42)),
        other => panic!("expected Data, got {other:?}"),
    }
}

#[tokio::test]
async fn test_subscription_lag_yields_gap_notice_then_items() {
    // Regression test for #779: falling behind the broadcast channel yields a
    // synthesized gap notice naming the dropped count, then the retained
    // items — not a silent resume.
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let (tx, rx) = broadcast::channel(2);
    for code in [2100, 2101, 2102, 2103] {
        tx.send(RoutedItem::Notice(test_notice(code, "n"))).unwrap();
    }

    // Capacity 2 with 4 sends: the 2 oldest frames were evicted.
    let internal = AsyncInternalSubscription::new(rx);
    let mut subscription: Subscription<NeverDecodes> =
        Subscription::new_from_internal(internal, message_bus, Some(9000), None, DecoderContext::default());

    match subscription.next().await {
        Some(Ok(SubscriptionItem::Notice(n))) => assert_eq!(n, crate::messages::subscription_lag_notice(2)),
        other => panic!("expected gap notice, got {other:?}"),
    }
    for expected in [2102, 2103] {
        match subscription.next().await {
            Some(Ok(SubscriptionItem::Notice(n))) => assert_eq!(n.code, expected, "retained notice after the gap"),
            other => panic!("expected retained notice {expected}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn test_routed_item_error_surfaces_through_async_subscription() {
    // The channel emits a terminal RoutedItem::Error that the consumer must
    // surface directly without ever invoking the decoder.
    let (mut subscription, tx) = subscription::<NeverDecodes>();

    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let result = subscription.next().await;
    assert!(matches!(result, Some(Err(Error::ConnectionReset))));
}

#[tokio::test]
async fn test_routed_item_notice_skipped_then_response_delivered() {
    let (subscription, tx) = subscription::<IntItem>();

    // The receiver-side contract: notices are consumed by `filter_data` and
    // the next data item is delivered.
    tx.send(RoutedItem::Notice(test_notice(2104, "Market data farm OK"))).unwrap();
    tx.send(int_frame(7)).unwrap();

    // First item via the raw Stream is the Notice (passes through);
    // filter_data() drops it and yields the Data.
    let mut data = subscription.filter_data();
    assert!(matches!(data.next().await, Some(Ok(IntItem(7)))));
}

#[tokio::test]
async fn test_subscription_next_with_error() {
    let (mut subscription, tx) = subscription::<DecodeError>();

    // Send a message that will trigger the error
    tx.send(int_frame(1)).unwrap();

    let result = subscription.next().await;
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
}

#[tokio::test]
async fn test_subscription_next_end_of_stream() {
    let (mut subscription, tx) = subscription::<EndOfStreamItem>();

    // Send a message that will trigger end of stream
    tx.send(int_frame(1)).unwrap();

    let result = subscription.next().await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_subscription_no_retries_after_end_of_stream() {
    use std::sync::atomic::AtomicUsize;

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// First frame ends the stream; any later call is a stray that must not happen.
    #[derive(Debug)]
    struct EndThenStray;

    impl StreamDecoder<EndThenStray> for EndThenStray {
        const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

        fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<EndThenStray, Error> {
            let n = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            if n == 0 {
                Err(Error::EndOfStream)
            } else {
                Err(Error::unexpected_response(&ResponseMessage::from("stray\0")))
            }
        }
    }

    let (mut subscription, tx) = subscription::<EndThenStray>();

    // First message triggers EndOfStream
    tx.send(int_frame(1)).unwrap();
    let result = subscription.next().await;
    assert!(result.is_none());

    // Send stray messages after stream ended
    tx.send(int_frame(2)).unwrap();
    tx.send(int_frame(3)).unwrap();

    // Subsequent calls should return None immediately without invoking decoder
    let result = subscription.next().await;
    assert!(result.is_none());

    // Decoder should have been called only once (for the EndOfStream message)
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_subscription_skips_undeclared_messages_without_retry_limit() {
    use std::sync::atomic::AtomicUsize;

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// Declares only `TickPrice`; `TickSize` frames must never reach `decode`.
    #[derive(Debug)]
    struct DeclaresTickPrice;

    impl StreamDecoder<DeclaresTickPrice> for DeclaresTickPrice {
        const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

        fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<DeclaresTickPrice, Error> {
            CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(DeclaresTickPrice)
        }
    }

    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<DeclaresTickPrice> =
        Subscription::new_from_internal(internal, message_bus, Some(1), None, DecoderContext::default());

    // Many undeclared frames, then one the decoder declares.
    for _ in 0..20 {
        tx.send(ResponseMessage::from("2\0stray\0").into()).unwrap();
    }
    tx.send(ResponseMessage::from("1\0msg\0").into()).unwrap();

    assert!(
        matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(_)))),
        "subscription should not have stopped while skipping undeclared messages"
    );
    assert_eq!(
        CALL_COUNT.load(Ordering::Relaxed),
        1,
        "the 20 undeclared frames must be filtered before decode, not skipped inside it"
    );
}

#[tokio::test]
async fn test_stream_yields_error_then_ends() {
    // A terminal error flips `stream_ended`, so later polls return `None`
    // instead of re-polling the channel for the queued item behind it.
    let (mut subscription, tx) = subscription::<IntItem>();

    tx.send(int_frame(1)).unwrap();
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();
    tx.send(int_frame(2)).unwrap();

    let first = subscription.next().await;
    assert!(matches!(first, Some(Ok(SubscriptionItem::Data(IntItem(1))))));

    let second = subscription.next().await;
    assert!(matches!(second, Some(Err(Error::ConnectionReset))));

    let third = subscription.next().await;
    assert!(third.is_none(), "stream must end after a terminal error");
}

#[tokio::test]
async fn test_routed_end_of_stream_ends_without_error() {
    // `EndOfStream` arriving as a routed error — not from the decoder — is a
    // graceful end: `None`, not `Some(Err(..))`. The historical-tick streams
    // read it this way (`market_data::historical::common::tick`), so the arm
    // is reachable in production.
    let (mut subscription, tx) = subscription::<IntItem>();

    tx.send(int_frame(1)).unwrap();
    tx.send(RoutedItem::Error(Error::EndOfStream)).unwrap();
    tx.send(int_frame(2)).unwrap();

    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(IntItem(1))))));
    assert!(subscription.next().await.is_none(), "routed EndOfStream ends the stream");
    // `stream_ended` latched, so the frame queued behind it is never yielded.
    assert!(subscription.next().await.is_none(), "stream stays ended");
}

/// Exercises `impl Stream for Subscription<T>` end-to-end via `StreamExt`:
/// one-shot `next().await` and combinator chaining on `&mut subscription`.
#[tokio::test]
async fn subscription_impls_stream() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel::<RoutedItem>(16);
    let internal = AsyncInternalSubscription::new(rx);

    let mut subscription: Subscription<IntItem> = Subscription::new_from_internal(internal, message_bus, Some(1), None, DecoderContext::default());

    tx.send(int_frame(10)).unwrap();
    tx.send(int_frame(20)).unwrap();
    tx.send(int_frame(30)).unwrap();

    // One-shot read via StreamExt::next.
    let first = subscription.next().await;
    assert!(matches!(first, Some(Ok(SubscriptionItem::Data(IntItem(10))))));

    // Combinator chain on &mut subscription (subscription stays usable after).
    let next_two: Vec<_> = (&mut subscription).take(2).collect().await;
    assert_eq!(next_two.len(), 2);
    assert!(matches!(next_two[0], Ok(SubscriptionItem::Data(IntItem(20)))));
    assert!(matches!(next_two[1], Ok(SubscriptionItem::Data(IntItem(30)))));
}

// ---- Cancel, clone, drop ----------------------------------------------------

#[tokio::test]
async fn test_subscription_cancel() {
    let f = subscription_with::<CancellableItem>(Some(123), None, DecoderContext::default());

    // Cancel the subscription: the decoder's cancel message goes to the bus.
    f.subscription.cancel().await;
    assert!(f.subscription.cancelled.load(Ordering::Relaxed));
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()]);

    // Cancel again is a no-op.
    f.subscription.cancel().await;
    assert_eq!(f.bus.request_messages().len(), 1);
}

#[tokio::test]
async fn test_subscription_cancel_without_cancel_message_sends_nothing() {
    // `StreamDecoder::cancel_message` defaults to `Err(NotImplemented)`.
    let f = subscription_with::<IntItem>(Some(123), None, DecoderContext::default());

    f.subscription.cancel().await;
    assert!(f.subscription.cancelled.load(Ordering::Relaxed));
    assert!(f.bus.request_messages().is_empty());
}

/// The next item, failing rather than hanging when a regression leaves the
/// stream waiting on the channel after `cancel()`.
async fn next_after_cancel<T: StreamDecoder<T> + Send + 'static>(sub: &mut Subscription<T>) -> Option<Result<SubscriptionItem<T>, Error>> {
    tokio::time::timeout(Duration::from_secs(1), sub.next())
        .await
        .expect("stream hung after cancel")
}

/// `cancel()` ends the stream locally whatever is written: each clone yields
/// `Err(Cancelled)` once, skipping frames not yet read, then ends. Covers the
/// paths that write nothing: no cancel message, and an order id.
#[tokio::test]
async fn test_cancel_stops_every_clone() {
    for (request_id, order_id) in [(Some(123), None), (None, Some(7))] {
        let Fixture { mut subscription, tx, bus } = subscription_with::<IntItem>(request_id, order_id, DecoderContext::default());
        let mut clone = subscription.clone();
        tx.send(int_frame(1)).unwrap();

        subscription.cancel().await;
        assert!(bus.request_messages().is_empty(), "IntItem has no cancel message");
        for sub in [&mut subscription, &mut clone] {
            assert!(
                matches!(next_after_cancel(sub).await, Some(Err(Error::Cancelled))),
                "{request_id:?}/{order_id:?}"
            );
            assert!(next_after_cancel(sub).await.is_none(), "stream must end after Cancelled");
        }
    }
}

/// A snapshot run to its sentinel, ready for `cancel()` or `Drop`. Its
/// decoder defines a cancel message, so a skipped write is the snapshot check.
async fn completed_snapshot() -> Fixture<CancellableSnapshotItem> {
    let mut f = subscription_with::<CancellableSnapshotItem>(Some(123), None, DecoderContext::default());

    f.tx.send(int_frame(-1)).unwrap();
    assert!(matches!(
        f.subscription.next().await,
        Some(Ok(SubscriptionItem::Data(CancellableSnapshotItem(-1))))
    ));
    assert!(
        f.subscription.snapshot_ended.load(Ordering::Relaxed),
        "sentinel must latch snapshot_ended"
    );
    f
}

#[tokio::test]
async fn test_completed_snapshot_cancel_writes_nothing_and_stops() {
    let mut f = completed_snapshot().await;

    f.subscription.cancel().await;
    assert!(f.bus.request_messages().is_empty(), "completed snapshot must not send a cancel");
    assert!(matches!(next_after_cancel(&mut f.subscription).await, Some(Err(Error::Cancelled))));
    assert!(next_after_cancel(&mut f.subscription).await.is_none());
}

#[tokio::test]
async fn test_completed_snapshot_skips_cancel_on_drop() {
    let f = completed_snapshot().await;

    drop(f.subscription);
    settle().await;
    assert!(
        f.bus.request_messages().is_empty(),
        "dropping a completed snapshot must not send a cancel"
    );
}

#[tokio::test]
async fn test_subscription_clone() {
    let Fixture { mut subscription, tx, .. } = subscription_with::<IntItem>(
        Some(456),
        Some(789),
        DecoderContext::default()
            .with_smart_depth(true)
            .with_request_type(OutgoingMessages::RequestPositions),
    );

    let mut cloned = subscription.clone();
    assert_eq!(cloned.request_id, Some(456));
    assert_eq!(cloned.order_id, Some(789));
    assert!(cloned.context.is_smart_depth);

    // Both clones receive frames sent after the clone; `stream_ended` is per clone.
    tx.send(int_frame(5)).unwrap();
    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(IntItem(5))))));
    assert!(matches!(cloned.next().await, Some(Ok(SubscriptionItem::Data(IntItem(5))))));
}

#[tokio::test]
async fn test_subscription_drop_with_cancel() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (_tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let _subscription: Subscription<CancellableItem> =
            Subscription::new_from_internal(internal, message_bus.clone(), Some(999), None, DecoderContext::default());
        // Dropped here: the cancel send is spawned onto the runtime.
    }

    // Give the spawned task time to execute.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(message_bus.request_messages(), vec![cancel_frame()]);
}

#[tokio::test]
async fn test_subscription_with_context() {
    let context = DecoderContext::default()
        .with_smart_depth(true)
        .with_request_type(OutgoingMessages::RequestMarketDepth);

    let f = subscription_with::<IntItem>(None, None, context.clone());

    assert_eq!(f.subscription.context, context);
}

#[tokio::test]
async fn test_subscription_new_from_internal_simple() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (_tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<CancellableItem> =
        Subscription::new_from_internal_simple(internal, message_bus.clone(), DecoderContext::default());

    assert_eq!(subscription.request_id(), None);
    assert_eq!(subscription.order_id, None);

    // No id, but the decoder's cancel message is still sent.
    subscription.cancel().await;
    assert_eq!(message_bus.request_messages(), vec![cancel_frame()]);
}

// ---- filter_data ------------------------------------------------------------

#[tokio::test]
async fn test_data_stream_collects_data_items() {
    let (subscription, tx) = subscription::<IntItem>();

    tx.send(int_frame(1)).unwrap();
    tx.send(int_frame(2)).unwrap();
    drop(tx); // close the channel so the stream terminates.

    let collected: Vec<_> = subscription.filter_data().collect().await;
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].as_ref().unwrap(), &IntItem(1));
    assert_eq!(collected[1].as_ref().unwrap(), &IntItem(2));
}

#[tokio::test]
async fn test_data_stream_yields_error_then_ends() {
    let (subscription, tx) = subscription::<IntItem>();

    tx.send(int_frame(1)).unwrap();
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();
    tx.send(int_frame(2)).unwrap();

    let mut stream = subscription.filter_data();

    let first = stream.next().await;
    assert_eq!(first.unwrap().unwrap(), IntItem(1));

    let second = stream.next().await;
    assert!(matches!(second, Some(Err(Error::ConnectionReset))));

    let third = stream.next().await;
    assert!(third.is_none(), "stream must end after a terminal error");
}

#[tokio::test]
async fn test_data_stream_filters_notices() {
    let (subscription, tx) = subscription::<IntItem>();

    tx.send(RoutedItem::Notice(test_notice(2104, "Market data farm OK"))).unwrap();
    tx.send(int_frame(3)).unwrap();
    drop(tx);

    let collected: Vec<_> = subscription.filter_data().collect().await;
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].as_ref().unwrap(), &IntItem(3));
}

/// The dispatcher emits `RoutedItem::Notice`; `Subscription<T>::next()`
/// surfaces it as `SubscriptionItem::Notice` without terminating the stream.
#[tokio::test]
async fn test_routed_item_notice_surfaces_as_subscription_item() {
    let (mut subscription, tx) = subscription::<IntItem>();

    tx.send(RoutedItem::Notice(test_notice(2104, "Market data farm OK"))).unwrap();
    tx.send(int_frame(3)).unwrap();

    match subscription.next().await {
        Some(Ok(SubscriptionItem::Notice(n))) => {
            assert_eq!(n.code, 2104);
            assert_eq!(n.message, "Market data farm OK");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
    assert!(matches!(subscription.next().await, Some(Ok(SubscriptionItem::Data(_)))));
}

/// `SubscriptionItemStreamExt::filter_data` drops `Notice` items (logged) and
/// yields the underlying `Result<T, Error>`. Errors must still propagate.
#[tokio::test]
async fn filter_data_stream_drops_notices() {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel::<RoutedItem>(16);
    let internal = AsyncInternalSubscription::new(rx);

    let subscription: Subscription<IntItem> = Subscription::new_from_internal(internal, message_bus, Some(7), None, DecoderContext::default());

    tx.send(int_frame(11)).unwrap();
    tx.send(RoutedItem::Notice(test_notice(2104, "data farm OK"))).unwrap();
    tx.send(int_frame(13)).unwrap();
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let mut data = subscription.filter_data();
    assert!(matches!(data.next().await, Some(Ok(IntItem(11)))));
    // Notice is filtered (logged at warn!) — the next yielded item is the 13 payload.
    assert!(matches!(data.next().await, Some(Ok(IntItem(13)))));
    // Errors still propagate through filter_data.
    assert!(matches!(data.next().await, Some(Err(Error::ConnectionReset))));
    // After a terminal error, the stream is exhausted.
    assert!(data.next().await.is_none());
}

// --- collect_for / collect_until ----------------------------------------

/// Build a `Subscription<IntItem>` pre-loaded with `items`. When `keep_open`
/// the broadcast sender is returned so the channel stays open (lets the timeout
/// branch fire); otherwise it is dropped so the stream ends after draining.
fn collect_subscription(items: Vec<RoutedItem>, keep_open: bool) -> (Subscription<IntItem>, Option<broadcast::Sender<RoutedItem>>) {
    let message_bus = Arc::new(MessageBusStub::default());
    let (tx, rx) = broadcast::channel(100);
    let internal = AsyncInternalSubscription::new(rx);
    for item in items {
        tx.send(item).unwrap();
    }
    let sub = Subscription::<IntItem>::new_from_internal(internal, message_bus, Some(1), None, DecoderContext::default());
    let keep = if keep_open { Some(tx) } else { None };
    (sub, keep)
}

#[tokio::test]
async fn test_collect_for_stops_at_snapshot_end() {
    let (mut sub, _keep) = collect_subscription(vec![int_frame(10), int_frame(20), int_frame(-1), int_frame(30)], true);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![IntItem(10), IntItem(20)]);
    assert!(sub.snapshot_ended.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_collect_until_stops_on_predicate() {
    let (mut sub, _keep) = collect_subscription(vec![int_frame(10), int_frame(20), int_frame(30), int_frame(40)], true);

    let collected = sub.collect_until(Duration::from_secs(30), |items| items.len() >= 2).await;

    assert_eq!(collected, vec![IntItem(10), IntItem(20)]);
}

#[tokio::test]
async fn test_collect_for_returns_prefix_on_terminal_error() {
    let (mut sub, _keep) = collect_subscription(vec![int_frame(10), RoutedItem::Error(Error::ConnectionReset), int_frame(20)], true);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![IntItem(10)]);
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
    let (mut sub, _keep) = collect_subscription(vec![int_frame(10), int_frame(20), int_frame(30)], false);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![IntItem(10), IntItem(20), IntItem(30)]);
}

#[tokio::test]
async fn test_collect_for_filters_notices() {
    let notice = RoutedItem::Notice(test_notice(2104, "Market data farm OK"));
    let (mut sub, _keep) = collect_subscription(vec![int_frame(10), notice, int_frame(20)], false);

    let collected = sub.collect_for(Duration::from_secs(30)).await;

    assert_eq!(collected, vec![IntItem(10), IntItem(20)]);
}

// ---- Cancel after the native end marker -------------------------------------

/// Drop's cancel is spawned; give it time to land before asserting it didn't.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(10)).await;
}

#[tokio::test]
async fn test_decoded_end_skips_cancel_on_cancel_and_drop() {
    let mut f = subscription_with::<CancellableEndItem>(Some(123), None, DecoderContext::default());

    f.tx.send(int_frame(1)).unwrap();
    f.tx.send(int_frame(-1)).unwrap();
    assert!(matches!(
        f.subscription.next().await,
        Some(Ok(SubscriptionItem::Data(CancellableEndItem(1))))
    ));
    assert!(f.subscription.next().await.is_none());
    assert!(f.subscription.ended_natively());

    f.subscription.cancel().await;
    drop(f.subscription);
    settle().await;
    assert!(f.bus.request_messages().is_empty(), "no cancel after the end marker");
}

#[tokio::test]
async fn test_routed_end_skips_cancel_on_drop() {
    let mut f = subscription_with::<CancellableItem>(Some(123), None, DecoderContext::default());

    f.tx.send(RoutedItem::Error(Error::EndOfStream)).unwrap();
    assert!(f.subscription.next().await.is_none());
    assert!(f.subscription.ended_natively());

    drop(f.subscription);
    settle().await;
    assert!(f.bus.request_messages().is_empty(), "no cancel after a routed end");
}

#[tokio::test]
async fn test_error_end_still_cancels() {
    // Only the end marker proves TWS finished; after an error the cancel goes out as before.
    let mut f = subscription_with::<CancellableEndItem>(Some(123), None, DecoderContext::default());

    f.tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();
    assert!(matches!(f.subscription.next().await, Some(Err(Error::ConnectionReset))));
    assert!(!f.subscription.ended_natively());

    f.subscription.cancel().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()]);
}

#[tokio::test]
async fn test_end_seen_by_one_clone_skips_cancel_from_another() {
    // The flag is shared: once any clone has seen the end, TWS is done with the request.
    let mut f = subscription_with::<CancellableEndItem>(Some(123), None, DecoderContext::default());
    let other = f.subscription.clone();

    f.tx.send(int_frame(-1)).unwrap();
    assert!(f.subscription.next().await.is_none());

    drop(other);
    settle().await;
    assert!(f.bus.request_messages().is_empty(), "unread clone must not cancel a finished request");
}

// ---- collect_to_end ---------------------------------------------------------

#[tokio::test]
async fn test_collect_to_end_returns_items_and_skips_notices() {
    let (mut sub, tx) = subscription::<IntItem>();
    tx.send(int_frame(10)).unwrap();
    tx.send(RoutedItem::Notice(test_notice(2104, "Market data farm OK"))).unwrap();
    tx.send(int_frame(20)).unwrap();
    tx.send(RoutedItem::Error(Error::EndOfStream)).unwrap();

    assert_eq!(sub.collect_to_end().await.unwrap(), vec![IntItem(10), IntItem(20)]);
}

#[tokio::test]
async fn test_collect_to_end_returns_terminal_error() {
    let (mut sub, tx) = subscription::<IntItem>();
    tx.send(int_frame(10)).unwrap();
    tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    assert!(matches!(sub.collect_to_end().await, Err(Error::ConnectionReset)));
}

#[tokio::test]
async fn test_collect_to_end_without_end_marker_is_unexpected_end() {
    // Sender dropped after the last item: the channel closes with no end marker.
    let (mut sub, tx) = subscription::<IntItem>();
    tx.send(int_frame(10)).unwrap();
    drop(tx);

    assert!(matches!(sub.collect_to_end().await, Err(Error::UnexpectedEndOfStream)));
}

// ---- cancel_and_drain -------------------------------------------------------

fn drain_deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + Duration::from_secs(2)
}

fn drain_fixture() -> Fixture<CancellableEndItem> {
    subscription_with::<CancellableEndItem>(Some(123), None, DecoderContext::default())
}

#[tokio::test]
async fn test_drain_after_end_writes_nothing() {
    let mut f = drain_fixture();
    f.tx.send(int_frame(1)).unwrap();
    f.tx.send(int_frame(-1)).unwrap();
    while f.subscription.next().await.is_some() {}

    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Ended);
    settle().await;
    assert!(f.bus.request_messages().is_empty());
}

#[tokio::test]
async fn test_drain_cancels_then_sees_end() {
    let f = drain_fixture();
    f.tx.send(int_frame(1)).unwrap();
    f.tx.send(int_frame(-1)).unwrap();

    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Ended);
    settle().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()], "one cancel, not repeated on drop");
}

#[tokio::test]
async fn test_drain_reports_tws_error() {
    let f = drain_fixture();
    f.tx.send(RoutedItem::Error(Error::Notice(crate::messages::Notice::synthesized(
        200,
        "No security definition".to_string(),
    ))))
    .unwrap();

    assert_eq!(
        f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(),
        Drained::Rejected(crate::messages::Notice::synthesized(200, "No security definition".to_string()))
    );
}

#[tokio::test]
async fn test_drain_deadline_is_unconfirmed() {
    let f = drain_fixture();

    let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
    assert_eq!(f.subscription.cancel_and_drain(deadline).await.unwrap(), Drained::Unconfirmed);
    settle().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()], "one cancel, not repeated on drop");
}

#[tokio::test]
async fn test_drain_after_error_is_unconfirmed_and_cancels_on_drop() {
    let mut f = drain_fixture();
    f.tx.send(RoutedItem::Error(Error::BufferLimitExceeded { limit: 1 })).unwrap();
    assert!(matches!(f.subscription.next().await, Some(Err(Error::BufferLimitExceeded { .. }))));

    let started = std::time::Instant::now();
    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Unconfirmed);
    assert!(started.elapsed() < Duration::from_secs(1), "no wait for evidence that can't arrive");
    settle().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()], "the drop writes the cancel");
}

#[tokio::test]
async fn test_drain_session_error_is_err() {
    let f = drain_fixture();
    f.tx.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    assert!(matches!(
        f.subscription.cancel_and_drain(drain_deadline()).await,
        Err(Error::ConnectionReset)
    ));
}

#[tokio::test]
async fn test_drain_closed_channel_is_unconfirmed() {
    let f = drain_fixture();
    drop(f.tx);

    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Unconfirmed);
}

#[tokio::test]
async fn test_drain_future_dropped_mid_wait_cancels_once() {
    let f = drain_fixture();

    let far = tokio::time::Instant::now() + Duration::from_secs(60);
    assert!(tokio::time::timeout(Duration::from_millis(20), f.subscription.cancel_and_drain(far))
        .await
        .is_err());
    settle().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()]);
}

#[tokio::test]
async fn test_drain_on_one_clone_leaves_others_reading() {
    let f = drain_fixture();
    let mut other = f.subscription.clone();
    f.tx.send(int_frame(1)).unwrap();
    f.tx.send(int_frame(-1)).unwrap();

    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Ended);
    assert!(matches!(other.next().await, Some(Ok(SubscriptionItem::Data(CancellableEndItem(1))))));
    assert!(other.next().await.is_none(), "the other clone still sees the end");

    drop(other);
    settle().await;
    assert_eq!(f.bus.request_messages(), vec![cancel_frame()], "one cancel across clones");
}

#[tokio::test]
async fn test_drain_after_snapshot_end_writes_nothing() {
    let mut f = subscription_with::<CancellableSnapshotItem>(Some(123), None, DecoderContext::default());
    f.tx.send(int_frame(-1)).unwrap();
    assert!(matches!(
        f.subscription.next().await,
        Some(Ok(SubscriptionItem::Data(CancellableSnapshotItem(-1))))
    ));

    let started = std::time::Instant::now();
    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Ended);
    assert!(started.elapsed() < Duration::from_secs(1), "no wait after a finished snapshot");
    settle().await;
    assert!(f.bus.request_messages().is_empty());
}

#[tokio::test]
async fn test_drain_without_request_id_returns_unconfirmed() {
    let f = subscription_with::<CancellableEndItem>(None, None, DecoderContext::default());

    assert_eq!(f.subscription.cancel_and_drain(drain_deadline()).await.unwrap(), Drained::Unconfirmed);
}
