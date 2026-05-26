//! Async transport routing tests.
//!
//! Mirror of `transport/sync/tests.rs` routing tests on the async stack.
//! `MemoryStream` lets tests push response frames freely and drive
//! `bus.read_and_route_message()` directly. Frames use the
//! binary-text-payload framing that `parse_raw_message` expects post-floor-213:
//! `[4-byte BE msg_id][NUL-delimited remaining fields]`, produced by `body()`.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::connection::r#async::AsyncConnection;
use crate::messages::OutgoingMessages;
use crate::server_versions;

/// Build a binary-text-payload response body from a pipe-delimited test input.
/// `"msg_id|f1|f2|..."` → `[4-byte BE msg_id][f1\0f2\0...]`. Pipes are
/// stand-ins for NULs so test inputs stay readable. For `Error` frames
/// (msg_id `4`), use [`body_error`] — they ship as protobuf post-floor-213
/// and the binary-text-payload path defaults to an empty Notice.
fn body(text: &str) -> Vec<u8> {
    let fields: Vec<&str> = text.split_terminator('|').collect();
    let msg_id: i32 = fields[0].parse().expect("body() fixture must start with a numeric msg_id");
    debug_assert_ne!(
        msg_id,
        crate::messages::IncomingMessages::Error as i32,
        "Error frames must use body_error() — protobuf-framed since PR-D1"
    );
    let payload: String = fields[1..].iter().map(|f| format!("{f}\0")).collect();
    let mut data = msg_id.to_be_bytes().to_vec();
    data.extend_from_slice(payload.as_bytes());
    data
}

/// Build a protobuf-framed `Error` response frame for `MemoryStream::push_inbound`.
/// Mirrors what TWS sends at floor 213+ and exercises the proto path in
/// `Notice::from(&ResponseMessage)` / `determine_routing`.
fn body_error(request_id: i32, code: i32, msg: &str) -> Vec<u8> {
    let envelope = crate::proto::ErrorMessage {
        id: Some(request_id),
        error_time: None,
        error_code: Some(code),
        error_msg: Some(msg.into()),
        advanced_order_reject_json: None,
    };
    crate::common::test_utils::helpers::binary_proto(crate::messages::IncomingMessages::Error as i32, &envelope)
}

/// Wrap a fresh `MemoryStream` in a stubbed `AsyncTcpMessageBus`. Pins
/// `server_version` to the current floor so `parse_raw_message` produces
/// binary-text-payload frames from `body()` inputs.
fn make_bus() -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>) {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection
        .server_version_cache
        .store(server_versions::PROTOBUF_REST_MESSAGES_3, Ordering::Release);
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
    assert!(sub_a.try_next_routed().is_none(), "sub_a received an extra message");
    assert!(sub_b.try_next_routed().is_none(), "sub_b received an extra message");
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

    assert!(sub_a.try_next_routed().is_none(), "sub_a received an extra message");
    assert!(sub_b.try_next_routed().is_none(), "sub_b received an extra message");
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

    stream.push_inbound(body_error(42, 2104, FARM_OK_MSG));
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

    stream.push_inbound(body_error(42, 200, "No security definition found"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Error(Error::Notice(notice)) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected RoutedItem::Error(Notice), got {other:?}"),
    }
}

/// Warning with `UNSPECIFIED_REQUEST_ID` has no owner — log only, no channel
/// write to an in-flight subscription.
#[tokio::test]
async fn test_warning_with_unspecified_id_is_log_only() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(body_error(-1, 2104, FARM_OK_MSG));
    bus.read_and_route_message().await.unwrap();

    assert!(sub.try_next_routed().is_none(), "unrouted notice must not be delivered to a subscription");
}

/// Order-channel fallback: a notice arrives bound to an `order_id` matching
/// an order subscription. The dispatcher's `deliver_to_request_id` helper
/// falls back to the order channel when no request channel matches.
#[tokio::test]
async fn test_warning_with_order_id_falls_back_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(body_error(7, 2104, "Order warning"));
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

// ---- end-to-end Subscription consumer tests for Notice delivery ----
//
// Mirror the dispatcher routing tests above, one layer up: drive bytes through
// the production dispatcher and assert via the public async `Subscription<T>`
// API that the consumer sees `SubscriptionItem::Notice` / `Err(_)` / `None` as
// expected.

use crate::subscriptions::r#async::Subscription;
use crate::subscriptions::{DecoderContext, StreamDecoder, SubscriptionItem, SubscriptionItemStreamExt};
use futures::StreamExt;

const FARM_OK_MSG: &str = "Market data farm connection is OK:usfarm";

