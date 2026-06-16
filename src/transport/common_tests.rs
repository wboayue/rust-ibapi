use super::*;

#[test]
fn test_is_benign_connectivity_notice() {
    // Data-farm-OK confirmations: System Notifications, not warnings.
    for code in BENIGN_CONNECTIVITY_CODES {
        assert!(is_benign_connectivity_notice(code), "code {code} should be benign");
    }

    // Other codes are not benign: the matching "connection broken" codes inside
    // WARNING_CODE_RANGE, the range boundaries, and a code outside the range.
    for code in [
        2100, 2103, // Market data farm connection is broken
        2105, // HMDS data farm connection is broken
        2157, // Sec-def data farm connection is broken
        2169, 200, // outside WARNING_CODE_RANGE entirely
    ] {
        assert!(!is_benign_connectivity_notice(code), "code {code} should not be benign");
    }
}

#[test]
fn test_log_unrouted_notice_traverses_all_severities() {
    // Smoke test: the project has no log-capture harness, so we can't assert the
    // emitted level. Drive each branch of log_unrouted_notice to confirm the
    // benign (info), warning (warn), and error paths are reachable and panic-free.
    log_unrouted_notice(&Notice::synthesized(BENIGN_CONNECTIVITY_CODES[0], "farm OK".into()));
    log_unrouted_notice(&Notice::synthesized(2103, "farm broken".into()));
    log_unrouted_notice(&Notice::synthesized(200, "no security definition".into()));
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
