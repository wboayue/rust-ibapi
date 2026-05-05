//! Async transport routing tests.
//!
//! Mirror of `transport/sync/tests.rs` routing tests on the async stack.
//! `MemoryStream` lets tests push response frames freely and drive
//! `bus.read_and_route_message()` directly. With `AsyncConnection::stubbed`
//! (server_version defaults to 0), `parse_raw_message` takes the text path,
//! so frame bodies are NUL-delimited strings starting with the message id.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast::error::TryRecvError;

use super::*;
use crate::connection::r#async::AsyncConnection;
use crate::messages::OutgoingMessages;

/// Build a text-format response body: `"msg_id|f1|f2|..."` → `b"msg_id\0f1\0f2\0..."`.
/// Pipes are stand-ins for NULs so test inputs stay readable.
fn body(text: &str) -> Vec<u8> {
    text.replace('|', "\0").into_bytes()
}

/// Wrap a fresh `MemoryStream` in a stubbed `AsyncTcpMessageBus`. server_version=0
/// keeps `parse_raw_message` on the text path.
fn make_bus() -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>) {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    let bus = Arc::new(AsyncTcpMessageBus::new(connection).unwrap());
    (stream, bus)
}

const TICK: Duration = Duration::from_millis(100);

/// Receive next message with a deadline; panics with context if the channel
/// times out, closes, or surfaces an error.
async fn next_message(sub: &mut AsyncInternalSubscription) -> ResponseMessage {
    tokio::time::timeout(TICK, sub.next())
        .await
        .expect("subscription got no message before timeout")
        .expect("subscription closed")
        .expect("subscription error")
}

/// Two in-flight `send_request` subscriptions: responses arrive in reverse order
/// and each subscription receives only its own message.
#[tokio::test]
async fn test_request_id_correlation_with_interleaved_responses() {
    let (stream, bus) = make_bus();

    let mut sub_a = bus.send_request(100, vec![]).await.unwrap();
    let mut sub_b = bus.send_request(200, vec![]).await.unwrap();

    // HistogramData (msg_id 89): request_id at field index 1.
    stream.push_inbound(body("89|200|payload-b|"));
    stream.push_inbound(body("89|100|payload-a|"));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let msg_a = next_message(&mut sub_a).await;
    let msg_b = next_message(&mut sub_b).await;
    assert_eq!(msg_a.peek_int(1).unwrap(), 100);
    assert_eq!(msg_b.peek_int(1).unwrap(), 200);

    // No cross-talk.
    assert!(
        matches!(sub_a.receiver.try_recv(), Err(TryRecvError::Empty)),
        "sub_a received an extra message"
    );
    assert!(
        matches!(sub_b.receiver.try_recv(), Err(TryRecvError::Empty)),
        "sub_b received an extra message"
    );
}

/// Same shape as the request_id test but on the orders channel: two in-flight
/// `send_order_request` subscriptions, OrderStatus responses interleaved.
#[tokio::test]
async fn test_order_id_correlation_with_interleaved_responses() {
    let (stream, bus) = make_bus();

    let mut sub_a = bus.send_order_request(11, vec![]).await.unwrap();
    let mut sub_b = bus.send_order_request(22, vec![]).await.unwrap();

    // OrderStatus (msg_id 3): order_id at field index 1.
    stream.push_inbound(body("3|22|Filled|0|100|0|0|0|0|0||0|"));
    stream.push_inbound(body("3|11|Submitted|0|0|0|0|0|0|0||0|"));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let msg_a = next_message(&mut sub_a).await;
    let msg_b = next_message(&mut sub_b).await;
    assert_eq!(msg_a.peek_int(1).unwrap(), 11);
    assert_eq!(msg_b.peek_int(1).unwrap(), 22);

    assert!(
        matches!(sub_a.receiver.try_recv(), Err(TryRecvError::Empty)),
        "sub_a received an extra message"
    );
    assert!(
        matches!(sub_b.receiver.try_recv(), Err(TryRecvError::Empty)),
        "sub_b received an extra message"
    );
}