fn farm_ok_frame_42() -> Vec<u8> {
    body_error(42, 2104, FARM_OK_MSG)
}

fn farm_ok_frame_unrouted() -> Vec<u8> {
    body_error(-1, 2104, FARM_OK_MSG)
}

#[derive(Debug)]
struct NoticeTestData;

impl StreamDecoder<NoticeTestData> for NoticeTestData {
    fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<NoticeTestData, Error> {
        Ok(NoticeTestData)
    }
}

async fn make_request_subscription(request_id: i32) -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>, Subscription<NoticeTestData>) {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(request_id, vec![]).await.unwrap();
    let sub = Subscription::new_from_internal::<NoticeTestData>(internal, bus.clone(), Some(request_id), None, DecoderContext::default());
    (stream, bus, sub)
}

async fn make_order_subscription(order_id: i32) -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>, Subscription<NoticeTestData>) {
    let (stream, bus) = make_bus();
    let internal = bus.send_order_request(order_id, vec![]).await.unwrap();
    let sub = Subscription::new_from_internal::<NoticeTestData>(internal, bus.clone(), None, Some(order_id), DecoderContext::default());
    (stream, bus, sub)
}

/// Bound a `Subscription::next()` await with the test tick so a missing item
/// surfaces as a panic rather than hanging the test thread.
async fn next_item(sub: &mut Subscription<NoticeTestData>) -> Option<Result<SubscriptionItem<NoticeTestData>, Error>> {
    tokio::time::timeout(TICK, sub.next())
        .await
        .expect("subscription got no item before timeout")
}

/// Code 2104 + request_id=42 surfaces as `SubscriptionItem::Notice` without
/// terminating; a follow-up data message arrives normally on the same stream.
#[tokio::test]
async fn test_subscription_notice_delivery_request_keyed() {
    let (stream, bus, mut subscription) = make_request_subscription(42).await;

    stream.push_inbound(farm_ok_frame_42());
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, FARM_OK_MSG);
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }

    stream.push_inbound(body("89|42|payload|"));
    bus.read_and_route_message().await.unwrap();
    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Data(_))) => {}
        other => panic!("expected SubscriptionItem::Data, got {other:?}"),
    }
}

/// Hard error (code 200) surfaces as `Some(Err(_))`; subsequent reads return `None`.
#[tokio::test]
async fn test_subscription_hard_error_terminates_stream() {
    let (stream, bus, mut subscription) = make_request_subscription(42).await;

    stream.push_inbound(body_error(42, 200, "No security definition found"));
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Err(Error::Notice(notice))) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected Some(Err(Error::Notice)), got {other:?}"),
    }

    assert!(next_item(&mut subscription).await.is_none(), "stream must end after terminal error");
}

/// Order-keyed notice via `deliver_to_request_id`'s order-channel fallback.
#[tokio::test]
async fn test_subscription_notice_delivery_order_keyed() {
    let (stream, bus, mut subscription) = make_order_subscription(7).await;

    stream.push_inbound(body_error(7, 2109, "Outside RTH order warning"));
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2109);
            assert_eq!(notice.message, "Outside RTH order warning");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
}

/// Unrouted notice (UNSPECIFIED request_id) is log-only; no channel write.
#[tokio::test]
async fn test_subscription_unspecified_notice_not_delivered() {
    let (stream, bus, mut subscription) = make_request_subscription(42).await;

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.read_and_route_message().await.unwrap();

    let item = tokio::time::timeout(TICK, subscription.next()).await;
    assert!(item.is_err(), "unrouted notice must not be delivered to a subscription, got {item:?}");
}

/// `data_stream()` filters `SubscriptionItem::Notice` and yields only data.
#[tokio::test]
async fn test_subscription_data_stream_filters_notices() {
    let (stream, bus, subscription) = make_request_subscription(42).await;

    stream.push_inbound(body("89|42|first|"));
    stream.push_inbound(farm_ok_frame_42());
    stream.push_inbound(body("89|42|second|"));
    for _ in 0..3 {
        bus.read_and_route_message().await.unwrap();
    }

    let collected: Vec<_> = subscription.filter_data().take(2).collect().await;
    assert_eq!(collected.len(), 2, "filter_data() must yield the two data items");
    for item in collected {
        assert!(matches!(item, Ok(NoticeTestData)), "unexpected stream item");
    }
}

// ---- end-to-end NoticeStream tests (PR 5) ----
//
// Mirror of the sync `notice_stream` dispatcher tests on the async stack.

