//! Async transport routing tests.
//!
//! Mirror of `transport/sync/tests.rs` routing tests on the async stack.
//! `MemoryStream` lets tests push response frames freely and drive
//! `bus.read_and_route_message()` directly. Frames use the
//! binary-text-payload framing that `parse_raw_message` expects post-floor-213:
//! `[4-byte BE msg_id][NUL-delimited remaining fields]`, produced by `body()`.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::client::ids::{OrderId, RequestId};
use crate::common::test_utils::helpers;
use crate::common::test_utils::helpers::{
    binary_proto, body, error_frame, execution_data_frame, farm_ok_frame_42, farm_ok_frame_unrouted, handshake_frames, NoticeTestData, FARM_OK_MSG,
};
use crate::connection::r#async::AsyncConnection;
use crate::messages::{OutgoingMessages, TRANSPORT_RECONNECT_CODE};
use crate::server_versions;
use crate::testdata::builders::contracts::contract_data;
use crate::testdata::builders::orders::order_bound;
use crate::testdata::builders::ResponseProtoEncoder;

/// Wrap a fresh `MemoryStream` in a stubbed `AsyncTcpMessageBus`. Pins
/// `server_version` to the current floor so `parse_raw_message` produces
/// binary-text-payload frames from `body()` inputs.
fn make_bus() -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>) {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(AsyncTcpMessageBus::new(connection).unwrap());
    (stream, bus)
}

const TICK: Duration = Duration::from_millis(100);

/// `with_channel_capacity` reaches the per-request channels: a capacity-2
/// channel holds at most 2 queued frames, evicting the oldest (#779).
#[tokio::test]
async fn test_with_channel_capacity_bounds_request_channels() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(AsyncTcpMessageBus::with_channel_capacity(connection, 2).unwrap());

    let _sub = bus.send_request(RequestId::nth(1), vec![]).await.unwrap();
    let sender = bus.requests.sender(&RequestId::nth(1)).unwrap();
    for _ in 0..3 {
        sender.send(RoutedItem::Error(Error::Cancelled)).unwrap();
    }
    assert_eq!(sender.len(), 2, "capacity-2 channel retains only the newest 2 frames");
}

/// Send `capacity + 1` items through `send`, then expect `subscription` to
/// report exactly one dropped frame with `class`'s lag notice: proof that its
/// channel holds `capacity` and that the subscription carries `class`.
async fn assert_capacity_and_class(name: &str, subscription: &mut AsyncInternalSubscription, send: impl Fn(), capacity: usize, class: ChannelClass) {
    for _ in 0..=capacity {
        send();
    }
    let expected = match class {
        ChannelClass::MarketData => crate::messages::subscription_lag_notice(1),
        ChannelClass::Order | ChannelClass::OrderStream => crate::messages::order_lag_notice(1),
    };
    match subscription.next_routed().await {
        Some(RoutedItem::Notice(notice)) => assert_eq!(notice, expected, "{name}"),
        other => panic!("{name}: expected a one-frame lag notice, got {other:?}"),
    }
}

/// A small `channel_capacity` sizes market-data channels but never shrinks
/// the order-class floors, and each subscription carries its channel's class
/// (#896).
#[tokio::test]
async fn test_channel_classes_under_small_channel_capacity() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(AsyncTcpMessageBus::with_channel_capacity(connection, 1).unwrap());
    let cancelled = || RoutedItem::Error(Error::Cancelled);

    let mut market = bus.send_request(RequestId::nth(1), vec![]).await.unwrap();
    let sender = bus.requests.sender(&RequestId::nth(1)).unwrap();
    let send = || drop(sender.send(cancelled()));
    assert_capacity_and_class("request", &mut market, send, 1, ChannelClass::MarketData).await;

    let mut executions = bus.send_executions_request(RequestId::nth(2), vec![]).await.unwrap();
    let sender = bus.requests.sender(&RequestId::nth(2)).unwrap();
    let send = || drop(sender.send(cancelled()));
    assert_capacity_and_class("executions", &mut executions, send, ORDER_CAPACITY, ChannelClass::Order).await;

    let mut order = bus.send_order_request(OrderId::new(3), vec![]).await.unwrap();
    let sender = bus.orders.sender(&OrderId::new(3)).unwrap();
    let send = || drop(sender.send(cancelled()));
    assert_capacity_and_class("order", &mut order, send, ORDER_CAPACITY, ChannelClass::Order).await;

    let mut updates = bus.create_order_update_subscription().await.unwrap();
    let sender = bus.order_update_stream.lock().unwrap().as_ref().unwrap().sender.clone();
    let send = || drop(sender.send(cancelled()));
    assert_capacity_and_class(
        "order update stream",
        &mut updates,
        send,
        ORDER_STREAM_CAPACITY,
        ChannelClass::OrderStream,
    )
    .await;

    // Shared channels: `notify_all` reaches every allocated one, so each is
    // subscribed only when its turn comes.
    let notify = || bus.shared_channels.notify_all(cancelled);
    let mut positions = bus.send_shared_request(OutgoingMessages::RequestPositions, vec![]).await.unwrap();
    assert_capacity_and_class("positions", &mut positions, notify, 1, ChannelClass::MarketData).await;
    let mut open_orders = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, vec![]).await.unwrap();
    assert_capacity_and_class("open orders", &mut open_orders, notify, ORDER_STREAM_CAPACITY, ChannelClass::OrderStream).await;
}

/// A `channel_capacity` above a floor raises the order-class channels too.
#[test]
fn test_channel_class_capacity() {
    assert_eq!(ChannelClass::MarketData.capacity(4), 4);
    assert_eq!(ChannelClass::Order.capacity(4), ORDER_CAPACITY);
    assert_eq!(ChannelClass::OrderStream.capacity(4), ORDER_STREAM_CAPACITY);
    let large = ORDER_STREAM_CAPACITY * 2;
    assert_eq!(ChannelClass::Order.capacity(large), large);
    assert_eq!(ChannelClass::OrderStream.capacity(large), large);
}

#[test]
fn test_shared_order_channels_are_order_class() {
    for request in [
        OutgoingMessages::RequestOpenOrders,
        OutgoingMessages::RequestAllOpenOrders,
        OutgoingMessages::RequestAutoOpenOrders,
        OutgoingMessages::RequestCompletedOrders,
    ] {
        assert_eq!(ChannelClass::of_shared(request), ChannelClass::OrderStream, "{request:?}");
    }
    assert_eq!(ChannelClass::of_shared(OutgoingMessages::RequestPositions), ChannelClass::MarketData);
}

/// A lag on an order-class subscription surfaces the order lag notice, and
/// clones keep the class.
#[tokio::test]
async fn test_order_class_lag_surfaces_order_lag_notice() {
    let (sender, receiver) = broadcast::channel(1);
    let mut subscription = AsyncInternalSubscription::new(receiver).class(ChannelClass::Order);
    let mut clone = subscription.clone();
    for _ in 0..4 {
        sender.send(RoutedItem::Error(Error::Cancelled)).unwrap();
    }

    for sub in [&mut subscription, &mut clone] {
        match sub.next_routed().await {
            Some(RoutedItem::Notice(notice)) => assert_eq!(notice, crate::messages::order_lag_notice(3)),
            other => panic!("expected order lag notice, got {other:?}"),
        }
    }
}

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

    let (id_a, id_b) = (RequestId::nth(100), RequestId::nth(200));
    let mut sub_a = bus.send_request(id_a, vec![]).await.unwrap();
    let mut sub_b = bus.send_request(id_b, vec![]).await.unwrap();

    // HistogramData (msg_id 89): request_id at field index 1.
    stream.push_inbound(body(&format!("89|{id_b}|payload-b|")));
    stream.push_inbound(body(&format!("89|{id_a}|payload-a|")));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let msg_a = next_message(&mut sub_a).await;
    let msg_b = next_message(&mut sub_b).await;
    assert_eq!(msg_a.peek_int(1).unwrap(), id_a.raw());
    assert_eq!(msg_b.peek_int(1).unwrap(), id_b.raw());

    // No cross-talk.
    assert!(sub_a.try_next_routed().is_none(), "sub_a received an extra message");
    assert!(sub_b.try_next_routed().is_none(), "sub_b received an extra message");
}

/// Same shape as the request_id test but on the orders channel: two in-flight
/// `send_order_request` subscriptions, OrderStatus responses interleaved.
#[tokio::test]
async fn test_order_id_correlation_with_interleaved_responses() {
    let (stream, bus) = make_bus();

    let mut sub_a = bus.send_order_request(OrderId::from(11), vec![]).await.unwrap();
    let mut sub_b = bus.send_order_request(OrderId::from(22), vec![]).await.unwrap();

    // OrderStatus carries `order_id` at proto tag 1.
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OrderStatus as i32,
        &crate::proto::OrderStatus {
            order_id: Some(22),
            status: Some("Filled".into()),
            ..Default::default()
        },
    ));
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OrderStatus as i32,
        &crate::proto::OrderStatus {
            order_id: Some(11),
            status: Some("Submitted".into()),
            ..Default::default()
        },
    ));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let msg_a = next_message(&mut sub_a).await;
    let msg_b = next_message(&mut sub_b).await;
    assert_eq!(msg_a.order_id(), Some(11));
    assert_eq!(msg_b.order_id(), Some(22));

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

    // OpenOrder carries `order_id` at proto tag 1; no matching order subscription
    // means the OrderOrShared strategy falls back to fan-out across shared subs.
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.read_and_route_message().await.unwrap();

    for (name, sub) in [("open", &mut sub_open), ("all", &mut sub_all), ("auto", &mut sub_auto)] {
        let msg = next_message(sub).await;
        assert_eq!(msg.message_type(), crate::messages::IncomingMessages::OpenOrder, "sub_{name}");
        assert_eq!(msg.order_id(), Some(42), "sub_{name}");
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

/// After shutdown, a new order-update stream is refused rather than stored
/// as a sender nothing will ever drop (#871).
#[tokio::test]
async fn test_create_order_update_subscription_after_shutdown_fails() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();
    mb.ensure_shutdown().await;

    let err = mb.create_order_update_subscription().await.err().expect("subscribe after shutdown");
    assert!(matches!(err, Error::Shutdown), "got: {err:?}");
    assert!(bus.order_update_stream.lock().unwrap().is_none());
}

