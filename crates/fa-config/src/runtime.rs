//! Settings that can come from the environment, the database (Settings, System) or a default.
//!
//! Precedence (D-075): env var (or `fa.toml`) > `instance_setting` > `FA_DEFAULT_<KEY>` env >
//! built-in default. A key set by env or `fa.toml` is locked in the UI. An empty env var counts
//! as unset for these keys, so a compose file can pass `FA_X: ${FA_X:-}` without locking it.

use std::collections::BTreeMap;
use std::net::IpAddr;

use fa_domain::system_api::{SettingKind, SettingSource, SettingView};
use figment::Figment;
use figment::providers::{Format, Serialized, Toml};
use serde_json::{Map, Value};
use url::Url;

use crate::process::{ConfigError, ProcessConfig};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeKey {
    pub key: &'static str,
    pub env: &'static str,
    pub kind: SettingKind,
    pub secret: bool,
}

const fn k(key: &'static str, env: &'static str, kind: SettingKind) -> RuntimeKey {
    RuntimeKey {
        key,
        env,
        kind,
        secret: matches!(kind, SettingKind::Secret),
    }
}

pub const RUNTIME_KEYS: &[RuntimeKey] = &[
    k("public_base_url", "FA_PUBLIC_BASE_URL", SettingKind::Url),
    k("tailnet_base_url", "FA_TAILNET_BASE_URL", SettingKind::Url),
    k("ntfy_url", "FA_NTFY_URL", SettingKind::Url),
    k("ntfy_token", "FA_NTFY_TOKEN", SettingKind::Secret),
    k("ntfy_public_url", "FA_NTFY_PUBLIC_URL", SettingKind::Url),
    k("google_client_id", "FA_GOOGLE_CLIENT_ID", SettingKind::Text),
    k(
        "google_client_secret",
        "FA_GOOGLE_CLIENT_SECRET",
        SettingKind::Secret,
    ),
    k("allow_http_feeds", "FA_ALLOW_HTTP_FEEDS", SettingKind::Bool),
    k(
        "allow_private_hosts",
        "FA_ALLOW_PRIVATE_HOSTS",
        SettingKind::Bool,
    ),
    k("trusted_proxies", "FA_TRUSTED_PROXIES", SettingKind::Cidrs),
    k(
        "lan_dashboard_url",
        "FA_LAN_DASHBOARD_URL",
        SettingKind::Url,
    ),
    k(
        "lan_allowed_cidrs",
        "FA_LAN_ALLOWED_CIDRS",
        SettingKind::Cidrs,
    ),
    k("lan_allow_pin", "FA_LAN_ALLOW_PIN", SettingKind::Bool),
];

pub fn runtime_key(key: &str) -> Option<&'static RuntimeKey> {
    RUNTIME_KEYS.iter().find(|r| r.key == key)
}

/// `addr/prefix`, or a bare address meaning that one host (as `fa_api::IpNet` parses it).
fn parse_cidr(s: &str) -> Result<(), String> {
    let s = s.trim();
    let (addr, prefix) = s.split_once('/').unwrap_or((s, ""));
    let addr: IpAddr = addr
        .parse()
        .map_err(|_| format!("'{s}': expected an address or addr/prefix, e.g. 10.0.0.0/8"))?;
    if prefix.is_empty() {
        return Ok(());
    }
    let max = if addr.is_ipv4() { 32 } else { 128 };
    match prefix.parse::<u8>() {
        Ok(p) if p <= max => Ok(()),
        _ => Err(format!("'{s}': prefix must be 0 to {max}")),
    }
}

