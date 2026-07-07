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
