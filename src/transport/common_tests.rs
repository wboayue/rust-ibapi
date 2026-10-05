use super::*;
use crate::common::test_utils::helpers::CapturingSink;
use crate::messages::UNKNOWN_MESSAGE_TYPE_CODE;

#[test]
fn test_validate_frame_length_accepts_the_legal_range() {
    // Boundaries derived from the constants, not restated: a body holding only
    // the message id is the smallest legal frame, and the C#-matching cap is
    // inclusive (`EReader` rejects on `>` MaxMsgSize).
    for length in [MIN_FRAME_LENGTH, MIN_FRAME_LENGTH + 1, MAX_FRAME_LENGTH - 1, MAX_FRAME_LENGTH] {
        assert_eq!(validate_frame_length(length).unwrap(), length, "length {length} should be accepted");
    }
}

#[test]
fn test_validate_frame_length_rejects_out_of_range_lengths() {
    // Below: a body that cannot hold the message id. Above: the desync
    // signature — four garbage bytes read as a length, which unbounded sizes an
    // allocation of up to 4 GiB and then consumes every real message until it
    // is satisfied.
    for length in [0, MIN_FRAME_LENGTH - 1, MAX_FRAME_LENGTH + 1, u32::MAX as usize] {
        let err = validate_frame_length(length).expect_err("an out-of-range length must be rejected");
        assert!(
            matches!(err, Error::InvalidFrame(_)),
            "length {length} must raise InvalidFrame, got {err:?}"
        );
        assert!(err.is_connection_lost(), "a desynchronized stream must drive a reconnect");
    }
}

#[test]
fn test_report_unroutable_frame_raises_a_notice_for_an_unknown_kind() {
    // Message id 9999 maps to no IncomingMessages variant, which is what a
    // desynchronized read produces: garbage where the id should be.
    let message = ResponseMessage::from("9999\01\0");
    assert_eq!(message.message_type(), IncomingMessages::NotValid, "fixture must be unroutable");

    let sink = CapturingSink::default();
    report_unroutable_frame(&message, &sink);

    let notices = sink.notices();
    assert_eq!(notices.len(), 1, "an unknown kind must be observable, not just logged");
    assert_eq!(notices[0].code, UNKNOWN_MESSAGE_TYPE_CODE);
    // Naming the id is the whole point: scattered ids mean the framing slipped,
    // one repeated id means IBKR added a message type. Without it the notice
    // cannot tell those apart.
    assert!(
        notices[0].message.contains("9999"),
        "notice must name the offending id, got {:?}",
        notices[0].message
    );
}

#[test]
fn test_report_unroutable_frame_stays_quiet_for_a_known_kind() {
    // A known type with no current subscriber is ordinary steady state — it
    // must not raise a desync notice, or the signal is worthless.
    let message = ResponseMessage::from("15\01\0DU1234567\0");
    assert_eq!(message.message_type(), IncomingMessages::ManagedAccounts);

    let sink = CapturingSink::default();
    report_unroutable_frame(&message, &sink);

    assert!(
        sink.notices().is_empty(),
        "a known kind with no listener is routine and must raise no notice"
    );
}

#[test]
fn test_fibonacci_backoff() {
    let mut backoff = FibonacciBackoff::new(10);

    assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    assert_eq!(backoff.next_delay(), Duration::from_secs(2));
    assert_eq!(backoff.next_delay(), Duration::from_secs(3));
    assert_eq!(backoff.next_delay(), Duration::from_secs(5));
    assert_eq!(backoff.next_delay(), Duration::from_secs(8));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10)); // capped at max
    assert_eq!(backoff.next_delay(), Duration::from_secs(10)); // stays at max
}

/// The raw Fibonacci sequence overflows u64 after ~93 steps, which can be
/// reached during a long outage with a large `max_reconnect_attempts`. The
/// internal steps must be clamped at `max` to avoid u64 overflow regardless of
/// the number of calls. Without that, this test will fail in debug builds with
/// `attempt to add with overflow` at call 93.
#[test]
fn test_fibonacci_backoff_never_overflows() {
    let mut backoff = FibonacciBackoff::new(30);
    for _ in 0..100 {
        assert!(backoff.next_delay() <= Duration::from_secs(30));
    }
    assert_eq!(backoff.next_delay(), Duration::from_secs(30));
}

/// A `max` above fib(93) lets the raw sum overflow before the clamp can
/// engage; `saturating_add` must cover that. Without it, call ~93 panics with
/// `attempt to add with overflow` in debug builds.
#[test]
fn test_fibonacci_backoff_never_overflows_with_huge_max() {
    let mut backoff = FibonacciBackoff::new(u64::MAX);
    for _ in 0..100 {
        backoff.next_delay();
    }
    assert_eq!(backoff.next_delay(), Duration::from_secs(u64::MAX));
}

/// `max: 0` means no delay, not a fixed 1s: `current` starts clamped at
/// `max`, so the delay must respect `max` from the first call.
#[test]
fn test_fibonacci_backoff_zero_max() {
    let mut backoff = FibonacciBackoff::new(0);
    assert_eq!(backoff.next_delay(), Duration::ZERO);
    assert_eq!(backoff.next_delay(), Duration::ZERO);
}

#[test]
fn lease_is_live_while_any_holder_is() {
    let lease = Lease::new();
    let lease_ref = lease.downgrade();
    let clone = lease.clone();

    drop(lease);
    assert!(lease_ref.is_live(), "a clone still holds the lease");

    drop(clone);
    assert!(!lease_ref.is_live(), "no holder left");
}

#[test]
fn lease_ref_identifies_its_lease() {
    let lease = Lease::new();
    let other = Lease::new();

    assert!(lease.downgrade().is(&lease.downgrade()));
    assert!(lease.downgrade().is(&lease.clone().downgrade()), "clones share identity");
    assert!(!lease.downgrade().is(&other.downgrade()));

    // Identity outlives the lease: a stale signal still matches its own
    // dead registration, and only that one.
    let lease_ref = lease.downgrade();
    drop(lease);
    assert!(lease_ref.is(&lease_ref.clone()));
    assert!(!lease_ref.is(&other.downgrade()));
}
