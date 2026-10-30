//! M3.5: first-run setup, system settings, master key, request scheme, config import, removals.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use base64::Engine;
use fa_api::{ApiConfig, AppState, LanConfig, Services, lan_router, router};
use fa_auth::{AuthConfig, AuthService};
use fa_config::CookieSecure;
use fa_store::{MasterKey, Store};
use figment::Figment;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

const PW: &str = "adult password 123";

struct App {
    router: Router,
    state: AppState,
    key_path: std::path::PathBuf,
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.key_path);
    }
}

struct Resp {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    fn cookie(&self) -> String {
        self.headers
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    }
}

fn key() -> MasterKey {
    MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
}

fn runtime(env: &[(&str, &str)]) -> Arc<fa_config::RuntimeSnapshot> {
    let mut e: BTreeMap<String, String> = env
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    e.entry("DATABASE_URL".into())
        .or_insert("postgres://x".into());
    let (_, snap) =
        fa_config::ProcessConfig::load_layered_from(&BTreeMap::new(), &e, Figment::new()).unwrap();
    Arc::new(snap)
}

async fn app_with(pool: PgPool, cfg: ApiConfig, auth: AuthConfig) -> App {
    let store = Store::from_pool(pool);
    let oauth = fa_auth::OAuthService::new(
        store.clone(),
        key(),
        None,
        "https://fa.example.com",
        fa_connectors::Http::new(false).unwrap(),
    );
    let services = Services {
        calendar: Arc::new(
            fa_calendar::CalendarService::new(
                store.clone(),
                key(),
                fa_calendar::ConnectorConfig::default(),
                Arc::new(oauth.clone()),
            )
            .unwrap(),
        ),
        weather: fa_weather::WeatherService::new(
            store.clone(),
            fa_weather::WeatherConfig::default(),
        )
        .unwrap(),
        tasks: Arc::new(fa_tasks::TaskService::new(store.clone())),
        notify: fa_notify::NotifyService::new(
            store.clone(),
            fa_notify::NotifyConfig {
                ntfy_url: None,
                ntfy_token: None,
                public_base_url: "https://fa.example.com".into(),
                subscribe_base: None,
            },
        ),
        mail: fa_api::MailServices::new(Arc::new(fa_mail::MailDeps {
            store: store.clone(),
            http: fa_connectors::Http::new(true).unwrap(),
            tokens: Arc::new(oauth.clone()),
            key: Arc::new(key()),
            cfg: fa_mail::MailConfig::default(),
        })),
        oauth,
    };
    let key_path = std::env::temp_dir().join(format!("fa-m35-{}", uuid::Uuid::new_v4()));
    std::fs::write(&key_path, [7u8; 32]).unwrap();
    let state = AppState::new(
        store.clone(),
        AuthService::new(store, auth),
        services,
        ApiConfig {
            master_key_path: key_path.clone(),
            request_timeout: std::time::Duration::from_secs(120),
            ..cfg
        },
        CancellationToken::new(),
    );
    App {
        router: router(state.clone()),
        state,
        key_path,
    }
}

async fn app(pool: PgPool) -> App {
    app_with(
        pool,
        ApiConfig {
            cookie_secure: CookieSecure::Auto,
            ..ApiConfig::default()
        },
        AuthConfig::default(),
    )
    .await
}

async fn call(
    router: &Router,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    peer: Option<&str>,
    body: Option<Value>,
) -> Resp {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    if method != Method::GET {
        b = b.header("x-fa-request", "1");
    }
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let mut req = match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    if let Some(p) = peer {
        let addr = SocketAddr::new(p.parse().unwrap(), 40000);
        req.extensions_mut().insert(ConnectInfo(addr));
    }
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    Resp {
        status,
        headers,
        body,
    }
}

async fn send(a: &App, method: Method, uri: &str, cookie: &str, body: Option<Value>) -> Resp {
    let h: Vec<(&str, &str)> = if cookie.is_empty() {
        vec![]
    } else {
        vec![("cookie", cookie)]
    };
    call(&a.router, method, uri, &h, None, body).await
}

