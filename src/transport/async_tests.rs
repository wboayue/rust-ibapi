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
