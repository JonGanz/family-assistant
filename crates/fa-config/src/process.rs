use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use figment::Figment;
use figment::providers::Serialized;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::runtime::{RUNTIME_KEYS, validate_value};

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
    /// `FA_PUBLIC_BASE_URL`: overrides the stored `public_base_url` setting.
    #[serde(default, deserialize_with = "empty_none")]
    pub public_base_url: Option<String>,
    /// `FA_COOKIE_SECURE`: `auto` (default), `true` or `false`.
    pub cookie_secure: CookieSecure,
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
    /// `FA_DATABASE_PASSWORD_FILE`: file holding the DB password, inserted into `database_url`
    /// when that URL has none.
    #[serde(default, deserialize_with = "empty_none")]
    pub database_password_file: Option<PathBuf>,
    /// `FA_TAILNET_BASE_URL`: the HTTPS tailnet address.
    #[serde(default, deserialize_with = "empty_none")]
    pub tailnet_base_url: Option<String>,
    /// `FA_NTFY_PUBLIC_URL`: the ntfy address phones subscribe to, when it differs from
    /// `ntfy_url` (which in compose is the internal `http://ntfy:80`). See `ntfy_subscribe_base`.
    #[serde(default, deserialize_with = "empty_none")]
    pub ntfy_public_url: Option<String>,
    /// `FA_ALLOW_PRIVATE_HOSTS`: let feeds, calendars and the like reach private addresses.
    pub allow_private_hosts: bool,
    /// `FA_TRUSTED_PROXIES`: CIDRs whose `X-Forwarded-*` headers are trusted.
    pub trusted_proxies: Vec<String>,
    /// `FA_LAN_DASHBOARD_URL`: address of the home-network dashboard listener; unset leaves it off.
    #[serde(default, deserialize_with = "empty_none")]
    pub lan_dashboard_url: Option<String>,
    /// `FA_LAN_DASHBOARD_BIND`.
    pub lan_dashboard_bind: String,
    /// `FA_LAN_ALLOWED_CIDRS`: empty means private and loopback ranges.
    pub lan_allowed_cidrs: Vec<String>,
    /// `FA_LAN_ALLOW_PIN`.
    pub lan_allow_pin: bool,
}

/// When the session cookie carries `Secure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CookieSecure {
    /// Per request: over TLS, or behind a trusted proxy sending `X-Forwarded-Proto: https`.
    #[default]
    Auto,
    Always,
    Never,
}

impl Serialize for CookieSecure {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Self::Auto => "auto",
            Self::Always => "true",
            Self::Never => "false",
        })
    }
}

