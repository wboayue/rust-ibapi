use super::*;
use crate::common::test_utils::helpers::{order_status_response, proto_response};
use crate::messages::Notice;
use crate::orders::OrderStatusKind;
use crate::testdata::builders::orders::{execution_data, order_status};
use crate::testdata::builders::ResponseProtoEncoder;
use crossbeam::channel::{Receiver, Sender};

type Taps = OrderTaps<Sender<RoutedItem>>;

fn taps() -> Taps {
    Taps::new()
}

fn publish(taps: &Taps, order_id: i32, status: OrderStatusKind) {
    taps.publish_status(&order_status_response(order_status().order_id(order_id).status(status)));
}

fn status_stream(taps: &Taps, order_id: i32) -> (Receiver<RoutedItem>, Lease) {
    let NewTap { receiver, lease, .. } = taps.subscribe_status(OrderId::from(order_id), 0).expect("subscribe");
    (receiver, lease)
}

/// Drop the stream's lease and send its cleanup, as a dropped subscription does.
fn release(taps: &Taps, order_id: i32, lease: Lease) {
    let lease_ref = lease.downgrade();
    drop(lease);
    taps.release_status(OrderId::from(order_id), &lease_ref);
}

fn next_response(receiver: &Receiver<RoutedItem>) -> ResponseMessage {
    match receiver.try_recv() {
        Ok(RoutedItem::Response(message)) => message,
        other => panic!("expected a response, got {other:?}"),
    }
}

#[test]
fn status_stream_starts_with_latest_status() {
    let taps = taps();
    publish(&taps, 7, OrderStatusKind::Submitted);
    publish(&taps, 7, OrderStatusKind::Cancelled);

    let (receiver, _lease) = status_stream(&taps, 7);

    let cancelled = order_status().order_id(7).status(OrderStatusKind::Cancelled).encode_proto();
    assert_eq!(next_response(&receiver).raw_bytes(), Some(cancelled.as_slice()), "latest only");
    assert!(receiver.try_recv().is_err());
}

#[test]
fn status_stream_of_unseen_order_starts_empty() {
    let taps = taps();
    publish(&taps, 7, OrderStatusKind::Filled);

    assert!(status_stream(&taps, 8).0.try_recv().is_err());
}

#[test]
fn publish_reaches_every_stream_on_the_order_only() {
    let taps = taps();
    let (first, _a) = status_stream(&taps, 7);
    let (second, _b) = status_stream(&taps, 7);
    let (other, _c) = status_stream(&taps, 8);

    publish(&taps, 7, OrderStatusKind::Submitted);

    assert_eq!(next_response(&first).order_id(), Some(7));
    assert_eq!(next_response(&second).order_id(), Some(7));
    assert!(other.try_recv().is_err());
}

#[test]
fn publish_ignores_other_frames() {
    let taps = taps();
    let (receiver, _lease) = status_stream(&taps, 7);

    taps.publish_status(&proto_response(
        IncomingMessages::ExecutionData,
        execution_data().order_id(7).encode_proto(),
    ));

    assert!(receiver.try_recv().is_err());
}

/// Route an error frame with `code` for `id`, as the dispatcher does.
fn reject(taps: &Taps, id: i32, code: i32) {
    let payload = crate::transport::routing::DecodedError {
        request_id: id,
        error_code: code,
        error_message: format!("error {code}"),
        ..Default::default()
    };
    let item = match crate::transport::routing::classify_error(payload) {
        crate::transport::routing::ErrorDisposition::Route(_, item) => item,
        other => panic!("{id} should route, got {other:?}"),
    };
    taps.publish_rejection(WireId::classify(id).expect("routable id"), &item);
}

