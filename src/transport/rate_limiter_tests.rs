use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::RateLimiter;

const SECOND: Duration = Duration::from_secs(1);

/// Greedy sender: each send goes out at `now + wait`, and the next is
/// attempted right then. Returns the send times.
fn greedy_sends(limiter: &RateLimiter, start: Instant, count: usize) -> Vec<Instant> {
    let mut now = start;
    (0..count)
        .map(|_| {
            now += limiter.reserve(now, false);
            now
        })
        .collect()
}

/// Most sends in any half-open one-second window.
fn max_in_any_second(sends: &[Instant]) -> usize {
    let mut window = VecDeque::new();
    let mut max = 0;
    for &t in sends {
        window.push_back(t);
        while let Some(&first) = window.front() {
            if t - first >= SECOND {
                window.pop_front();
            } else {
                break;
            }
        }
        max = max.max(window.len());
    }
    max
}

#[test]
fn burst_goes_out_at_once_then_paced() {
    let limiter = RateLimiter::per_second(50);
    let start = Instant::now();
    for i in 0..5 {
        assert_eq!(limiter.reserve(start, false), Duration::ZERO, "burst send {i}");
    }
    // Burst of 5, then 46/s.
    let wait = limiter.reserve(start, false);
    assert!(wait > Duration::ZERO && wait <= SECOND / 46 + Duration::from_nanos(1), "{wait:?}");
}

#[test]
fn never_more_than_n_in_any_second() {
    for n in [1, 2, 5, 9, 10, 11, 50, 99, 100, 250] {
        let limiter = RateLimiter::per_second(n);
        let sends = greedy_sends(&limiter, Instant::now(), n as usize * 5);
        assert_eq!(max_in_any_second(&sends), n as usize, "per_second({n})");
    }
}

#[test]
fn idle_refill_caps_at_burst() {
    let limiter = RateLimiter::per_second(50);
    let start = Instant::now();
    greedy_sends(&limiter, start, 100);
    // Long idle: only the burst goes out without waiting.
    let later = start + Duration::from_secs(60);
    for i in 0..5 {
        assert_eq!(limiter.reserve(later, false), Duration::ZERO, "burst send {i}");
    }
    assert!(limiter.reserve(later, false) > Duration::ZERO);
}

#[test]
fn cancel_never_waits_but_takes_a_slot() {
    let limiter = RateLimiter::per_second(10);
    let start = Instant::now();
    assert_eq!(limiter.reserve(start, false), Duration::ZERO); // burst of 1
    let request_wait = limiter.reserve(start, false);
    assert!(request_wait > Duration::ZERO);

    assert_eq!(limiter.reserve(start, true), Duration::ZERO, "cancel waits");
    // The request after the cancel waits one interval longer than it would have.
    let after_cancel = limiter.reserve(start, false);
    assert_eq!(after_cancel, request_wait * 3);
}

#[test]
fn clones_share_one_budget() {
    let a = RateLimiter::per_second(10);
    let b = a.clone();
    let start = Instant::now();
    assert_eq!(a.reserve(start, false), Duration::ZERO);
    assert!(b.reserve(start, false) > Duration::ZERO, "clone did not see the first send");
    assert_eq!(a.reserved_until(), b.reserved_until());
}

#[test]
fn separate_limiters_are_independent() {
    let a = RateLimiter::per_second(10);
    let b = RateLimiter::per_second(10);
    let start = Instant::now();
    assert_eq!(a.reserve(start, false), Duration::ZERO);
    assert_eq!(b.reserve(start, false), Duration::ZERO);
}

#[test]
fn default_is_fifty_per_second() {
    assert_eq!(RateLimiter::default().messages_per_second(), 50);
    assert_eq!(RateLimiter::per_second(0).messages_per_second(), 0);
}
