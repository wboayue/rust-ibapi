use super::*;
use crate::orders::OrderStatusKind;
use crate::Notice;

fn status(kind: OrderStatusKind, filled: f64, remaining: f64) -> OrderStatus {
    OrderStatus {
        status: kind,
        filled,
        remaining,
        ..Default::default()
    }
}

#[test]
fn outcome_of_each_status() {
    use OrderStatusKind::*;
    let cases = [
        ("filled", Filled, 100.0, 0.0, Some("Filled")),
        ("nothing remaining after a fill", Submitted, 100.0, 0.0, Some("Filled")),
        ("nothing remaining without a fill", PreSubmitted, 0.0, 0.0, None),
        ("partial fill", Submitted, 40.0, 60.0, None),
        ("cancelled", Cancelled, 40.0, 60.0, Some("Ended")),
        ("api cancelled", ApiCancelled, 0.0, 100.0, Some("Ended")),
        ("inactive", Inactive, 0.0, 100.0, Some("Ended")),
        ("api pending", ApiPending, 0.0, 100.0, None),
        ("unknown", Unknown("Mystery".into()), 0.0, 100.0, None),
    ];
    for (name, kind, filled, remaining, expected) in cases {
        let input = status(kind, filled, remaining);
        let outcome = FillTracker::default().observe(input.clone());
        let got = match &outcome {
            Some(OrderOutcome::Filled(s)) => Some(("Filled", s)),
            Some(OrderOutcome::Ended(s)) => Some(("Ended", s)),
            Some(OrderOutcome::TimedOut(_)) => panic!("{name}: observe never times out"),
            Some(OrderOutcome::Rejected(_)) => panic!("{name}: a status is never a rejection"),
            None => None,
        };
        assert_eq!(got.map(|(variant, _)| variant), expected, "{name}");
        if let Some((_, carried)) = got {
            assert_eq!(carried, &input, "{name}: outcome carries the status");
        }
    }
}

#[test]
fn timed_out_carries_last_working_status() {
    let mut tracker = FillTracker::default();
    tracker.observe(status(OrderStatusKind::Submitted, 10.0, 90.0));
    tracker.observe(status(OrderStatusKind::Submitted, 40.0, 60.0));

    assert!(matches!(tracker.timed_out(), OrderOutcome::TimedOut(Some(s)) if s.filled == 40.0));
}

#[test]
fn timed_out_without_a_status() {
    assert_eq!(FillTracker::default().timed_out(), OrderOutcome::TimedOut(None));
}

#[test]
fn error_of_a_notice_is_a_rejection() {
    let notice = Notice {
        code: 110,
        message: "The price does not conform to the minimum price variation for this contract.".into(),
        request_id: Some(13),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };

    assert_eq!(FillTracker::error(Error::Notice(notice.clone())).unwrap(), OrderOutcome::Rejected(notice));
    assert!(matches!(FillTracker::error(Error::ConnectionReset), Err(Error::ConnectionReset)));
}

fn order_row(client_id: i32, order_id: i32, kind: OrderStatusKind, filled: f64) -> Orders {
    let mut data = OrderData::default();
    data.order.client_id = client_id;
    data.order.order_id = order_id;
    data.order.perm_id = 900 + i64::from(order_id);
    data.order.parent_id = 3;
    // A 100-share order: TWS reports a completed order's unfilled quantity as its total.
    data.order.total_quantity = 100.0 - filled;
    data.order.filled_quantity = filled;
    data.order_state.status = kind;
    Orders::OrderData(data)
}

#[test]
fn lists_order_by_order_or_status() {
    let status_row = Orders::OrderStatus(OrderStatus {
        order_id: 13,
        ..Default::default()
    });
    let cases = [
        ("order row", vec![order_row(7, 13, OrderStatusKind::Submitted, 0.0)], true),
        ("status row", vec![status_row], true),
        ("other order", vec![order_row(7, 14, OrderStatusKind::Submitted, 0.0)], false),
        ("empty reply", Vec::new(), false),
    ];
    for (name, rows, expected) in cases {
        assert_eq!(lists_order(&rows, OrderId(13)), expected, "{name}");
    }
}

#[test]
fn completed_status_matches_client_and_order() {
    use OrderStatusKind::*;
    let cases = [
        ("match", vec![order_row(7, 13, Cancelled, 40.0)], Some(Cancelled)),
        ("other client", vec![order_row(8, 13, Cancelled, 0.0)], None),
        ("other order", vec![order_row(7, 14, Cancelled, 0.0)], None),
        (
            "last match wins",
            vec![order_row(7, 13, Cancelled, 0.0), order_row(7, 13, Filled, 100.0)],
            Some(Filled),
        ),
        ("empty reply", Vec::new(), None),
    ];
    for (name, rows, expected) in cases {
        let found = completed_status(&rows, 7, OrderId(13));
        assert_eq!(found.as_ref().map(|s| s.status.clone()), expected, "{name}");
    }
}

#[test]
fn completed_status_is_built_from_the_order() {
    let found = completed_status(&[order_row(7, 13, OrderStatusKind::Cancelled, 40.0)], 7, OrderId(13));

    let expected = OrderStatus {
        order_id: 13,
        status: OrderStatusKind::Cancelled,
        filled: 40.0,
        remaining: 60.0,
        average_fill_price: None,
        perm_id: 913,
        parent_id: 3,
        last_fill_price: None,
        client_id: 7,
        why_held: String::new(),
        market_cap_price: None,
    };
    assert_eq!(found, Some(expected));
}

#[test]
fn lookup_wait_is_at_least_the_minimum() {
    assert_eq!(lookup_wait(Instant::now()), LOOKUP_MIN_WAIT);
    assert!(lookup_wait(Instant::now() + Duration::from_secs(30)) > Duration::from_secs(29));
}

/// TWS sends a filled completed order's total as `0` (#1116), so nothing remains.
#[test]
fn completed_status_of_a_filled_order_has_nothing_remaining() {
    let found = completed_status(&[order_row(7, 13, OrderStatusKind::Filled, 100.0)], 7, OrderId(13)).unwrap();

    assert_eq!((found.filled, found.remaining), (100.0, 0.0));
}