/// Checks a value for `key` and returns it normalised (URLs lose a trailing slash, text is
/// trimmed, CIDR lists become arrays of trimmed strings).
pub fn validate_value(key: &str, v: &Value) -> Result<Value, String> {
    let rk = runtime_key(key).ok_or_else(|| format!("unknown setting '{key}'"))?;
    match rk.kind {
        SettingKind::Url => {
            let s = v.as_str().ok_or("expected a URL string")?.trim();
            let u = Url::parse(s).map_err(|e| format!("not a valid URL: {e}"))?;
            if !matches!(u.scheme(), "http" | "https") {
                return Err("must start with http:// or https://".into());
            }
            if u.host_str().is_none_or(str::is_empty) {
                return Err("URL has no host".into());
            }
            Ok(Value::String(s.trim_end_matches('/').to_string()))
        }
        SettingKind::Bool => v
            .as_bool()
            .map(Value::Bool)
            .ok_or_else(|| "expected true or false".into()),
        SettingKind::Text | SettingKind::Secret => {
            let s = v.as_str().ok_or("expected a string")?.trim();
            if s.is_empty() {
                return Err("must not be empty".into());
            }
            Ok(Value::String(s.to_string()))
        }
        SettingKind::Cidrs => {
            let items: Vec<String> = match v {
                Value::Array(a) => a
                    .iter()
                    .map(|x| {
                        x.as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or("expected a list of strings".to_string())
                    })
                    .collect::<Result<_, _>>()?,
                Value::String(s) => s
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect(),
                _ => return Err("expected a list of CIDRs".into()),
            };
            for i in &items {
                parse_cidr(i)?;
            }
            Ok(Value::Array(items.into_iter().map(Value::String).collect()))
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    key: &'static str,
    kind: SettingKind,
    value: Option<Value>,
    is_set: bool,
    source: SettingSource,
}

/// Where each runtime setting's effective value came from. Secrets are never stored here.
#[derive(Debug, Clone, Default)]
pub struct RuntimeSnapshot {
    entries: Vec<Entry>,
}

impl RuntimeSnapshot {
    pub fn views(&self) -> Vec<SettingView> {
        self.entries
            .iter()
            .map(|e| SettingView {
                key: e.key.to_string(),
                kind: e.kind,
                value: e.value.clone(),
                is_set: e.is_set,
                source: e.source,
                locked: e.source == SettingSource::Env,
            })
            .collect()
    }

    pub fn source(&self, key: &str) -> Option<SettingSource> {
        self.entries.iter().find(|e| e.key == key).map(|e| e.source)
    }

    pub fn is_locked(&self, key: &str) -> bool {
        self.source(key) == Some(SettingSource::Env)
    }
}

const BOOL_FIELDS: &[&str] = &[
    "allow_http_feeds",
    "allow_private_hosts",
    "lan_allow_pin",
    "allow_plain_imap",
    "imap_tls_insecure",
];
const LIST_FIELDS: &[&str] = &["trusted_proxies", "lan_allowed_cidrs"];

fn env_value(env_name: &str, field: &str, raw: &str) -> Result<Option<Value>, String> {
    if BOOL_FIELDS.contains(&field) {
        return match raw.trim() {
            "" => Ok(None),
            "1" | "true" => Ok(Some(Value::Bool(true))),
            "0" | "false" => Ok(Some(Value::Bool(false))),
            v => Err(format!("{env_name}: expected true or false, got {v:?}")),
        };
    }
    if LIST_FIELDS.contains(&field) {
        let items = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Value::String(s.to_string()))
            .collect::<Vec<_>>();
        return Ok((!items.is_empty()).then_some(Value::Array(items)));
    }
    if runtime_key(field).is_some() && raw.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Value::String(raw.to_string())))
}

impl ProcessConfig {
    /// Loads from the real environment and `fa.toml`, with `db` (stored settings, secrets
    /// already decrypted) in between. See the module docs for the precedence.
    pub fn load_layered(
        db: &BTreeMap<String, Value>,
    ) -> Result<(ProcessConfig, RuntimeSnapshot), ConfigError> {
        let env: BTreeMap<String, String> = std::env::vars().collect();
        let path = env
            .get("FA_CONFIG")
            .cloned()
            .unwrap_or_else(|| "fa.toml".into());
        Self::load_layered_from(db, &env, Figment::from(Toml::file(path)))
    }