impl<'de> Deserialize<'de> for CookieSecure {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            B(bool),
            S(String),
        }
        match Raw::deserialize(d)? {
            Raw::B(true) => Ok(Self::Always),
            Raw::B(false) => Ok(Self::Never),
            Raw::S(s) => match s.trim().to_ascii_lowercase().as_str() {
                "auto" | "" => Ok(Self::Auto),
                "true" | "1" => Ok(Self::Always),
                "false" | "0" => Ok(Self::Never),
                _ => Err(serde::de::Error::custom(
                    "FA_COOKIE_SECURE must be auto, true or false",
                )),
            },
        }
    }
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
    cookie_secure: CookieSecure,
    log_format: LogFormat,
    log_level: &'static str,
    web_dist_dir: &'static str,
    open_meteo_url: &'static str,
    allow_http_feeds: bool,
    allow_plain_imap: bool,
    imap_tls_insecure: bool,
    allow_private_hosts: bool,
    trusted_proxies: [&'static str; 2],
    lan_dashboard_bind: &'static str,
    lan_allowed_cidrs: [&'static str; 0],
    lan_allow_pin: bool,
}

const DEFAULTS: Defaults = Defaults {
    bind_addr: "127.0.0.1:8080",
    master_key_path: "/run/secrets/fa_master_key",
    cookie_secure: CookieSecure::Auto,
    log_format: LogFormat::Pretty,
    log_level: "info",
    web_dist_dir: "/srv/web",
    open_meteo_url: "https://api.open-meteo.com",
    allow_http_feeds: false,
    allow_plain_imap: false,
    imap_tls_insecure: false,
    allow_private_hosts: false,
    trusted_proxies: ["127.0.0.0/8", "::1/128"],
    lan_dashboard_bind: "0.0.0.0:8081",
    lan_allowed_cidrs: [],
    lan_allow_pin: false,
};

impl ProcessConfig {
    /// Loads from the real environment (and `fa.toml` / `$FA_CONFIG` if present), without
    /// stored settings.
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_layered(&BTreeMap::new()).map(|(c, _)| c)
    }

    /// Defaults only; merge further providers onto it and pass to [`Self::from_figment`].
    pub fn base_figment() -> Figment {
        Figment::from(Serialized::defaults(DEFAULTS))
    }

    pub fn from_figment(fig: Figment) -> Result<Self, ConfigError> {
        let mut cfg: Self = fig.extract().map_err(|e| ConfigError::Load(Box::new(e)))?;
        cfg.apply_password_file()?;
        cfg.validate()?;
        cfg.public_base_url = cfg
            .public_base_url
            .map(|u| u.trim_end_matches('/').to_string());
        Ok(cfg)
    }

    /// `public_base_url`, or `http://localhost:8080` when unset.
    /// The ntfy base address shown to phones: `ntfy_public_url`; else the tailnet address on
    /// `:8443` (the compose sidecar's ntfy port); else the app's own host on `:8090` (compose's
    /// default `FA_NTFY_PORT`); else `ntfy_url`. `None` when notifications are off.
    pub fn ntfy_subscribe_base(&self) -> Option<String> {
        let ntfy = self.ntfy_url.as_ref()?;
        if let Some(u) = &self.ntfy_public_url {
            return Some(u.trim_end_matches('/').to_owned());
        }
        let derive = |base: &str, port: u16| {
            let mut u = Url::parse(base).ok()?;
            u.set_port(Some(port)).ok()?;
            Some(u.as_str().trim_end_matches('/').to_owned())
        };
        self.tailnet_base_url
            .as_deref()
            .and_then(|t| derive(t, 8443))
            .or_else(|| {
                self.public_base_url
                    .as_deref()
                    .and_then(|p| derive(p, 8090))
            })
            .or_else(|| Some(ntfy.as_str().trim_end_matches('/').to_owned()))
    }

    pub fn effective_base_url(&self) -> &str {
        self.public_base_url
            .as_deref()
            .unwrap_or("http://localhost:8080")
    }

    fn apply_password_file(&mut self) -> Result<(), ConfigError> {
        let Some(path) = &self.database_password_file else {
            return Ok(());
        };
        let raw = std::fs::read_to_string(path).map_err(|e| {
            ConfigError::Invalid(vec![format!(
                "FA_DATABASE_PASSWORD_FILE {}: {e}",
                path.display()
            )])
        })?;
        let pw = raw.trim_end_matches(['\n', '\r']);
        if pw.is_empty() {
            return Err(ConfigError::Invalid(vec![format!(
                "FA_DATABASE_PASSWORD_FILE {} is empty",
                path.display()
            )]));
        }
        self.database_url =
            insert_password(&self.database_url, pw).map_err(|e| ConfigError::Invalid(vec![e]))?;
        Ok(())
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
        if self.lan_dashboard_bind.parse::<SocketAddr>().is_err() {
            errs.push(format!(
                "FA_LAN_DASHBOARD_BIND '{}' is not a valid host:port socket address",
                self.lan_dashboard_bind
            ));
        }
        if self
            .google_client_id
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
            && self
                .google_client_secret
                .as_ref()
                .is_none_or(|s| s.expose().trim().is_empty())
        {
            errs.push("FA_GOOGLE_CLIENT_ID is set but FA_GOOGLE_CLIENT_SECRET is not".to_string());
        }
        if let Ok(serde_json::Value::Object(m)) = serde_json::to_value(self) {
            for k in RUNTIME_KEYS {
                if let Some(v) = m.get(k.key).filter(|v| !v.is_null())
                    && let Err(e) = validate_value(k.key, v)
                {
                    errs.push(format!("{}: {e}", k.env));
                }
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

/// Percent-encodes everything but RFC 3986 unreserved characters.
fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Adds `:password` to the user part of a `postgres://user@host/db` URL; leaves a URL that
/// already has a password alone.
fn insert_password(url: &str, password: &str) -> Result<String, String> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or("DATABASE_URL is not a URL".to_string())?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (auth, tail) = rest.split_at(end);
    let Some((userinfo, host)) = auth.rsplit_once('@') else {
        return Err(
            "FA_DATABASE_PASSWORD_FILE needs a user name in DATABASE_URL (postgres://user@host/db)"
                .to_string(),
        );
    };
    if userinfo.contains(':') {
        return Ok(url.to_string());
    }
    Ok(format!(
        "{scheme}://{userinfo}:{}@{host}{tail}",
        pct_encode(password)
    ))
}

#[cfg(test)]
mod tests {
    use figment::providers::{Format, Toml};

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
        assert_eq!(c.cookie_secure, CookieSecure::Auto);
        assert_eq!(c.public_base_url.as_deref(), Some("https://h.example.com"));
        assert_eq!(c.trusted_proxies, ["127.0.0.0/8", "::1/128"]);
        assert_eq!(c.effective_base_url(), "https://h.example.com");
        assert_eq!(c.lan_dashboard_bind, "0.0.0.0:8081");
        assert!(c.backup_dir.is_none());
        let none = ProcessConfig::from_figment(fig("database_url='postgres://x'")).unwrap();
        assert_eq!(none.effective_base_url(), "http://localhost:8080");
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
        assert!(
            e.contains("DATABASE_URL") && e.contains("FA_BIND_ADDR") && e.contains("PUBLIC"),
            "{e}"
        );
    }

    const BASE: &str = "database_url='postgres://x'\npublic_base_url='https://h.example.com'\n";

    #[test]
    fn ntfy_subscribe_base_is_derived_for_phones() {
        let c = |extra: &str| ProcessConfig::from_figment(fig(&format!("{BASE}{extra}"))).unwrap();
        assert_eq!(c("").ntfy_subscribe_base(), None, "notifications off");
        let on = "ntfy_url='http://ntfy:80'\n";
        assert_eq!(
            c(on).ntfy_subscribe_base().as_deref(),
            Some("https://h.example.com:8090")
        );
        assert_eq!(
            c(&format!(
                "{on}tailnet_base_url='https://family.tail1.ts.net'\n"
            ))
            .ntfy_subscribe_base()
            .as_deref(),
            Some("https://family.tail1.ts.net:8443")
        );
        assert_eq!(
            c(&format!(
                "{on}ntfy_public_url='http://192.168.1.10:9000/'\n"
            ))
            .ntfy_subscribe_base()
            .as_deref(),
            Some("http://192.168.1.10:9000")
        );
    }

    #[test]
    fn m1_defaults() {
        let c = ProcessConfig::from_figment(fig(BASE)).unwrap();
        assert_eq!(c.open_meteo_url.as_str(), "https://api.open-meteo.com/");
        assert!(!c.allow_http_feeds);
        assert!(c.google_client_id.is_none() && c.public_url.is_none());
    }

    #[test]
    fn google_requires_secret() {
        let e = ProcessConfig::from_figment(fig(&format!("{BASE}google_client_id='id'")))
            .unwrap_err()
            .to_string();
        assert!(e.contains("CLIENT_SECRET"), "{e}");
        let ok = format!("{BASE}google_client_id='id'\ngoogle_client_secret='s3cret'");
        let c = ProcessConfig::from_figment(fig(&ok)).unwrap();
        assert!(!format!("{c:?}").contains("s3cret"));
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
    fn http_base_url_is_allowed_with_any_cookie_mode() {
        let t = "database_url='postgres://x'\npublic_base_url='http://home.lan'";
        for (v, want) in [
            ("auto", CookieSecure::Auto),
            ("true", CookieSecure::Always),
            ("false", CookieSecure::Never),
        ] {
            let c = ProcessConfig::from_figment(fig(&format!("{t}\ncookie_secure='{v}'"))).unwrap();
            assert_eq!(c.cookie_secure, want);
        }
        let c = ProcessConfig::from_figment(fig(&format!("{t}\ncookie_secure=false"))).unwrap();
        assert_eq!(c.cookie_secure, CookieSecure::Never);
        assert!(ProcessConfig::from_figment(fig(&format!("{t}\ncookie_secure='x'"))).is_err());
        let c = ProcessConfig::from_figment(fig(&format!("{t}\nlog_format='json'"))).unwrap();
        assert_eq!(c.log_format, LogFormat::Json);
    }

    #[test]
    fn password_file_is_inserted_and_encoded() {
        let dir = std::env::temp_dir().join(format!("fa-pw-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("pw");
        std::fs::write(&f, "p@ss:w/rd%41 #?\n").unwrap();
        let t = format!(
            "database_url='postgres://fa@db:5432/fa?sslmode=disable'\ndatabase_password_file='{}'",
            f.display()
        );
        let c = ProcessConfig::from_figment(fig(&t)).unwrap();
        assert_eq!(
            c.database_url,
            "postgres://fa:p%40ss%3Aw%2Frd%2541%20%23%3F@db:5432/fa?sslmode=disable"
        );
        let parsed = Url::parse(&c.database_url).unwrap();
        assert_eq!(
            percent_decode(parsed.password().unwrap()),
            "p@ss:w/rd%41 #?"
        );
        // A URL that already has a password is left alone.
        let t2 = format!(
            "database_url='postgres://fa:own@db/fa'\ndatabase_password_file='{}'",
            f.display()
        );
        let c = ProcessConfig::from_figment(fig(&t2)).unwrap();
        assert_eq!(c.database_url, "postgres://fa:own@db/fa");
        // Unreadable file.
        let e = ProcessConfig::from_figment(fig(
            "database_url='postgres://fa@db/fa'\ndatabase_password_file='/nonexistent/pw'",
        ))
        .unwrap_err()
        .to_string();
        assert!(e.contains("FA_DATABASE_PASSWORD_FILE"), "{e}");
        std::fs::remove_dir_all(dir).ok();
    }

    fn percent_decode(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }
}
