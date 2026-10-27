//! The optional dashboard-only listener for screens on the home network that cannot run Tailscale
//! (D-064). It wraps the normal router with a guard: clients must be in the allowed networks, only
//! the calls a dashboard makes are routed, cookies are dropped, and [`crate::Auth`] then accepts
//! device tokens only.

use std::net::SocketAddr;

use axum::Router;
use axum::extract::connect_info::MockConnectInfo;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{Method, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};

use crate::{ApiError, AppState, IpNet};

#[derive(Debug, Clone)]
pub struct LanConfig {
    /// Clients outside these networks are refused.
    pub allowed: Vec<IpNet>,
    /// PIN elevation; off by default because the listener is plain HTTP.
    pub allow_pin: bool,
}

impl LanConfig {
    /// Private, link-local and loopback ranges.
    pub fn default_allowed() -> Vec<IpNet> {
        [
            "10.0.0.0/8",
            "172.16.0.0/12",
            "192.168.0.0/16",
            "169.254.0.0/16",
            "127.0.0.0/8",
            "fc00::/7",
            "fe80::/10",
            "::1/128",
        ]
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect()
    }
}

/// Request extension set on the LAN listener; [`crate::Auth`] then accepts devices only.
#[derive(Debug, Clone, Copy)]
pub struct LanOrigin;

const ELEVATE: &str = "device/elevate";

/// The `/api/v1` calls of the dashboard; `*` matches one path segment.
const ALLOWED: &[(Method, &str)] = &[
    (Method::GET, "me"),
    (Method::GET, "dashboard"),
    (Method::GET, "stream"),
    (Method::GET, "household"),
    (Method::GET, "task-lists"),
    (Method::GET, "members/*/avatar"),
    (Method::POST, "tasks"),
    (Method::POST, "tasks/*/complete"),
    (Method::POST, "tasks/*/reopen"),
    (Method::PATCH, "routine-runs/*/steps/*"),
    (Method::POST, ELEVATE),
];

fn matches(pattern: &str, path: &str) -> bool {
    let (mut p, mut s) = (pattern.split('/'), path.split('/'));
    loop {
        match (p.next(), s.next()) {
            (None, None) => return true,
            (Some("*"), Some(seg)) if !seg.is_empty() => {}
            (Some(a), Some(b)) if a == b => {}
            _ => return false,
        }
    }
}

/// The normal router behind the LAN guard.
pub fn lan_router(state: AppState, cfg: LanConfig) -> Router {
    crate::router(state).layer(middleware::from_fn_with_state(cfg, guard))
}

async fn guard(State(cfg): State<LanConfig>, mut req: Request, next: Next) -> Response {
    // Only tests set `MockConnectInfo`.
    let ext = req.extensions();
    let peer = ext
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .or_else(|| ext.get::<MockConnectInfo<SocketAddr>>().map(|m| m.0.ip()));
    if !peer.is_some_and(|ip| cfg.allowed.iter().any(|n| n.contains(ip))) {
        return ApiError::Forbidden("this address is not allowed".into()).into_response();
    }
    let path = req.uri().path();
    if let Some(api) = path.strip_prefix("/api/v1/") {
        if !ALLOWED
            .iter()
            .any(|(m, p)| m == req.method() && matches(p, api))
        {
            return ApiError::NotFound.into_response();
        }
        if api == ELEVATE && !cfg.allow_pin {
            return ApiError::Forbidden(
                "PIN entry is turned off on the home-network address".into(),
            )
            .into_response();
        }
    } else if path.starts_with("/api/") || !matches!(*req.method(), Method::GET | Method::HEAD) {
        return ApiError::NotFound.into_response();
    }
    let headers = req.headers_mut();
    headers.remove(header::COOKIE);
    // The client IP is the peer itself: no proxy sits in front of this listener.
    headers.remove("x-forwarded-for");
    headers.remove("x-forwarded-proto");
    headers.remove("x-forwarded-host");
    req.extensions_mut().insert(LanOrigin);
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn patterns_match_whole_segments() {
        assert!(matches("tasks/*/complete", "tasks/abc/complete"));
        assert!(!matches("tasks/*/complete", "tasks//complete"));
        assert!(!matches("tasks/*/complete", "tasks/abc/complete/x"));
        assert!(!matches("tasks", "tasks/abc"));
        assert!(matches("routine-runs/*/steps/*", "routine-runs/r/steps/s"));
        assert!(!matches("me", "members"));
    }
}