/// Shutdown ends a live notice stream, and one opened afterwards is already
/// ended. Before, the connection kept the only sender alive and both waited
/// forever (#871).
#[tokio::test]
async fn test_notice_stream_ends_on_shutdown() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();
    let mut live = mb.notice_subscribe();

    mb.ensure_shutdown().await;

    let ended = tokio::time::timeout(Duration::from_millis(500), live.next()).await;
    assert!(matches!(ended, Ok(None)), "live stream: {ended:?}");

    let mut late = mb.notice_subscribe();
    bus.connection.notice_broadcaster.broadcast(Notice::synthesized(-1, "late".into()));
    let ended = tokio::time::timeout(Duration::from_millis(500), late.next()).await;
    assert!(matches!(ended, Ok(None)), "late stream: {ended:?}");
}

/// `request_shutdown_sync` (the `Drop` path) ends notice streams too.
#[tokio::test]
async fn test_notice_stream_ends_on_request_shutdown_sync() {
    let (_, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();
    let mut live = mb.notice_subscribe();

    mb.request_shutdown_sync();

    let ended = tokio::time::timeout(Duration::from_millis(500), live.next()).await;
    assert!(matches!(ended, Ok(None)), "live stream: {ended:?}");
}

/// `Client::drop` calls `request_shutdown_sync`, which must end a live
/// order-update stream (whose subscription holds the bus).
#[tokio::test]
async fn test_order_update_stream_ends_on_request_shutdown_sync() {
    let (_, bus) = make_bus();
    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");
    let mut updates = bus.create_order_update_subscription().await.unwrap();

    bus.request_shutdown_sync();

    let drained = tokio::time::timeout(Duration::from_millis(500), async { while updates.next().await.is_some() {} }).await;
    assert!(drained.is_ok(), "order-update stream did not end");
}

/// Like the sync bus, shutdown fails every kind of subscription with
/// `Error::Shutdown` before ending it, so a reader can tell shutdown from a
/// stream that simply closed. Through `request_shutdown_sync`, the
/// `Client::drop` path, with no dispatcher to finish it.
#[tokio::test]
async fn test_shutdown_fails_every_subscription_then_ends_it() {
    let (_, bus) = make_bus();
    let subscriptions = [
        ("request", bus.send_request(RequestId::nth(1), vec![]).await.unwrap()),
        ("order", bus.send_order_request(OrderId::from(7), vec![]).await.unwrap()),
        (
            "shared",
            bus.send_shared_request(OutgoingMessages::RequestPositions, vec![]).await.unwrap(),
        ),
        ("order update", bus.create_order_update_subscription().await.unwrap()),
    ];

    bus.request_shutdown_sync();

    for (name, mut subscription) in subscriptions {
        let first = tokio::time::timeout(TICK, subscription.next_routed()).await.expect(name);
        assert!(matches!(first, Some(RoutedItem::Error(Error::Shutdown))), "{name}: {first:?}");
        let end = tokio::time::timeout(TICK, subscription.next_routed()).await.expect(name);
        assert!(end.is_none(), "{name} did not end: {end:?}");
    }
}

/// A `place_order` or `executions` subscription that has received an
/// execution is also held in the execution-id map, so the commission report
/// that follows can reach it. Shutdown must drop that alias too: a sender left
/// there kept the channel open, and the subscription's reader waited forever
/// after `Client::drop`.
#[tokio::test]
async fn test_subscriptions_with_executions_end_on_request_shutdown_sync() {
    let (stream, bus) = make_bus();
    let mut order = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();
    let mut executions = bus.send_request(RequestId::nth(99), vec![]).await.unwrap();

    // Mapped by order id, and by request id where no order channel matches.
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 0, "exec-request"));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();
    for (name, sub) in [("order", &mut order), ("executions", &mut executions)] {
        let message = next_message(sub).await;
        assert_eq!(message.message_type(), crate::messages::IncomingMessages::ExecutionData, "{name}");
    }
    assert_eq!(bus.executions.len(), 2, "both executions mapped");

    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");
    bus.request_shutdown_sync();

    for (name, sub) in [("order", &mut order), ("executions", &mut executions)] {
        let drained = tokio::time::timeout(Duration::from_millis(500), async { while sub.next().await.is_some() {} }).await;
        assert!(drained.is_ok(), "{name} subscription with an execution did not end");
    }
    assert!(bus.executions.is_empty());
}

/// A frame with message id `-2` is `IncomingMessages::Shutdown`: the router
/// shuts the bus down and the dispatcher exits. Like `Client::drop`, that path
/// reaches `request_shutdown` with no reconnect reset before it, so it must
/// drop execution-id aliases as well.
#[tokio::test]
async fn test_shutdown_frame_ends_subscriptions_with_executions() {
    let (stream, bus) = make_bus();
    let mut order = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    bus.read_and_route_message().await.unwrap();
    let message = next_message(&mut order).await;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::ExecutionData);

    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");
    stream.push_inbound((crate::messages::IncomingMessages::Shutdown as i32).to_be_bytes().to_vec());

    let drained = tokio::time::timeout(Duration::from_millis(500), async { while order.next().await.is_some() {} }).await;
    assert!(drained.is_ok(), "order subscription with an execution did not end");
    assert!(!bus.is_connected());
    assert!(bus.executions.is_empty());
}

/// An order-update stream dropped while its cleanup signal is still queued
/// leaves a sender with no receivers behind. An order frame arriving in that
/// window fails the order-update send, and routing carries on: the frame still
/// reaches the order's own subscription.
#[tokio::test]
async fn test_order_frame_routes_while_a_dropped_order_update_stream_awaits_cleanup() {
    let (stream, bus) = make_bus();
    let mut order = bus.send_order_request(OrderId::from(22), vec![]).await.unwrap();
    let updates = bus.create_order_update_subscription().await.unwrap();

    // Stall the cleanup task, so the stream's cleanup signal cannot run
    // before the frame is routed.
    let cleanup_gate = bus.cleanup_gate.lock().await;
    drop(updates);
    assert!(
        bus.order_update_stream.lock().unwrap().is_some(),
        "the dropped stream is still registered"
    );

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OrderStatus as i32,
        &crate::proto::OrderStatus {
            order_id: Some(22),
            status: Some("Filled".into()),
            ..Default::default()
        },
    ));
    tokio::time::timeout(Duration::from_millis(500), bus.read_and_route_message())
        .await
        .expect("routing blocked")
        .unwrap();

    assert_eq!(next_message(&mut order).await.order_id(), Some(22));
    drop(cleanup_gate);
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
    let request_id = RequestId::nth(42);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    stream.push_inbound(error_frame(request_id.raw(), 2104, FARM_OK_MSG));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm connection is OK:usfarm");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: a follow-up data message is delivered.
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.read_and_route_message().await.unwrap();
    let item = next_routed(&mut sub).await;
    assert!(matches!(item, RoutedItem::Response(_)), "got: {item:?}");
}

/// Data advisory (code 10167) bound to a real request_id is informational:
/// TWS proceeds with delayed data, so it is delivered as a `RoutedItem::Notice`
/// and the stream stays open for the follow-up data — not routed as an error
/// that would terminate the subscription.
#[tokio::test]
async fn test_data_advisory_with_request_id_keeps_stream_open() {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    let code = 10167; // data advisory: "Displaying delayed market data."
    stream.push_inbound(error_frame(request_id.raw(), code, "Displaying delayed market data."));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, code);
            assert!(notice.is_data_advisory());
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: the delayed data the advisory promised arrives.
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.read_and_route_message().await.unwrap();
    let item = next_routed(&mut sub).await;
    assert!(matches!(item, RoutedItem::Response(_)), "got: {item:?}");
}

