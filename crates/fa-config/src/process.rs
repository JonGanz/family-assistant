use std::net::SocketAddr;
use std::path::PathBuf;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};
use url::Url;

/// A string that never shows up in `Debug` output or logs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read configuration: {0}")]
    Load(Box<figment::Error>),
    #[error("invalid configuration:\n  - {}", .0.join("\n  - "))]
    Invalid(Vec<String>),
}

/// Process-level configuration: defaults, then optional `fa.toml` (path in `FA_CONFIG`),
/// then `FA_*` env vars plus a bare `DATABASE_URL`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub database_url: String,
    pub bind_addr: String,
    pub master_key_path: PathBuf,
    pub public_base_url: String,
    pub cookie_secure: bool,
    pub log_format: LogFormat,
    pub log_level: String,
    pub web_dist_dir: PathBuf,
    pub backup_dir: Option<PathBuf>,
    /// `FA_GOOGLE_CLIENT_ID`; without it Google sources are hidden in the UI.
    #[serde(default)]
    pub google_client_id: Option<String>,
    /// `FA_GOOGLE_CLIENT_SECRET` (redacted in `Debug`).
    #[serde(default)]
    pub google_client_secret: Option<SecretString>,
    /// `FA_PUBLIC_URL`: base of the OAuth redirect. Required when a Google client ID is set.
    #[serde(default)]
    pub public_url: Option<Url>,
    /// `FA_OPEN_METEO_URL`.
    pub open_meteo_url: Url,
    /// `FA_ALLOW_HTTP_FEEDS`: also accept plain `http` for feeds and OAuth redirects (LAN, dev).
    pub allow_http_feeds: bool,
    /// `FA_NTFY_URL`: ntfy server for notifications. Without it none are sent.
    #[serde(default, deserialize_with = "empty_none")]
    pub ntfy_url: Option<Url>,
    /// `FA_NTFY_TOKEN` (redacted in `Debug`).
    #[serde(default, deserialize_with = "empty_none")]
    pub ntfy_token: Option<SecretString>,
    /// `FA_ALLOW_PLAIN_IMAP`: allow plain IMAP on port 143 without STARTTLS. Tests and e2e only.
    pub allow_plain_imap: bool,
    /// `FA_IMAP_TLS_INSECURE`: accept self-signed IMAPS certificates. Only for the greenmail test.
    pub imap_tls_insecure: bool,
    /// `FA_GOOGLE_API_BASE`: base URL of the Google APIs (default `https://www.googleapis.com`
    /// when unset). For stubs.
    #[serde(default, deserialize_with = "empty_none")]
    pub google_api_base: Option<Url>,
}

/// Empty strings mean unset.
fn empty_none<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let v = Option::<serde_json::Value>::deserialize(d)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(v) => serde_json::from_value(v)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

#[derive(Serialize)]
struct Defaults {
    bind_addr: &'static str,
    master_key_path: &'static str,
    cookie_secure: bool,
    log_format: LogFormat,
    log_level: &'static str,
    web_dist_dir: &'static str,
    open_meteo_url: &'static str,
    allow_http_feeds: bool,
    allow_plain_imap: bool,
    imap_tls_insecure: bool,
}

const DEFAULTS: Defaults = Defaults {
    bind_addr: "127.0.0.1:8080",
    master_key_path: "/run/secrets/fa_master_key",
    cookie_secure: true,
    log_format: LogFormat::Pretty,
    log_level: "info",
    web_dist_dir: "/srv/web",
    open_meteo_url: "https://api.open-meteo.com",
    allow_http_feeds: false,
    allow_plain_imap: false,
    imap_tls_insecure: false,
};

impl ProcessConfig {
    /// Loads from the real environment (and `fa.toml` / `$FA_CONFIG` if present).
    pub fn load() -> Result<Self, ConfigError> {
        let path = std::env::var("FA_CONFIG").unwrap_or_else(|_| "fa.toml".into());
        let fig = Self::base_figment()
            .merge(Toml::file(path))
            .merge(Env::prefixed("FA_"))
            .merge(Env::raw().only(&["DATABASE_URL"]));
        Self::from_figment(fig)
    }

    /// Defaults only; merge further providers onto it and pass to [`Self::from_figment`].
    pub fn base_figment() -> Figment {
        Figment::from(Serialized::defaults(DEFAULTS))
    }