    /// [`Self::load_layered`] with an explicit environment and `fa.toml` provider.
    pub fn load_layered_from(
        db: &BTreeMap<String, Value>,
        env: &BTreeMap<String, String>,
        toml: Figment,
    ) -> Result<(ProcessConfig, RuntimeSnapshot), ConfigError> {
        let mut errs = Vec::new();

        let mut env_layer = Map::new();
        for (name, raw) in env {
            let field = if name == "DATABASE_URL" {
                "database_url".to_string()
            } else if let Some(rest) = name.strip_prefix("FA_")
                && !rest.starts_with("DEFAULT_")
                && rest != "CONFIG"
            {
                rest.to_ascii_lowercase()
            } else {
                continue;
            };
            match env_value(name, &field, raw) {
                Ok(Some(v)) => {
                    env_layer.insert(field, v);
                }
                Ok(None) => {}
                Err(e) => errs.push(e),
            }
        }

        let mut default_layer = Map::new();
        let mut db_layer = Map::new();
        for rk in RUNTIME_KEYS {
            let dname = format!("FA_DEFAULT_{}", rk.key.to_ascii_uppercase());
            if let Some(raw) = env.get(&dname) {
                match env_value(&dname, rk.key, raw) {
                    Ok(Some(v)) => {
                        default_layer.insert(rk.key.to_string(), v);
                    }
                    Ok(None) => {}
                    Err(e) => errs.push(e),
                }
            }
            if let Some(v) = db.get(rk.key).filter(|v| !v.is_null()) {
                match validate_value(rk.key, v) {
                    Ok(v) => {
                        db_layer.insert(rk.key.to_string(), v);
                    }
                    Err(e) => tracing::warn!(key = rk.key, "ignoring invalid stored setting: {e}"),
                }
            }
        }
        if !errs.is_empty() {
            return Err(ConfigError::Invalid(errs));
        }

        let in_toml: Vec<&str> = RUNTIME_KEYS
            .iter()
            .filter(|rk| toml.contains(rk.key))
            .map(|rk| rk.key)
            .collect();

        let fig = ProcessConfig::base_figment()
            .merge(Serialized::defaults(default_layer.clone()))
            .merge(Serialized::defaults(db_layer.clone()))
            .merge(toml)
            .merge(Serialized::defaults(env_layer.clone()));
        let cfg = ProcessConfig::from_figment(fig)?;

        let json = serde_json::to_value(&cfg).unwrap_or_default();
        let entries = RUNTIME_KEYS
            .iter()
            .map(|rk| {
                let source = if env_layer.contains_key(rk.key) || in_toml.contains(&rk.key) {
                    SettingSource::Env
                } else if db_layer.contains_key(rk.key) {
                    SettingSource::Db
                } else {
                    SettingSource::Default
                };
                let v = json.get(rk.key).filter(|v| !v.is_null()).cloned();
                Entry {
                    key: rk.key,
                    kind: rk.kind,
                    is_set: v.is_some(),
                    value: if rk.secret { None } else { v },
                    source,
                }
            })
            .collect();
        Ok((cfg, RuntimeSnapshot { entries }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn load(
        db: &[(&str, Value)],
        env: &[(&str, &str)],
        toml: &str,
    ) -> (ProcessConfig, RuntimeSnapshot) {
        let db = db.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        let mut env: BTreeMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        env.entry("DATABASE_URL".into())
            .or_insert("postgres://x".into());
        ProcessConfig::load_layered_from(&db, &env, Figment::from(Toml::string(toml))).unwrap()
    }

    fn view(s: &RuntimeSnapshot, key: &str) -> SettingView {
        s.views().into_iter().find(|v| v.key == key).unwrap()
    }

    #[test]
    fn cidr_lists_accept_bare_addresses() {
        let v = validate_value(
            "trusted_proxies",
            &serde_json::json!("127.0.0.0/8, ::1/128, 172.30.89.10"),
        )
        .unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
        assert!(validate_value("trusted_proxies", &serde_json::json!("10.0.0.0/40")).is_err());
        assert!(validate_value("trusted_proxies", &serde_json::json!("nope")).is_err());
    }

    #[test]
    fn precedence_env_db_default_builtin() {
        // built-in
        let (c, s) = load(&[], &[], "");
        assert!(c.ntfy_url.is_none());
        assert_eq!(view(&s, "ntfy_url").source, SettingSource::Default);
        assert!(!view(&s, "ntfy_url").is_set);
        // FA_DEFAULT beats built-in, does not lock
        let env = [("FA_DEFAULT_NTFY_URL", "http://d.example.com")];
        let (c, s) = load(&[], &env, "");
        assert_eq!(c.ntfy_url.unwrap().host_str(), Some("d.example.com"));
        let v = view(&s, "ntfy_url");
        assert_eq!((v.source, v.locked), (SettingSource::Default, false));
        // db beats FA_DEFAULT
        let db = [("ntfy_url", json!("https://db.example.com/"))];
        let (c, s) = load(&db, &env, "");
        assert_eq!(c.ntfy_url.unwrap().host_str(), Some("db.example.com"));
        let v = view(&s, "ntfy_url");
        assert_eq!((v.source, v.locked), (SettingSource::Db, false));
        assert_eq!(v.value, Some(json!("https://db.example.com/")));
        // env beats db and locks
        let env2 = [
            ("FA_DEFAULT_NTFY_URL", "http://d.example.com"),
            ("FA_NTFY_URL", "http://env.example.com"),
        ];
        let (c, s) = load(&db, &env2, "");
        assert_eq!(c.ntfy_url.unwrap().host_str(), Some("env.example.com"));
        let v = view(&s, "ntfy_url");
        assert_eq!((v.source, v.locked), (SettingSource::Env, true));
        // fa.toml counts as env
        let (_, s) = load(&db, &[], "ntfy_url='http://t.example.com'");
        assert!(view(&s, "ntfy_url").locked);
    }

    #[test]
    fn empty_env_is_unset_and_does_not_lock() {
        let db = [("ntfy_url", json!("http://db.example.com"))];
        let (c, s) = load(&db, &[("FA_NTFY_URL", "")], "");
        assert!(c.ntfy_url.is_some());
        assert!(!view(&s, "ntfy_url").locked);
    }

    #[test]
    fn bools_lists_and_builtins() {
        let (c, s) = load(&[], &[], "");
        assert!(!c.allow_private_hosts);
        assert_eq!(c.trusted_proxies, ["127.0.0.0/8", "::1/128"]);
        let v = view(&s, "trusted_proxies");
        assert!(v.is_set && v.source == SettingSource::Default);
        let env = [
            ("FA_ALLOW_PRIVATE_HOSTS", "true"),
            ("FA_TRUSTED_PROXIES", "10.0.0.0/8, 172.16.0.0/12"),
            ("FA_LAN_ALLOW_PIN", "1"),
        ];
        let db = [("allow_private_hosts", json!(false))];
        let (c, s) = load(&db, &env, "");
        assert!(c.allow_private_hosts && c.lan_allow_pin);
        assert_eq!(c.trusted_proxies, ["10.0.0.0/8", "172.16.0.0/12"]);
        assert!(view(&s, "allow_private_hosts").locked);
        // from db
        let db = [
            ("lan_allowed_cidrs", json!(["192.168.0.0/16"])),
            ("lan_allow_pin", json!(true)),
        ];
        let (c, s) = load(&db, &[], "");
        assert_eq!(c.lan_allowed_cidrs, ["192.168.0.0/16"]);
        assert!(c.lan_allow_pin);
        assert_eq!(view(&s, "lan_allow_pin").source, SettingSource::Db);
    }

    #[test]
    fn secrets_are_redacted_in_views() {
        let db = [("ntfy_token", json!("tk_s3cret"))];
        let (c, s) = load(&db, &[], "");
        assert_eq!(c.ntfy_token.as_ref().unwrap().expose(), "tk_s3cret");
        let v = view(&s, "ntfy_token");
        assert!(v.is_set && v.value.is_none());
        assert!(!format!("{s:?}").contains("tk_s3cret"));
    }

    #[test]
    fn invalid_db_value_is_ignored_invalid_env_fails() {
        let db = [("ntfy_url", json!("ftp://x"))];
        let (c, _) = load(&db, &[], "");
        assert!(c.ntfy_url.is_none());
        let env: BTreeMap<String, String> = [
            ("DATABASE_URL", "postgres://x"),
            ("FA_NTFY_URL", "ftp://x.example.com"),
        ]
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .into();
        assert!(ProcessConfig::load_layered_from(&BTreeMap::new(), &env, Figment::new()).is_err());
    }

    #[test]
    fn validate_values() {
        assert_eq!(
            validate_value("public_base_url", &json!(" http://h.lan:8080/ ")).unwrap(),
            json!("http://h.lan:8080")
        );
        assert!(validate_value("public_base_url", &json!("ftp://h")).is_err());
        assert!(validate_value("public_base_url", &json!("nope")).is_err());
        assert!(validate_value("public_base_url", &json!(3)).is_err());
        assert!(validate_value("allow_http_feeds", &json!(true)).is_ok());
        assert!(validate_value("allow_http_feeds", &json!("true")).is_err());
        assert_eq!(
            validate_value("trusted_proxies", &json!("10.0.0.0/8, ::1/128")).unwrap(),
            json!(["10.0.0.0/8", "::1/128"])
        );
        assert!(validate_value("trusted_proxies", &json!(["10.0.0.1/99"])).is_err());
        assert!(validate_value("trusted_proxies", &json!(["10.0.0.0/33"])).is_err());
        assert!(validate_value("trusted_proxies", &json!([])).is_ok());
        assert_eq!(
            validate_value("google_client_id", &json!(" id ")).unwrap(),
            json!("id")
        );
        assert!(validate_value("ntfy_token", &json!("  ")).is_err());
        assert!(validate_value("nope", &json!("x")).is_err());
    }
}
