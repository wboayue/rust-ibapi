use super::*;
use crate::messages::{CONNECTIVITY_LOST_CODE, FARM_OK_CODES};

#[test]
fn test_is_benign_connectivity_notice() {
    // Logging-policy invariant: only ConnectivityStatus::Ok (data-farm-OK
    // confirmations) and system code 1102 (restored, data maintained) are
    // benign → info. Broken/Inactive/Connecting stay at warn.
    for code in FARM_OK_CODES {
        let notice = Notice::synthesized(code, "farm OK".into());
        assert!(is_benign_connectivity_notice(&notice), "code {code} should be benign");
    }
    // 1102: connectivity restored, market data maintained — nothing lost.
    let notice = Notice::synthesized(CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE, "restored, data maintained".into());
    assert!(is_benign_connectivity_notice(&notice), "code 1102 should be benign");

    // Not benign: broken codes (Broken), inactive/connecting codes (still warn),
    // the range boundaries, a code outside WARNING_CODE_RANGE entirely, and the
    // non-benign system codes (1100 lost, 1101 restored-but-data-lost).
    for code in [
        2100,
        2103, // Market data farm connection is broken
        2105, // HMDS data farm connection is broken
        2157, // Sec-def data farm connection is broken
        2107,
        2108, // inactive but available on demand — not benign
        2119, // connecting — not benign
        2169,
        200,                                  // outside / boundary
        CONNECTIVITY_LOST_CODE,               // 1100 — hard error
        CONNECTIVITY_RESTORED_DATA_LOST_CODE, // 1101 — warn (resubscribe)
    ] {
        let notice = Notice::synthesized(code, "not benign".into());
        assert!(!is_benign_connectivity_notice(&notice), "code {code} should not be benign");
    }
}

#[test]
fn test_log_unrouted_notice_traverses_all_severities() {
    // Smoke test: the project has no log-capture harness, so we can't assert the
    // emitted level. Drive each branch of log_unrouted_notice to confirm the
    // benign (info), warning (warn), and error paths are reachable and panic-free.
    log_unrouted_notice(&Notice::synthesized(FARM_OK_CODES[0], "farm OK".into()));
    log_unrouted_notice(&Notice::synthesized(CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE, "1102 info".into()));
    log_unrouted_notice(&Notice::synthesized(2103, "farm broken".into()));
    log_unrouted_notice(&Notice::synthesized(CONNECTIVITY_RESTORED_DATA_LOST_CODE, "1101 warn".into()));
    log_unrouted_notice(&Notice::synthesized(CONNECTIVITY_LOST_CODE, "1100 error".into()));
    log_unrouted_notice(&Notice::synthesized(200, "no security definition".into()));
}

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

/// Collects everything delivered to the notice sink.
#[derive(Default)]
struct CapturingSink {
    notices: std::sync::Mutex<Vec<Notice>>,
}

impl NoticeSink for CapturingSink {
    fn deliver(&self, notice: Notice) {
        self.notices.lock().unwrap().push(notice);
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

    let notices = sink.notices.lock().unwrap();
    assert_eq!(notices.len(), 1, "an unknown kind must be observable, not just logged");
    assert_eq!(notices[0].code, UNKNOWN_MESSAGE_TYPE_CODE);
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
        sink.notices.lock().unwrap().is_empty(),
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
