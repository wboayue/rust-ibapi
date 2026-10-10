use super::*;
use crate::common::test_utils::helpers::proto_response;
use crate::messages::IncomingMessages;
use crate::orders::OrderStatusKind;
use crate::testdata::builders::orders::order_status;
use crate::testdata::builders::ResponseProtoEncoder;
use crossbeam::channel::Sender;

type Board = OrderStatusBoard<Sender<RoutedItem>>;
type Watch = OrderStatusWatch<Sender<RoutedItem>>;

fn status_frame(order_id: i32, status: OrderStatusKind) -> ResponseMessage {
    proto_response(
        IncomingMessages::OrderStatus,
        order_status().order_id(order_id).status(status).encode_proto(),
    )
}

fn watch(board: &Arc<Board>, order_id: i32) -> Watch {
    board.watch(OrderId::from(order_id)).expect("watch")
}

fn waiters(board: &Board, order_id: i32) -> usize {
    board.lock().slots[&OrderId::from(order_id)].watchers.len()
}

fn order_id_of(item: RoutedItem) -> Option<i32> {
    match item {
        RoutedItem::Response(message) => message.order_id(),
        other => panic!("expected a response, got {other:?}"),
    }
}

#[test]
fn watch_returns_status_published_before_it() {
    let board = Arc::new(Board::default());
    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Filled));

    let watch = watch(&board, 7);

    assert_eq!(watch.latest.and_then(|m| m.order_id()), Some(7));
}

#[test]
fn watch_of_unseen_order_has_no_latest() {
    let board = Arc::new(Board::default());
    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Filled));

    assert!(watch(&board, 8).latest.is_none());
}

#[test]
fn publish_reaches_every_waiter_on_the_order_only() {
    let board = Arc::new(Board::default());
    let first = watch(&board, 7);
    let second = watch(&board, 7);
    let other = watch(&board, 8);

    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Submitted));

    assert_eq!(order_id_of(first.receiver.try_recv().unwrap()), Some(7));
    assert_eq!(order_id_of(second.receiver.try_recv().unwrap()), Some(7));
    assert!(other.receiver.try_recv().is_err());
}

#[test]
fn publish_replaces_latest() {
    let board = Arc::new(Board::default());
    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Submitted));
    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Cancelled));

    let latest = watch(&board, 7).latest.unwrap();

    let cancelled = order_status().order_id(7).status(OrderStatusKind::Cancelled).encode_proto();
    assert_eq!(latest.raw_bytes(), Some(cancelled.as_slice()));
}

#[test]
fn dropped_watch_deregisters() {
    let board = Arc::new(Board::default());
    let kept = watch(&board, 7);
    drop(watch(&board, 7));

    assert_eq!(waiters(&board, 7), 1);
    drop(kept);
    assert_eq!(waiters(&board, 7), 0);
}

#[test]
fn waiter_whose_receiver_dropped_is_pruned_on_publish() {
    let board = Arc::new(Board::default());
    let Watch { receiver, _registration, .. } = watch(&board, 7);
    drop(receiver);

    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Submitted));

    assert_eq!(waiters(&board, 7), 0);
    drop(_registration);
}

#[test]
fn fail_all_ends_waiters_and_keeps_latest() {
    let board = Arc::new(Board::default());
    board.publish(OrderId::from(7), &status_frame(7, OrderStatusKind::Submitted));
    let failed = watch(&board, 7);

    board.fail_all(|| Error::ConnectionReset);

    assert!(matches!(failed.receiver.try_recv(), Ok(RoutedItem::Error(Error::ConnectionReset))));
    assert!(failed.receiver.recv().is_err(), "sender dropped after the error");
    assert!(watch(&board, 7).latest.is_some(), "a reset keeps the latest status");
}

#[test]
fn close_ends_waiters_and_refuses_new_ones() {
    let board = Arc::new(Board::default());
    let closed = watch(&board, 7);

    board.close();

    assert!(matches!(closed.receiver.try_recv(), Ok(RoutedItem::Error(Error::Shutdown))));
    assert!(matches!(board.watch(OrderId::from(7)), Err(Error::Shutdown)));
}