/// Hard error (code 200) bound to a real request_id is delivered as a
/// `RoutedItem::Error` to the owning subscription.
#[tokio::test]
async fn test_hard_error_with_request_id_terminates_subscription() {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    match item {
        RoutedItem::Error(Error::Notice(notice)) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
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
    let mut sub = bus.send_request(RequestId::nth(42), vec![]).await.unwrap();

    stream.push_inbound(error_frame(-1, 2104, FARM_OK_MSG));
    bus.read_and_route_message().await.unwrap();

    assert!(sub.try_next_routed().is_none(), "unrouted notice must not be delivered to a subscription");
}

/// Request-less hard error (id = -1) is uncorrelatable, so it fails every
/// in-flight *one-shot* shared request fast (`RequestIds` here) while leaving
/// *streaming* shared requests (`RequestPositions`) untouched — and still fans
/// out to the global notice stream. Regression for #694 (callers hung forever).
#[tokio::test]
async fn test_request_less_hard_error_fails_one_shot_and_spares_stream() {
    let (stream, bus) = make_bus();
    let mut notice_stream = bus.notice_subscribe();
    let mut one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, vec![]).await.unwrap();
    let mut streaming = bus.send_shared_request(OutgoingMessages::RequestPositions, vec![]).await.unwrap();

    // 321 "read-only mode" is the live-reproduced case; non-warning, id = -1.
    stream.push_inbound(error_frame(-1, 321, READ_ONLY_MSG));
    bus.read_and_route_message().await.unwrap();

    // One-shot caller fails fast with the real error instead of hanging. Read via
    // the legacy `next()` projection — the same path `next_valid_order_id` and the
    // `one_shot_shared` helper consume — so a `Some(Err(..))` surfaces to callers.
    let item = tokio::time::timeout(TICK, one_shot.next())
        .await
        .expect("one-shot got no error before timeout")
        .expect("subscription closed");
    match item {
        Err(Error::Notice(notice)) => {
            assert_eq!(notice.code, 321);
            assert_eq!(notice.message, READ_ONLY_MSG);
        }
        other => panic!("expected Err(Notice), got {other:?}"),
    }
    // Streaming shared subscription is not terminated by the unrelated error.
    assert!(
        streaming.try_next_routed().is_none(),
        "streaming shared sub must not receive the request-less error"
    );
    // Global notice stream still observes it.
    let notice = tokio::time::timeout(TICK, notice_stream.next()).await.unwrap().unwrap();
    assert_eq!(notice.request_id, None);
    assert_eq!(notice.code, 321);
}

/// A request-less *warning* stays notice-only: it must not fail an in-flight
/// one-shot shared request (only non-warning hard errors trip fail-fast).
#[tokio::test]
async fn test_request_less_warning_does_not_fail_one_shot() {
    let (stream, bus) = make_bus();
    let mut one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, vec![]).await.unwrap();

    stream.push_inbound(error_frame(-1, 2104, FARM_OK_MSG));
    bus.read_and_route_message().await.unwrap();

    assert!(one_shot.try_next_routed().is_none(), "warning must not fail a one-shot shared request");
}

/// A request-less *system message* (1102, connectivity restored with data
/// maintained) reports a connection-wide state change, not a failed request.
/// It must reach the notice stream without failing in-flight one-shot shared
/// requests - `managed_accounts`, `server_time`, `next_valid_order_id`.
#[tokio::test]
async fn test_request_less_system_message_does_not_fail_one_shot() {
    let (stream, bus) = make_bus();
    let mut notice_stream = bus.notice_subscribe();
    let mut one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, vec![]).await.unwrap();

    let code = crate::messages::CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE;
    stream.push_inbound(error_frame(-1, code, CONNECTIVITY_RESTORED_MSG));
    bus.read_and_route_message().await.unwrap();

    assert!(
        one_shot.try_next_routed().is_none(),
        "system message must not fail a one-shot shared request"
    );

    let notice = tokio::time::timeout(TICK, notice_stream.next()).await.unwrap().unwrap();
    assert_eq!(notice.code, code);
    assert!(notice.is_system_message());
}

/// A notice bound to an id below the request floor is an order's: it goes to
/// the order subscription for that id.
#[tokio::test]
async fn test_warning_with_order_id_routes_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();

    stream.push_inbound(error_frame(7, 2104, "Order warning"));
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

const CONNECTIVITY_RESTORED_MSG: &str = "Connectivity between IB and TWS has been restored - data maintained.";
const READ_ONLY_MSG: &str = "The API interface is currently in Read-Only mode.";

async fn make_request_subscription(request_id: RequestId) -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>, Subscription<NoticeTestData>) {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(request_id, vec![]).await.unwrap();
    let sub = Subscription::new_from_internal(internal, bus.clone(), Some(request_id.raw()), None, DecoderContext::default());
    (stream, bus, sub)
}

async fn make_order_subscription(order_id: OrderId) -> (MemoryStream, Arc<AsyncTcpMessageBus<MemoryStream>>, Subscription<NoticeTestData>) {
    let (stream, bus) = make_bus();
    let internal = bus.send_order_request(order_id, vec![]).await.unwrap();
    let sub = Subscription::new_from_internal(internal, bus.clone(), None, Some(order_id.value()), DecoderContext::default());
    (stream, bus, sub)
}

/// Number of length-prefixed frames in `captured` whose payload is `payload`.
fn count_frames(captured: &[u8], payload: &[u8]) -> usize {
    let mut rest = captured;
    let mut count = 0;
    while rest.len() >= 4 {
        let len = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let (frame, tail) = rest[4..].split_at(len);
        count += usize::from(frame == payload);
        rest = tail;
    }
    count
}

/// Wait until `captured` holds `expected` frames equal to `payload`, or the
/// deadline passes. Async `Drop` spawns the cancel send, so a count can only
/// be asserted after that task has had a chance to run.
async fn wait_for_frames(stream: &MemoryStream, payload: &[u8], expected: usize) -> usize {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let count = count_frames(&stream.captured(), payload);
        if count >= expected || std::time::Instant::now() >= deadline {
            return count;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// TWS keeps one positions stream per client (`CancelPositions` carries no id),
/// so with two live `RequestPositions` subscriptions the first drop must not
/// write the cancel, the survivor must keep receiving, and the last drop writes
/// exactly one cancel. Subscriptions are built through the production
/// `SubscriptionBuilder::send_shared`.
#[tokio::test]
async fn test_shared_subscription_cancel_waits_for_last_subscriber() {
    use crate::accounts::PositionUpdate;
    use crate::client::builders::r#async::SubscriptionBuilder;

    let (stream, bus) = make_bus();
    let cancel = <PositionUpdate as StreamDecoder<PositionUpdate>>::cancel_message(0, None, None).unwrap();

    let make = || async {
        let message_bus: Arc<dyn AsyncMessageBus> = bus.clone();
        SubscriptionBuilder::<PositionUpdate>::new_with_components(DecoderContext::default(), message_bus)
            .send_shared(OutgoingMessages::RequestPositions, b"positions".to_vec())
            .await
            .unwrap()
    };
    let first = make().await;
    let mut second = make().await;

    drop(first);
    // Give a wrongly spawned cancel time to land before asserting its absence.
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        count_frames(&stream.captured(), &cancel),
        0,
        "cancel written while a subscription is still live"
    );

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.read_and_route_message().await.unwrap();
    let item = next_item(&mut second).await.expect("survivor received nothing").unwrap();
    assert!(matches!(item, SubscriptionItem::Data(PositionUpdate::PositionEnd)), "got: {item:?}");

    drop(second);
    assert_eq!(wait_for_frames(&stream, &cancel, 1).await, 1, "last drop must write exactly one cancel");
}

/// `Orders` has no cancel message, so its drop writes nothing; the count for
/// its request type must still return to zero, or a later cancel for the
/// type would be withheld. The release runs in the task `Drop` spawns.
///
/// Built the way `open_orders()` builds it: `send_shared_request` then
/// `new_from_internal_simple`, with nothing naming the request type. The
/// drop can only reach the count because the subscription derives the type
/// from the internal subscription's cleanup signal.
#[tokio::test]
async fn test_shared_subscription_without_cancel_message_releases_count() {
    use crate::orders::Orders;

    let (stream, bus) = make_bus();
    let count = || async { bus.shared_channels.live(OutgoingMessages::RequestOpenOrders).await };

    let internal = bus
        .send_shared_request(OutgoingMessages::RequestOpenOrders, b"open-orders".to_vec())
        .await
        .unwrap();
    let sub = Subscription::<Orders>::new_from_internal_simple(internal, bus.clone(), DecoderContext::default());
    assert_eq!(count().await, 1);

    drop(sub);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while count().await != 0 && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(count().await, 0, "count leaked for a type with no cancel message");
    assert_eq!(count_frames(&stream.captured(), b"open-orders"), 1, "only the request was written");
}

/// A shared subscription that reads its end marker still releases its count on
/// drop: the cancel-after-end guard applies only to request-id subscriptions.
#[tokio::test]
async fn test_shared_subscription_ended_natively_still_releases_count() {
    use crate::orders::Orders;

    let (stream, bus) = make_bus();
    let count = || async { bus.shared_channels.live(OutgoingMessages::RequestOpenOrders).await };

    let internal = bus
        .send_shared_request(OutgoingMessages::RequestOpenOrders, b"open-orders".to_vec())
        .await
        .unwrap();
    let mut sub = Subscription::<Orders>::new_from_internal_simple(internal, bus.clone(), DecoderContext::default());

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrderEnd as i32,
        &crate::proto::OpenOrdersEnd {},
    ));
    bus.read_and_route_message().await.unwrap();
    assert!(next_item(&mut sub).await.is_none(), "OpenOrderEnd ends the stream");
    assert!(sub.ended_natively());

    drop(sub);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while count().await != 0 && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(count().await, 0, "count leaked after a natural end");
}

type PositionsSubscription = Subscription<crate::accounts::PositionUpdate>;

async fn positions_subscription(bus: &Arc<AsyncTcpMessageBus<MemoryStream>>) -> PositionsSubscription {
    let internal = bus
        .send_shared_request(OutgoingMessages::RequestPositions, b"positions".to_vec())
        .await
        .unwrap();
    Subscription::new_from_internal(internal, bus.clone(), None, None, DecoderContext::default())
}