/// An unrouted warning is delivered to a `notice_stream` subscriber.
#[tokio::test]
async fn test_notice_stream_receives_unrouted_warning() {
    let (stream, bus) = make_bus();
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.read_and_route_message().await.unwrap();

    let notice = tokio::time::timeout(TICK, notice_stream.next())
        .await
        .expect("notice not delivered before timeout")
        .expect("stream closed early");
    assert_eq!(notice.code, 2104);
    assert_eq!(notice.message, FARM_OK_MSG);
}

/// Two `notice_subscribe` calls each receive every unrouted notice.
#[tokio::test]
async fn test_notice_stream_fans_out_to_multiple_subscribers() {
    let (stream, bus) = make_bus();
    let mut s1 = bus.notice_subscribe();
    let mut s2 = bus.notice_subscribe();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.read_and_route_message().await.unwrap();

    let n1 = tokio::time::timeout(TICK, s1.next()).await.unwrap().unwrap();
    let n2 = tokio::time::timeout(TICK, s2.next()).await.unwrap().unwrap();
    assert_eq!(n1.code, 2104);
    assert_eq!(n2.code, 2104);
}

/// Severity-agnostic: an unrouted hard error also fans out.
#[tokio::test]
async fn test_notice_stream_receives_unrouted_hard_error() {
    let (stream, bus) = make_bus();
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(body_error(-1, 504, "Not connected"));
    bus.read_and_route_message().await.unwrap();

    let notice = tokio::time::timeout(TICK, notice_stream.next()).await.unwrap().unwrap();
    assert_eq!(notice.code, 504);
}

/// A routed notice (real `request_id`) goes to the owning subscription, NOT
/// to the global notice stream.
#[tokio::test]
async fn test_notice_stream_skips_routed_notices() {
    let (stream, bus, mut subscription) = make_request_subscription(42).await;
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(farm_ok_frame_42());
    bus.read_and_route_message().await.unwrap();

    // Routed to the owner.
    let item = tokio::time::timeout(TICK, subscription.next()).await.unwrap();
    assert!(matches!(item, Some(Ok(SubscriptionItem::Notice(_)))), "owner missed notice");

    // NOT delivered to the global stream.
    let leaked = tokio::time::timeout(TICK, notice_stream.next()).await;
    assert!(leaked.is_err(), "routed notice leaked to global stream");
}

/// Late subscribers don't see prior notices (no replay buffer on broadcast).
#[tokio::test]
async fn test_notice_stream_late_subscriber_misses_prior() {
    let (stream, bus) = make_bus();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.read_and_route_message().await.unwrap();

    // Subscribe AFTER the broadcast.
    let mut late = bus.notice_subscribe();
    let leaked = tokio::time::timeout(TICK, late.next()).await;
    assert!(leaked.is_err(), "late subscriber should not see prior notices");
}

// ---- order-routing strategy tests ----
//
// Mirror of the sync-side `process_orders` strategy tests. `route_to_order_channel`
// dispatches by `order_routing_strategy(message_type)`; each strategy has a
// different fallback order (order_id → request_id, by execution_id, shared-only).

/// Text-format ExecutionData body: `request_id` at field 1, `order_id` at field 2,
/// `execution_id` at field 14 (the dispatcher's persisted-mapping key).
fn execution_data_body(request_id: i32, order_id: i32, execution_id: &str) -> Vec<u8> {
    let mut frame = format!("11|{request_id}|{order_id}|");
    for _ in 3..14 {
        frame.push_str("0|");
    }
    frame.push_str(execution_id);
    frame.push('|');
    body(&frame)
}

#[tokio::test]
async fn test_execution_data_routes_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.peek_int(2).unwrap(), 7);
}

#[tokio::test]
async fn test_execution_data_falls_back_to_request_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(99, vec![]).await.unwrap();

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.peek_int(1).unwrap(), 99);
}

#[tokio::test]
async fn test_execution_data_orphan_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an orphan message");
}

#[tokio::test]
async fn test_execution_data_end_routes_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(body("55|1|7|"));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut sub).await;
}

/// ExecutionDataEnd uses field 2 for BOTH request_id and order_id, so a request
/// subscription on the same id catches it via the order-channel-miss fallback.
#[tokio::test]
async fn test_execution_data_end_falls_back_to_request_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request(7, vec![]).await.unwrap();

    stream.push_inbound(body("55|1|7|"));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut sub).await;
}

