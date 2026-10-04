use super::*;
use crate::client::ids::{OrderId, RequestId};
use crate::messages::{encode_protobuf_message, IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::stubs::MessageBusStub;
use crate::subscriptions::{Drained, SubscriptionItem};
use crate::transport::common::Lease;
use std::sync::Arc;

#[derive(Debug)]
struct EndOfStreamItem;

impl StreamDecoder<EndOfStreamItem> for EndOfStreamItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<EndOfStreamItem, Error> {
        Err(Error::EndOfStream)
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[]))
    }
}

#[test]
fn test_subscription_skips_undeclared_messages_without_limit() {
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

    // Many undeclared messages, then one the decoder declares.
    let mut responses: Vec<String> = (0..20).map(|_| "2|stray".to_string()).collect();
    responses.push("1|msg".to_string());
    // Sentinel to avoid blocking on the channel after success
    responses.push("1|done".to_string());

    let stub = MessageBusStub::with_responses(responses);
    let message_bus = Arc::new(stub);

    let sub: Subscription<DeclaresTickPrice> = {
        let internal = message_bus.send_request(RequestId::nth(1), &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    assert!(sub.next().is_some(), "subscription should survive 20 skips and return valid message");
    assert_eq!(
        CALL_COUNT.load(Ordering::Relaxed),
        1,
        "the 20 undeclared frames must be filtered before decode, not skipped inside it"
    );
}

#[test]
fn test_routed_item_error_terminates_subscription() {
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    #[derive(Debug)]
    struct DataItem;

    impl StreamDecoder<DataItem> for DataItem {
        const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

        fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<DataItem, Error> {
            Ok(DataItem)
        }
    }

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _) = channel::unbounded();
    sender.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let internal = SubscriptionBuilder::new()
        .receiver(receiver)
        .signaler(signaler)
        .lease(Lease::new())
        .request_id(RequestId::nth(1))
        .build();

    let stub = Arc::new(MessageBusStub::default());
    let sub: Subscription<DataItem> = Subscription::new(stub, internal, DecoderContext::default());

    // First call surfaces the terminal error via the Err arm.
    assert!(matches!(sub.next(), Some(Err(Error::ConnectionReset))));
    // Subsequent calls return None — the stream is terminated.
    assert!(sub.next().is_none());
}

#[test]
fn test_routed_item_notice_surfaces_as_subscription_item() {
    use crate::messages::Notice;
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    #[derive(Debug)]
    struct DataItem;

    impl StreamDecoder<DataItem> for DataItem {
        const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

        fn decode(_context: &DecoderContext, _msg: &ResponseMessage) -> Result<DataItem, Error> {
            Ok(DataItem)
        }
    }

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _) = channel::unbounded();

    sender
        .send(RoutedItem::Notice(Notice {
            request_id: None,
            code: 2104,
            message: "Market data farm OK".into(),
            error_time: None,
            advanced_order_reject_json: String::new(),
        }))
        .unwrap();
    sender.send(RoutedItem::Response(ResponseMessage::from("1\0data\0"))).unwrap();

    let internal = SubscriptionBuilder::new()
        .receiver(receiver)
        .signaler(signaler)
        .lease(Lease::new())
        .request_id(RequestId::nth(1))
        .build();
    let stub = Arc::new(MessageBusStub::default());
    let sub: Subscription<DataItem> = Subscription::new(stub, internal, DecoderContext::default());

    // The notice surfaces as a non-terminal SubscriptionItem::Notice.
    match sub.next() {
        Some(Ok(SubscriptionItem::Notice(n))) => {
            assert_eq!(n.code, 2104);
            assert_eq!(n.message, "Market data farm OK");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
    // Stream stays open: the next data item arrives normally.
    assert!(matches!(sub.next(), Some(Ok(SubscriptionItem::Data(_)))));
}

#[test]
fn test_no_retries_after_end_of_stream() {
    let stub = MessageBusStub::with_responses(vec![
        "1|data".to_string(),  // triggers EndOfStream via decoder
        "1|stray".to_string(), // stray message after stream ended
    ]);
    let message_bus = Arc::new(stub);

    let sub: Subscription<EndOfStreamItem> = {
        let internal = message_bus.send_request(RequestId::nth(1), &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    // First call hits EndOfStream, returns None
    assert!(sub.next().is_none());

    // Second call should return None immediately (stream_ended guard)
    assert!(sub.next().is_none());
    assert!(sub.stream_ended.load(Ordering::Relaxed));
}

// --- collect_for / collect_until ----------------------------------------

use crate::subscriptions::common::RoutedItem;
use crate::transport::{Signal, SubscriptionBuilder};
use crossbeam::channel;
use std::time::Duration;

/// Test decoder for the collect tests: the value `-1` marks a snapshot-end
/// sentinel (mirrors `TickTypes::SnapshotEnd`).
#[derive(Debug, PartialEq)]
struct CollectItem(i32);

impl StreamDecoder<CollectItem> for CollectItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<CollectItem, Error> {
        Ok(CollectItem(msg.peek_int(1)?))
    }

    fn is_snapshot_end(&self) -> bool {
        self.0 == -1
    }
}

/// Build a `Subscription<CollectItem>` pre-loaded with `items`. When `keep_open`
/// the channel sender is returned so the channel stays open (lets the timeout
/// branch fire); otherwise it is dropped so the stream ends after draining.
fn collect_subscription(items: Vec<RoutedItem>, keep_open: bool) -> (Subscription<CollectItem>, Option<channel::Sender<RoutedItem>>) {
    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _signaler_rx) = channel::unbounded();
    for item in items {
        sender.send(item).unwrap();
    }
    let internal = SubscriptionBuilder::new()
        .receiver(receiver)
        .signaler(signaler)
        .lease(Lease::new())
        .request_id(RequestId::nth(1))
        .build();
    let stub = Arc::new(MessageBusStub::default());
    let sub = Subscription::new(stub, internal, DecoderContext::default());
    let keep = if keep_open { Some(sender) } else { None };
    (sub, keep)
}

/// Field 0 is the message id (`TickPrice`, matching `CollectItem`'s
/// declaration); the payload the decoder reads sits at field 1.
fn data(value: i32) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("1\0{value}\0")))
}

