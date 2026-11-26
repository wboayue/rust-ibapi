use super::*;
use crate::orders::{OrderStatus, OrderStatusKind};

const BY_ID: OrderKey = OrderKey::Id {
    client_id: 7,
    order_id: OrderId(13),
};

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

fn status_row(order_id: i32, kind: OrderStatusKind) -> Orders {
    Orders::OrderStatus(OrderStatus {
        order_id,
        client_id: 7,
        status: kind,
        perm_id: 900 + i64::from(order_id),
        ..Default::default()
    })
}

fn variant(found: Option<OrderLookup>) -> &'static str {
    match found {
        Some(OrderLookup::Working { .. }) => "Working",
        Some(OrderLookup::Finished { .. }) => "Finished",
        Some(OrderLookup::NotFound) => "NotFound",
        None => "none",
    }
}

#[test]
fn working_matches_a_listed_order_that_is_not_terminal() {
    use OrderStatusKind::*;
    let cases = [
        ("order row", vec![order_row(7, 13, Submitted, 0.0)], BY_ID, "Working"),
        ("status row", vec![status_row(13, PreSubmitted)], BY_ID, "Working"),
        (
            "other client's id, from an overlapping all_open_orders reply",
            vec![order_row(8, 13, Submitted, 0.0)],
            BY_ID,
            "none",
        ),
        ("perm id", vec![order_row(7, 13, Submitted, 0.0)], OrderKey::PermId(913), "Working"),
        ("other perm id", vec![order_row(7, 13, Submitted, 0.0)], OrderKey::PermId(914), "none"),
        ("other order", vec![order_row(7, 14, Submitted, 0.0)], BY_ID, "none"),
        ("listed inactive", vec![order_row(7, 13, Inactive, 0.0)], BY_ID, "none"),
        (
            "last status terminal",
            vec![order_row(7, 13, Submitted, 0.0), status_row(13, Inactive)],
            BY_ID,
            "none",
        ),
        ("empty reply", Vec::new(), BY_ID, "none"),
    ];
    for (name, rows, key, expected) in cases {
        assert_eq!(variant(key.working(rows)), expected, "{name}");
    }
}

#[test]
fn working_carries_the_last_order_and_status() {
    let rows = vec![
        order_row(7, 13, OrderStatusKind::PreSubmitted, 0.0),
        status_row(13, OrderStatusKind::PreSubmitted),
        order_row(7, 13, OrderStatusKind::Submitted, 0.0),
        status_row(13, OrderStatusKind::Submitted),
    ];

    let Some(OrderLookup::Working { order, status }) = BY_ID.working(rows) else {
        panic!("not working");
    };
    assert_eq!(order.map(|order| order.order_state.status), Some(OrderStatusKind::Submitted));
    assert_eq!(status.map(|status| status.status), Some(OrderStatusKind::Submitted));
}

#[test]
fn finished_matches_client_and_order_or_perm_id() {
    use OrderStatusKind::*;
    let cases = [
        ("match", vec![order_row(7, 13, Cancelled, 40.0)], BY_ID, Some(Cancelled)),
        ("other client, same id", vec![order_row(8, 13, Cancelled, 0.0)], BY_ID, None),
        ("other order", vec![order_row(7, 14, Cancelled, 0.0)], BY_ID, None),
        (
            "perm id of another client",
            vec![order_row(8, 13, Filled, 100.0)],
            OrderKey::PermId(913),
            Some(Filled),
        ),
        ("other perm id", vec![order_row(7, 13, Filled, 100.0)], OrderKey::PermId(914), None),
        (
            "last match wins",
            vec![order_row(7, 13, Cancelled, 0.0), order_row(7, 13, Filled, 100.0)],
            BY_ID,
            Some(Filled),
        ),
        ("status rows are not records", vec![status_row(13, Filled)], BY_ID, None),
        ("empty reply", Vec::new(), BY_ID, None),
    ];
    for (name, rows, key, expected) in cases {
        let found = match key.finished(rows) {
            Some(OrderLookup::Finished { status, .. }) => Some(status.status),
            other => {
                assert_eq!(other, None, "{name}");
                None
            }
        };
        assert_eq!(found, expected, "{name}");
    }
}

#[test]
fn finished_status_is_built_from_the_order() {
    let row = order_row(7, 13, OrderStatusKind::Cancelled, 40.0);
    let Orders::OrderData(record) = &row else { unreachable!() };
    let record = record.clone();

    let found = BY_ID.finished(vec![row]);

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
    assert_eq!(
        found,
        Some(OrderLookup::Finished {
            order: record,
            status: expected
        })
    );
}

/// TWS sends a filled completed order's total as `0` (#1116), so nothing remains.
#[test]
fn finished_status_of_a_filled_order_has_nothing_remaining() {
    let Some(OrderLookup::Finished { status, .. }) = BY_ID.finished(vec![order_row(7, 13, OrderStatusKind::Filled, 100.0)]) else {
        panic!("not finished");
    };

    assert_eq!((status.filled, status.remaining), (100.0, 0.0));
}

#[test]
fn recheck_delay_ends_at_the_deadline_and_slows_after_the_window() {
    let start = Instant::now();
    let early = start + Duration::from_millis(300);
    let late = start + LOOKUP_RECHECK_WINDOW;
    let cases = [
        ("passed", Some(early), early, None),
        ("past", Some(early - Duration::from_millis(5)), early, None),
        ("far", Some(early + Duration::from_secs(3)), early, Some(LOOKUP_RECHECK_INTERVAL)),
        ("near", Some(early + Duration::from_millis(30)), early, Some(Duration::from_millis(30))),
        ("no deadline", None, early, Some(LOOKUP_RECHECK_INTERVAL)),
        (
            "far after the window",
            Some(late + Duration::from_secs(30)),
            late,
            Some(LOOKUP_RECHECK_INTERVAL_SLOW),
        ),
        (
            "near after the window",
            Some(late + Duration::from_millis(400)),
            late,
            Some(Duration::from_millis(400)),
        ),
        ("no deadline after the window", None, late, Some(LOOKUP_RECHECK_INTERVAL_SLOW)),
    ];
    for (name, deadline, now, expected) in cases {
        assert_eq!(recheck_delay(deadline, start, now), expected, "{name}");
    }
}
