use axum::extract::{FromRequest, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use fa_domain::{Action, Actor, PolicyCtx, Resource, authorize};

use crate::{ApiError, AppState, SESSION_COOKIE};

/// The authenticated principal of a request (session cookie or device bearer token).
pub struct Auth {
    pub actor: Actor,
    /// The `fa_session` cookie value when present (needed by logout).
    pub session_id: Option<String>,
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let lan = parts.extensions.get::<crate::LanOrigin>().is_some();
        let jar = CookieJar::from_headers(&parts.headers);
        let session_id = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_owned())
            .filter(|_| !lan);
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim);
        let actor = state
            .auth
            .resolve_actor(session_id.as_deref(), bearer)
            .await?;
        // The home-network listener serves dashboards only (D-064).
        if lan && !matches!(actor, Actor::Device { .. } | Actor::ElevatedDevice { .. }) {
            return Err(ApiError::Unauthenticated);
        }
        Ok(Auth { actor, session_id })
    }
}

/// Runs the domain policy; denial becomes a 403.
pub fn require(actor: &Actor, action: Action, resource: &Resource) -> Result<(), ApiError> {
    authorize(actor, action, resource, &PolicyCtx::default())?;
    Ok(())
}

/// Like [`require`], with the household and list flags the M2 task policy depends on.
pub fn require_ctx(
    actor: &Actor,
    action: Action,
    resource: &Resource,
    ctx: &PolicyCtx,
) -> Result<(), ApiError> {
    authorize(actor, action, resource, ctx)?;
    Ok(())
}

/// JSON body whose parse failures render as problem+json.
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, ApiError> {
        Ok(Self(axum::Json::<T>::from_request(req, state).await?.0))
    }
}

pub struct ApiPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiPath<T>
where
    T: serde::de::DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        Ok(Self(
            axum::extract::Path::<T>::from_request_parts(parts, state)
                .await?
                .0,
        ))
    }
}

pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        Ok(Self(
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await?
                .0,
        ))
    }
}

/// Peer address of the connection, when the server provides it (absent in in-process tests).
pub struct ClientIp(pub Option<std::net::IpAddr>);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|c| c.0.ip());
        let forwarded = || {
            parts
                .headers
                .get_all("x-forwarded-for")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .collect::<Vec<_>>()
                .join(",")
        };
        Ok(Self(match peer {
            Some(ip) if state.cfg.trusted_proxies.iter().any(|n| n.contains(ip)) => {
                rightmost_forwarded(&forwarded()).or(Some(ip))
            }
            other => other,
        }))
    }
}

/// An IP network in CIDR notation (`127.0.0.0/8`, `::1/128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpNet {
    addr: std::net::IpAddr,
    prefix: u8,
}

impl IpNet {
    /// Loopback plus Docker's default bridge pool; the API port is only published on 127.0.0.1.
    pub fn default_trusted() -> Vec<Self> {
        ["127.0.0.0/8", "::1/128", "172.16.0.0/12"]
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    }

    pub fn contains(&self, ip: std::net::IpAddr) -> bool {
        use std::net::IpAddr::{V4, V6};
        match (self.addr, ip) {
            (V4(net), V4(ip)) => mask_eq(&net.octets(), &ip.octets(), self.prefix),
            (V6(net), V6(ip)) => mask_eq(&net.octets(), &ip.octets(), self.prefix),
            _ => false,
        }
    }
}

fn mask_eq(net: &[u8], ip: &[u8], prefix: u8) -> bool {
    let (full, rest) = ((prefix / 8) as usize, prefix % 8);
    net[..full] == ip[..full] && (rest == 0 || (net[full] ^ ip[full]) & (0xffu8 << (8 - rest)) == 0)
}

impl std::str::FromStr for IpNet {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (addr, prefix) = s
            .trim()
            .split_once('/')
            .ok_or_else(|| format!("{s}: expected addr/prefix"))?;
        let addr: std::net::IpAddr = addr.parse().map_err(|e| format!("{s}: {e}"))?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix: u8 = prefix.parse().map_err(|e| format!("{s}: {e}"))?;
        if prefix > max {
            return Err(format!("{s}: prefix > {max}"));
        }
        Ok(Self { addr, prefix })
    }
}

/// The last parseable address in an `X-Forwarded-For` list: the one our own proxy appended.
fn rightmost_forwarded(header: &str) -> Option<std::net::IpAddr> {
    header.rsplit(',').find_map(|s| s.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{IpNet, rightmost_forwarded};

    #[test]
    fn rightmost_forwarded_picks_proxy_appended_entry() {
        assert_eq!(
            rightmost_forwarded("1.2.3.4, 100.64.0.7")
                .unwrap()
                .to_string(),
            "100.64.0.7"
        );
        assert_eq!(
            rightmost_forwarded("fd7a::1").unwrap().to_string(),
            "fd7a::1"
        );
        assert!(rightmost_forwarded("").is_none());
        assert!(rightmost_forwarded("garbage").is_none());
    }

    #[test]
    fn ipnet_contains() {
        let nets = IpNet::default_trusted();
        let hit = |s: &str| nets.iter().any(|n| n.contains(s.parse().unwrap()));
        assert!(hit("127.0.0.1"));
        assert!(hit("::1"));
        assert!(hit("172.18.0.1"));
        assert!(!hit("172.32.0.1"));
        assert!(!hit("100.64.0.7"));
        assert!(!hit("192.168.1.10"));
        assert!("10.0.0.0/33".parse::<IpNet>().is_err());
    }
}