#[test]
fn test_collect_for_stops_at_snapshot_end() {
    // 10, 20, snapshot-end, 30 — collection stops at the sentinel (excluded);
    // 30 is never consumed.
    let (sub, _keep) = collect_subscription(vec![data(10), data(20), data(-1), data(30)], true);

    let collected = sub.collect_for(Duration::from_secs(30));

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}

#[test]
fn test_collect_until_stops_on_predicate() {
    // No sentinel; the predicate halts collection once two items arrive.
    let (sub, _keep) = collect_subscription(vec![data(10), data(20), data(30), data(40)], true);

    let collected = sub.collect_until(Duration::from_secs(30), |items| items.len() >= 2);

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}

#[test]
fn test_collect_for_returns_prefix_on_terminal_error() {
    // One datum, then a terminal error — the prefix collected so far is returned.
    let (sub, _keep) = collect_subscription(vec![data(10), RoutedItem::Error(Error::ConnectionReset), data(20)], true);

    let collected = sub.collect_for(Duration::from_secs(30));

    assert_eq!(collected, vec![CollectItem(10)]);
}

#[test]
fn test_collect_for_returns_empty_on_timeout() {
    // Channel stays open with no data; the total timeout bounds the wait.
    let (sub, _keep) = collect_subscription(vec![], true);

    let collected = sub.collect_for(Duration::from_millis(50));

    assert!(collected.is_empty());
}

#[test]
fn test_collect_for_zero_timeout_returns_immediately() {
    // A zero deadline trips the top-of-loop guard before any item is read,
    // even though data is queued.
    let (sub, _keep) = collect_subscription(vec![data(10), data(20)], true);

    let collected = sub.collect_for(Duration::ZERO);

    assert!(collected.is_empty());
}

#[test]
fn test_collect_for_drains_to_stream_end() {
    // No sentinel and the sender is dropped, so collection ends at stream end.
    let (sub, _keep) = collect_subscription(vec![data(10), data(20), data(30)], false);

    let collected = sub.collect_for(Duration::from_secs(30));

    assert_eq!(collected, vec![CollectItem(10), CollectItem(20), CollectItem(30)]);
}

#[test]
fn test_collect_for_filters_notices() {
    use crate::messages::Notice;

    let notice = RoutedItem::Notice(Notice {
        request_id: None,
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    });
    let (sub, _keep) = collect_subscription(vec![data(10), notice, data(20)], false);

    let collected = sub.collect_for(Duration::from_secs(30));

    // Notice is dropped (logged); only data is collected.
    assert_eq!(collected, vec![CollectItem(10), CollectItem(20)]);
}

