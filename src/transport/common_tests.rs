use super::*;

#[test]
fn test_is_benign_connectivity_notice() {
    // Data-farm-OK confirmations: System Notifications, not warnings.
    assert!(is_benign_connectivity_notice(2104));
    assert!(is_benign_connectivity_notice(2106));
    assert!(is_benign_connectivity_notice(2158));

    // Other codes in WARNING_CODE_RANGE are real warnings (e.g. connection
    // broken/inactive), not benign.
    assert!(!is_benign_connectivity_notice(2100));
    assert!(!is_benign_connectivity_notice(2103)); // Market data farm connection is broken
    assert!(!is_benign_connectivity_notice(2105)); // HMDS data farm connection is broken
    assert!(!is_benign_connectivity_notice(2157)); // Sec-def data farm connection is broken
    assert!(!is_benign_connectivity_notice(2169));

    // Outside the warning range entirely.
    assert!(!is_benign_connectivity_notice(200));
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