fn positions_cancel() -> Vec<u8> {
    <crate::accounts::PositionUpdate as StreamDecoder<crate::accounts::PositionUpdate>>::cancel_message(0, None, None).unwrap()
}

async fn positions_live(bus: &Arc<AsyncTcpMessageBus<MemoryStream>>) -> usize {
    bus.shared_channels.live(OutgoingMessages::RequestPositions).await
}

/// Wait until the count for `RequestPositions` reaches `expected`, or the
/// deadline passes; the release runs in the task `Drop` spawns.
async fn wait_for_positions_live(bus: &Arc<AsyncTcpMessageBus<MemoryStream>>, expected: usize) -> usize {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let live = positions_live(bus).await;
        if live == expected || std::time::Instant::now() >= deadline {
            return live;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// A shared subscription dropped on a thread with no runtime still writes its
/// cancel and releases its count, so the next subscription of the type can
/// cancel too (#848). Before, the count leaked and every later drop of the
/// type withheld its cancel until the next reconnect.
#[tokio::test]
async fn test_shared_subscription_dropped_outside_runtime_cancels() {
    let (stream, bus) = make_bus();
    let cancel = positions_cancel();

    let sub = positions_subscription(&bus).await;
    std::thread::spawn(move || drop(sub)).join().expect("drop outside a runtime panicked");
    assert_eq!(wait_for_frames(&stream, &cancel, 1).await, 1, "off-runtime drop must write the cancel");

    drop(positions_subscription(&bus).await);
    assert_eq!(
        wait_for_frames(&stream, &cancel, 2).await,
        2,
        "count leaked: next drop withheld its cancel"
    );
}

/// A reset ends every shared subscription, so the count restarts at zero for
/// the new session: a resubscription made after the reset is the only live one
/// and its drop writes the cancel even while the dead handle is still held.
#[tokio::test]
async fn test_shared_count_restarts_after_reset() {
    let (stream, bus) = make_bus();
    let cancel = positions_cancel();

    let old = positions_subscription(&bus).await;
    bus.reset_channels().await;
    assert_eq!(positions_live(&bus).await, 0);

    let new = positions_subscription(&bus).await;
    drop(new);
    assert_eq!(wait_for_frames(&stream, &cancel, 1).await, 1, "cancel withheld by a dead handle");

    drop(old);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(count_frames(&stream.captured(), &cancel), 1, "dead handle wrote a cancel");
}

/// A handle from before the reset is dead: its drop must neither cancel on the
/// new session, which has no such stream, nor touch the new session's count.
#[tokio::test]
async fn test_stale_shared_handle_neither_cancels_nor_decrements() {
    let (stream, bus) = make_bus();
    let cancel = positions_cancel();

    let old = positions_subscription(&bus).await;
    bus.reset_channels().await;
    let new = positions_subscription(&bus).await;

    drop(old);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(count_frames(&stream.captured(), &cancel), 0, "dead handle wrote a cancel");
    assert_eq!(positions_live(&bus).await, 1);

    drop(new);
    assert_eq!(wait_for_frames(&stream, &cancel, 1).await, 1);
    assert_eq!(wait_for_positions_live(&bus, 0).await, 0);
}

/// The open-orders channel maps three response types. A reset that walked the
/// channels by response type failed it three times; it must fail it once.
#[tokio::test]
async fn test_reset_fails_a_shared_channel_once() {
    let (_stream, bus) = make_bus();
    let mut open_orders = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, vec![]).await.unwrap();

    bus.reset_channels().await;

    let items: Vec<_> = std::iter::from_fn(|| open_orders.try_next_routed()).collect();
    assert!(matches!(items.as_slice(), [RoutedItem::Error(Error::ConnectionReset)]), "{items:?}");
}

type AccountUpdatesSubscription = Subscription<crate::accounts::AccountUpdate>;

fn account(name: &str) -> crate::accounts::types::AccountId {
    crate::accounts::types::AccountId(name.to_string())
}

// The request bytes name the account so each account's writes can be counted.
fn account_updates_request(name: &str) -> Vec<u8> {
    format!("account-updates {name}").into_bytes()
}

async fn account_updates_subscription(bus: &Arc<AsyncTcpMessageBus<MemoryStream>>, name: &str) -> Result<AccountUpdatesSubscription, Error> {
    let internal = bus.send_account_updates_request(&account(name), account_updates_request(name)).await?;
    Ok(Subscription::new_from_internal(
        internal,
        bus.clone(),
        None,
        None,
        DecoderContext::default(),
    ))
}

fn account_updates_cancel() -> Vec<u8> {
    <crate::accounts::AccountUpdate as StreamDecoder<crate::accounts::AccountUpdate>>::cancel_message(0, None, None).unwrap()
}

async fn account_updates_slot(bus: &AsyncTcpMessageBus<MemoryStream>) -> Option<crate::accounts::types::AccountId> {
    bus.shared_channels.account_updates().await
}

/// The same account again shares the one TWS stream: both requests go out,
/// and only the last cancel frees the slot.
#[tokio::test]
async fn test_account_updates_same_account_shares() {
    let (stream, bus) = make_bus();
    let cancel = account_updates_cancel();

    let first = account_updates_subscription(&bus, "DU1").await.unwrap();
    let second = account_updates_subscription(&bus, "DU1").await.unwrap();
    assert_eq!(count_frames(&stream.captured(), &account_updates_request("DU1")), 2);
    assert_eq!(bus.shared_channels.live(OutgoingMessages::RequestAccountData).await, 2);

    first.cancel().await;
    assert_eq!(
        count_frames(&stream.captured(), &cancel),
        0,
        "cancel written while a subscription is still live"
    );
    assert_eq!(account_updates_slot(&bus).await, Some(account("DU1")));

    second.cancel().await;
    assert_eq!(count_frames(&stream.captured(), &cancel), 1);
    assert_eq!(account_updates_slot(&bus).await, None);
}

/// TWS has one account-updates slot: a second account would switch the live
/// subscription to its data. It is refused before anything is written, and
/// the live subscription keeps its data.
#[tokio::test]
async fn test_account_updates_other_account_refused() {
    use crate::accounts::AccountUpdate;

    let (stream, bus) = make_bus();
    let mut first = account_updates_subscription(&bus, "DU1").await.unwrap();

    let refused = account_updates_subscription(&bus, "DU2").await.map(|_| ());
    assert!(
        matches!(&refused, Err(Error::AccountUpdatesInUse { active, requested }) if *active == account("DU1") && *requested == account("DU2")),
        "got: {refused:?}"
    );
    assert_eq!(
        count_frames(&stream.captured(), &account_updates_request("DU2")),
        0,
        "refused request was written"
    );
    assert_eq!(bus.shared_channels.live(OutgoingMessages::RequestAccountData).await, 1);

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::AccountDownloadEnd as i32,
        &crate::proto::AccountDataEnd {
            account_name: Some("DU1".to_string()),
        },
    ));
    bus.read_and_route_message().await.unwrap();
    let item = next_item(&mut first).await.expect("live subscription received nothing").unwrap();
    assert!(matches!(item, SubscriptionItem::Data(AccountUpdate::End)), "got: {item:?}");
}

/// Once the last subscription of an account is cancelled, another account is
/// accepted. `cancel().await` releases before it returns; a plain drop
/// releases in a spawned task, so it is not used to switch.
#[tokio::test]
async fn test_account_updates_other_account_after_cancel() {
    let (stream, bus) = make_bus();

    let first = account_updates_subscription(&bus, "DU1").await.unwrap();
    first.cancel().await;
    assert_eq!(count_frames(&stream.captured(), &account_updates_cancel()), 1);

    let _second = account_updates_subscription(&bus, "DU2").await.unwrap();
    assert_eq!(count_frames(&stream.captured(), &account_updates_request("DU2")), 1);
    assert_eq!(account_updates_slot(&bus).await, Some(account("DU2")));
}

/// A reset ends every subscription, so the slot is free on the new session,
/// and a dead handle's drop must not free the new account's slot.
#[tokio::test]
async fn test_account_updates_slot_restarts_after_reset() {
    let (stream, bus) = make_bus();

    let old = account_updates_subscription(&bus, "DU1").await.unwrap();
    bus.reset_channels().await;
    let _new = account_updates_subscription(&bus, "DU2").await.unwrap();

    drop(old);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        count_frames(&stream.captured(), &account_updates_cancel()),
        0,
        "dead handle wrote a cancel"
    );
    assert_eq!(account_updates_slot(&bus).await, Some(account("DU2")));
}

/// Bound a `Subscription::next()` await with the test tick so a missing item
/// surfaces as a panic rather than hanging the test thread.
async fn next_item<T: StreamDecoder<T> + Send + 'static>(sub: &mut Subscription<T>) -> Option<Result<SubscriptionItem<T>, Error>> {
    tokio::time::timeout(TICK, sub.next())
        .await
        .expect("subscription got no item before timeout")
}

/// Code 2104 on a request id surfaces as `SubscriptionItem::Notice` without
/// terminating; a follow-up data message arrives normally on the same stream.
#[tokio::test]
async fn test_subscription_notice_delivery_request_keyed() {
    let (stream, bus, mut subscription) = make_request_subscription(RequestId::nth(42)).await;

    stream.push_inbound(farm_ok_frame_42());
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, FARM_OK_MSG);
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }

    stream.push_inbound(body(&format!("89|{}|payload|", RequestId::nth(42))));
    bus.read_and_route_message().await.unwrap();
    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Data(_))) => {}
        other => panic!("expected SubscriptionItem::Data, got {other:?}"),
    }
}