/// The complement of `test_subscription_skips_undeclared_messages_without_limit`.
/// Declaring a type the `decode` match does not handle is a bug, and it must be
/// loud: the `_` arm's `UnexpectedResponse` is no longer skippable, so it
/// terminates instead of silently yielding nothing.
#[test]
fn test_declared_type_with_no_decode_arm_terminates() {
    #[derive(Debug)]
    struct DeclaresMoreThanItHandles;

    impl StreamDecoder<DeclaresMoreThanItHandles> for DeclaresMoreThanItHandles {
        const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice, IncomingMessages::TickSize];

        fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<DeclaresMoreThanItHandles, Error> {
            match msg.message_type() {
                IncomingMessages::TickPrice => Ok(DeclaresMoreThanItHandles),
                _ => Err(Error::unexpected_response(msg)),
            }
        }
    }

    let message_bus = Arc::new(MessageBusStub::with_responses(vec!["2|declared but unhandled".to_string()]));
    let sub: Subscription<DeclaresMoreThanItHandles> = {
        let internal = message_bus.send_request(RequestId::nth(1), &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    assert!(
        matches!(sub.next(), Some(Err(Error::UnexpectedResponse(_)))),
        "a declared-but-unhandled type must surface, not vanish"
    );
}

// --- cancel after the native end marker ----------------------------------

/// A subscription routed as `builder` says (request or order id), fed `items`,
/// plus the stub bus (to see whether a cancel was written) and the cleanup
/// signals (to see whether it unregistered). It holds its sender, as
/// production's does, so cancelling can send the signal.
fn routed_subscription<T: StreamDecoder<T>>(
    builder: SubscriptionBuilder,
    items: Vec<RoutedItem>,
) -> (Subscription<T>, Arc<MessageBusStub>, channel::Receiver<Signal>) {
    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, signals) = channel::unbounded();
    for item in items {
        sender.send(item).unwrap();
    }
    let internal = builder.receiver(receiver).sender(sender).signaler(signaler).lease(Lease::new()).build();
    let stub = Arc::new(MessageBusStub::default());
    (Subscription::new(stub.clone(), internal, DecoderContext::default()), stub, signals)
}

fn request_subscription<T: StreamDecoder<T>>(items: Vec<RoutedItem>) -> (Subscription<T>, Arc<MessageBusStub>, channel::Receiver<Signal>) {
    routed_subscription(SubscriptionBuilder::new().request_id(RequestId::nth(1)), items)
}

/// Whether `cancel()` sent the cleanup signal for request id 1.
fn unregistered_request(signals: &channel::Receiver<Signal>) -> bool {
    matches!(signals.try_recv(), Ok(Signal::Request(id, _)) if id == RequestId::nth(1))
}

#[test]
fn test_decoded_end_skips_cancel_on_drop() {
    let (sub, bus, _signals) = request_subscription::<EndOfStreamItem>(vec![data(1)]);

    assert!(sub.next().is_none());
    assert!(sub.ended_natively());

    sub.cancel();
    drop(sub);
    assert!(bus.request_messages().is_empty(), "no cancel after the end marker");
}

#[test]
fn test_routed_end_skips_cancel_on_drop() {
    let (sub, bus, _signals) = request_subscription::<EndOfStreamItem>(vec![RoutedItem::Error(Error::EndOfStream)]);

    assert!(sub.next().is_none());
    assert!(sub.ended_natively());

    drop(sub);
    assert!(bus.request_messages().is_empty(), "no cancel after a routed end");
}

#[test]
fn test_error_end_still_cancels() {
    // Only the end marker proves TWS finished; after an error the cancel goes out as before.
    let (sub, bus, signals) = request_subscription::<EndOfStreamItem>(vec![RoutedItem::Error(Error::ConnectionReset)]);

    assert!(matches!(sub.next(), Some(Err(Error::ConnectionReset))));
    assert!(!sub.ended_natively());

    drop(sub);
    assert_eq!(bus.request_messages().len(), 1, "cancel written after an error end");
    assert!(unregistered_request(&signals), "route kept after cancel");
}

// --- cancel releases the route ------------------------------------------
// The cancel message is optional; unregistering is not, so a handle kept
// after `cancel()` stops collecting frames.

