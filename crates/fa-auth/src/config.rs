use std::time::Duration;

use fa_domain::Role;

const DAY: u64 = 24 * 60 * 60;

/// Tunables. Defaults follow D-019 / D-031.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Used for `pair_url` (no trailing slash needed).
    pub public_base_url: String,
    pub adult_session_ttl: Duration,
    pub child_session_ttl: Duration,
    /// Failed passwords before the account locks.
    pub max_login_failures: i32,
    pub login_lock: Duration,
    /// Failed PINs before the member's PIN locks.
    pub max_pin_failures: i32,
    pub pin_lock: Duration,
    pub elevation_ttl: Duration,
    /// Per-IP token bucket: burst size and refill interval per token.
    pub ip_burst: u32,
    pub ip_refill: Duration,
    /// `device.last_seen` is written at most this often.
    pub device_touch_interval: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            public_base_url: "http://localhost:8080".into(),
            adult_session_ttl: Duration::from_secs(30 * DAY),
            child_session_ttl: Duration::from_secs(180 * DAY),
            max_login_failures: 5,
            login_lock: Duration::from_secs(15 * 60),
            max_pin_failures: 5,
            pin_lock: Duration::from_secs(15 * 60),
            elevation_ttl: Duration::from_secs(5 * 60),
            ip_burst: 20,
            ip_refill: Duration::from_secs(30),
            device_touch_interval: Duration::from_secs(60),
        }
    }
}

impl AuthConfig {
    pub fn session_ttl(&self, role: Role) -> Duration {
        match role {
            Role::Adult => self.adult_session_ttl,
            Role::Child => self.child_session_ttl,
        }
    }
}
