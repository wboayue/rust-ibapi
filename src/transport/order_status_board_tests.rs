use super::*;
use crate::common::test_utils::helpers::{order_status_response, proto_response};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::orders::OrderStatusKind;
use crate::testdata::builders::orders::{execution_data, order_status};
use crate::testdata::builders::ResponseProtoEncoder;
use crossbeam::channel::Sender;

type Board = OrderStatusBoard<Sender<RoutedItem>>;
type Watch = OrderStatusWatch<Sender<RoutedItem>>;

fn publish(board: &Board, order_id: i32, status: OrderStatusKind) {
    board.publish(&order_status_response(order_status().order_id(order_id).status(status)));
}

fn watch(board: &Arc<Board>, order_id: i32) -> Watch {
    board.watch(OrderId::from(order_id)).expect("watch")
}

fn waiters(board: &Board, order_id: i32) -> usize {
    board.lock().slots.get(&OrderId::from(order_id)).map_or(0, |slot| slot.watchers.len())
}

fn next_response(watch: &Watch) -> ResponseMessage {
    match watch.receiver.try_recv() {
        Ok(RoutedItem::Response(message)) => message,
        other => panic!("expected a response, got {other:?}"),
    }
}

#[test]
fn watch_starts_with_status_published_before_it() {
    let board = Arc::new(Board::default());
    publish(&board, 7, OrderStatusKind::Submitted);
    publish(&board, 7, OrderStatusKind::Cancelled);

    let watch = watch(&board, 7);

    let cancelled = order_status().order_id(7).status(OrderStatusKind::Cancelled).encode_proto();
    assert_eq!(next_response(&watch).raw_bytes(), Some(cancelled.as_slice()), "latest only");
    assert!(watch.receiver.try_recv().is_err());
}

#[test]
fn watch_of_unseen_order_starts_empty() {
    let board = Arc::new(Board::default());
    publish(&board, 7, OrderStatusKind::Filled);

    assert!(watch(&board, 8).receiver.try_recv().is_err());
}

#[test]
fn publish_reaches_every_waiter_on_the_order_only() {
    let board = Arc::new(Board::default());
    let first = watch(&board, 7);
    let second = watch(&board, 7);
    let other = watch(&board, 8);

    publish(&board, 7, OrderStatusKind::Submitted);

    assert_eq!(next_response(&first).order_id(), Some(7));
    assert_eq!(next_response(&second).order_id(), Some(7));
    assert!(other.receiver.try_recv().is_err());
}

#[test]
fn publish_ignores_other_frames() {
    let board = Arc::new(Board::default());
    let watch = watch(&board, 7);

    board.publish(&proto_response(
        IncomingMessages::ExecutionData,
        execution_data().order_id(7).encode_proto(),
    ));

    assert!(watch.receiver.try_recv().is_err());
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

    publish(&board, 7, OrderStatusKind::Submitted);

    assert_eq!(waiters(&board, 7), 0);
    drop(_registration);
}

#[test]
fn reset_ends_waiters_and_keeps_latest() {
    let board = Arc::new(Board::default());
    let failed = watch(&board, 7);
    publish(&board, 7, OrderStatusKind::Submitted);
    next_response(&failed);

    board.reset();

    assert!(matches!(failed.receiver.try_recv(), Ok(RoutedItem::Error(Error::ConnectionReset))));
    assert!(failed.receiver.recv().is_err(), "sender dropped after the error");
    assert_eq!(next_response(&watch(&board, 7)).order_id(), Some(7), "a reset keeps the latest status");
}

#[test]
fn close_ends_waiters_and_refuses_new_ones() {
    let board = Arc::new(Board::default());
    let closed = watch(&board, 7);

    board.close();

    assert!(matches!(closed.receiver.try_recv(), Ok(RoutedItem::Error(Error::Shutdown))));
    assert!(matches!(board.watch(OrderId::from(7)), Err(Error::Shutdown)));
}