fn next_rejection(receiver: &Receiver<RoutedItem>) -> Notice {
    match receiver.try_recv() {
        Ok(RoutedItem::Error(Error::Notice(notice))) => notice,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[test]
fn rejection_reaches_open_streams_and_starts_later_ones() {
    let taps = taps();
    let (open, _lease) = status_stream(&taps, 7);

    reject(&taps, 7, 110);

    assert_eq!(next_rejection(&open).code, 110);
    assert_eq!(next_rejection(&status_stream(&taps, 7).0).code, 110, "a later stream starts with it");
}

#[test]
fn rejection_after_a_status_is_ignored() {
    let taps = taps();
    publish(&taps, 7, OrderStatusKind::Submitted);
    let (receiver, _lease) = status_stream(&taps, 7);
    next_response(&receiver);

    // e.g. a refused modify: the order keeps working.
    reject(&taps, 7, 110);

    assert!(receiver.try_recv().is_err());
    assert_eq!(next_response(&status_stream(&taps, 7).0).order_id(), Some(7), "latest status kept");
}

#[test]
fn first_rejection_is_kept() {
    let taps = taps();
    reject(&taps, 7, 110);
    let (receiver, _lease) = status_stream(&taps, 7);
    next_rejection(&receiver);

    reject(&taps, 7, 10147);

    assert!(receiver.try_recv().is_err(), "no second rejection");
    assert_eq!(next_rejection(&status_stream(&taps, 7).0).code, 110);
}

#[test]
fn status_after_rejection_replaces_it() {
    let taps = taps();
    reject(&taps, 7, 110);

    publish(&taps, 7, OrderStatusKind::Inactive);

    assert_eq!(next_response(&status_stream(&taps, 7).0).order_id(), Some(7));
}

#[test]
fn informational_and_request_errors_are_not_rejections() {
    let taps = taps();
    let (order, _a) = status_stream(&taps, 7);

    reject(&taps, 7, 2109); // a warning on the order
    let request_id = crate::client::ids::REQUEST_ID_FLOOR;
    reject(&taps, request_id, 200);

    assert!(order.try_recv().is_err(), "warning");
    assert!(!taps.has_status_entry(OrderId::from(request_id)), "request id");
}

#[test]
fn release_removes_only_its_stream_then_the_empty_entry() {
    let taps = taps();
    let (_kept_receiver, kept) = status_stream(&taps, 7);
    let (_receiver, released) = status_stream(&taps, 7);

    release(&taps, 7, released);
    assert_eq!(taps.status_streams(OrderId::from(7)), 1);

    release(&taps, 7, kept);
    assert!(
        !taps.has_status_entry(OrderId::from(7)),
        "an entry with no stream and no status is removed"
    );
}

#[test]
fn release_keeps_entry_holding_a_latest_status() {
    let taps = taps();
    publish(&taps, 7, OrderStatusKind::Submitted);
    let (_receiver, lease) = status_stream(&taps, 7);

    release(&taps, 7, lease);

    assert!(taps.has_status_entry(OrderId::from(7)));
    assert_eq!(taps.status_streams(OrderId::from(7)), 0);
}

#[test]
fn release_ignores_a_live_lease() {
    let taps = taps();
    let (_receiver, lease) = status_stream(&taps, 7);

    taps.release_status(OrderId::from(7), &lease.downgrade());

    assert_eq!(taps.status_streams(OrderId::from(7)), 1);
}

#[test]
fn oldest_latest_statuses_are_evicted_past_capacity() {
    let taps = taps();
    let (watched, _lease) = status_stream(&taps, 0);
    for order_id in 0..=LATEST_STATUS_CAPACITY as i32 {
        publish(&taps, order_id, OrderStatusKind::Submitted);
    }
    next_response(&watched);

    assert!(status_stream(&taps, 0).0.try_recv().is_err(), "oldest status evicted");
    assert!(taps.has_status_entry(OrderId::from(0)), "entry kept while a stream is open");
    let newest = LATEST_STATUS_CAPACITY as i32;
    assert_eq!(next_response(&status_stream(&taps, newest).0).order_id(), Some(newest));
}

#[test]
fn reset_ends_status_streams_and_keeps_updates_and_final_latest() {
    let taps = taps();
    let NewTap {
        receiver: updates,
        lease: _updates_lease,
        ..
    } = taps.subscribe_updates(0).unwrap();
    publish(&taps, 7, OrderStatusKind::Filled);
    let (receiver, _lease) = status_stream(&taps, 7);
    next_response(&receiver);

    taps.reset();

    assert!(matches!(receiver.try_recv(), Ok(RoutedItem::Error(Error::ConnectionReset))));
    assert!(receiver.recv().is_err(), "sender dropped after the error");
    assert_eq!(
        next_response(&status_stream(&taps, 7).0).order_id(),
        Some(7),
        "a reset keeps a final status"
    );
    assert!(taps.send_update(order_status_response(order_status()).into()));
    assert!(updates.try_recv().is_ok(), "a reset keeps the update stream");
}

/// TWS does not replay what happened while disconnected, so only statuses
/// no later frame can change survive a reset (#1031).
#[test]
fn reset_keeps_only_latest_that_cannot_change() {
    use OrderStatusKind::*;
    let cases = [
        ("pre-submitted", PreSubmitted, false),
        ("submitted", Submitted, false),
        ("filled", Filled, true),
        ("cancelled", Cancelled, true),
        ("inactive", Inactive, true),
    ];
    for (name, status, kept) in cases {
        let taps = taps();
        publish(&taps, 7, status);

        taps.reset();

        assert_eq!(status_stream(&taps, 7).0.try_recv().is_ok(), kept, "{name}");
        assert_eq!(taps.recent_len(), usize::from(kept), "{name}: recent follows the latest");
    }
}

#[test]
fn reset_keeps_a_rejection() {
    let taps = taps();
    reject(&taps, 7, 110);

    taps.reset();

    assert_eq!(next_rejection(&status_stream(&taps, 7).0).code, 110);
}

/// A status dropped by a reset and seen again counts once toward the capacity.
#[test]
fn status_after_reset_is_recent_once() {
    let taps = taps();
    publish(&taps, 7, OrderStatusKind::Submitted);
    taps.reset();

    publish(&taps, 7, OrderStatusKind::Submitted);

    assert_eq!(taps.recent_len(), 1);
}

#[test]
fn close_ends_every_stream_and_refuses_new_ones() {
    let taps = taps();
    let NewTap {
        receiver: updates,
        lease: _updates_lease,
        ..
    } = taps.subscribe_updates(0).unwrap();
    let (receiver, _lease) = status_stream(&taps, 7);

    taps.close();

    assert!(matches!(updates.try_recv(), Ok(RoutedItem::Error(Error::Shutdown))));
    assert!(matches!(receiver.try_recv(), Ok(RoutedItem::Error(Error::Shutdown))));
    assert!(matches!(taps.subscribe_status(OrderId::from(7), 0), Err(Error::Shutdown)));
    assert!(matches!(taps.subscribe_updates(0), Err(Error::Shutdown)));
}

#[test]
fn update_stream_is_exclusive_until_released() {
    let taps = taps();
    let NewTap {
        receiver: _receiver, lease, ..
    } = taps.subscribe_updates(0).unwrap();
    assert!(matches!(taps.subscribe_updates(0), Err(Error::AlreadySubscribed)));

    let lease_ref = lease.downgrade();
    drop(lease);
    let _replacement = taps.subscribe_updates(0).expect("a dead registration is replaced");
    assert!(!taps.release_updates(&lease_ref), "a stale release leaves the replacement");
    assert!(taps.updates_lease().is_some());
}
