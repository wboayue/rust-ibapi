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
