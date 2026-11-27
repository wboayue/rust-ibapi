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
fn account_request_reserves_one_slot_and_replay_follows_the_actual_unsubscribe() {
    let limiter = RateLimiter::per_second(1);
    let start = Instant::now();
    assert_eq!(limiter.reserve(start, false), Duration::ZERO);
    assert_eq!(limiter.reserved_until(), Some(start + SECOND));

    let off = start + limiter.reserve(start, false);
    assert_eq!(off, start + SECOND);
    // Account for a writer that flushes after the original send slot.
    let flushed = off + SECOND * 2;
    limiter.observe_account_write(off, flushed);
    let on = flushed + limiter.reserve(flushed, false);
    assert_eq!(on, flushed + SECOND);
    assert_eq!(limiter.reserved_until(), Some(on + SECOND));
}

#[test]
fn account_write_delay_preserves_existing_rate_and_burst_budget() {
    let start = Instant::now();
    let limiter = RateLimiter::per_second(2);
    let half = SECOND / 2;
    for (permit, actual) in [(start, start + SECOND), (start + SECOND + half, start + SECOND + half)] {
        let reserved = if permit == start { start } else { start + SECOND };
        assert_eq!(reserved + limiter.reserve(reserved, false), permit);
        limiter.observe_account_write(permit, actual);
    }
    assert_eq!(limiter.reserved_until(), Some(start + 2 * SECOND));

    let burst = RateLimiter::per_second(20);
    assert_eq!(burst.reserve(start, false), Duration::ZERO);
    burst.observe_account_write(start, start);
    assert_eq!(burst.reserve(start, false), Duration::ZERO);
    assert_eq!(burst.reserve(start, false), Duration::from_nanos(52_631_579));
}

#[test]
fn late_account_write_does_not_shift_another_clients_reservations() {
    let slow = RateLimiter::per_second(2);
    let fast = slow.clone();
    let start = Instant::now();
    let ten_seconds_later = start + 10 * SECOND;

    assert_eq!(slow.reserve(start, false), Duration::ZERO);
    assert_eq!(fast.reserve(ten_seconds_later, false), Duration::ZERO);
    let completed = ten_seconds_later + Duration::from_millis(100);
    slow.observe_account_write(start, completed);

    // Both frames draw on this window. The old writer's ten-second delay
    // must not move the fast client's accumulated debt by ten seconds.
    assert_eq!(fast.reserved_until(), Some(ten_seconds_later + SECOND));
    assert_eq!(fast.reserve(completed, false), Duration::from_millis(900));
    assert_eq!(fast.reserve(completed, true), Duration::ZERO);
    assert_eq!(fast.reserved_until(), Some(ten_seconds_later + 2 * SECOND));
}

#[test]
fn account_write_short_delay_preserves_remaining_debt() {
    let slow = RateLimiter::per_second(2);
    let fast = slow.clone();
    let start = Instant::now();
    let completed = start + Duration::from_millis(100);

    assert_eq!(slow.reserve(start, false), Duration::ZERO);
    assert_eq!(fast.reserve(start, false), Duration::from_millis(500));
    slow.observe_account_write(start, completed);

    assert_eq!(fast.reserved_until(), Some(completed + SECOND));
    assert_eq!(fast.reserve(completed, false), SECOND);
}

#[test]
fn timely_account_write_after_idle_uses_one_slot() {
    let limiter = RateLimiter::per_second(2);
    let start = Instant::now();
    let resumed = start + 10 * SECOND;

    assert_eq!(limiter.reserve(start, false), Duration::ZERO);
    assert_eq!(limiter.reserve(resumed, false), Duration::ZERO);
    limiter.observe_account_write(resumed, resumed);
    assert_eq!(limiter.reserved_until(), Some(resumed + SECOND / 2));
    assert_eq!(limiter.reserve(resumed, false), SECOND / 2);
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