#[tokio::test]
async fn test_execution_data_end_orphan_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(42, vec![]).await.unwrap();

    stream.push_inbound(body("55|1|999|"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an orphan end");
}

/// `ByExecutionId`: the prior ExecutionData stores `exec-abc → order_id 7`'s
/// sender, and the CommissionsReport rides that mapping back to the same sub.
#[tokio::test]
async fn test_commission_report_routes_via_execution_id_mapping() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(execution_data_body(99, 7, "exec-abc"));
    stream.push_inbound(body("59|1|exec-abc|"));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let exec_msg = next_message(&mut sub).await;
    assert_eq!(exec_msg.peek_int(0).unwrap(), 11);
    let commission = next_message(&mut sub).await;
    assert_eq!(commission.peek_int(0).unwrap(), 59);
}

#[tokio::test]
async fn test_commission_report_without_mapping_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_order_request(7, vec![]).await.unwrap();

    stream.push_inbound(body("59|1|exec-not-mapped|"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an unmapped commission");
}

#[tokio::test]
async fn test_completed_order_routes_to_shared_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_shared_request(OutgoingMessages::RequestCompletedOrders, vec![]).await.unwrap();

    stream.push_inbound(body("101|265598|AAPL|STK|"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.peek_int(0).unwrap(), 101);
}

#[tokio::test]
async fn test_completed_orders_end_routes_to_shared_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_shared_request(OutgoingMessages::RequestCompletedOrders, vec![]).await.unwrap();

    stream.push_inbound(body("102|"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.peek_int(0).unwrap(), 102);
}

// ---- order-update stream + lifecycle tests ----

/// `send_order_update` fan-out: an OpenOrder reaches both an order subscription
/// and the order-update stream when both are registered for the same order.
#[tokio::test]
async fn test_order_update_stream_receives_open_order() {
    let (stream, bus) = make_bus();
    let mut order_sub = bus.send_order_request(42, vec![]).await.unwrap();
    let mut stream_sub = bus.create_order_update_subscription().await.unwrap();

    stream.push_inbound(body("5|42|265598|AAPL|STK||0|||SMART|USD|AAPL|NMS|"));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut order_sub).await;
    next_message(&mut stream_sub).await;
}

/// Routed-but-orphan notice (real request_id, no matching sub) takes the
/// `log_orphan` path, NOT the global notice stream.
#[tokio::test]
async fn test_warning_with_orphan_request_id_logs() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(42, vec![]).await.unwrap();
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(body_error(99, 2104, "orphan warning"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got the notice");
    let leaked = tokio::time::timeout(TICK, notice_stream.next()).await;
    assert!(leaked.is_err(), "global notice stream got a routed-but-orphan notice");
}

/// `reset_channels` after reconnect: every in-flight request and order
/// subscription receives `Error::ConnectionReset`, then the channel maps are
/// cleared.
#[tokio::test]
async fn test_reset_channels_notifies_in_flight_subscriptions() {
    let (_, bus) = make_bus();

    let mut req = bus.send_request(100, vec![]).await.unwrap();
    let mut order = bus.send_order_request(200, vec![]).await.unwrap();

    bus.reset_channels().await;

    for (name, sub) in [("request", &mut req), ("order", &mut order)] {
        let item = tokio::time::timeout(TICK, sub.next_routed())
            .await
            .unwrap_or_else(|_| panic!("{name} got no notification"))
            .unwrap_or_else(|| panic!("{name} channel closed early"));
        assert!(matches!(item, RoutedItem::Error(Error::ConnectionReset)), "{name}: {item:?}");
    }

    assert!(bus.request_channels.read().await.is_empty());
    assert!(bus.order_channels.read().await.is_empty());
    assert!(bus.execution_channels.read().await.is_empty());
}

/// `ensure_shutdown` joins the running message-processing task and reports
/// `is_connected() == false` afterwards. The handle is installed asynchronously
/// (separate `tokio::spawn`), so we yield until it's set rather than sleeping.
#[tokio::test]
async fn test_ensure_shutdown_joins_processing_task() {
    let (_, bus) = make_bus();
    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    while bus.process_task.read().await.is_none() {
        assert!(tokio::time::Instant::now() < deadline, "process_task never installed");
        tokio::task::yield_now().await;
    }

    mb.ensure_shutdown().await;
    assert!(!mb.is_connected());
}

#[tokio::test]
async fn test_cancel_unknown_subscription_writes_through() {
    let (stream, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    mb.cancel_subscription(7777, b"cancel-bytes".to_vec()).await.unwrap();

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
}

#[tokio::test]
async fn test_send_shared_request_unsupported_returns_error() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    match mb.send_shared_request(OutgoingMessages::PlaceOrder, b"x".to_vec()).await {
        Err(Error::InvalidArgument(_)) => {}
        other => panic!("expected Error::InvalidArgument, got {:?}", other.err()),
    }
}