#[test]
fn test_cancel_without_cancel_message_unregisters() {
    let (sub, bus, signals) = request_subscription::<CollectItem>(vec![]);

    sub.cancel();
    assert!(bus.request_messages().is_empty(), "the type has no cancel message");
    assert!(unregistered_request(&signals), "route kept after cancel");
    assert!(matches!(sub.next(), Some(Err(Error::Cancelled))));
}

#[test]
fn test_cancel_after_end_marker_unregisters() {
    let (sub, bus, signals) = request_subscription::<EndOfStreamItem>(vec![data(1)]);
    assert!(sub.next().is_none());

    sub.cancel();
    assert!(bus.request_messages().is_empty(), "no cancel after the end marker");
    assert!(unregistered_request(&signals), "route kept after cancel");
}

#[test]
fn test_cancel_after_snapshot_end_unregisters() {
    let (sub, bus, signals) = request_subscription::<DrainItem>(vec![data(-1)]);
    assert!(matches!(sub.next(), Some(Ok(SubscriptionItem::Data(DrainItem(-1))))));

    sub.cancel();
    assert!(bus.request_messages().is_empty(), "no cancel after the snapshot end");
    assert!(unregistered_request(&signals), "route kept after cancel");
}

#[test]
fn test_order_subscription_cancel_unregisters() {
    let (sub, bus, signals) = routed_subscription::<CollectItem>(SubscriptionBuilder::new().order_id(OrderId::from(7)), vec![]);

    sub.cancel();
    assert!(bus.request_messages().is_empty(), "order streams have no cancel message");
    assert!(
        matches!(signals.try_recv(), Ok(Signal::Order(id, _)) if id == OrderId::from(7)),
        "route kept after cancel"
    );
    assert!(matches!(sub.next(), Some(Err(Error::Cancelled))));
}

#[test]
fn test_order_update_stream_cancel_unregisters() {
    // No request id, order id or shared ticket: the order update stream.
    let (sub, bus, signals) = routed_subscription::<CollectItem>(SubscriptionBuilder::new(), vec![]);

    sub.cancel();
    assert!(bus.request_messages().is_empty());
    assert!(matches!(signals.try_recv(), Ok(Signal::OrderUpdateStream(_))), "route kept after cancel");
    assert!(matches!(sub.next(), Some(Err(Error::Cancelled))));
}

/// The drop signal needs only the lease, so a subscription built without a
/// sender still sends it, with its lease already released.
#[test]
fn test_drop_signals_without_a_sender() {
    let (_sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, signals) = channel::unbounded();
    let internal = SubscriptionBuilder::new()
        .receiver(receiver)
        .signaler(signaler)
        .lease(Lease::new())
        .request_id(RequestId::nth(1))
        .build();

    drop(internal);

    match signals.try_recv() {
        Ok(Signal::Request(id, lease)) => {
            assert_eq!(id, RequestId::nth(1));
            assert!(!lease.is_live(), "signal sent before the lease was released");
        }
        other => panic!("expected a request drop signal, got {}", other.is_ok()),
    }
}

// --- collect_to_end ------------------------------------------------------

#[test]
fn test_collect_to_end_returns_items_and_skips_notices() {
    use crate::messages::Notice;

    let notice = RoutedItem::Notice(Notice {
        request_id: None,
        code: 2104,
        message: "Market data farm OK".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    });
    let (sub, _keep) = collect_subscription(vec![data(10), notice, data(20), RoutedItem::Error(Error::EndOfStream)], true);

    assert_eq!(sub.collect_to_end().unwrap(), vec![CollectItem(10), CollectItem(20)]);
}

#[test]
fn test_collect_to_end_returns_terminal_error() {
    let (sub, _keep) = collect_subscription(vec![data(10), RoutedItem::Error(Error::ConnectionReset)], true);

    assert!(matches!(sub.collect_to_end(), Err(Error::ConnectionReset)));
}

#[test]
fn test_collect_to_end_without_end_marker_is_unexpected_end() {
    // Channel closes after the last item, with no end marker.
    let (sub, _keep) = collect_subscription(vec![data(10)], false);

    assert!(matches!(sub.collect_to_end(), Err(Error::UnexpectedEndOfStream)));
}

// --- cancel_and_drain ----------------------------------------------------

/// Decodes the integer at field 1, and has a cancel message.
#[derive(Debug, PartialEq)]
struct DrainItem(i32);