/// Partial entitlement can precede delayed Greeks on the same request.
#[tokio::test]
async fn test_subscription_10091_preserves_later_option_computation() {
    use crate::contracts::tick_types::TickType;
    use crate::market_data::realtime::TickTypes;
    use crate::testdata::builders::{market_data::tick_option_computation, ResponseProtoEncoder};

    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let internal = bus.send_request(request_id, vec![]).await.unwrap();
    let mut subscription = Subscription::new_from_internal(internal, bus.clone(), Some(request_id.raw()), None, DecoderContext::default());
    let computation = tick_option_computation()
        .request_id(request_id.raw())
        .tick_type(TickType::DelayedModelOption as i32)
        .tick_attrib(0)
        .delta(0.5)
        .to_proto();

    // Both frames are dispatched before polling: the error must not hide
    // an already-queued computation on the same request.
    stream.push_inbound(error_frame(request_id.raw(), 10091, "Synthetic partial-entitlement advisory"));
    stream.push_inbound(binary_proto(IncomingMessages::TickOptionComputation as i32, &computation));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
            assert_eq!(notice.code, 10091);
            assert_eq!(notice.message, "Synthetic partial-entitlement advisory");
            assert!(notice.is_data_advisory());
        }
        other => panic!("expected nonterminal 10091 notice, got {other:?}"),
    }
    match next_item(&mut subscription).await {
        Some(Ok(SubscriptionItem::Data(TickTypes::OptionComputation(greeks)))) => {
            assert_eq!(greeks.field, TickType::DelayedModelOption);
            assert_eq!(greeks.tick_attribute, Some(0));
            assert_eq!(greeks.delta, Some(0.5));
            assert_eq!(greeks.implied_volatility, None);
        }
        other => panic!("option computation after 10091 lost: {other:?}"),
    }
}

/// A depth-book reset (317) precedes the rows that rebuild it on the same
/// request. It must not end the depth stream (#806), and arrives as
/// `MarketDepths::Reset` so `filter_data()` keeps it (#899).
#[tokio::test]
async fn test_subscription_317_yields_reset_then_rows() {
    use crate::market_data::realtime::MarketDepths;
    use crate::testdata::builders::{market_data::market_depth_response, ResponseProtoEncoder};

    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let internal = bus.send_request(request_id, vec![]).await.unwrap();
    let subscription: Subscription<MarketDepths> =
        Subscription::new_from_internal(internal, bus.clone(), Some(request_id.raw()), None, DecoderContext::default());
    let row = market_depth_response()
        .request_id(request_id.raw())
        .position(0)
        .operation(0)
        .side(1)
        .price(101.5)
        .size(3.0)
        .to_proto();

    // Both frames are dispatched before polling: the reset must not hide the
    // first row of the rebuilt book.
    stream.push_inbound(error_frame(
        request_id.raw(),
        317,
        "Market depth data has been RESET. Please empty deep book contents before applying any new entries.",
    ));
    stream.push_inbound(binary_proto(IncomingMessages::MarketDepth as i32, &row));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let mut depths = subscription.filter_data();
    match depths.next().await {
        Some(Ok(MarketDepths::Reset)) => {}
        other => panic!("expected MarketDepths::Reset, got {other:?}"),
    }
    match depths.next().await {
        Some(Ok(MarketDepths::MarketDepth(depth))) => {
            assert_eq!(depth.position, 0);
            assert_eq!(depth.operation, 0);
            assert_eq!(depth.side, 1);
            assert_eq!(depth.price, 101.5);
            assert_eq!(depth.size, 3.0);
        }
        other => panic!("market depth row after 317 lost: {other:?}"),
    }
}

/// Hard error (code 200) surfaces as `Some(Err(_))`; subsequent reads return `None`.
#[tokio::test]
async fn test_subscription_hard_error_terminates_stream() {
    let (stream, bus, mut subscription) = make_request_subscription(RequestId::nth(42)).await;

    let request_id = RequestId::nth(42);
    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    match next_item(&mut subscription).await {
        Some(Err(Error::Notice(notice))) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected Some(Err(Error::Notice)), got {other:?}"),
    }

    assert!(next_item(&mut subscription).await.is_none(), "terminal error must hide even queued data");
}

/// Order-keyed notice: an id below the request floor reaches the order subscription.
#[tokio::test]
async fn test_subscription_notice_delivery_order_keyed() {
    let (stream, bus, mut subscription) = make_order_subscription(OrderId::from(7)).await;

    stream.push_inbound(error_frame(7, 2109, "Outside RTH order warning"));
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
    let (stream, bus, mut subscription) = make_request_subscription(RequestId::nth(42)).await;

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.read_and_route_message().await.unwrap();

    let item = tokio::time::timeout(TICK, subscription.next()).await;
    assert!(item.is_err(), "unrouted notice must not be delivered to a subscription, got {item:?}");
}

/// `data_stream()` filters `SubscriptionItem::Notice` and yields only data.
#[tokio::test]
async fn test_subscription_data_stream_filters_notices() {
    let (stream, bus, subscription) = make_request_subscription(RequestId::nth(42)).await;

    let request_id = RequestId::nth(42);
    stream.push_inbound(body(&format!("89|{request_id}|first|")));
    stream.push_inbound(farm_ok_frame_42());
    stream.push_inbound(body(&format!("89|{request_id}|second|")));
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

    stream.push_inbound(error_frame(-1, 504, "Not connected"));
    bus.read_and_route_message().await.unwrap();

    let notice = tokio::time::timeout(TICK, notice_stream.next()).await.unwrap().unwrap();
    assert_eq!(notice.code, 504);
}

/// A routed notice (real `request_id`) goes to the owning subscription, NOT
/// to the global notice stream.
#[tokio::test]
async fn test_notice_stream_skips_routed_notices() {
    let (stream, bus, mut subscription) = make_request_subscription(RequestId::nth(42)).await;
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

#[tokio::test]
async fn test_execution_data_routes_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.order_id(), Some(7));
}

#[tokio::test]
async fn test_execution_data_falls_back_to_request_channel() {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(99);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    stream.push_inbound(execution_data_frame(request_id.raw(), 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    let msg = next_message(&mut sub).await;
    assert_eq!(msg.request_id(), Some(request_id.raw()));
}

#[tokio::test]
async fn test_execution_data_orphan_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(RequestId::nth(42), vec![]).await.unwrap();

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-1"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an orphan message");
}

#[tokio::test]
async fn test_execution_data_end_routes_to_order_channel() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd { req_id: Some(7) },
    ));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut sub).await;
}

/// ExecutionDataEnd's `req_id` doubles as the order_id key for the router; a
/// request-range id misses the order channel and falls back to the request
/// channel.
#[tokio::test]
async fn test_execution_data_end_falls_back_to_request_channel() {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(7);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd {
            req_id: Some(request_id.raw()),
        },
    ));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut sub).await;
}

#[tokio::test]
async fn test_execution_data_end_orphan_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(RequestId::nth(42), vec![]).await.unwrap();

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd {
            req_id: Some(RequestId::nth(999).raw()),
        },
    ));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an orphan end");
}

/// `ByExecutionId`: the prior ExecutionData stores `exec-abc → order_id 7`'s
/// sender, and the CommissionsReport rides that mapping back to the same sub.
#[tokio::test]
async fn test_commission_report_routes_via_execution_id_mapping() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-abc"));
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::CommissionsReport as i32,
        &crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec-abc".into()),
            ..Default::default()
        },
    ));

    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    let exec_msg = next_message(&mut sub).await;
    assert_eq!(exec_msg.message_type(), crate::messages::IncomingMessages::ExecutionData);
    let commission = next_message(&mut sub).await;
    assert_eq!(commission.message_type(), crate::messages::IncomingMessages::CommissionsReport);
}

#[tokio::test]
async fn test_commission_report_without_mapping_dropped() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::CommissionsReport as i32,
        &crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec-not-mapped".into()),
            ..Default::default()
        },
    ));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got an unmapped commission");
}

/// #880: an execution-id alias holds a sender clone, so it must go when the
/// order or request subscription that owns it is dropped.
#[tokio::test]
async fn test_execution_aliases_pruned_when_subscriptions_drop() {
    let (stream, bus) = make_bus();
    let order = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();
    let executions = bus.send_request(RequestId::nth(99), vec![]).await.unwrap();
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 0, "exec-request"));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();
    assert_eq!(bus.executions.len(), 2, "both executions mapped");

    drop(order);
    drain_cleanup_signals(&bus).await;
    assert!(!bus.executions.contains(&"exec-order".to_string()), "order alias leaked");
    assert!(bus.executions.contains(&"exec-request".to_string()), "live request alias pruned");

    drop(executions);
    drain_cleanup_signals(&bus).await;
    assert!(bus.executions.is_empty(), "request alias leaked");
}

