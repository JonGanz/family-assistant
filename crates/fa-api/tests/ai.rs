//! M4 T-08: the `/ai/*` routes against an in-process stub LLM.
#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use chrono::Utc;
use fa_api::{ApiConfig, AppState, Services, router, run_bridge_signalled};
use fa_auth::{AuthConfig, AuthService, hash_password};
use fa_domain::*;
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

const ADULT_PW: &str = "adult password 123";
const CHILD_PW: &str = "kid1";

struct Fx {
    app: Router,
    state: AppState,
    adult: MemberId,
    child: MemberId,
    adult2: MemberId,
}

fn member(h: HouseholdId, name: &str, role: Role) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: name.into(),
        role,
        avatar: None,
        color: None,
        birthdate: None,
        grade: None,
        interests: vec![],
        hobbies: vec![],
        responsibilities: vec![],
        aliases: Vec::new(),
        gender: None,
        relevance_tags: Vec::new(),
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn mail_services(store: &Store, oauth: &fa_auth::OAuthService) -> fa_api::MailServices {
    let key = std::sync::Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap());
    let provider =
        fa_llm::Provider::new(store.clone(), key.clone(), fa_llm::default_http().unwrap());
    fa_api::MailServices::new(std::sync::Arc::new(fa_mail::MailDeps {
        llm: Some(std::sync::Arc::new(provider)),
        store: store.clone(),
        http: fa_connectors::Http::new(true).unwrap(),
        tokens: std::sync::Arc::new(oauth.clone()),
        key: std::sync::Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        cfg: fa_mail::MailConfig {
            allow_private_hosts: true,
            ..Default::default()
        },
    }))
}

