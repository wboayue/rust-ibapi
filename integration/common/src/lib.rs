use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// === Client ID Pool ===

static POOL: Mutex<Option<BTreeSet<i32>>> = Mutex::new(None);

const ID_RANGE: std::ops::Range<i32> = 200..400;

/// RAII guard for a unique client ID. Returns the ID to the pool on drop.
pub struct ClientId(i32);

impl ClientId {
    /// Acquire the lowest available client ID from the pool.
    ///
    /// # Panics
    /// Panics if all IDs in the pool are currently in use.
    pub fn get() -> Self {
        let mut pool = POOL.lock().unwrap();
        let set = pool.get_or_insert_with(|| ID_RANGE.collect());
        let id = *set.iter().next().expect("client ID pool exhausted");
        set.remove(&id);
        ClientId(id)
    }

    pub fn id(&self) -> i32 {
        self.0
    }
}

impl Drop for ClientId {
    fn drop(&mut self) {
        if let Ok(mut pool) = POOL.lock() {
            if let Some(set) = pool.as_mut() {
                set.insert(self.0);
            }
        }
    }
}

// === Rate Limiter ===

const MAX_TOKENS: f64 = 50.0;
const REFILL_RATE: f64 = 50.0;

static RATE_LIMITER: Mutex<Option<TokenBucket>> = Mutex::new(None);

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new() -> Self {
        Self {
            tokens: MAX_TOKENS,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * REFILL_RATE).min(MAX_TOKENS);
        self.last_refill = now;
    }
}

/// Block until a request token is available. Enforces 50 req/sec limit.
pub fn rate_limit() {
    loop {
        let mut guard = RATE_LIMITER.lock().unwrap();
        let bucket = guard.get_or_insert_with(TokenBucket::new);
        bucket.refill();
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return;
        }
        let wait = Duration::from_secs_f64((1.0 - bucket.tokens) / REFILL_RATE);
        drop(guard);
        std::thread::sleep(wait);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_unique_ids() {
        let a = ClientId::get();
        let b = ClientId::get();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn returns_id_on_drop() {
        let id = {
            let c = ClientId::get();
            c.id()
        };
        let d = ClientId::get();
        assert!(d.id() <= id);
    }

    #[test]
    fn ids_in_expected_range() {
        let c = ClientId::get();
        assert!(ID_RANGE.contains(&c.id()));
    }

    #[test]
    fn rate_limit_does_not_block_under_capacity() {
        let start = Instant::now();
        for _ in 0..10 {
            rate_limit();
        }
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
