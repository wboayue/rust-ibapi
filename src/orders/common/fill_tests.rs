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
            Some(OrderOutcome::PendingCreation(_)) => panic!("{name}: a status is never a 1104"),
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

#[test]
fn notice_ends_the_wait_only_for_1104() {
    let notice = |code: i32| Notice {
        code,
        message: format!("notice {code}"),
        request_id: None,
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let pending = notice(crate::messages::ORDER_CREATION_PENDING_CODE);

    assert_eq!(FillTracker::notice(pending.clone()), Some(OrderOutcome::PendingCreation(pending)));
    for code in [399, 2109, 1100, 1102] {
        assert_eq!(FillTracker::notice(notice(code)), None, "{code}");
    }
}

#[test]
fn lookup_wait_is_at_least_the_minimum() {
    assert_eq!(lookup_wait(Instant::now()), LOOKUP_MIN_WAIT);
    assert!(lookup_wait(Instant::now() + Duration::from_secs(30)) > Duration::from_secs(29));
}

/// A completed record's status alone decides, so a `totalQuantity` TWS left
/// out (decoded `0`) can't turn a partly filled, cancelled order into a fill.
#[test]
fn observe_completed_goes_by_status() {
    use OrderStatusKind::*;
    let cases = [
        ("filled", status(Filled, 100.0, 0.0), Some("Filled")),
        ("cancelled after a partial fill", status(Cancelled, 40.0, 60.0), Some("Ended")),
        ("cancelled, total left out", status(Cancelled, 40.0, 0.0), Some("Ended")),
        ("inactive", status(Inactive, 0.0, 100.0), Some("Ended")),
        ("still working", status(Submitted, 40.0, 0.0), None),
    ];
    for (name, input, expected) in cases {
        let outcome = FillTracker::default().observe_completed(input);
        let variant = outcome.map(|o| match o {
            OrderOutcome::Filled(_) => "Filled",
            OrderOutcome::Ended(_) => "Ended",
            OrderOutcome::TimedOut(_) => "TimedOut",
            OrderOutcome::Rejected(_) => "Rejected",
            OrderOutcome::PendingCreation(_) => "PendingCreation",
        });
        assert_eq!(variant, expected, "{name}");
    }
}
