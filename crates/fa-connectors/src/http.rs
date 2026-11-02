use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::{Method, RequestBuilder, Response, StatusCode, redirect};
use url::Url;

use crate::error::{ConnectorError, Result};

pub const MAX_BODY: usize = 10 * 1024 * 1024;

/// Shared outbound client: rustls, 20 s timeout, gzip, capped body, https-only unless allowed.
#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
    allow_http: bool,
    allow_private_hosts: bool,
    max_body: usize,
}

/// True for loopback, RFC 1918, link-local, CGNAT, unique-local, unspecified, benchmarking,
/// reserved, NAT64, 6to4 and IPv4-compatible addresses.
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => is_private_v4(a),
        IpAddr::V6(a) => {
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_private_v4(v4);
            }
            is_private_v6(a)
        }
    }
}

fn is_private_v4(a: Ipv4Addr) -> bool {
    let o = a.octets();
    a.is_loopback()
        || a.is_private()
        || a.is_link_local()
        || a.is_unspecified()
        || a.is_broadcast()
        || (o[0] == 100 && (64..128).contains(&o[1]))
        // 198.18.0.0/15 benchmarking, 192.0.0.0/24 IETF protocol assignments, 240.0.0.0/4 reserved.
        || (o[0] == 198 && (o[1] & 0xfe) == 18)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || o[0] >= 240
}

fn is_private_v6(a: Ipv6Addr) -> bool {
    let s = a.segments();
    a.is_loopback()
        || a.is_unspecified()
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
        // IPv4-compatible ::/96, NAT64 64:ff9b::/96 and 64:ff9b:1::/48, 6to4 2002::/16: all can
        // carry an IPv4 destination that a gateway would reach, so refuse them wholesale.
        || s[..6].iter().all(|x| *x == 0)
        || (s[0] == 0x64 && s[1] == 0xff9b && (s[2..6].iter().all(|x| *x == 0) || s[2] == 1))
        || s[0] == 0x2002
}

fn url_is_private_literal(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(a)) => is_private_ip(IpAddr::V4(a)),
        Some(url::Host::Ipv6(a)) => is_private_ip(IpAddr::V6(a)),
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        None => true,
    }
}

fn redirect_allowed(url: &Url, allow_http: bool, allow_private_hosts: bool) -> bool {
    (url.scheme() == "https" || allow_http) && (allow_private_hosts || !url_is_private_literal(url))
}

/// Resolves through the system resolver and drops private addresses.
struct PublicOnly;