async fn seed(pool: PgPool) -> Fx {
    let store = Store::from_pool(pool);
    let h = HouseholdId::new();
    let adult = member(h, "Alex", Role::Adult);
    let adult2 = member(h, "Jo", Role::Adult);
    let child = member(h, "Sam", Role::Child);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(
        &mut tx,
        &Household {
            dashboard_pending_approvals: true,
            setup_completed_at: None,
            id: h,
            name: "Example".into(),
            tz: "UTC".into(),
            location: None,
            meal_plan_horizon_weeks: 2,
            dashboard_completion_override: false,
            units: fa_domain::Units::Metric,
            week_start: Default::default(),
            calendar_past_days: 30,
            calendar_future_days: 400,
            weather_retention_days: 7,
            private_events_as_busy: true,
            now_list_size: 7,
            now_weights: fa_domain::tasks::NowWeights::default(),
            quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).expect("time"),
            quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
            currency: fa_domain::DEFAULT_CURRENCY.to_string(),
            gender_terms: Default::default(),
            disabled_signals: Vec::new(),
            events: Default::default(),
            inbox: Default::default(),
            retention_mail: Default::default(),
            bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    for (m, user, pw) in [
        (&adult, Some("alex"), ADULT_PW),
        (&child, Some("sam"), CHILD_PW),
        (&adult2, None, ADULT_PW),
    ] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
        if let Some(user) = user {
            UserAccountRepo::insert(
                &mut tx,
                &UserAccount {
                    id: UserAccountId::new(),
                    household_id: h,
                    member_id: m.id,
                    username: user.into(),
                    password_hash: hash_password(pw).unwrap(),
                    disabled_at: None,
                    initial_password: false,
                    created_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        }
    }
    tx.commit().await.unwrap();
    let auth = AuthService::new(store.clone(), AuthConfig::default());
    let oauth = fa_auth::OAuthService::new(
        store.clone(),
        MasterKey::from_bytes(&[7u8; 32], 1).unwrap(),
        None,
        "https://fa.example.com",
        fa_connectors::Http::new(false).unwrap(),
    );
    let services = Services {
        calendar: std::sync::Arc::new(
            fa_calendar::CalendarService::new(
                store.clone(),
                MasterKey::from_bytes(&[7u8; 32], 1).unwrap(),
                fa_calendar::ConnectorConfig {
                    allow_private_hosts: true,
                    ..Default::default()
                },
                std::sync::Arc::new(oauth.clone()),
            )
            .unwrap(),
        ),
        weather: fa_weather::WeatherService::new(
            store.clone(),
            fa_weather::WeatherConfig {
                allow_private_hosts: true,
                ..Default::default()
            },
        )
        .unwrap(),
        tasks: std::sync::Arc::new(fa_tasks::TaskService::new(store.clone())),
        notify: fa_notify::NotifyService::new(
            store.clone(),
            fa_notify::NotifyConfig {
                ntfy_url: None,
                ntfy_token: None,
                public_base_url: "https://fa.example.com".into(),
                subscribe_base: None,
            },
        ),
        mail: mail_services(&store, &oauth),
        oauth,
    };
    let state = AppState::new(
        store.clone(),
        auth,
        services,
        ApiConfig {
            cookie_secure: fa_config::CookieSecure::Never,
            ..ApiConfig::default()
        },
        CancellationToken::new(),
    );
    // Writers publish through NOTIFY; the bridge turns that into SSE events.
    let ready = std::sync::Arc::new(tokio::sync::Notify::new());
    tokio::spawn(run_bridge_signalled(
        store,
        state.hub.clone(),
        CancellationToken::new(),
        Some(ready.clone()),
    ));
    tokio::time::timeout(Duration::from_secs(5), ready.notified())
        .await
        .expect("the bridge starts listening");
    Fx {
        app: router(state.clone()),
        state,
        adult: adult.id,
        child: child.id,
        adult2: adult2.id,
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
}

fn request(method: Method, uri: &str, auth: &[(&str, &str)], body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    if method != Method::GET {
        b = b.header("x-fa-request", "1");
    }
    for (k, v) in auth {
        b = b.header(*k, *v);
    }
    match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    auth: &[(&str, &str)],
    body: Option<Value>,
) -> Resp {
    let res = app
        .clone()
        .oneshot(request(method, uri, auth, body))
        .await
        .unwrap();
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

/// Logs in and returns the `fa_session=...` cookie pair.
async fn login(fx: &Fx, user: &str, pw: &str) -> String {
    let r = send(
        &fx.app,
        Method::POST,
        "/api/v1/auth/login",
        &[],
        Some(json!({"username": user, "password": pw})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{:?}",
        String::from_utf8_lossy(&r.body)
    );
    let c = r.headers.get("set-cookie").unwrap().to_str().unwrap();
    assert!(c.contains("HttpOnly") && c.contains("SameSite=Lax"));
    c.split(';').next().unwrap().to_owned()
}

async fn pin_and_device(fx: &Fx, adult: &str) -> String {
    let r = send(
        &fx.app,
        Method::PUT,
        &format!("/api/v1/members/{}/pin", fx.adult),
        &[("cookie", adult)],
        Some(json!({"pin": "4321"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let paired = send(
        &fx.app,
        Method::POST,
        "/api/v1/devices",
        &[("cookie", adult)],
        Some(json!({"name": "Kitchen"})),
    )
    .await;
    assert_eq!(paired.status, StatusCode::CREATED);
    format!("Bearer {}", paired.json()["token"].as_str().unwrap())
}

const RECIPE_LLM: &str = r#"{"name":"Plain Soup","yield_text":"4 bowls","servings":4,"prep_minutes":5,"cook_minutes":10,"total_minutes":15,"ingredients":["2 cups water","1 tsp salt"],"steps":["Boil the water."],"tags":[]}"#;

/// A stub OpenAI-compatible server; returns its base URL (`.../v1`).
async fn stub_llm() -> String {
    use axum::routing::{get, post};
    let app = Router::new()
        .route(
            "/v1/models",
            get(|| async { axum::Json(json!({"data": [{"id": "stub-model"}]})) }),
        )
        .route(
            "/v1/chat/completions",
            post(|| async {
                axum::Json(json!({"choices": [{"message": {"content": RECIPE_LLM}}],
                                  "usage": {"prompt_tokens": 10, "completion_tokens": 20}}))
            }),
        );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{addr}/v1")
}

async fn closed_port_url() -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    format!("http://{addr}/v1")
}

const AI: &str = "/api/v1/ai";

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn children_and_devices_get_403(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let child = login(&fx, "sam", CHILD_PW).await;
    let bearer = pin_and_device(&fx, &adult).await;
    let ep = json!({"base_url": "http://127.0.0.1:1/v1", "model": "m", "dialect": "openai_chat",
                    "timeout_ms": 5000, "enabled": true});
    let calls: Vec<(Method, String, Option<Value>)> = vec![
        (Method::GET, format!("{AI}/llm-endpoint"), None),
        (Method::PUT, format!("{AI}/llm-endpoint"), Some(ep)),
        (
            Method::POST,
            format!("{AI}/llm-endpoint/test"),
            Some(json!({})),
        ),
        (Method::GET, format!("{AI}/llm-endpoint/models"), None),
        (Method::GET, format!("{AI}/settings"), None),
        (
            Method::PUT,
            format!("{AI}/settings"),
            Some(json!({"mail_extract_mode": "off", "max_input_chars": 5000})),
        ),
        (Method::GET, format!("{AI}/pipeline"), None),
        (
            Method::PUT,
            format!("{AI}/pipeline/email.category"),
            Some(json!({"llm_enabled": true, "llm_threshold": 0.8})),
        ),
        (Method::GET, format!("{AI}/stats"), None),
        (
            Method::POST,
            format!("{AI}/recipes/extract"),
            Some(json!({"html": "<p>x</p>"})),
        ),
    ];
    let child_c = [("cookie", child.as_str())];
    let dev = [("authorization", bearer.as_str())];
    for (m, uri, body) in calls {
        for who in [&child_c[..], &dev[..]] {
            let r = send(&fx.app, m.clone(), &uri, who, body.clone()).await;
            assert_eq!(r.status, StatusCode::FORBIDDEN, "{m} {uri}");
        }
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn endpoint_key_is_never_returned(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let uri = format!("{AI}/llm-endpoint");
    assert_eq!(
        send(&fx.app, Method::GET, &uri, &c, None).await.status,
        StatusCode::NOT_FOUND
    );
    let mut body = json!({"base_url": "http://192.0.2.10:11434/v1/", "model": "m",
        "dialect": "openai_chat", "timeout_ms": 5000, "enabled": true, "api_key": "sk-secret-1"});
    let r = send(&fx.app, Method::PUT, &uri, &c, Some(body.clone())).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.json()["has_api_key"], true);
    assert_eq!(r.json()["base_url"], "http://192.0.2.10:11434/v1");
    assert!(!String::from_utf8_lossy(&r.body).contains("sk-secret-1"));
    let g = send(&fx.app, Method::GET, &uri, &c, None).await;
    assert_eq!(g.json()["has_api_key"], true);
    assert!(!String::from_utf8_lossy(&g.body).contains("sk-secret"));
    // absent keeps the key
    body.as_object_mut().unwrap().remove("api_key");
    body["model"] = json!("m2");
    let r = send(&fx.app, Method::PUT, &uri, &c, Some(body.clone())).await;
    assert_eq!(
        (r.json()["has_api_key"].clone(), r.json()["model"].clone()),
        (json!(true), json!("m2"))
    );
    // null clears it
    body["api_key"] = Value::Null;
    let r = send(&fx.app, Method::PUT, &uri, &c, Some(body.clone())).await;
    assert_eq!(r.json()["has_api_key"], false);
    // the audit trail never holds the key
    let mut tx = fx.state.store.tx_system().await.unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_entry WHERE after::text LIKE '%sk-secret%'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(n, 0);
    // validation
    for (field, v) in [
        ("base_url", json!("ftp://x/v1")),
        ("base_url", json!("http://u:p@host/v1")),
        ("base_url", json!("http://host/v1?a=1")),
        ("model", json!(" ")),
        ("timeout_ms", json!(10)),
    ] {
        let mut b = body.clone();
        b[field] = v;
        let r = send(&fx.app, Method::PUT, &uri, &c, Some(b)).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
        assert_eq!(r.json()["field"], field);
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn endpoint_test_and_models(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let test = format!("{AI}/llm-endpoint/test");
    let stub = stub_llm().await;
    // unsaved values
    let r = send(
        &fx.app,
        Method::POST,
        &test,
        &c,
        Some(json!({"base_url": stub, "model": "stub-model"})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.json()["ok"], true);
    assert_eq!(r.json()["model_found"], true);
    let r = send(
        &fx.app,
        Method::POST,
        &test,
        &c,
        Some(json!({"base_url": stub, "model": "other"})),
    )
    .await;
    assert_eq!(
        (r.json()["ok"].clone(), r.json()["model_found"].clone()),
        (json!(true), json!(false))
    );
    // a closed port is ok:false, never a 5xx
    let dead = closed_port_url().await;
    let r = send(
        &fx.app,
        Method::POST,
        &test,
        &c,
        Some(json!({"base_url": dead, "model": "m"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["ok"], false);
    assert_eq!(r.json()["error"], "unavailable");
    // nothing stored, nothing to test
    let r = send(&fx.app, Method::POST, &test, &c, None).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json()["code"], "bad_request");
    // stored values; models
    let put = json!({"base_url": stub, "model": "stub-model", "dialect": "openai_chat",
                     "timeout_ms": 5000, "enabled": true});
    send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/llm-endpoint"),
        &c,
        Some(put),
    )
    .await;
    let r = send(&fx.app, Method::POST, &test, &c, None).await;
    assert_eq!(r.json()["ok"], true);
    let r = send(
        &fx.app,
        Method::GET,
        &format!("{AI}/llm-endpoint/models"),
        &c,
        None,
    )
    .await;
    assert_eq!(r.json()["models"], json!(["stub-model"]));
    // the endpoint goes away: 502 llm_unavailable
    let put = json!({"base_url": dead, "model": "m", "dialect": "openai_chat",
                     "timeout_ms": 5000, "enabled": true});
    send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/llm-endpoint"),
        &c,
        Some(put),
    )
    .await;
    let r = send(
        &fx.app,
        Method::GET,
        &format!("{AI}/llm-endpoint/models"),
        &c,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_GATEWAY);
    assert_eq!(r.json()["code"], "llm_unavailable");
    // the test calls are in the stats
    let s = send(&fx.app, Method::GET, &format!("{AI}/stats"), &c, None).await;
    let calls = s.json()["calls"].clone();
    let t = calls
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["purpose"] == "endpoint.test")
        .unwrap();
    assert!(t["total"].as_i64().unwrap() >= 4);
    assert!(t["ok"].as_i64().unwrap() >= 3 && t["ok"].as_i64() < t["total"].as_i64());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pipeline_put_makes_a_version_and_stats_shape(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let g = send(&fx.app, Method::GET, &format!("{AI}/pipeline"), &c, None).await;
    assert_eq!(
        g.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&g.body)
    );
    let before = g.json();
    let pts = before.as_array().unwrap();
    assert_eq!(pts.len(), 4);
    assert!(pts.iter().all(|p| p["llm_enabled"] == false));
    let v0 = pts
        .iter()
        .find(|p| p["decision_point"] == "email.category")
        .unwrap()["version_id"]
        .clone();
    let uri = format!("{AI}/pipeline/email.category");
    let r = send(
        &fx.app,
        Method::PUT,
        &uri,
        &c,
        Some(json!({"llm_enabled": true, "llm_threshold": 0.8})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.json()["llm_enabled"], true);
    assert!((r.json()["llm_threshold"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    assert_ne!(r.json()["version_id"], v0);
    let g = send(&fx.app, Method::GET, &format!("{AI}/pipeline"), &c, None).await;
    let now = g.json();
    let cat = now
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["decision_point"] == "email.category")
        .unwrap()
        .clone();
    assert_eq!(cat["version_id"], r.json()["version_id"]);
    // validation and the other points
    let bad = send(
        &fx.app,
        Method::PUT,
        &uri,
        &c,
        Some(json!({"llm_enabled": true, "llm_threshold": 1.5})),
    )
    .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let other = send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/pipeline/event.calendar_route"),
        &c,
        Some(json!({"llm_enabled": true, "llm_threshold": 0.5})),
    )
    .await;
    assert_eq!(other.status, StatusCode::NOT_FOUND);
    let nope = send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/pipeline/nope"),
        &c,
        Some(json!({"llm_enabled": true, "llm_threshold": 0.5})),
    )
    .await;
    assert_eq!(nope.status, StatusCode::NOT_FOUND);
    // stats
    let s = send(
        &fx.app,
        Method::GET,
        &format!("{AI}/stats?days=500"),
        &c,
        None,
    )
    .await;
    assert_eq!(s.status, StatusCode::OK);
    let j = s.json();
    assert!(chrono::DateTime::parse_from_rfc3339(j["since"].as_str().unwrap()).is_ok());
    assert!(j["calls"].is_array() && j["stages"].is_array());
    // settings
    let r = send(&fx.app, Method::GET, &format!("{AI}/settings"), &c, None).await;
    assert_eq!(r.json()["mail_extract_mode"], "fallback");
    let r = send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/settings"),
        &c,
        Some(json!({"mail_extract_mode": "always", "max_input_chars": 8000})),
    )
    .await;
    assert_eq!(
        (
            r.json()["mail_extract_mode"].clone(),
            r.json()["max_input_chars"].clone()
        ),
        (json!("always"), json!(8000))
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_settings_rules_threshold_and_catchup(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let settings = format!("{AI}/settings");
    // The defaults of D-107.
    let r = send(&fx.app, Method::GET, &settings, &c, None).await;
    let j = r.json();
    assert_eq!(
        (j["mail_mode"].clone(), j["catchup_days"].clone()),
        (json!("llm_first"), json!(30))
    );
    assert!((j["extract_fallback_cutoff"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    // The old body keeps the new fields as they are.
    let r = send(
        &fx.app,
        Method::PUT,
        &settings,
        &c,
        Some(json!({"mail_extract_mode": "fallback", "max_input_chars": 8000})),
    )
    .await;
    assert_eq!(r.json()["mail_mode"], "llm_first");
    let r = send(
        &fx.app,
        Method::PUT,
        &settings,
        &c,
        Some(
            json!({"mail_extract_mode": "fallback", "max_input_chars": 8000,
                    "mail_mode": "rules_first", "catchup_days": 7, "extract_fallback_cutoff": 0.9}),
        ),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let j = send(&fx.app, Method::GET, &settings, &c, None).await.json();
    assert_eq!(
        (j["mail_mode"].clone(), j["catchup_days"].clone()),
        (json!("rules_first"), json!(7))
    );
    assert!((j["extract_fallback_cutoff"].as_f64().unwrap() - 0.9).abs() < 1e-6);
    for (field, v) in [
        ("catchup_days", json!(366)),
        ("extract_fallback_cutoff", json!(0.4)),
        ("extract_fallback_cutoff", json!(1.0)),
    ] {
        let mut b = json!({"mail_extract_mode": "fallback", "max_input_chars": 8000});
        b[field] = v;
        let r = send(&fx.app, Method::PUT, &settings, &c, Some(b)).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
        assert_eq!(r.json()["field"], field);
    }
    let r = send(
        &fx.app,
        Method::PUT,
        &settings,
        &c,
        Some(
            json!({"mail_extract_mode": "fallback", "max_input_chars": 8000, "mail_mode": "nope"}),
        ),
    )
    .await;
    assert!(r.status.is_client_error());

    // rules_threshold: default 0.8, a change is a new version, absent keeps it.
    let uri = format!("{AI}/pipeline/email.is_bill");
    let g = send(&fx.app, Method::GET, &format!("{AI}/pipeline"), &c, None)
        .await
        .json();
    let p = g
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["decision_point"] == "email.is_bill")
        .unwrap()
        .clone();
    assert!((p["rules_threshold"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    let put = |body: Value| {
        let (app, uri, cookie) = (fx.app.clone(), uri.clone(), adult.clone());
        async move {
            send(
                &app,
                Method::PUT,
                &uri,
                &[("cookie", cookie.as_str())],
                Some(body),
            )
            .await
        }
    };
    let r = put(json!({"llm_enabled": false, "llm_threshold": 0.7, "rules_threshold": 0.9})).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert!((r.json()["rules_threshold"].as_f64().unwrap() - 0.9).abs() < 1e-6);
    assert_ne!(r.json()["version_id"], p["version_id"]);
    let v1 = r.json()["version_id"].clone();
    let r = put(json!({"llm_enabled": false, "llm_threshold": 0.7})).await;
    assert!((r.json()["rules_threshold"].as_f64().unwrap() - 0.9).abs() < 1e-6);
    assert_eq!(r.json()["version_id"], v1);
    for bad in [0.4, 1.0] {
        let r =
            put(json!({"llm_enabled": false, "llm_threshold": 0.7, "rules_threshold": bad})).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json()["field"], "rules_threshold");
    }

    // Saving an enabled endpoint queues a catch-up; a disabled one does not.
    let ep = format!("{AI}/llm-endpoint");
    let mut body = json!({"base_url": "http://192.0.2.10:11434/v1", "model": "m",
        "dialect": "openai_chat", "timeout_ms": 5000, "enabled": false});
    send(&fx.app, Method::PUT, &ep, &c, Some(body.clone())).await;
    let mut tx = fx.state.store.tx_system().await.unwrap();
    let n = |kind: &'static str| {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM job WHERE kind = $1").bind(kind)
    };
    assert_eq!(n("mail.llm.catchup").fetch_one(&mut *tx).await.unwrap(), 0);
    tx.rollback().await.unwrap();
    body["enabled"] = json!(true);
    send(&fx.app, Method::PUT, &ep, &c, Some(body.clone())).await;
    send(&fx.app, Method::PUT, &ep, &c, Some(body)).await;
    let mut tx = fx.state.store.tx_system().await.unwrap();
    let payloads: Vec<Value> =
        sqlx::query_scalar("SELECT payload FROM job WHERE kind = 'mail.llm.catchup'")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    assert_eq!(payloads.len(), 2);
    assert!(payloads.iter().all(|p| p["household"].is_string()));
}

/// Needs `/ai/backlog` registered in `lib.rs` (and `ai::get_backlog` in `openapi.rs`), which T-22
/// does not own; the main session registers it and removes the `ignore`.
#[sqlx::test(migrations = "../fa-store/migrations")]
async fn backlog_counts_waiting_messages(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let r = send(&fx.app, Method::GET, &format!("{AI}/backlog"), &c, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json(), json!({"awaiting": 0, "queued": 0}));
    let child = login(&fx, "sam", CHILD_PW).await;
    let r = send(
        &fx.app,
        Method::GET,
        &format!("{AI}/backlog"),
        &[("cookie", child.as_str())],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

const JSONLD: &str = r#"<html><head><script type="application/ld+json">
{"@context":"https://schema.org","@type":"Recipe","name":"Plain Soup","recipeIngredient":["2 cups water","1 tsp salt"],
 "recipeInstructions":[{"@type":"HowToStep","text":"Boil the water."}],"prepTime":"PT5M"}
</script></head><body><h1>Plain Soup</h1></body></html>"#;

const NO_JSONLD: &str =
    "<html><body><h1>Plain Soup</h1><p>2 cups water, 1 tsp salt. Boil the water.</p></body></html>";

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn recipe_extract(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let uri = format!("{AI}/recipes/extract");
    // JSON-LD needs no endpoint
    let r = send(
        &fx.app,
        Method::POST,
        &uri,
        &c,
        Some(json!({"html": JSONLD})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.json()["origin"], "jsonld");
    assert_eq!(r.json()["prep_minutes"], 5);
    // no JSON-LD and no endpoint: 503
    let r = send(
        &fx.app,
        Method::POST,
        &uri,
        &c,
        Some(json!({"html": NO_JSONLD})),
    )
    .await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.json()["code"], "llm_unavailable");
    // a disabled endpoint is the same
    let stub = stub_llm().await;
    let mut put = json!({"base_url": stub, "model": "stub-model", "dialect": "openai_chat",
                         "timeout_ms": 5000, "enabled": false});
    send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/llm-endpoint"),
        &c,
        Some(put.clone()),
    )
    .await;
    let r = send(
        &fx.app,
        Method::POST,
        &uri,
        &c,
        Some(json!({"html": NO_JSONLD})),
    )
    .await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    // enabled: the stub answers
    put["enabled"] = json!(true);
    send(
        &fx.app,
        Method::PUT,
        &format!("{AI}/llm-endpoint"),
        &c,
        Some(put),
    )
    .await;
    let r = send(
        &fx.app,
        Method::POST,
        &uri,
        &c,
        Some(json!({"html": NO_JSONLD})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.json()["origin"], "llm");
    assert_eq!(r.json()["name"], "Plain Soup");
    let s = send(&fx.app, Method::GET, &format!("{AI}/stats"), &c, None).await;
    assert!(
        s.json()["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["purpose"] == "recipe.extract" && x["ok"] == 1)
    );
    // exactly one of url and html
    for b in [
        json!({}),
        json!({"url": "https://example.com/r", "html": "<p></p>"}),
    ] {
        let r = send(&fx.app, Method::POST, &uri, &c, Some(b)).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json()["code"], "bad_request");
    }
    // private hosts are blocked
    let r = send(
        &fx.app,
        Method::POST,
        &uri,
        &c,
        Some(json!({"url": "http://127.0.0.1:9/r"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json()["code"], "fetch_failed");
}