    pub fn from_figment(fig: Figment) -> Result<Self, ConfigError> {
        let mut cfg: Self = fig.extract().map_err(|e| ConfigError::Load(Box::new(e)))?;
        cfg.validate()?;
        cfg.public_base_url = cfg.public_base_url.trim_end_matches('/').to_string();
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let mut errs = Vec::new();
        if !(self.database_url.starts_with("postgres://")
            || self.database_url.starts_with("postgresql://"))
        {
            errs.push("DATABASE_URL must start with postgres:// or postgresql://".to_string());
        }
        if self.bind_addr.parse::<SocketAddr>().is_err() {
            errs.push(format!(
                "FA_BIND_ADDR '{}' is not a valid host:port socket address (e.g. 127.0.0.1:8080)",
                self.bind_addr
            ));
        }
        let url = self.public_base_url.as_str();
        let host = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"));
        match host {
            None => errs.push(format!(
                "FA_PUBLIC_BASE_URL '{url}' must start with http:// or https://"
            )),
            Some(h) if h.trim_end_matches('/').is_empty() => {
                errs.push("FA_PUBLIC_BASE_URL has no host".to_string())
            }
            Some(h) => {
                let local = h.starts_with("localhost") || h.starts_with("127.0.0.1");
                if self.cookie_secure && url.starts_with("http://") && !local {
                    errs.push(
                        "FA_COOKIE_SECURE=true requires an https:// FA_PUBLIC_BASE_URL \
                         (or set FA_COOKIE_SECURE=false for plain http)"
                            .to_string(),
                    );
                }
            }
        }
        if self
            .google_client_id
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            if self
                .google_client_secret
                .as_ref()
                .is_none_or(|s| s.expose().trim().is_empty())
            {
                errs.push(
                    "FA_GOOGLE_CLIENT_ID is set but FA_GOOGLE_CLIENT_SECRET is not".to_string(),
                );
            }
            match &self.public_url {
                None => errs.push(
                    "FA_PUBLIC_URL is required when FA_GOOGLE_CLIENT_ID is set (OAuth redirect)"
                        .to_string(),
                ),
                Some(u) => match u.scheme() {
                    "https" => {}
                    "http" if self.allow_http_feeds => {}
                    _ => errs.push(format!(
                        "FA_PUBLIC_URL '{u}' must be https (http only with FA_ALLOW_HTTP_FEEDS=true)"
                    )),
                },
            }
        }
        if !matches!(self.open_meteo_url.scheme(), "https" | "http") {
            errs.push("FA_OPEN_METEO_URL must be an http(s) URL".to_string());
        }
        if let Some(u) = &self.ntfy_url
            && !matches!(u.scheme(), "https" | "http")
        {
            errs.push("FA_NTFY_URL must be an http(s) URL".to_string());
        }
        if let Some(u) = &self.google_api_base
            && !matches!(u.scheme(), "https" | "http")
        {
            errs.push("FA_GOOGLE_API_BASE must be an http(s) URL".to_string());
        }
        if self.allow_plain_imap {
            tracing::warn!(
                "FA_ALLOW_PLAIN_IMAP is set: IMAP without TLS is allowed (tests and e2e only)"
            );
        }
        if self.imap_tls_insecure {
            tracing::warn!(
                "FA_IMAP_TLS_INSECURE is set: IMAPS certificates are not verified (tests only)"
            );
        }
        if self.log_level.trim().is_empty() {
            errs.push("FA_LOG_LEVEL must not be empty".to_string());
        }
        if self.master_key_path.as_os_str().is_empty() {
            errs.push("FA_MASTER_KEY_PATH must not be empty".to_string());
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(errs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(toml: &str) -> Figment {
        ProcessConfig::base_figment().merge(Toml::string(toml))
    }

    #[test]
    fn defaults_and_required() {
        let c = ProcessConfig::from_figment(fig(
            "database_url='postgres://x'\npublic_base_url='https://h.example.com/'",
        ))
        .unwrap();
        assert_eq!(c.bind_addr, "127.0.0.1:8080");
        assert!(c.cookie_secure);
        assert_eq!(c.public_base_url, "https://h.example.com");
        assert!(c.backup_dir.is_none());
        assert!(matches!(
            ProcessConfig::from_figment(fig("")),
            Err(ConfigError::Load(_))
        ));
    }

    #[test]
    fn validation_collects_errors() {
        let e = ProcessConfig::from_figment(fig(
            "database_url='mysql://x'\npublic_base_url='ftp://h'\nbind_addr='nope'",
        ))
        .unwrap_err()
        .to_string();
        assert!(e.contains("DATABASE_URL") && e.contains("FA_BIND_ADDR") && e.contains("PUBLIC"));
    }

    const BASE: &str = "database_url='postgres://x'\npublic_base_url='https://h.example.com'\n";

    #[test]
    fn m1_defaults() {
        let c = ProcessConfig::from_figment(fig(BASE)).unwrap();
        assert_eq!(c.open_meteo_url.as_str(), "https://api.open-meteo.com/");
        assert!(!c.allow_http_feeds);
        assert!(c.google_client_id.is_none() && c.public_url.is_none());
    }

    #[test]
    fn google_requires_secret_and_public_url() {
        let e = ProcessConfig::from_figment(fig(&format!("{BASE}google_client_id='id'")))
            .unwrap_err()
            .to_string();
        assert!(
            e.contains("FA_PUBLIC_URL") && e.contains("CLIENT_SECRET"),
            "{e}"
        );
        let ok = format!(
            "{BASE}google_client_id='id'\ngoogle_client_secret='s3cret'\npublic_url='https://h.ts.net'"
        );
        let c = ProcessConfig::from_figment(fig(&ok)).unwrap();
        assert!(!format!("{c:?}").contains("s3cret"));
    }

    #[test]
    fn google_public_url_must_be_https_unless_allowed() {
        let t = format!(
            "{BASE}google_client_id='id'\ngoogle_client_secret='s'\npublic_url='http://h.lan'"
        );
        assert!(ProcessConfig::from_figment(fig(&t)).is_err());
        let ok = format!("{t}\nallow_http_feeds=true");
        assert!(ProcessConfig::from_figment(fig(&ok)).is_ok());
    }

    #[test]
    fn ntfy_optional_empty_unset_and_redacted() {
        let c = ProcessConfig::from_figment(fig(BASE)).unwrap();
        assert!(c.ntfy_url.is_none() && c.ntfy_token.is_none());
        let c =
            ProcessConfig::from_figment(fig(&format!("{BASE}ntfy_url=''\nntfy_token=''"))).unwrap();
        assert!(c.ntfy_url.is_none() && c.ntfy_token.is_none());
        let c = ProcessConfig::from_figment(fig(&format!(
            "{BASE}ntfy_url='https://ntfy.example.com'\nntfy_token='tk_s3cret'"
        )))
        .unwrap();
        assert_eq!(c.ntfy_url.unwrap().host_str(), Some("ntfy.example.com"));
        assert!(!format!("{:?}", c.ntfy_token).contains("tk_s3cret"));
        assert!(ProcessConfig::from_figment(fig(&format!("{BASE}ntfy_url='nope'"))).is_err());
    }

    #[test]
    fn m3_defaults_and_overrides() {
        let c = ProcessConfig::from_figment(fig(BASE)).unwrap();
        assert!(!c.allow_plain_imap && !c.imap_tls_insecure && c.google_api_base.is_none());
        let c = ProcessConfig::from_figment(fig(&format!(
            "{BASE}allow_plain_imap=true\nimap_tls_insecure=true\ngoogle_api_base='http://127.0.0.1:9999'"
        )))
        .unwrap();
        assert!(c.allow_plain_imap && c.imap_tls_insecure);
        assert_eq!(c.google_api_base.unwrap().host_str(), Some("127.0.0.1"));
        let c = ProcessConfig::from_figment(fig(&format!("{BASE}google_api_base=''"))).unwrap();
        assert!(c.google_api_base.is_none());
        assert!(
            ProcessConfig::from_figment(fig(&format!(
                "{BASE}google_api_base='ftp://x.example.com'"
            )))
            .is_err()
        );
    }

    #[test]
    fn bad_urls_fail_to_load() {
        assert!(ProcessConfig::from_figment(fig(&format!("{BASE}open_meteo_url='nope'"))).is_err());
        assert!(
            ProcessConfig::from_figment(fig(&format!(
                "{BASE}open_meteo_url='ftp://x.example.com'"
            )))
            .is_err()
        );
    }

    #[test]
    fn insecure_cookie_over_http_rejected() {
        let t = "database_url='postgres://x'\npublic_base_url='http://home.lan'";
        assert!(ProcessConfig::from_figment(fig(t)).is_err());
        let ok = format!("{t}\ncookie_secure=false\nlog_format='json'");
        assert_eq!(
            ProcessConfig::from_figment(fig(&ok)).unwrap().log_format,
            LogFormat::Json
        );
    }
}
