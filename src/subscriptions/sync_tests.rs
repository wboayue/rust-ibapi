use super::*;
use crate::messages::{encode_protobuf_message, OutgoingMessages, ResponseMessage};
use crate::stubs::MessageBusStub;
use std::sync::Arc;

#[derive(Debug)]
struct EndOfStreamItem;

impl StreamDecoder<EndOfStreamItem> for EndOfStreamItem {
    fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<EndOfStreamItem, Error> {
        Err(Error::EndOfStream)
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[]))
    }
}

#[test]
fn test_subscription_skips_unexpected_messages_without_limit() {
    use std::sync::atomic::AtomicUsize;

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct SkipThenSuccess;

    impl StreamDecoder<SkipThenSuccess> for SkipThenSuccess {
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<SkipThenSuccess, Error> {
            let n = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            if n < 20 {
                Err(Error::unexpected_response(&ResponseMessage::from("stray\0")))
            } else {
                Ok(SkipThenSuccess)
            }
        }
    }

    CALL_COUNT.store(0, Ordering::Relaxed);

    // 20 stray messages + 1 valid (more than the old MAX_DECODE_RETRIES=10)
    let mut responses: Vec<String> = (0..21).map(|_| "1|msg".to_string()).collect();
    // Sentinel to avoid blocking on the channel after success
    responses.push("1|done".to_string());

    let stub = MessageBusStub::with_responses(responses);
    let message_bus = Arc::new(stub);

    let sub: Subscription<SkipThenSuccess> = {
        let internal = message_bus.send_request(1, &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    let result = sub.next();
    assert!(result.is_some(), "subscription should survive 20 skips and return valid message");
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 21);
}

#[test]
fn test_routed_item_error_terminates_subscription() {
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    #[derive(Debug)]
    struct DataItem;

    impl StreamDecoder<DataItem> for DataItem {
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<DataItem, Error> {
            Ok(DataItem)
        }
    }

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _) = channel::unbounded();
    sender.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(1).build();

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
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<DataItem, Error> {
            Ok(DataItem)
        }
    }

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _) = channel::unbounded();

    sender
        .send(RoutedItem::Notice(Notice {
            code: 2104,
            message: "Market data farm OK".into(),
            error_time: None,
            advanced_order_reject_json: String::new(),
        }))
        .unwrap();
    sender.send(RoutedItem::Response(ResponseMessage::from("1|data\0"))).unwrap();

    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(1).build();
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
        let internal = message_bus.send_request(1, &[]).unwrap();
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
use crate::transport::SubscriptionBuilder;
use crossbeam::channel;
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

/// Build a `Subscription<CollectItem>` pre-loaded with `items`. When `keep_open`
/// the channel sender is returned so the channel stays open (lets the timeout
/// branch fire); otherwise it is dropped so the stream ends after draining.
fn collect_subscription(items: Vec<RoutedItem>, keep_open: bool) -> (Subscription<CollectItem>, Option<channel::Sender<RoutedItem>>) {
    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _signaler_rx) = channel::unbounded();
    for item in items {
        sender.send(item).unwrap();
    }
    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(1).build();
    let stub = Arc::new(MessageBusStub::default());
    let sub = Subscription::new(stub, internal, DecoderContext::default());
    let keep = if keep_open { Some(sender) } else { None };
    (sub, keep)
}

fn data(value: i32) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("{value}\0")))
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
