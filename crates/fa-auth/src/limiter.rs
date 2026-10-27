use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Per-IP limiter for login.
pub type IpRateLimiter = KeyedRateLimiter<IpAddr>;

/// In-memory per-key token bucket. One token per attempt.
#[derive(Debug)]
pub struct KeyedRateLimiter<K> {
    burst: f64,
    refill: Duration,
    buckets: Mutex<HashMap<K, (f64, Instant)>>,
}

impl<K: Hash + Eq + Clone> KeyedRateLimiter<K> {
    pub fn new(burst: u32, refill: Duration) -> Self {
        Self {
            burst: f64::from(burst.max(1)),
            refill,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Takes a token; false when the IP is out of budget.
    pub fn check(&self, key: K) -> bool {
        self.check_at(key, Instant::now())
    }

    /// Returns a token taken by `check`, for an attempt that turned out legitimate.
    pub fn refund(&self, key: &K) {
        let mut map = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((tokens, _)) = map.get_mut(key) {
            *tokens = (*tokens + 1.0).min(self.burst);
        }
    }

    fn check_at(&self, key: K, now: Instant) -> bool {
        let per_sec = 1.0 / self.refill.as_secs_f64().max(f64::EPSILON);
        let mut map = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        if map.len() > 10_000 {
            let burst = self.burst;
            map.retain(|_, (t, at)| {
                (*t + now.saturating_duration_since(*at).as_secs_f64() * per_sec) < burst
            });
        }
        let (tokens, at) = map.entry(key).or_insert((self.burst, now));
        let elapsed = now.saturating_duration_since(*at).as_secs_f64();
        *tokens = (*tokens + elapsed * per_sec).min(self.burst);
        *at = now;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_drains_and_refills() {
        let l = IpRateLimiter::new(2, Duration::from_secs(10));
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let other: IpAddr = "192.0.2.2".parse().unwrap();
        let t0 = Instant::now();
        assert!(l.check_at(ip, t0));
        assert!(l.check_at(ip, t0));
        assert!(!l.check_at(ip, t0));
        assert!(l.check_at(other, t0));
        assert!(l.check_at(ip, t0 + Duration::from_secs(10)));
        assert!(!l.check_at(ip, t0 + Duration::from_secs(10)));
    }

    #[test]
    fn keyed_by_arbitrary_key() {
        let l: KeyedRateLimiter<u32> = KeyedRateLimiter::new(1, Duration::from_secs(60));
        let t0 = Instant::now();
        assert!(l.check_at(1, t0));
        assert!(!l.check_at(1, t0));
        assert!(l.check_at(2, t0));
    }

    #[test]
    fn refund_returns_a_token_up_to_the_burst() {
        let l = KeyedRateLimiter::new(1, Duration::from_secs(3600));
        assert!(l.check(7u8));
        assert!(!l.check(7u8));
        l.refund(&7u8);
        assert!(l.check(7u8));
        l.refund(&7u8);
        l.refund(&7u8);
        assert!(l.check(7u8));
        assert!(!l.check(7u8));
    }
}