/// Shared-channel fan-out: `RequestOpenOrders`, `RequestAllOpenOrders`, and
/// `RequestAutoOpenOrders` all map to `[OpenOrder, OrderStatus, OpenOrderEnd]`
/// in `CHANNEL_MAPPINGS`. With no order subscriber for the incoming order_id,
/// the OrderOrShared strategy fans the message out to every shared subscriber.
#[tokio::test]
async fn test_shared_channel_fan_out_for_open_orders() {
    let (stream, bus) = make_bus();

    let mut sub_open = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, vec![]).await.unwrap();
    let mut sub_all = bus.send_shared_request(OutgoingMessages::RequestAllOpenOrders, vec![]).await.unwrap();
    let mut sub_auto = bus.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, vec![]).await.unwrap();

    // OpenOrder (msg_id 5): order_id at index 1. No matching order subscription,
    // so the OrderOrShared strategy falls back to fan-out.
    stream.push_inbound(body("5|42|265598|AAPL|STK||0|||SMART|USD|AAPL|NMS|"));
    bus.read_and_route_message().await.unwrap();

    for (name, sub) in [("open", &mut sub_open), ("all", &mut sub_all), ("auto", &mut sub_auto)] {
        let msg = next_message(sub).await;
        assert_eq!(msg.peek_int(0).unwrap(), 5, "sub_{name}");
        assert_eq!(msg.peek_int(1).unwrap(), 42, "sub_{name}");
    }
}

