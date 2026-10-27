//! How a request reached us: scheme, origin and whether to mark the cookie `Secure`.
//!
//! TLS is never terminated here. A request counts as secure only when `X-Forwarded-Proto: https`
//! arrives from a peer in `ApiConfig::trusted_proxies` (a `tailscale serve` sidecar, say). The
//! header from any other peer is ignored. `Host`, or `X-Forwarded-Host` from a trusted proxy,
//! gives the origin. No HSTS is ever sent (D-072).

use std::net::{IpAddr, SocketAddr};

use axum::extract::connect_info::MockConnectInfo;
use axum::extract::{ConnectInfo, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap, header::HOST};
use axum_extra::extract::cookie::{Cookie, SameSite};
use fa_config::CookieSecure;

use crate::{ApiConfig, AppState, SESSION_COOKIE};

fn peer_ip(ext: &Extensions) -> Option<IpAddr> {
    ext.get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .or_else(|| ext.get::<MockConnectInfo<SocketAddr>>().map(|m| m.0.ip()))
}

fn trusted(ext: &Extensions, cfg: &ApiConfig) -> bool {
    peer_ip(ext).is_some_and(|ip| cfg.trusted_proxies.iter().any(|n| n.contains(ip)))
}

fn header<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
    h.get(name)?.to_str().ok().map(str::trim)
}

fn secure_of(h: &HeaderMap, ext: &Extensions, cfg: &ApiConfig) -> bool {
    trusted(ext, cfg)
        && header(h, "x-forwarded-proto").is_some_and(|v| {
            v.split(',')
                .next_back()
                .is_some_and(|p| p.trim().eq_ignore_ascii_case("https"))
        })
}

/// True when the request came over TLS: `X-Forwarded-Proto: https` from a trusted proxy.
pub fn request_is_secure(req: &Request, cfg: &ApiConfig) -> bool {
    secure_of(req.headers(), req.extensions(), cfg)
}

fn host_ok(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 255
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

/// What a handler needs to know about the connection.
#[derive(Debug, Clone)]
pub(crate) struct Conn {
    pub secure: bool,
    /// `scheme://host[:port]`.
    pub origin: String,
    host: String,
}

impl Conn {
    pub(crate) fn from_parts(parts: &Parts, cfg: &ApiConfig) -> Self {
        let secure = secure_of(&parts.headers, &parts.extensions, cfg);
        let forwarded = trusted(&parts.extensions, cfg)
            .then(|| header(&parts.headers, "x-forwarded-host"))
            .flatten()
            .and_then(|v| v.split(',').next())
            .map(str::trim);
        let host = forwarded
            .or_else(|| header(&parts.headers, HOST.as_str()))
            .filter(|h| host_ok(h))
            .unwrap_or("localhost")
            .to_owned();
        let origin = format!("{}://{host}", if secure { "https" } else { "http" });
        Self {
            secure,
            origin,
            host,
        }
    }

    /// Secure and the host is a tailnet name.
    pub(crate) fn via_tailscale(&self) -> bool {
        let name = match self.host.rsplit_once(':') {
            Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) => h,
            _ => &self.host,
        };
        self.secure && name.to_ascii_lowercase().ends_with(".ts.net")
    }

    /// The session cookie (C-15): `HttpOnly`, `SameSite=Lax`, `Secure` per `cookie_secure`.
    /// `max_age` of 0 clears it.
    pub(crate) fn session_cookie(&self, cfg: &ApiConfig, value: &str, max_age: u64) -> String {
        // `time` is not re-exported by axum-extra, so Max-Age goes through the parser.
        let mut c = Cookie::parse(format!("{SESSION_COOKIE}={value}; Max-Age={max_age}"))
            .unwrap_or_else(|_| Cookie::new(SESSION_COOKIE, value.to_owned()));
        c.set_path("/");
        c.set_http_only(true);
        c.set_same_site(SameSite::Lax);
        c.set_secure(match cfg.cookie_secure {
            CookieSecure::Always => true,
            CookieSecure::Never => false,
            CookieSecure::Auto => self.secure,
        });
        c.to_string()
    }
}

impl FromRequestParts<AppState> for Conn {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_parts(parts, &state.cfg))
    }
}