/// Each clone sends its own cleanup signal; the alias stays while any clone
/// can still read the channel and goes with the last one.
#[tokio::test]
async fn test_execution_alias_kept_while_a_clone_is_alive() {
    let (stream, bus) = make_bus();
    let order = bus.send_order_request(OrderId::from(7), vec![]).await.unwrap();
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    bus.read_and_route_message().await.unwrap();

    let clone = order.clone();
    drop(order);
    drain_cleanup_signals(&bus).await;
    assert!(bus.executions.contains(&"exec-order".to_string()), "alias pruned while a clone is alive");

    drop(clone);
    drain_cleanup_signals(&bus).await;
    assert!(bus.executions.is_empty(), "alias leaked");
}

/// A stale drop signal releases the old subscription's aliases and keeps those
/// of a newer registration under the same order id.
#[tokio::test]
async fn test_stale_cleanup_keeps_newer_execution_aliases() {
    let (stream, bus) = make_bus();
    let sub_a = bus.send_order_request(OrderId::from(42), vec![]).await.unwrap();
    stream.push_inbound(execution_data_frame(0, 42, "exec-a"));
    bus.read_and_route_message().await.unwrap();
    let sub_b = bus.send_order_request(OrderId::from(42), vec![]).await.unwrap();
    stream.push_inbound(execution_data_frame(0, 42, "exec-b"));
    bus.read_and_route_message().await.unwrap();

    drop(sub_a);
    drain_cleanup_signals(&bus).await;
    assert!(!bus.executions.contains(&"exec-a".to_string()), "stale subscription's alias leaked");
    assert!(bus.executions.contains(&"exec-b".to_string()), "newer subscription's alias pruned");

    drop(sub_b);
    drain_cleanup_signals(&bus).await;
    assert!(bus.executions.is_empty());
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
    let mut order_sub = bus.send_order_request(OrderId::from(42), vec![]).await.unwrap();
    let mut stream_sub = bus.create_order_update_subscription().await.unwrap();

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.read_and_route_message().await.unwrap();

    next_message(&mut order_sub).await;
    next_message(&mut stream_sub).await;
}

/// A targeted hard error reaches the order-update stream as a notice so the
/// stream can continue with later order updates.
#[tokio::test]
async fn test_order_update_stream_receives_order_error_as_notice() {
    let (stream, bus) = make_bus();
    let mut stream_sub = bus.create_order_update_subscription().await.unwrap();

    stream.push_inbound(error_frame(42, 201, "Order rejected"));
    bus.read_and_route_message().await.unwrap();

    match next_routed(&mut stream_sub).await {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.request_id, Some(42));
            assert_eq!(notice.code, 201);
            assert_eq!(notice.message, "Order rejected");
        }
        other => panic!("expected RoutedItem::Notice, received {other:?}"),
    }

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.read_and_route_message().await.unwrap();

    assert!(matches!(next_routed(&mut stream_sub).await, RoutedItem::Response(_)));
}

/// An error with a request-range id stays on that request's subscription:
/// the order-update stream must not receive a copy.
#[tokio::test]
async fn test_order_update_stream_skips_data_request_error() {
    let (stream, bus) = make_bus();
    let mut stream_sub = bus.create_order_update_subscription().await.unwrap();
    let request_id = RequestId::nth(42);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.read_and_route_message().await.unwrap();

    let item = next_routed(&mut sub).await;
    assert!(matches!(item, RoutedItem::Error(Error::Notice(_))), "got: {item:?}");
    assert!(
        stream_sub.try_next_routed().is_none(),
        "order-update stream must not receive a data-request error"
    );
}

/// #789: an order-range error id reaches the order subscription and the
/// order-update stream, never a request. A range-routing guard: the request
/// below sits at the same small number plus the floor, so the ids cannot
/// collide; the collision itself is what the floor rules out.
#[tokio::test]
async fn test_issue_789_order_error_reaches_order_side() {
    let (stream, bus) = make_bus();
    let order_id = OrderId::from(7);
    let mut request = bus.send_request(RequestId::nth(order_id.value()), vec![]).await.unwrap();
    let mut order = bus.send_order_request(order_id, vec![]).await.unwrap();
    let mut updates = bus.create_order_update_subscription().await.unwrap();

    stream.push_inbound(error_frame(order_id.value(), 202, "Order Canceled"));
    stream.push_inbound(error_frame(order_id.value(), 201, "Order rejected"));
    bus.read_and_route_message().await.unwrap();
    bus.read_and_route_message().await.unwrap();

    match next_routed(&mut order).await {
        RoutedItem::Notice(notice) => assert_eq!(notice.code, 202),
        other => panic!("expected the 202 notice, got {other:?}"),
    }
    match next_routed(&mut order).await {
        RoutedItem::Error(Error::Notice(notice)) => assert_eq!(notice.code, 201),
        other => panic!("expected the 201 error, got {other:?}"),
    }
    for code in [202, 201] {
        match next_routed(&mut updates).await {
            RoutedItem::Notice(notice) => {
                assert_eq!(notice.request_id, Some(order_id.value()));
                assert_eq!(notice.code, code);
            }
            other => panic!("expected the {code} notice on the order-update stream, got {other:?}"),
        }
    }
    assert!(request.try_next_routed().is_none(), "request received an order's error");
}

/// #789: an error for a request that is gone is not published as order-bound.
#[tokio::test]
async fn test_issue_789_late_request_error_stays_off_order_stream() {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(5);
    drop(bus.send_request(request_id, vec![]).await.unwrap());
    drain_cleanup_signals(&bus).await;
    let mut updates = bus.create_order_update_subscription().await.unwrap();

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.read_and_route_message().await.unwrap();

    assert!(
        try_next_routed(&mut updates).await.is_none(),
        "late request error reached the order-update stream"
    );
}

/// Routed-but-orphan notice (real request_id, no matching sub) takes the
/// `log_orphan` path, NOT the global notice stream.
#[tokio::test]
async fn test_warning_with_orphan_request_id_logs() {
    let (stream, bus) = make_bus();
    let mut unrelated = bus.send_request(RequestId::nth(42), vec![]).await.unwrap();
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(error_frame(RequestId::nth(99).raw(), 2104, "orphan warning"));
    bus.read_and_route_message().await.unwrap();

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got the notice");
    let leaked = tokio::time::timeout(TICK, notice_stream.next()).await;
    assert!(leaked.is_err(), "global notice stream got a routed-but-orphan notice");
}

/// Queue a marker cleanup signal behind everything already queued and wait
/// until the cleanup task has processed it. Signals are processed FIFO by a
/// single task, so once the marker's registration is gone, every signal sent
/// before it has been handled too.
pub(super) async fn drain_cleanup_signals<S: AsyncStream>(bus: &Arc<AsyncTcpMessageBus<S>>) {
    const MARKER_REQUEST_ID: RequestId = RequestId::nth(987_654);
    let marker = bus.send_request(MARKER_REQUEST_ID, vec![]).await.unwrap();
    drop(marker);

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if !bus.requests.contains(&MARKER_REQUEST_ID) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("cleanup task did not process the marker signal");
}

/// Regression test for #773: dropping an old order subscription must not
/// unregister a newer subscription under the same order id (place then cancel
/// on one id). The stale signal has to find the replacement live and skip it.
#[tokio::test]
async fn test_stale_order_cleanup_preserves_newer_subscription() {
    let (_, bus) = make_bus();

    let order_id = OrderId::from(42);
    let sub_a = bus.send_order_request(order_id, vec![]).await.unwrap();
    let mut sub_b = bus.send_order_request(order_id, vec![]).await.unwrap();
    drop(sub_a);

    drain_cleanup_signals(&bus).await;

    let sender = bus.orders.sender(&order_id).expect("stale cleanup removed the newer subscription");
    sender
        .send(RoutedItem::Error(Error::Cancelled))
        .expect("registered channel has no receivers");
    let item = tokio::time::timeout(TICK, sub_b.next_routed()).await.expect("sub_b got nothing");
    assert!(matches!(item, Some(RoutedItem::Error(Error::Cancelled))), "{item:?}");
    drop(sender);

    // The replacement's own drop still cleans up.
    drop(sub_b);
    drain_cleanup_signals(&bus).await;
    assert!(!bus.orders.contains(&order_id), "order channel leaked");
}

/// Dropping a clone must not unregister the channel while a sibling is still
/// consuming it; the registration goes away with the last holder.
#[tokio::test]
async fn test_dropping_clone_keeps_order_channel_registered() {
    let (_, bus) = make_bus();

    let order_id = OrderId::from(7);
    let sub = bus.send_order_request(order_id, vec![]).await.unwrap();
    let clone = sub.clone();
    drop(clone);

    drain_cleanup_signals(&bus).await;
    assert!(bus.orders.contains(&order_id), "clone drop unregistered a live subscription");

    drop(sub);
    drain_cleanup_signals(&bus).await;
    assert!(!bus.orders.contains(&order_id), "order channel leaked");
}

/// The lease is released before the cleanup signal is sent, not by the field
/// drops after it: otherwise a concurrently processed signal could find the
/// registration still live and skip the removal, leaking it.
#[tokio::test]
async fn test_cleanup_signal_follows_lease_release() {
    let (_, bus) = make_bus();
    let request_id = RequestId::nth(5);
    let mut sub = bus.send_request(request_id, vec![]).await.unwrap();
    let is_live = || bus.requests.with_route(&request_id, |route| route.lease.is_live());
    assert_eq!(is_live(), Some(true));

    sub.send_cleanup_signal();
    assert_eq!(is_live(), Some(false), "signal sent while the handle still held the lease");

    drain_cleanup_signals(&bus).await;
    assert!(!bus.requests.contains(&request_id), "request channel leaked");
    drop(sub);
}