/// Shared-channel routing: `send_shared_request` for `RequestCurrentTime`
/// receives the `CurrentTime` response via the channel mapping in
/// `shared_channel_configuration::CHANNEL_MAPPINGS`.
#[tokio::test]
async fn test_shared_channel_routing_current_time() {
    let (stream, bus) = make_bus();

    let mut sub = bus.send_shared_request(OutgoingMessages::RequestCurrentTime, vec![]).await.unwrap();

    stream.push_inbound(body("49|1|1700000000|"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.peek_int(0).unwrap(), 49);
    assert_eq!(msg.peek_int(2).unwrap(), 1_700_000_000);
}

/// EOF on the stream surfaces from `read_and_route_message` as `Io(UnexpectedEof)`.
/// The bus does not silently spin on the closed queue. (The production
/// `process_messages` loop catches this error and triggers reconnect; here we
/// drive `read_and_route_message` once to verify the error is surfaced rather
/// than swallowed.)
#[tokio::test]
async fn test_read_and_route_surfaces_eof() {
    let (stream, bus) = make_bus();

    stream.close();
    let err = bus.read_and_route_message().await.expect_err("dispatch should surface an error");
    assert!(
        matches!(err, Error::Io(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
        "unexpected error: {err:?}"
    );
}

/// `AsyncMessageBus::cancel_subscription` writes the cancel bytes through and
/// drops the in-flight request channel so it stops accepting routes.
#[tokio::test]
async fn test_cancel_subscription_writes_and_clears_channel() {
    let (stream, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    let _sub = mb.send_request(100, b"req-bytes".to_vec()).await.unwrap();
    mb.cancel_subscription(100, b"cancel-bytes".to_vec()).await.unwrap();

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
}

/// `AsyncMessageBus::cancel_order_subscription` mirrors cancel_subscription on
/// the orders channel.
#[tokio::test]
async fn test_cancel_order_subscription_writes_through() {
    let (stream, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    let _sub = mb.send_order_request(42, b"order-bytes".to_vec()).await.unwrap();
    mb.cancel_order_subscription(42, b"cancel-bytes".to_vec()).await.unwrap();

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
}

/// `AsyncMessageBus::send_message` writes through to the connection.
#[tokio::test]
async fn test_send_message_writes_through() {
    let (stream, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    mb.send_message(b"global-cancel-bytes".to_vec()).await.unwrap();

    let captured = stream.captured();
    assert!(captured.windows(b"global-cancel-bytes".len()).any(|w| w == b"global-cancel-bytes"));
}

/// `AsyncMessageBus::create_order_update_subscription` returns
/// `AlreadySubscribed` on duplicate calls.
#[tokio::test]
async fn test_create_order_update_subscription_is_unique() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    let _first = mb.create_order_update_subscription().await.unwrap();
    let err = mb.create_order_update_subscription().await.err().expect("duplicate fails");
    assert!(matches!(err, Error::AlreadySubscribed), "got: {err:?}");
}

/// `AsyncMessageBus::is_connected` reflects the bus state — true initially,
/// false after `request_shutdown_sync` flips the flag.
#[tokio::test]
async fn test_is_connected_reflects_shutdown_flag() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    assert!(mb.is_connected());
    mb.request_shutdown_sync();
    assert!(!mb.is_connected());
}

/// Receive next routed envelope with a deadline.
async fn next_routed(sub: &mut AsyncInternalSubscription) -> RoutedItem {
    tokio::time::timeout(TICK, sub.next_routed())
        .await
        .expect("subscription got no item before timeout")
        .expect("subscription closed")
}

/// Warning code (2104) bound to a real request_id is delivered as a
/// `RoutedItem::Notice` to the owning subscription — stream stays open.
#[tokio::test]
async fn test_warning_with_request_id_delivers_notice() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(body("4|2|42|2104|Market data farm connection is OK:usfarm|"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm connection is OK:usfarm");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: a follow-up data message is delivered.
    stream.push_inbound(body("89|42|payload|"));
    bus.read_and_route_message().await.unwrap();
    let item = next_routed(&mut sub).await;
    assert!(matches!(item, RoutedItem::Response(_)), "got: {item:?}");
}

/// Hard error (code 200) bound to a real request_id is delivered as a
/// `RoutedItem::Error` to the owning subscription.
#[tokio::test]
async fn test_hard_error_with_request_id_terminates_subscription() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(body("4|2|42|200|No security definition found|"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Error(Error::Message(code, msg)) => {
            assert_eq!(code, 200);
            assert_eq!(msg, "No security definition found");
        }
        other => panic!("expected RoutedItem::Error(Message), got {other:?}"),
    }
}

/// Warning with `UNSPECIFIED_REQUEST_ID` has no owner — log only, no channel
/// write to an in-flight subscription.
#[tokio::test]
async fn test_warning_with_unspecified_id_is_log_only() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(body("4|2|-1|2104|Market data farm connection is OK:usfarm|"));
    bus.read_and_route_message().await.unwrap();

    assert!(
        matches!(sub.receiver.try_recv(), Err(TryRecvError::Empty)),
        "unrouted notice must not be delivered to a subscription"
    );
}

/// Order-channel fallback: a notice arrives bound to an `order_id` matching
/// an order subscription. The dispatcher's `deliver_to_request_id` helper
/// falls back to the order channel when no request channel matches.
#[tokio::test]
async fn test_warning_with_order_id_falls_back_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(body("4|2|7|2104|Order warning|"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Order warning");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }
}

// ---- end-to-end Subscription consumer tests for Notice delivery (PR 4) ----
//
// PR 3 verifies dispatcher classification at the `AsyncInternalSubscription`
// seam. These tests close the loop one layer up: drive bytes through the
// production dispatcher and assert via the public async `Subscription<T>` API
// that the consumer sees `SubscriptionItem::Notice` / `Err(_)` / `None` as
// expected.

use crate::subscriptions::r#async::Subscription;
use crate::subscriptions::{DecoderContext, StreamDecoder, SubscriptionItem};
use futures::StreamExt;

/// Trivial test decoder: any non-error message body decodes as `Ok(NoticeTestData)`.
#[derive(Debug)]
struct NoticeTestData;

impl StreamDecoder<NoticeTestData> for NoticeTestData {
    fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<NoticeTestData, Error> {
        Ok(NoticeTestData)
    }
}

fn wrap_subscription(
    bus: Arc<AsyncTcpMessageBus<MemoryStream>>,
    internal: AsyncInternalSubscription,
    request_id: Option<i32>,
    order_id: Option<i32>,
) -> Subscription<NoticeTestData> {
    Subscription::new_from_internal::<NoticeTestData>(internal, bus, request_id, order_id, None, DecoderContext::default())
}

/// Bound a `Subscription::next()` await with the test tick so a missing item
/// surfaces as a panic rather than hanging the test thread.
async fn next_item(sub: &mut Subscription<NoticeTestData>) -> Option<Result<SubscriptionItem<NoticeTestData>, Error>> {
    tokio::time::timeout(TICK, sub.next())
        .await
        .expect("subscription got no item before timeout")
}

/// Code 2104 + request_id=42: dispatcher classifies as `RoutedItem::Notice`,
/// `Subscription<T>::next()` surfaces it as `SubscriptionItem::Notice` without
/// terminating. A follow-up data message arrives normally on the same stream.
#[tokio::test]
async fn test_subscription_notice_delivery_request_keyed() {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(42, vec![]).await.unwrap();
    let mut subscription = wrap_subscription(bus.clone(), internal, Some(42), None);

    stream.push_inbound(body("4|2|42|2104|Market data farm connection is OK:usfarm|"));
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm connection is OK:usfarm");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }

    // Stream stays open: a follow-up data message decodes normally.
    stream.push_inbound(body("89|42|payload|"));
    bus.read_and_route_message().await.unwrap();
    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Data(_))) => {}
        other => panic!("expected SubscriptionItem::Data, got {other:?}"),
    }
}

/// Code 200 + request_id=42: dispatcher classifies as `RoutedItem::Error`,
/// `Subscription<T>::next()` surfaces `Some(Err(_))` and subsequent calls
/// return `None`.
#[tokio::test]
async fn test_subscription_hard_error_terminates_stream() {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(42, vec![]).await.unwrap();
    let mut subscription = wrap_subscription(bus.clone(), internal, Some(42), None);

    stream.push_inbound(body("4|2|42|200|No security definition found|"));
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Err(Error::Message(code, message))) => {
            assert_eq!(code, 200);
            assert_eq!(message, "No security definition found");
        }
        other => panic!("expected Some(Err(Error::Message)), got {other:?}"),
    }

    // Stream is terminated: subsequent reads return None.
    assert!(next_item(&mut subscription).await.is_none(), "stream must end after terminal error");
}

