use super::*;
use crate::common::test_utils::helpers::order_status_response;
use crate::messages::Notice;
use crate::testdata::builders::orders::{order_status, OrderStatusResponse};

fn observe(status: OrderStatusResponse) -> Option<OrderOutcome> {
    FillTracker::default().observe(&order_status_response(status)).unwrap()
}

#[test]
fn filled_status_is_filled() {
    let outcome = observe(order_status().status(OrderStatusKind::Filled).filled(100.0).remaining(0.0));
    assert!(matches!(outcome, Some(OrderOutcome::Filled(s)) if s.filled == 100.0));
}

#[test]
fn nothing_remaining_after_a_fill_is_filled() {
    let outcome = observe(order_status().status(OrderStatusKind::Submitted).filled(100.0).remaining(0.0));
    assert!(matches!(outcome, Some(OrderOutcome::Filled(s)) if s.status == OrderStatusKind::Submitted));
}

#[test]
fn nothing_remaining_without_a_fill_keeps_waiting() {
    assert_eq!(
        observe(order_status().status(OrderStatusKind::PreSubmitted).filled(0.0).remaining(0.0)),
        None
    );
}

#[test]
fn partial_fill_keeps_waiting() {
    assert_eq!(
        observe(order_status().status(OrderStatusKind::Submitted).filled(40.0).remaining(60.0)),
        None
    );
}

#[test]
fn cancelled_api_cancelled_and_inactive_end() {
    for kind in [OrderStatusKind::Cancelled, OrderStatusKind::ApiCancelled, OrderStatusKind::Inactive] {
        let outcome = observe(order_status().status(kind.clone()).filled(40.0).remaining(60.0));
        assert!(
            matches!(&outcome, Some(OrderOutcome::Ended(s)) if s.status == kind && s.filled == 40.0),
            "{kind:?}: {outcome:?}"
        );
    }
}

#[test]
fn unknown_and_api_pending_keep_waiting() {
    for kind in [OrderStatusKind::Unknown("Mystery".into()), OrderStatusKind::ApiPending] {
        assert_eq!(observe(order_status().status(kind.clone())), None, "{kind:?}");
    }
}

#[test]
fn timed_out_carries_last_working_status() {
    let mut tracker = FillTracker::default();
    tracker
        .observe(&order_status_response(order_status().filled(10.0).remaining(90.0)))
        .unwrap();
    tracker
        .observe(&order_status_response(order_status().filled(40.0).remaining(60.0)))
        .unwrap();

    assert!(matches!(tracker.timed_out(), OrderOutcome::TimedOut(Some(s)) if s.filled == 40.0));
}

#[test]
fn timed_out_without_a_status() {
    assert_eq!(FillTracker::default().timed_out(), OrderOutcome::TimedOut(None));
}

#[test]
fn notice_is_skipped_and_error_ends_the_wait() {
    let mut tracker = FillTracker::default();
    let notice = Notice::synthesized(2109, "warning".into());
    assert!(matches!(tracker.on_item(RoutedItem::Notice(notice)), Ok(None)));
    assert!(matches!(
        tracker.on_item(RoutedItem::Error(Error::ConnectionReset)),
        Err(Error::ConnectionReset)
    ));
}