fn household_body() -> Value {
    json!({"name": "Example", "tz": "UTC", "location": null, "units": "metric",
           "currency": "USD", "base_url": "http://fa.example.com"})
}

fn owner_body() -> Value {
    json!({"display_name": "Alex", "color": "purple", "username": "alex",
           "password": PW, "pin": "1234"})
}

fn pair_of(r: &Resp) -> String {
    r.cookie().split(';').next().unwrap().to_owned()
}

/// Runs both setup steps and returns the owner's `fa_session=...` pair.
async fn setup(a: &App) -> String {
    let r = send(
        a,
        Method::POST,
        "/api/v1/setup/household",
        "",
        Some(household_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let r = send(
        a,
        Method::POST,
        "/api/v1/setup/owner",
        "",
        Some(owner_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_eq!(r.json()["actor"]["kind"], "user");
    assert_eq!(r.json()["actor"]["member"]["role"], "adult");
    pair_of(&r)
}

async fn count(a: &App, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(a.state.store.pool())
        .await
        .unwrap()
}

// ---- setup ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn setup_happy_path_and_closing(pool: PgPool) {
    let a = app(pool).await;
    let r = send(&a, Method::GET, "/api/v1/setup", "", None).await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(
        (j["needed"].clone(), j["owner_needed"].clone()),
        (json!(true), json!(false))
    );
    assert_eq!(j["completed"], json!(false));
    assert_eq!(j["secure"], json!(false));

    // Everything else waits for the setup.
    for (m, uri) in [
        (Method::GET, "/api/v1/me"),
        (Method::GET, "/api/v1/household"),
        (Method::POST, "/api/v1/auth/login"),
    ] {
        let r = send(
            &a,
            m,
            uri,
            "",
            Some(json!({"username": "a", "password": "b"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{uri}");
        assert_eq!(r.json()["code"], "setup_required", "{uri}");
    }
    // The owner needs the household first.
    let r = send(
        &a,
        Method::POST,
        "/api/v1/setup/owner",
        "",
        Some(owner_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    let r = send(
        &a,
        Method::POST,
        "/api/v1/setup/household",
        "",
        Some(household_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert!(r.json()["household_id"].is_string());
    // The base URL is kept as the public one.
    let stored = count(
        &a,
        "SELECT count(*) FROM instance_setting WHERE key = 'public_base_url' AND value = '\"http://fa.example.com\"'::jsonb",
    )
    .await;
    assert_eq!(stored, 1);

    let j = send(&a, Method::GET, "/api/v1/setup", "", None)
        .await
        .json();
    assert_eq!(
        (j["needed"].clone(), j["owner_needed"].clone()),
        (json!(false), json!(true))
    );
    let r = send(
        &a,
        Method::POST,
        "/api/v1/setup/household",
        "",
        Some(household_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.json()["code"], "setup_closed");
    let r = send(&a, Method::GET, "/api/v1/me", "", None).await;
    assert_eq!(r.json()["code"], "setup_required");

    let r = send(
        &a,
        Method::POST,
        "/api/v1/setup/owner",
        "",
        Some(owner_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert!(r.cookie().contains("fa_session="));
    assert!(r.cookie().contains("HttpOnly") && r.cookie().contains("SameSite=Lax"));
    let cookie = pair_of(&r);
    let r = send(
        &a,
        Method::POST,
        "/api/v1/setup/owner",
        "",
        Some(owner_body()),
    )
    .await;
    assert_eq!(
        (r.status, r.json()["code"].clone()),
        (StatusCode::CONFLICT, json!("setup_closed"))
    );

    // The session works, the PIN and account exist, and the wizard can finish.
    let me = send(&a, Method::GET, "/api/v1/me", &cookie, None).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.text());
    assert_eq!(me.json()["actor"]["member"]["has_account"], json!(true));
    assert_eq!(
        count(&a, "SELECT count(*) FROM member WHERE pin_hash IS NOT NULL").await,
        1
    );
    let j = send(&a, Method::GET, "/api/v1/setup", "", None)
        .await
        .json();
    assert_eq!(
        (
            j["needed"].clone(),
            j["owner_needed"].clone(),
            j["completed"].clone()
        ),
        (json!(false), json!(false), json!(false))
    );
    assert_eq!(
        send(&a, Method::POST, "/api/v1/setup/complete", "", None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&a, Method::POST, "/api/v1/setup/complete", &cookie, None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let j = send(&a, Method::GET, "/api/v1/setup", "", None)
        .await
        .json();
    assert_eq!(j["completed"], json!(true));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn setup_validates_its_input(pool: PgPool) {
    let a = app(pool).await;
    let mut bad = household_body();
    bad["tz"] = json!("Mars/Base");
    let r = send(&a, Method::POST, "/api/v1/setup/household", "", Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
    let mut bad = household_body();
    bad["base_url"] = json!("ftp://x");
    let r = send(&a, Method::POST, "/api/v1/setup/household", "", Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    // A tailnet https origin is stored as the tailnet address.
    let mut tn = household_body();
    tn["base_url"] = json!("https://fa.tail1234.ts.net");
    let r = send(&a, Method::POST, "/api/v1/setup/household", "", Some(tn)).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM instance_setting WHERE key = 'tailnet_base_url'"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM instance_setting WHERE key = 'public_base_url'"
        )
        .await,
        0
    );

    let mut weak = owner_body();
    weak["password"] = json!("short");
    let r = send(&a, Method::POST, "/api/v1/setup/owner", "", Some(weak)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
    let mut bad_pin = owner_body();
    bad_pin["pin"] = json!("12");
    let r = send(&a, Method::POST, "/api/v1/setup/owner", "", Some(bad_pin)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Nothing was half-created by the failures.
    assert_eq!(count(&a, "SELECT count(*) FROM member").await, 0);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn concurrent_setup_has_one_winner(pool: PgPool) {
    let a = app(pool).await;
    let results = futures::future::join_all((0..4).map(|_| {
        let router = a.router.clone();
        async move {
            call(
                &router,
                Method::POST,
                "/api/v1/setup/household",
                &[],
                None,
                Some(household_body()),
            )
            .await
            .status
        }
    }))
    .await;
    assert_eq!(
        results
            .iter()
            .filter(|s| **s == StatusCode::CREATED)
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|s| **s == StatusCode::CONFLICT)
            .count(),
        3,
        "{results:?}"
    );
    assert_eq!(count(&a, "SELECT count(*) FROM household").await, 1);

    let results = futures::future::join_all((0..4).map(|i| {
        let router = a.router.clone();
        async move {
            let mut body = owner_body();
            body["username"] = json!(format!("owner{i}"));
            call(
                &router,
                Method::POST,
                "/api/v1/setup/owner",
                &[],
                None,
                Some(body),
            )
            .await
            .status
        }
    }))
    .await;
    assert_eq!(
        results
            .iter()
            .filter(|s| **s == StatusCode::CREATED)
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|s| **s == StatusCode::CONFLICT)
            .count(),
        3,
        "{results:?}"
    );
    assert_eq!(count(&a, "SELECT count(*) FROM user_account").await, 1);
    assert_eq!(count(&a, "SELECT count(*) FROM member").await, 1);
}

// ---- Secure per request ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn secure_cookie_follows_a_trusted_proxy_only(pool: PgPool) {
    let a = app(pool).await;
    setup(&a).await;
    let login = |peer: Option<&'static str>, headers: Vec<(&'static str, &'static str)>| {
        let router = a.router.clone();
        async move {
            call(
                &router,
                Method::POST,
                "/api/v1/auth/login",
                &headers,
                peer,
                Some(json!({"username": "alex", "password": PW})),
            )
            .await
        }
    };
    let https = vec![("x-forwarded-proto", "https")];
    let r = login(Some("127.0.0.1"), https.clone()).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.cookie().contains("Secure"), "{}", r.cookie());
    assert!(r.cookie().contains("HttpOnly") && r.cookie().contains("SameSite=Lax"));
    // The header from a peer that is not a trusted proxy is ignored.
    let r = login(Some("203.0.113.9"), https.clone()).await;
    assert!(!r.cookie().contains("Secure"), "{}", r.cookie());
    // The Docker bridge is not trusted by default any more (C-04).
    let r = login(Some("172.18.0.1"), https.clone()).await;
    assert!(!r.cookie().contains("Secure"), "{}", r.cookie());
    // Plain HTTP from the trusted proxy, and no peer at all.
    let r = login(Some("127.0.0.1"), vec![("x-forwarded-proto", "http")]).await;
    assert!(!r.cookie().contains("Secure"));
    let r = login(None, https).await;
    assert!(!r.cookie().contains("Secure"));
    // No HSTS, ever.
    assert!(r.headers.get("strict-transport-security").is_none());

    // The setup status reports the connection the same way.
    let r = call(
        &a.router,
        Method::GET,
        "/api/v1/setup",
        &[
            ("x-forwarded-proto", "https"),
            ("host", "fa.tail1234.ts.net"),
        ],
        Some("127.0.0.1"),
        None,
    )
    .await;
    let j = r.json();
    assert_eq!(j["secure"], json!(true));
    assert_eq!(j["origin"], json!("https://fa.tail1234.ts.net"));
    assert_eq!(j["via_tailscale"], json!(true));
    let r = call(
        &a.router,
        Method::GET,
        "/api/v1/setup",
        &[
            ("x-forwarded-proto", "https"),
            ("host", "fa.tail1234.ts.net"),
        ],
        Some("198.51.100.1"),
        None,
    )
    .await;
    let j = r.json();
    assert_eq!(
        (j["secure"].clone(), j["via_tailscale"].clone()),
        (json!(false), json!(false))
    );
    assert_eq!(j["origin"], json!("http://fa.tail1234.ts.net"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn cookie_secure_modes_override_the_request(pool: PgPool) {
    for (mode, want) in [(CookieSecure::Always, true), (CookieSecure::Never, false)] {
        let a = app_with(
            pool.clone(),
            ApiConfig {
                cookie_secure: mode,
                ..ApiConfig::default()
            },
            AuthConfig::default(),
        )
        .await;
        if want {
            setup(&a).await;
        }
        let r = call(
            &a.router,
            Method::POST,
            "/api/v1/auth/login",
            &[("x-forwarded-proto", if want { "http" } else { "https" })],
            Some("127.0.0.1"),
            Some(json!({"username": "alex", "password": PW})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.text());
        assert_eq!(r.cookie().contains("Secure"), want, "{}", r.cookie());
    }
}

// ---- master key ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn master_key_needs_the_password_and_is_audited(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let uri = "/api/v1/system/master-key";
    let r = send(&a, Method::POST, uri, "", Some(json!({"password": PW}))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = send(
        &a,
        Method::POST,
        uri,
        &cookie,
        Some(json!({"password": "wrong wrong wrong"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM audit_entry WHERE action = 'system.master_key_download'"
        )
        .await,
        0
    );

    let r = send(
        &a,
        Method::POST,
        uri,
        &cookie,
        Some(json!({"password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let b64 = r.json()["key_base64"].as_str().unwrap().to_owned();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(b64.clone())
            .unwrap(),
        vec![7u8; 32]
    );
    assert_eq!(r.headers.get("cache-control").unwrap(), "no-store");
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM audit_entry WHERE action = 'system.master_key_download'"
        )
        .await,
        1
    );
    // The key is never written to the audit trail.
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_entry WHERE after::text LIKE '%' || $1 || '%'"
        )
        .bind(&b64)
        .fetch_one(a.state.store.pool())
        .await
        .unwrap(),
        0
    );

    // Confirming the backup is recorded and shown.
    let r = send(
        &a,
        Method::POST,
        "/api/v1/system/master-key/confirm",
        &cookie,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = send(&a, Method::GET, "/api/v1/system/settings", &cookie, None).await;
    assert!(
        r.json()["key_backup_confirmed_at"].is_string(),
        "{}",
        r.text()
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn wrong_master_key_passwords_count_toward_lockout(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let uri = "/api/v1/system/master-key";
    for _ in 0..5 {
        let r = send(
            &a,
            Method::POST,
            uri,
            &cookie,
            Some(json!({"password": "wrong wrong wrong"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    }
    // The account is locked now: even the right password is refused, here and at login.
    let r = send(
        &a,
        Method::POST,
        uri,
        &cookie,
        Some(json!({"password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::LOCKED, "{}", r.text());
    let r = send(
        &a,
        Method::POST,
        "/api/v1/auth/login",
        "",
        Some(json!({"username": "alex", "password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::LOCKED);
}

// ---- settings ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn settings_validate_lock_and_never_echo_secrets(pool: PgPool) {
    let a = app_with(
        pool,
        ApiConfig {
            cookie_secure: CookieSecure::Auto,
            runtime: runtime(&[("FA_PUBLIC_BASE_URL", "http://env.example.com")]),
            ..ApiConfig::default()
        },
        AuthConfig::default(),
    )
    .await;
    let cookie = setup(&a).await;
    let uri = "/api/v1/system/settings";
    assert_eq!(
        send(&a, Method::GET, uri, "", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let r = send(&a, Method::GET, uri, &cookie, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let view = r.json();
    let find = |v: &Value, k: &str| {
        v["settings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == k)
            .cloned()
            .unwrap()
    };
    assert_eq!(find(&view, "public_base_url")["locked"], json!(true));
    assert_eq!(find(&view, "public_base_url")["source"], json!("env"));
    assert_eq!(find(&view, "ntfy_url")["locked"], json!(false));
    assert_eq!(view["connection"]["secure"], json!(false));

    // Locked key: 409 setting_locked, nothing written.
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"public_base_url": "http://other.example.com"}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.text());
    assert_eq!(r.json()["code"], "setting_locked");
    assert_eq!(r.json()["field"], "public_base_url");
    // Invalid value: 422 naming the key.
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"ntfy_url": "not a url"}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
    assert_eq!(r.json()["code"], "invalid_setting");
    assert_eq!(r.json()["field"], "ntfy_url");
    assert!(r.json()["detail"].as_str().unwrap().contains("ntfy_url"));
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"trusted_proxies": ["nope"]}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"no_such_key": true}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    // A bad key in the batch stops the whole batch.
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"allow_http_feeds": true, "ntfy_url": "x"}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM instance_setting WHERE key = 'allow_http_feeds'"
        )
        .await,
        0
    );

    // Valid values are stored and shown as pending; a secret is sealed and never echoed.
    let secret = "s3cret-token-value";
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"ntfy_url": "http://ntfy.example.com/", "ntfy_token": secret, "allow_http_feeds": true}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(!r.text().contains(secret));
    let v = r.json();
    assert_eq!(
        find(&v, "ntfy_url")["value"],
        json!("http://ntfy.example.com")
    );
    assert_eq!(find(&v, "ntfy_url")["source"], json!("db"));
    assert_eq!(find(&v, "ntfy_token")["is_set"], json!(true));
    assert_eq!(find(&v, "ntfy_token")["value"], Value::Null);
    let r = send(&a, Method::GET, uri, &cookie, None).await;
    assert!(!r.text().contains(secret));
    assert_eq!(find(&r.json(), "ntfy_token")["is_set"], json!(true));
    let sealed: (Option<Vec<u8>>, Option<Value>) =
        sqlx::query_as("SELECT ciphertext, value FROM instance_setting WHERE key = 'ntfy_token'")
            .fetch_one(a.state.store.pool())
            .await
            .unwrap();
    assert!(sealed.0.is_some() && sealed.1.is_none());
    let mut tx = a.state.store.tx_system().await.unwrap();
    let plain = fa_store::InstanceSettingRepo::secret(&mut tx, "ntfy_token", &key())
        .await
        .unwrap();
    assert_eq!(plain.as_deref(), Some(secret));
    // The audit trail names keys, never values.
    assert_eq!(
        count(&a, &format!("SELECT count(*) FROM audit_entry WHERE after::text LIKE '%{secret}%' OR after::text LIKE '%ntfy.example.com%'")).await,
        0
    );

    // null resets to the default.
    let r = send(
        &a,
        Method::PATCH,
        uri,
        &cookie,
        Some(json!({"values": {"ntfy_token": null, "ntfy_url": null}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(find(&r.json(), "ntfy_token")["is_set"], json!(false));
    assert_eq!(find(&r.json(), "ntfy_url")["is_set"], json!(false));
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM instance_setting WHERE key IN ('ntfy_token','ntfy_url')"
        )
        .await,
        0
    );
}

// ---- config import / export ----

async fn text_post(a: &App, uri: &str, cookie: &str, body: &str, ctype: &str) -> Resp {
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("x-fa-request", "1")
        .header("cookie", cookie)
        .header("content-type", ctype)
        .body(Body::from(body.to_owned()))
        .unwrap();
    let res = a.router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    Resp {
        status,
        headers,
        body,
    }
}

const TOML: &str = r#"
[household]
name = "Example"
tz = "UTC"
units = "metric"

[[members]]
key = "riley"
display_name = "Riley"
role = "child"
"#;

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn config_import_dry_run_writes_nothing(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let before = count(&a, "SELECT count(*) FROM member").await;

    let r = text_post(
        &a,
        "/api/v1/config/import?dry_run=true",
        &cookie,
        TOML,
        "text/plain",
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["members_created"].as_array().unwrap().len(), 1);
    assert_eq!(count(&a, "SELECT count(*) FROM member").await, before);
    assert_eq!(
        count(
            &a,
            "SELECT count(*) FROM audit_entry WHERE action = 'config.import'"
        )
        .await,
        0
    );

    let r = text_post(
        &a,
        "/api/v1/config/import?dry_run=false",
        &cookie,
        TOML,
        "text/plain",
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(count(&a, "SELECT count(*) FROM member").await, before + 1);

    // Wrong type, bad TOML, oversize, anonymous.
    let r = text_post(
        &a,
        "/api/v1/config/import",
        &cookie,
        TOML,
        "application/json",
    )
    .await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let r = text_post(
        &a,
        "/api/v1/config/import",
        &cookie,
        "this is [not toml",
        "text/plain",
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let big = "# x\n".repeat(300_000);
    let r = text_post(&a, "/api/v1/config/import", &cookie, &big, "text/plain").await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    let r = text_post(&a, "/api/v1/config/import", "", TOML, "text/plain").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    let r = send(&a, Method::GET, "/api/v1/config/export", &cookie, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers.get("content-disposition").unwrap(),
        "attachment; filename=\"household.toml\""
    );
    assert!(r.text().contains("Riley") && !r.text().contains("password"));
}

// ---- removals, dashboard, own password ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn removed_routes_are_404_on_both_listeners(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let r = send(
        &a,
        Method::POST,
        "/api/v1/devices",
        &cookie,
        Some(json!({"name": "Kitchen"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let token = r.json()["token"].as_str().unwrap().to_owned();
    let bearer = format!("Bearer {token}");
    let dismiss = format!("/api/v1/advisories/{}/dismiss", uuid::Uuid::new_v4());
    let lan = lan_router(
        a.state.clone(),
        LanConfig {
            allowed: LanConfig::default_allowed(),
            allow_pin: false,
        },
    );
    for (name, router, peer) in [
        ("main", &a.router, None),
        ("lan", &lan, Some("192.168.1.20")),
    ] {
        let r = call(
            router,
            Method::POST,
            &dismiss,
            &[("authorization", &bearer)],
            peer,
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{name} dismiss");
    }
    // The LAN dashboard has no inbox routes either.
    for (m, uri) in [
        (Method::GET, "/api/v1/inbox".to_owned()),
        (
            Method::POST,
            format!("/api/v1/inbox/bill/{}/approve", uuid::Uuid::new_v4()),
        ),
        (
            Method::POST,
            format!("/api/v1/inbox/bill/{}/reject", uuid::Uuid::new_v4()),
        ),
    ] {
        let r = call(
            &lan,
            m,
            &uri,
            &[("authorization", &bearer)],
            Some("192.168.1.20"),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "lan {uri}");
    }
    // On the main listener a device is refused (403), not served.
    let r = call(
        &a.router,
        Method::GET,
        "/api/v1/inbox",
        &[("authorization", &bearer)],
        None,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pending_approvals_follow_the_household_switch(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let r = send(
        &a,
        Method::POST,
        "/api/v1/devices",
        &cookie,
        Some(json!({"name": "Kitchen"})),
    )
    .await;
    let bearer = format!("Bearer {}", r.json()["token"].as_str().unwrap());
    let dash = |a: &App, bearer: String| {
        let router = a.router.clone();
        async move {
            call(
                &router,
                Method::GET,
                "/api/v1/dashboard",
                &[("authorization", &bearer)],
                None,
                None,
            )
            .await
        }
    };
    let r = dash(&a, bearer.clone()).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.json().get("inbox").is_none());
    let list = r.json()["pending_approvals"].clone();
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    assert_eq!(list[0]["count"], json!(0));

    let h = send(&a, Method::GET, "/api/v1/household", &cookie, None).await;
    assert_eq!(h.json()["dashboard_pending_approvals"], json!(true));
    let version = h.json()["version"].to_string();
    let r = call(
        &a.router,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &cookie), ("if-match", &version)],
        None,
        Some(json!({"dashboard_pending_approvals": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["dashboard_pending_approvals"], json!(false));
    let r = dash(&a, bearer).await;
    assert_eq!(r.json()["pending_approvals"], json!([]));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn own_password_change_needs_the_current_one_and_ends_other_sessions(pool: PgPool) {
    let a = app(pool).await;
    let first = setup(&a).await;
    let r = send(
        &a,
        Method::POST,
        "/api/v1/auth/login",
        "",
        Some(json!({"username": "alex", "password": PW})),
    )
    .await;
    let second = pair_of(&r);
    let me = send(&a, Method::GET, "/api/v1/me", &first, None).await;
    let id = me.json()["actor"]["member"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let uri = format!("/api/v1/members/{id}/account/password");

    // Wrong current password: 401, and the old password still works.
    let r = send(&a, Method::POST, &uri, &first, Some(json!({"current_password": "wrong wrong wrong", "new_password": "a brand new password"}))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.text());
    // The old single-field body no longer changes your own password.
    let r = send(
        &a,
        Method::POST,
        &uri,
        &first,
        Some(json!({"password": "a brand new password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    // A weak new password is refused.
    let r = send(
        &a,
        Method::POST,
        &uri,
        &first,
        Some(json!({"current_password": PW, "new_password": "short"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);

    let r = send(
        &a,
        Method::POST,
        &uri,
        &first,
        Some(json!({"current_password": PW, "new_password": "a brand new password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    // The calling session survives; every other one is gone.
    assert_eq!(
        send(&a, Method::GET, "/api/v1/me", &first, None)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        send(&a, Method::GET, "/api/v1/me", &second, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let r = send(
        &a,
        Method::POST,
        "/api/v1/auth/login",
        "",
        Some(json!({"username": "alex", "password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = send(
        &a,
        Method::POST,
        "/api/v1/auth/login",
        "",
        Some(json!({"username": "alex", "password": "a brand new password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn member_writes_are_adult_only(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    // A child with an account.
    let r = send(
        &a,
        Method::POST,
        "/api/v1/members",
        &cookie,
        Some(json!({"display_name": "Sam", "role": "child"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let child = r.json()["id"].as_str().unwrap().to_owned();
    let r = send(
        &a,
        Method::POST,
        &format!("/api/v1/members/{child}/account"),
        &cookie,
        Some(json!({"username": "sam", "password": "kid1"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let r = send(
        &a,
        Method::POST,
        "/api/v1/auth/login",
        "",
        Some(json!({"username": "sam", "password": "kid1"})),
    )
    .await;
    let kid = pair_of(&r);
    let me = send(&a, Method::GET, "/api/v1/me", &cookie, None)
        .await
        .json();
    let adult = me["actor"]["member"]["id"].as_str().unwrap().to_owned();

    for (m, uri, body) in [
        (
            Method::POST,
            "/api/v1/members".to_owned(),
            Some(json!({"display_name": "X", "role": "child"})),
        ),
        (
            Method::PATCH,
            format!("/api/v1/members/{adult}"),
            Some(json!({"display_name": "Y", "version": 1})),
        ),
        (
            Method::PATCH,
            format!("/api/v1/members/{child}"),
            Some(json!({"display_name": "Y", "version": 1})),
        ),
        (
            Method::POST,
            format!("/api/v1/members/{child}/account"),
            Some(json!({"username": "z", "password": "kid1"})),
        ),
        (
            Method::POST,
            format!("/api/v1/members/{adult}/account/password"),
            Some(json!({"password": "kid2"})),
        ),
        (
            Method::PUT,
            format!("/api/v1/members/{adult}/pin"),
            Some(json!({"pin": "4321"})),
        ),
        (
            Method::DELETE,
            format!("/api/v1/members/{child}/avatar"),
            None,
        ),
    ] {
        let r = send(&a, m.clone(), &uri, &kid, body).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{m} {uri}: {}", r.text());
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn member_colours_are_hues(pool: PgPool) {
    let a = app(pool).await;
    let cookie = setup(&a).await;
    let post = |body: Value| send(&a, Method::POST, "/api/v1/members", &cookie, Some(body));

    let r = post(json!({"display_name": "Sam", "role": "child", "color": "purple"})).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_eq!(r.json()["color"], "purple");
    let id = r.json()["id"].as_str().unwrap().to_owned();
    let v = r.json()["version"].as_i64().unwrap();

    let r = post(json!({"display_name": "Kit", "role": "child", "color": "#ff0000"})).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());

    let uri = format!("/api/v1/members/{id}");
    let r = call(
        &a.router,
        Method::PATCH,
        &uri,
        &[("cookie", cookie.as_str()), ("if-match", &v.to_string())],
        None,
        Some(json!({"color": null})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.json()["color"].is_null());

    // The setup owner took a hue.
    let r = send(&a, Method::GET, "/api/v1/members", &cookie, None).await;
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["color"] == "purple")
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn about_shows_the_build_to_adults_only(pool: PgPool) {
    let a = app_with(
        pool,
        ApiConfig {
            build: fa_api::BuildInfo {
                version: "1.2.3".into(),
                commit: Some("abc123def456".into()),
                built_at: Some("2026-10-29T12:00:00Z".into()),
            },
            ..ApiConfig::default()
        },
        AuthConfig::default(),
    )
    .await;
    let cookie = setup(&a).await;
    let r = send(&a, Method::GET, "/api/v1/system/about", &cookie, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let j = r.json();
    assert_eq!(j["version"], "1.2.3");
    assert_eq!(j["commit"], "abc123def456");
    assert_eq!(j["built_at"], "2026-10-29T12:00:00Z");
    let r = send(&a, Method::GET, "/api/v1/system/about", "", None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.text());
}