impl StreamDecoder<DrainItem> for DrainItem {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickPrice];

    fn decode(_context: &DecoderContext, msg: &ResponseMessage) -> Result<DrainItem, Error> {
        Ok(DrainItem(msg.peek_int(1)?))
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(drain_cancel_frame())
    }

    fn is_snapshot_end(&self) -> bool {
        self.0 == -1
    }
}

fn drain_cancel_frame() -> Vec<u8> {
    encode_protobuf_message(OutgoingMessages::CancelContractData as i32, &[])
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn test_drain_after_end_writes_nothing() {
    let (sub, bus, _signals) = request_subscription::<DrainItem>(vec![data(1), RoutedItem::Error(Error::EndOfStream)]);
    while sub.next().is_some() {}

    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Ended);
    assert!(bus.request_messages().is_empty());
}

#[test]
fn test_drain_cancels_then_sees_end() {
    let (sub, bus, _signals) = request_subscription::<DrainItem>(vec![data(1), data(2), RoutedItem::Error(Error::EndOfStream)]);

    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Ended);
    assert_eq!(bus.request_messages(), vec![drain_cancel_frame()], "one cancel, not repeated on drop");
}

#[test]
fn test_drain_reports_tws_error() {
    let (sub, _bus, _signals) = request_subscription::<DrainItem>(vec![
        data(1),
        RoutedItem::Error(Error::Notice(crate::messages::Notice::synthesized(
            200,
            "No security definition".to_string(),
        ))),
    ]);

    assert_eq!(
        sub.cancel_and_drain(deadline()).unwrap(),
        Drained::Rejected(crate::messages::Notice::synthesized(200, "No security definition".to_string()))
    );
}

#[test]
fn test_drain_deadline_is_unconfirmed() {
    let (sub, bus, _signals) = request_subscription::<DrainItem>(vec![data(1)]);

    let started = Instant::now();
    assert_eq!(
        sub.cancel_and_drain(Instant::now() + Duration::from_millis(50)).unwrap(),
        Drained::Unconfirmed
    );
    assert!(started.elapsed() >= Duration::from_millis(50));
    assert_eq!(bus.request_messages(), vec![drain_cancel_frame()], "one cancel, not repeated on drop");
}

#[test]
fn test_drain_after_error_is_unconfirmed_and_cancels_on_drop() {
    let (sub, bus, _signals) = request_subscription::<DrainItem>(vec![RoutedItem::Error(Error::BufferLimitExceeded { limit: 1 })]);
    assert!(matches!(sub.next(), Some(Err(Error::BufferLimitExceeded { .. }))));

    let started = Instant::now();
    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Unconfirmed);
    assert!(started.elapsed() < Duration::from_secs(1), "no wait for evidence that can't arrive");
    assert_eq!(bus.request_messages().len(), 1, "the drop writes the cancel");
}

#[test]
fn test_drain_session_error_is_err() {
    let (sub, _bus, _signals) = request_subscription::<DrainItem>(vec![RoutedItem::Error(Error::ConnectionReset)]);

    assert!(matches!(sub.cancel_and_drain(deadline()), Err(Error::ConnectionReset)));
}

#[test]
fn test_drain_without_cancel_message_waits_for_natural_end() {
    // CollectItem has no cancel message (like contract details below server 215).
    let (sub, bus, _signals) = request_subscription::<CollectItem>(vec![data(1), RoutedItem::Error(Error::EndOfStream)]);

    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Ended);
    assert!(bus.request_messages().is_empty());
}

#[test]
fn test_drain_after_snapshot_end_writes_nothing() {
    let (sub, bus, _signals) = request_subscription::<DrainItem>(vec![data(-1)]);
    assert!(matches!(sub.next(), Some(Ok(SubscriptionItem::Data(DrainItem(-1))))));

    let started = Instant::now();
    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Ended);
    assert!(started.elapsed() < Duration::from_secs(1), "no wait after a finished snapshot");
    assert!(bus.request_messages().is_empty());
}

#[test]
fn test_drain_without_request_id_cancels_and_returns_unconfirmed() {
    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _signaler_rx) = channel::unbounded();
    let internal = SubscriptionBuilder::new()
        .receiver(receiver)
        .signaler(signaler)
        .lease(Lease::new())
        .build();
    let bus = Arc::new(MessageBusStub::default());
    let sub: Subscription<DrainItem> = Subscription::new(bus.clone(), internal, DecoderContext::default());

    assert_eq!(sub.cancel_and_drain(deadline()).unwrap(), Drained::Unconfirmed);
    drop(sender);
}