/// Regression test for #778: drop then immediately recreate the order update
/// stream. The dead registration is replaced without waiting for the cleanup
/// task, and the old stream's stale signal must not clear the replacement.
#[tokio::test]
async fn test_drop_then_recreate_order_update_stream() {
    let (_, bus) = make_bus();

    let s1 = bus.create_order_update_subscription().await.unwrap();
    drop(s1);

    // No yielding: recreation must succeed even before the stale signal is
    // processed.
    let mut s2 = bus.create_order_update_subscription().await.expect("immediate recreation failed");

    // Process s1's stale OrderUpdateStream signal; s2's registration survives.
    drain_cleanup_signals(&bus).await;
    let sender = {
        let stream = bus.order_update_stream.lock().unwrap();
        stream.as_ref().expect("stale cleanup cleared the replacement stream").sender.clone()
    };
    sender
        .send(RoutedItem::Error(Error::Cancelled))
        .expect("replacement stream has no receivers");
    let item = tokio::time::timeout(TICK, s2.next_routed()).await.expect("s2 got nothing");
    assert!(matches!(item, Some(RoutedItem::Error(Error::Cancelled))), "{item:?}");
    drop(sender);

    drop(s2);
    drain_cleanup_signals(&bus).await;
    assert!(bus.order_update_stream.lock().unwrap().is_none(), "order update stream leaked");
}

/// `reset_channels` after reconnect: every in-flight request and order
/// subscription receives `Error::ConnectionReset`, then the channel maps are
/// cleared.
#[tokio::test]
async fn test_reset_channels_notifies_in_flight_subscriptions() {
    let (_, bus) = make_bus();

    let mut req = bus.send_request(RequestId::nth(100), vec![]).await.unwrap();
    let mut order = bus.send_order_request(OrderId::from(200), vec![]).await.unwrap();
    // Streaming shared subscription — the population that hung forever when
    // reset skipped shared channels (#776).
    let mut shared = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, vec![]).await.unwrap();

    bus.reset_channels().await;

    for (name, sub) in [("request", &mut req), ("order", &mut order), ("shared", &mut shared)] {
        let item = tokio::time::timeout(TICK, sub.next_routed())
            .await
            .unwrap_or_else(|_| panic!("{name} got no notification"))
            .unwrap_or_else(|| panic!("{name} channel closed early"));
        assert!(matches!(item, RoutedItem::Error(Error::ConnectionReset)), "{name}: {item:?}");
    }

    assert!(bus.requests.is_empty());
    assert!(bus.orders.is_empty());
    assert!(bus.executions.is_empty());

    // A shared subscription created after the reset resubscribes at the
    // channel's current tail: it must not read the stale ConnectionReset.
    let mut late = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, vec![]).await.unwrap();
    assert!(late.try_next_routed().is_none(), "post-reset shared subscription read a stale reset");
}

/// A finished reconnect publishes the reconnect notice to the notice stream:
/// a connection-state consumer subscribed there learns the socket generation
/// changed even with no live subscription to carry a `ConnectionReset`. TWS
/// never replays 1101/1102 on the new connection, so this notice is the only
/// signal that un-strands state recorded from the previous one (a held 1100).
/// It is published once the session is live again - the channel reset runs
/// before the reconnect, so the notice cannot ride on it - and a consumer
/// that resubscribes on it lands on the new session.
#[tokio::test]
async fn test_reconnect_publishes_reconnect_notice_to_notice_stream() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);

    // First read is a body too short for the message id: InvalidFrame, which
    // the processing loop classifies as connection lost, so this also guards
    // the #891 short-body path end to end (it used to panic the dispatcher).
    // Then the frames the reconnect handshake consumes.
    stream.push_inbound(b"xx".to_vec());
    for frame in handshake_frames(server_versions::PROTOBUF_REST_MESSAGES_3, "EST", 5000) {
        stream.push_inbound(frame);
    }

    let bus = Arc::new(AsyncTcpMessageBus::new(connection).unwrap());
    let mut notices = bus.connection.notice_broadcaster.subscribe();

    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");

    let notice = tokio::time::timeout(Duration::from_secs(2), notices.recv())
        .await
        .expect("no reconnect notice on the notice stream")
        .expect("notice stream closed");
    assert_eq!(notice.code, TRANSPORT_RECONNECT_CODE, "{notice:?}");
    assert!(bus.is_connected(), "the notice must follow the session going live");
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

/// A successful automatic reconnect replays the handshake, whose
/// `NextValidId` is a fresh server floor for order IDs. The bus must raise
/// the client's generator from it: before this, only the initial connection
/// seeded the generator and every reconnect silently discarded the value,
/// leaving allocation stale against the server.
#[tokio::test]
async fn test_reconnect_raises_order_ids_from_handshake() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);

    // First read fails as InvalidFrame (a body too short to hold a message
    // id), which the processing loop classifies as connection lost.
    stream.push_inbound(b"xx".to_vec());
    // Frames the reconnect handshake consumes, in order.
    for frame in handshake_frames(server_versions::PROTOBUF_REST_MESSAGES_3, "EST", 5000) {
        stream.push_inbound(frame);
    }

    let bus = Arc::new(AsyncTcpMessageBus::new(connection).unwrap());
    let order_ids = Arc::new(crate::client::id_generator::ClientIdManager::new(100).unwrap());
    bus.set_order_ids(order_ids.clone());

    bus.clone().process_messages(0, Duration::from_millis(0)).expect("process_messages");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while order_ids.current_order_id() < 5000 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "order-id generator was never raised from the reconnect handshake"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
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

/// An unknown message id must reach the notice stream, not vanish. This is the
/// observable form of a framing desync: before this, `route_to_shared_channel`
/// dropped anything no channel claimed without a log or an error, so a
/// desynchronized stream was indistinguishable from an idle one.
#[tokio::test]
async fn test_unknown_message_id_reaches_the_notice_stream() {
    let (stream, bus) = make_bus();
    let mut notice_stream = bus.notice_subscribe();

    stream.push_inbound(crate::common::test_utils::helpers::unknown_message_frame());

    bus.read_and_route_message().await.unwrap();

    let notice = tokio::time::timeout(TICK, notice_stream.next())
        .await
        .expect("unknown frame must raise a notice")
        .expect("notice stream closed");
    assert_eq!(notice.code, crate::messages::UNKNOWN_MESSAGE_TYPE_CODE);
    // The id survives the protobuf path, where `kind` alone would have lost it.
    assert!(
        notice.message.contains(&helpers::UNKNOWN_MESSAGE_ID.to_string()),
        "notice must name the offending id, got {:?}",
        notice.message
    );
}

/// Every send is refused while the session is down: nothing reaches the socket
/// the reconnect is replacing, and nothing is registered on a channel no reset
/// would clear again.
#[tokio::test]
async fn test_sends_are_refused_while_disconnected() {
    let (stream, bus) = make_bus();
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    bus.connection_state.set_disconnected();

    assert!(matches!(
        mb.send_request(RequestId::nth(100), b"req-bytes".to_vec()).await,
        Err(Error::ConnectionReset)
    ));
    assert!(matches!(
        mb.send_order_request(OrderId::from(42), b"order-bytes".to_vec()).await,
        Err(Error::ConnectionReset)
    ));
    assert!(matches!(
        mb.send_shared_request(OutgoingMessages::RequestManagedAccounts, b"shared-bytes".to_vec())
            .await,
        Err(Error::ConnectionReset)
    ));
    assert!(matches!(mb.send_message(b"message-bytes".to_vec()).await, Err(Error::ConnectionReset)));

    assert!(bus.requests.is_empty(), "a refused request must register nothing");
    assert!(bus.orders.is_empty(), "a refused order request must register nothing");
    assert!(stream.captured().is_empty(), "a refused send must not reach the socket");

    // The same send goes through once the handshake has put the session back.
    bus.connection_state.set_connected();
    assert!(mb.send_request(RequestId::nth(100), b"req-bytes".to_vec()).await.is_ok());
    assert!(!stream.captured().is_empty());
}

/// `wait_connected` holds the one-shot retry until the session is back, and a
/// shutdown releases it rather than leaving the caller there for good.
#[tokio::test]
async fn test_wait_connected_returns_shutdown_when_the_bus_shuts_down() {
    let (_stream, bus) = make_bus();
    bus.connection_state.set_disconnected();

    let closer = Arc::clone(&bus);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        closer.request_shutdown_sync();
    });

    let result = tokio::time::timeout(Duration::from_secs(5), bus.wait_connected())
        .await
        .expect("wait_connected did not return on shutdown");
    assert!(matches!(result, Err(Error::Shutdown)), "got: {result:?}");
}

/// Writes fail, reads delegate to a `MemoryStream`. Stands in for the window
/// the send gate cannot close: the write is refused after the registration
/// went in.
#[derive(Clone, Debug, Default)]
struct FailingWriteStream(MemoryStream);

#[async_trait::async_trait]
impl AsyncIo for FailingWriteStream {
    async fn read_message(&self) -> Result<Vec<u8>, Error> {
        self.0.read_message().await
    }

    async fn write_all(&self, _buf: &[u8]) -> Result<(), Error> {
        Err(Error::ConnectionReset)
    }
}

#[async_trait::async_trait]
impl AsyncReconnect for FailingWriteStream {
    async fn reconnect(&self) -> Result<(), Error> {
        self.0.reconnect().await
    }