impl Resolve for PublicOnly {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host = name.as_str().to_owned();
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, 0))
                .await?
                .filter(|a| !is_private_ip(a.ip()))
                .collect();
            if addrs.is_empty() {
                return Err(Box::new(std::io::Error::other("private address refused"))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

impl Http {
    /// Allows loopback and private hosts (LAN ntfy, tests) and honours `HTTP(S)_PROXY`. Use
    /// `with_policy(_, false)` for anything that takes a user-supplied URL.
    pub fn permissive(allow_http: bool) -> Result<Self> {
        Self::with_max_body(allow_http, MAX_BODY)
    }

    /// Old name of [`Http::permissive`]; kept so callers outside this crate keep building.
    pub fn new(allow_http: bool) -> Result<Self> {
        Self::permissive(allow_http)
    }

    /// Permissive (see [`Http::permissive`]) with a different default body cap.
    pub fn with_max_body(allow_http: bool, max_body: usize) -> Result<Self> {
        Self::build(allow_http, true, max_body)
    }

    /// Like `permissive`, but with `allow_private_hosts = false` it refuses loopback, private,
    /// link-local, CGNAT, unique-local and unspecified addresses at connect time and on redirects.
    pub fn with_policy(allow_http: bool, allow_private_hosts: bool) -> Result<Self> {
        Self::build(allow_http, allow_private_hosts, MAX_BODY)
    }

    fn build(allow_http: bool, allow_private_hosts: bool, max_body: usize) -> Result<Self> {
        let policy = redirect::Policy::custom(move |attempt| {
            let ok = redirect_allowed(attempt.url(), allow_http, allow_private_hosts);
            if attempt.previous().len() >= 5 || !ok {
                attempt.stop()
            } else {
                attempt.follow()
            }
        });
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .gzip(true)
            .redirect(policy)
            .user_agent(concat!("family-assistant/", env!("CARGO_PKG_VERSION")));
        if !allow_private_hosts {
            // A proxy resolves names itself, which would bypass `PublicOnly`.
            builder = builder.no_proxy().dns_resolver(Arc::new(PublicOnly));
        }
        let client = builder
            .build()
            .map_err(|e| ConnectorError::Network(e.without_url().to_string()))?;
        Ok(Self {
            client,
            allow_http,
            allow_private_hosts,
            max_body,
        })
    }

    pub fn parse_url(&self, url: &str) -> Result<Url> {
        let url = Url::parse(url).map_err(|e| ConnectorError::Parse(format!("bad url: {e}")))?;
        self.check_url(&url)?;
        Ok(url)
    }

    pub fn check_url(&self, url: &Url) -> Result<()> {
        if !self.allow_private_hosts && url_is_private_literal(url) {
            return Err(ConnectorError::PrivateHost);
        }
        match url.scheme() {
            "https" => Ok(()),
            "http" if self.allow_http => Ok(()),
            "http" => Err(ConnectorError::InsecureUrl),
            other => Err(ConnectorError::Parse(format!("unsupported scheme {other}"))),
        }
    }

    pub fn request(&self, method: Method, url: &Url) -> Result<RequestBuilder> {
        self.check_url(url)?;
        Ok(self.client.request(method, url.clone()))
    }

    pub fn get(&self, url: &Url) -> Result<RequestBuilder> {
        self.request(Method::GET, url)
    }

    /// Sends the request. Errors never carry the URL (it may hold secrets); only the host is logged.
    pub async fn send(&self, rb: RequestBuilder) -> Result<Response> {
        let resp = rb.send().await.map_err(|e| {
            let e = e.without_url();
            tracing::warn!(error = %e, "outbound request failed");
            ConnectorError::Network(e.to_string())
        })?;
        tracing::debug!(
            host = resp.url().host_str().unwrap_or(""),
            status = resp.status().as_u16(),
            "outbound response"
        );
        Ok(resp)
    }

    /// Maps non-success statuses to errors.
    pub fn check_status(status: StatusCode) -> Result<()> {
        match status.as_u16() {
            401 => Err(ConnectorError::Unauthorized),
            410 => Err(ConnectorError::Gone),
            304 => Err(ConnectorError::NotModified),
            _ if status.is_success() => Ok(()),
            s => Err(ConnectorError::Http(s)),
        }
    }

    /// Reads the body while enforcing the default size cap.
    pub async fn body(&self, resp: Response) -> Result<Bytes> {
        self.body_with_limit(resp, self.max_body).await
    }

    /// Reads the body while enforcing `max_body` instead of the client's default cap.
    pub async fn body_with_limit(&self, mut resp: Response, max_body: usize) -> Result<Bytes> {
        if resp.content_length().is_some_and(|l| l > max_body as u64) {
            return Err(ConnectorError::TooLarge);
        }
        let mut buf = BytesMut::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| ConnectorError::Network(e.without_url().to_string()))?
        {
            if buf.len() + chunk.len() > max_body {
                return Err(ConnectorError::TooLarge);
            }
            buf.extend_from_slice(&chunk);
        }
        Ok(buf.freeze())
    }

    /// Status check, then capped body.
    pub async fn ok_body(&self, resp: Response) -> Result<Bytes> {
        Self::check_status(resp.status())?;
        self.body(resp).await
    }

    pub async fn get_json(&self, rb: RequestBuilder) -> Result<serde_json::Value> {
        let resp = self.send(rb).await?;
        let bytes = self.ok_body(resp).await?;
        serde_json::from_slice(&bytes).map_err(|e| ConnectorError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn classifies_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.5.4",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "::",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "198.18.0.1",
            "198.19.255.254",
            "192.0.0.8",
            "240.0.0.1",
            "255.255.255.254",
            "64:ff9b::7f00:1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b:1::1",
            "2002:7f00:1::1",
            "2002:a9fe:a9fe::",
            "::127.0.0.1",
            "::10.0.0.1",
        ] {
            assert!(is_private_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "8.8.8.8",
            "93.184.216.34",
            "100.128.0.1",
            "198.20.0.1",
            "192.0.1.1",
            "239.255.255.255",
            "2606:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(!is_private_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn strict_policy_refuses_private_literals() {
        let strict = Http::with_policy(true, false).unwrap();
        for url in [
            "http://127.0.0.1/x",
            "http://10.0.0.1/x",
            "http://169.254.169.254/latest/meta-data",
            "http://[::1]/x",
            "http://localhost/x",
        ] {
            assert_eq!(
                strict.get(&u(url)).err(),
                Some(ConnectorError::PrivateHost),
                "{url}"
            );
        }
        assert!(strict.get(&u("http://93.184.216.34/x")).is_ok());
        let open = Http::with_policy(true, true).unwrap();
        assert!(open.get(&u("http://127.0.0.1/x")).is_ok());
        assert!(
            Http::permissive(true)
                .unwrap()
                .get(&u("http://10.0.0.1/"))
                .is_ok()
        );
    }

    #[test]
    fn redirects_to_private_addresses_are_refused() {
        assert!(!redirect_allowed(&u("https://10.0.0.1/"), true, false));
        assert!(!redirect_allowed(
            &u("http://169.254.169.254/"),
            true,
            false
        ));
        assert!(redirect_allowed(&u("https://example.com/"), true, false));
        assert!(redirect_allowed(&u("http://10.0.0.1/"), true, true));
        assert!(!redirect_allowed(&u("http://example.com/"), false, true));
    }

    #[tokio::test]
    async fn resolver_drops_loopback_names() {
        let name: Name = "localhost".parse().unwrap();
        assert!(PublicOnly.resolve(name).await.is_err());
    }
}
