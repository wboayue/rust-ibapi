//! Outbound message rate limiter (#950).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::common::lock;

/// Caps how fast a client sends messages to TWS / IB Gateway.
///
/// TWS limits the messages an API client may send (IBKR documents 50 per
/// second, rejecting the excess with error 100 and possibly disconnecting a
/// session that keeps exceeding it). The library enforces nothing unless a
/// limiter is installed with `ClientBuilder::rate_limiter`. With one installed,
/// a message over the budget is delayed, never rejected: the sync client
/// blocks the sending thread, the async client awaits without blocking the
/// runtime.
///
/// # What is counted
///
/// Every message the client sends after the connection handshake: requests,
/// orders and cancels. The handshake itself, including reconnects, is not.
///
/// Cancels (`Cancel*` messages, the global order cancel) count against the
/// budget but are sent at once, so unwanted data stops immediately and the
/// next request takes the delay. Cancels expressed as a request with an off
/// flag, such as account updates with `subscribe = false`, wait like requests.
///
/// # The limit
///
/// [`per_second(n)`](Self::per_second) sends at most `n` messages in any
/// one-second window: a small burst goes out at once, the rest at an even
/// pace. Sending cancels can briefly go above `n`; the requests after them
/// wait it out. [`Default`] is `per_second(50)`, IBKR's documented rate.
///
/// IBKR ties the rate to market-data lines (about lines / 2 per second), so
/// accounts with booster packs may allow more. The API does not report an
/// account's lines, so set a higher `n` yourself if yours allows it. A limit
/// that is too low only delays requests.
///
/// # Sharing
///
/// IBKR applies the limit to all clients of one TWS / IB Gateway together.
/// Clones of a limiter share one budget, so pass clones to every client
/// connected to the same gateway. Sync and async clients can share one.
///
/// An async send that is cancelled while waiting (dropped by a timeout, say)
/// still uses its slot.
///
/// Historical-data pacing (error 162) and market-data lines (error 101) are
/// separate limits this does not enforce.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn run() -> Result<(), ibapi::Error> {
/// use ibapi::{Client, RateLimiter};
///
/// // Two clients on one gateway, 50 messages per second between them.
/// let limiter = RateLimiter::default();
/// let trading = Client::builder()
///     .address("127.0.0.1:4002")
///     .client_id(100)
///     .rate_limiter(limiter.clone())
///     .connect()
///     .await?;
/// let research = Client::builder()
///     .address("127.0.0.1:4002")
///     .client_id(101)
///     .rate_limiter(limiter)
///     .connect()
///     .await?;
/// # drop((trading, research));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct RateLimiter {
    inner: Arc<Mutex<Gcra>>,
}

impl RateLimiter {
    /// At most `n` messages in any one-second window. `0` is rejected when
    /// the client connects.
    pub fn per_second(n: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Gcra::per_second(n))),
        }
    }

    /// The `n` this limiter was built with.
    pub(crate) fn messages_per_second(&self) -> u32 {
        self.state().per_second
    }

    /// Reserve a send slot at `now`; returns how long to wait before sending.
    /// A cancel takes a slot but never waits.
    pub(crate) fn reserve(&self, now: Instant, is_cancel: bool) -> Duration {
        let wait = self.state().reserve(now);
        if is_cancel {
            Duration::ZERO
        } else {
            wait
        }
    }

    /// Block the calling thread until a slot is free.
    #[cfg(feature = "sync")]
    pub(crate) fn acquire_blocking(&self, is_cancel: bool) {
        let wait = self.reserve(Instant::now(), is_cancel);
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
    }

    /// Wait, without blocking the runtime, until a slot is free.
    #[cfg(feature = "async")]
    pub(crate) async fn acquire(&self, is_cancel: bool) {
        let wait = self.reserve(Instant::now(), is_cancel);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    /// When the next request could go out without waiting, if any slot was
    /// ever reserved.
    #[cfg(test)]
    pub(crate) fn reserved_until(&self) -> Option<Instant> {
        self.state().tat
    }

    fn state(&self) -> std::sync::MutexGuard<'_, Gcra> {
        lock(&self.inner)
    }
}

impl Default for RateLimiter {
    /// `per_second(50)`, the rate IBKR documents.
    fn default() -> Self {
        Self::per_second(50)
    }
}

/// Token bucket in GCRA form: one theoretical arrival time instead of a token
/// count. Each send moves `tat` one `interval` later; a send may go out once
/// `tat` is within `tolerance` of now, which admits `burst` sends at once.
#[derive(Debug)]
struct Gcra {
    per_second: u32,
    interval: Duration,
    tolerance: Duration,
    tat: Option<Instant>,
}

impl Gcra {
    /// A burst of `max(1, n / 10)` then `n - burst + 1` per second. GCRA admits
    /// `burst + rate - 1` sends in a one-second window when the interval
    /// rounds up, which is `n`.
    fn per_second(n: u32) -> Self {
        let burst = (n / 10).max(1);
        let rate = (n + 1).saturating_sub(burst).max(1);
        let interval = Duration::from_nanos(1_000_000_000u64.div_ceil(u64::from(rate)));
        Self {
            per_second: n,
            interval,
            tolerance: interval * (burst - 1),
            tat: None,
        }
    }

    fn reserve(&mut self, now: Instant) -> Duration {
        let tat = self.tat.map_or(now, |tat| tat.max(now));
        let send_at = tat.checked_sub(self.tolerance).map_or(now, |earliest| earliest.max(now));
        self.tat = Some(tat + self.interval);
        send_at - now
    }
}

#[cfg(test)]
#[path = "rate_limiter_tests.rs"]
mod tests;