/// Order-keyed notice: a warning bound to an `order_id` is delivered to the
/// order subscription via `deliver_to_request_id`'s order-channel fallback,
/// surfacing as `SubscriptionItem::Notice` to the consumer.
#[tokio::test]
async fn test_subscription_notice_delivery_order_keyed() {
    let (stream, bus) = make_bus();
    let internal = bus.send_order_request(7, vec![]).await.unwrap();
    let mut subscription = wrap_subscription(bus.clone(), internal, None, Some(7));

    stream.push_inbound(body("4|2|7|2109|Outside RTH order warning|"));
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2109);
            assert_eq!(notice.message, "Outside RTH order warning");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
}

/// Code 2104 + request_id=UNSPECIFIED: no owner — dispatcher logs and skips
/// the channel write. An unrelated in-flight subscription sees nothing within
/// the test tick window.
#[tokio::test]
async fn test_subscription_unspecified_notice_not_delivered() {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(42, vec![]).await.unwrap();
    let mut subscription = wrap_subscription(bus.clone(), internal, Some(42), None);

    stream.push_inbound(body("4|2|-1|2104|Market data farm connection is OK:usfarm|"));
    bus.read_and_route_message().await.unwrap();

    let item = tokio::time::timeout(TICK, subscription.next()).await;
    assert!(item.is_err(), "unrouted notice must not be delivered to a subscription, got {item:?}");
}

/// `data_stream()` filters `SubscriptionItem::Notice` entries (logging them)
/// and yields only the underlying data values.
#[tokio::test]
async fn test_subscription_data_stream_filters_notices() {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(42, vec![]).await.unwrap();
    let mut subscription = wrap_subscription(bus.clone(), internal, Some(42), None);

    stream.push_inbound(body("89|42|first|"));
    stream.push_inbound(body("4|2|42|2104|Market data farm connection is OK:usfarm|"));
    stream.push_inbound(body("89|42|second|"));
    for _ in 0..3 {
        bus.read_and_route_message().await.unwrap();
    }

    let collected: Vec<_> = subscription.data_stream().take(2).collect().await;
    assert_eq!(collected.len(), 2, "data_stream must yield the two data items");
    for item in collected {
        assert!(matches!(item, Ok(NoticeTestData)), "unexpected stream item");
    }
}