    async fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        self.0.sleep(duration, shutdown).await
    }
}

impl AsyncStream for FailingWriteStream {}

/// A send whose write fails takes its registration with it, so no channel is
/// left waiting on a response that was never requested. Covers the window the
/// `ensure_connected` check cannot close, where the session goes down between
/// the check and the write.
#[tokio::test]
async fn test_failed_write_leaves_no_registration() {
    let connection = AsyncConnection::stubbed(FailingWriteStream::default(), 28);
    connection.set_server_version_for_test(server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(AsyncTcpMessageBus::new(connection).unwrap());
    let mb: &dyn AsyncMessageBus = bus.as_ref();

    assert!(mb.send_request(RequestId::nth(100), b"req-bytes".to_vec()).await.is_err());
    assert!(bus.requests.is_empty(), "a failed write must leave no request registered");

    assert!(mb.send_order_request(OrderId::from(42), b"order-bytes".to_vec()).await.is_err());
    assert!(bus.orders.is_empty(), "a failed write must leave no order registered");
}

#[tokio::test]
async fn order_binding_reaches_updates_without_using_raw_order_id() {
    let (stream, bus) = make_bus();
    let mut order_sub = bus.send_order_request(OrderId::from(42), vec![]).await.unwrap();
    let mut update_sub = bus.create_order_update_subscription().await.unwrap();
    stream.push_inbound(binary_proto(IncomingMessages::OrderBound as i32, &order_bound().client_id(73).to_proto()));
    bus.read_and_route_message().await.unwrap();
    let message = next_message(&mut update_sub).await;
    assert_eq!(message.message_type(), IncomingMessages::OrderBound);
    assert!(tokio::time::timeout(TICK, order_sub.next()).await.is_err());
}

// ---- buffer_limit: bounded request routes ------------------------------------

/// A cap of `limit`, ending on `ContractDataEnd`.
pub(super) fn bound(limit: usize) -> BufferBound {
    BufferBound {
        limit,
        end: IncomingMessages::ContractDataEnd,
    }
}

/// The request id `contract_row` and `contract_end` frames carry.
const CONTRACT_REQUEST_ID: RequestId = RequestId::nth(0);

fn contract_row(contract_id: i32) -> Vec<u8> {
    binary_proto(
        IncomingMessages::ContractData as i32,
        &contract_data().request_id(CONTRACT_REQUEST_ID.raw()).contract_id(contract_id).to_proto(),
    )
}

fn contract_end() -> Vec<u8> {
    binary_proto(
        IncomingMessages::ContractDataEnd as i32,
        &crate::proto::ContractDataEnd {
            req_id: Some(CONTRACT_REQUEST_ID.raw()),
        },
    )
}

async fn route_histograms(stream: &MemoryStream, bus: &AsyncTcpMessageBus<MemoryStream>, frames: usize, request_id: RequestId) {
    for n in 0..frames {
        // HistogramData (msg_id 89): request_id at field index 1, `n` at 2.
        stream.push_inbound(body(&format!("89|{request_id}|{n}|")));
        bus.read_and_route_message().await.unwrap();
    }
}

/// The next routed item, or `None` if nothing arrives within `TICK`.
async fn try_next_routed(sub: &mut AsyncInternalSubscription) -> Option<RoutedItem> {
    tokio::time::timeout(TICK, sub.next_routed()).await.ok().flatten()
}

#[tokio::test]
async fn test_bounded_request_fails_after_limit_unread() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request_bounded(RequestId::nth(100), vec![], bound(2)).await.unwrap();

    route_histograms(&stream, &bus, 4, RequestId::nth(100)).await;

    // The first frame was not evicted: the overflow error took the reserved slot.
    match try_next_routed(&mut sub).await {
        Some(RoutedItem::Response(message)) => assert_eq!(message.peek_int(2).unwrap(), 0),
        other => panic!("expected the first row, got {other:?}"),
    }
    assert!(matches!(try_next_routed(&mut sub).await, Some(RoutedItem::Response(_))));
    assert!(matches!(
        try_next_routed(&mut sub).await,
        Some(RoutedItem::Error(Error::BufferLimitExceeded { limit: 2 }))
    ));
    assert!(
        try_next_routed(&mut sub).await.is_none(),
        "no lag notice, and frames after the overflow are discarded"
    );
}

#[tokio::test]
async fn test_bounded_request_counts_unread_not_total() {
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request_bounded(RequestId::nth(100), vec![], bound(2)).await.unwrap();

    for _ in 0..6 {
        route_histograms(&stream, &bus, 1, RequestId::nth(100)).await;
        assert!(
            matches!(try_next_routed(&mut sub).await, Some(RoutedItem::Response(_))),
            "a reader that keeps up never overflows"
        );
    }
}

#[tokio::test]
async fn test_reset_skips_overflowed_route() {
    let (stream, bus) = make_bus();
    let mut overflowed = bus.send_request_bounded(RequestId::nth(100), vec![], bound(1)).await.unwrap();
    let mut at_limit = bus.send_request_bounded(RequestId::nth(200), vec![], bound(1)).await.unwrap();

    route_histograms(&stream, &bus, 2, RequestId::nth(100)).await;
    route_histograms(&stream, &bus, 1, RequestId::nth(200)).await;
    bus.reset_channels().await;

    assert!(matches!(try_next_routed(&mut overflowed).await, Some(RoutedItem::Response(_))));
    assert!(matches!(
        try_next_routed(&mut overflowed).await,
        Some(RoutedItem::Error(Error::BufferLimitExceeded { .. }))
    ));
    assert!(try_next_routed(&mut overflowed).await.is_none(), "no second terminal error after reset");

    // At its limit but not overflowed: the reset error uses the reserved slot, evicting nothing.
    assert!(matches!(try_next_routed(&mut at_limit).await, Some(RoutedItem::Response(_))));
    assert!(matches!(
        try_next_routed(&mut at_limit).await,
        Some(RoutedItem::Error(Error::ConnectionReset))
    ));
}

#[tokio::test]
async fn test_overflowed_subscription_cancels_on_drop() {
    use crate::contracts::ContractDetails;

    let (stream, bus) = make_bus();
    let internal = bus.send_request_bounded(CONTRACT_REQUEST_ID, vec![], bound(1)).await.unwrap();
    let mut subscription: Subscription<ContractDetails> = Subscription::new_from_internal(
        internal,
        bus.clone(),
        Some(CONTRACT_REQUEST_ID.raw()),
        None,
        DecoderContext::new(server_versions::CANCEL_CONTRACT_DATA),
    );

    for contract_id in [1, 2] {
        stream.push_inbound(contract_row(contract_id));
        bus.read_and_route_message().await.unwrap();
    }

    assert!(matches!(next_item(&mut subscription).await, Some(Ok(_))));
    assert!(matches!(
        next_item(&mut subscription).await,
        Some(Err(Error::BufferLimitExceeded { limit: 1 }))
    ));
    drop(subscription);

    let cancel = <ContractDetails as StreamDecoder<ContractDetails>>::cancel_message(
        server_versions::CANCEL_CONTRACT_DATA,
        Some(CONTRACT_REQUEST_ID.raw()),
        None,
    )
    .unwrap();
    assert_eq!(wait_for_frames(&stream, &cancel, 1).await, 1, "overflow leaves the cancel to drop");
}

#[tokio::test]
async fn test_bounded_request_end_marker_at_limit_still_ends() {
    // A result exactly `limit` rows long, read late: the end marker takes the
    // spare slot, so the stream ends normally and nothing is evicted.
    let (stream, bus) = make_bus();
    let mut sub = bus.send_request_bounded(CONTRACT_REQUEST_ID, vec![], bound(1)).await.unwrap();

    for frame in [contract_row(1), contract_end(), contract_row(2)] {
        stream.push_inbound(frame);
        bus.read_and_route_message().await.unwrap();
    }

    match try_next_routed(&mut sub).await {
        Some(RoutedItem::Response(message)) => assert_eq!(message.message_type(), IncomingMessages::ContractData),
        other => panic!("expected the row, got {other:?}"),
    }
    match try_next_routed(&mut sub).await {
        Some(RoutedItem::Response(message)) => assert_eq!(message.message_type(), IncomingMessages::ContractDataEnd),
        other => panic!("expected the end marker, got {other:?}"),
    }
    assert!(try_next_routed(&mut sub).await.is_none(), "frames after the end marker are discarded");
}

/// Shutdown fails the request with `Error::Shutdown`; the drain reports it, as
/// the sync drain does.
#[tokio::test]
async fn test_drain_reports_shutdown() {
    use crate::contracts::ContractDetails;
    use crate::subscriptions::Drained;

    let (_stream, bus) = make_bus();
    let internal = bus.send_request(CONTRACT_REQUEST_ID, vec![]).await.unwrap();
    let subscription: Subscription<ContractDetails> = Subscription::new_from_internal(
        internal,
        bus.clone(),
        Some(CONTRACT_REQUEST_ID.raw()),
        None,
        DecoderContext::new(server_versions::CANCEL_CONTRACT_DATA),
    );

    let drain = tokio::spawn(subscription.cancel_and_drain(tokio::time::Instant::now() + Duration::from_secs(5)));
    tokio::time::sleep(Duration::from_millis(20)).await;
    bus.request_shutdown();

    let outcome: Result<Drained, Error> = drain.await.unwrap();
    assert!(matches!(outcome, Err(Error::Shutdown)), "got {outcome:?}");
}
