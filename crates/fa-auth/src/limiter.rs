use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// In-memory per-IP token bucket. One token per login attempt.
#[derive(Debug)]
pub struct IpRateLimiter {
    burst: f64,
    refill: Duration,
    buckets: Mutex<HashMap<IpAddr, (f64, Instant)>>,
}

impl IpRateLimiter {
    pub fn new(burst: u32, refill: Duration) -> Self {
        Self {
            burst: f64::from(burst.max(1)),
            refill,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Takes a token; false when the IP is out of budget.
    pub fn check(&self, ip: IpAddr) -> bool {
        self.check_at(ip, Instant::now())
    }

    fn check_at(&self, ip: IpAddr, now: Instant) -> bool {
        let per_sec = 1.0 / self.refill.as_secs_f64().max(f64::EPSILON);
        let Ok(mut map) = self.buckets.lock() else {
            return true;
        };
        if map.len() > 10_000 {
            let burst = self.burst;
            map.retain(|_, (t, at)| {
                (*t + now.saturating_duration_since(*at).as_secs_f64() * per_sec) < burst
            });
        }
        let (tokens, at) = map.entry(ip).or_insert((self.burst, now));
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
}
