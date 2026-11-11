//! Status page: failed and retrying job diagnostics.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use chrono::{Duration as Dur, Utc};
use fa_api::{ApiConfig, AppState, Services, router, run_bridge_signalled};
use fa_auth::{AuthConfig, AuthService, DeviceService, hash_password};
use fa_domain::*;
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

const PW: &str = "adult password 123";

struct Fx {
    app: Router,
    store: Store,
    hid: HouseholdId,
    adult: MemberId,
    adult2: MemberId,
    child: MemberId,
    device: DeviceId,
    device_token: String,
    adult_cookie: String,
    child_cookie: String,
    /// A device token with an active adult PIN elevation.
    elevated_token: String,
}

struct Resp {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
    fn header(&self, k: &str) -> String {
        self.headers
            .get(k)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
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
    fa_api::MailServices::new(std::sync::Arc::new(fa_mail::MailDeps {
        llm: None,
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

fn key() -> MasterKey {
    MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
}

fn services(store: &Store) -> Services {
    let oauth = fa_auth::OAuthService::new(
        store.clone(),
        key(),
        None,
        "https://fa.example.com",
        fa_connectors::Http::new(false).unwrap(),
    );
    Services {
        calendar: Arc::new(
            fa_calendar::CalendarService::new(
                store.clone(),
                key(),
                fa_calendar::ConnectorConfig {
                    allow_private_hosts: true,
                    ..Default::default()
                },
                Arc::new(oauth.clone()),
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
        mail: mail_services(store, &oauth),
        oauth,
        tasks: Arc::new(fa_tasks::TaskService::new(store.clone())),
        meals: std::sync::Arc::new(fa_meals::MealsService::new(store.clone(), None, true)),
        notify: fa_notify::NotifyService::new(
            store.clone(),
            fa_notify::NotifyConfig {
                ntfy_url: None,
                ntfy_token: None,
                public_base_url: "https://fa.example.com".into(),
                subscribe_base: None,
            },
        ),
    }
}

async fn seed(pool: PgPool) -> Fx {
    let store = Store::from_pool(pool);
    let hid = HouseholdId::new();
    let adult = member(hid, "Alex", Role::Adult);
    let adult2 = member(hid, "Jo", Role::Adult);
    let child = member(hid, "Sam", Role::Child);
    // One hash for everything: debug-build argon2 is slow when many tests run at once.
    let hash = hash_password(PW).unwrap();
    let mut sessions: Vec<(MemberId, UserAccountId, String)> = vec![];
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(
        &mut tx,
        &Household {
            dashboard_pending_approvals: true,
            setup_completed_at: None,
            id: hid,
            name: "Example".into(),
            tz: "UTC".into(),
            location: None,
            meal_plan_horizon_weeks: 2,
            dashboard_completion_override: false,
            units: Units::Metric,
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
    for (m, user) in [(&adult, "alex"), (&child, "sam"), (&adult2, "jo")] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
        let account = UserAccountId::new();
        sessions.push((m.id, account, format!("test-session-{user}")));
        UserAccountRepo::insert(
            &mut tx,
            &UserAccount {
                id: account,
                household_id: hid,
                member_id: m.id,
                username: user.into(),
                password_hash: hash.clone(),
                disabled_at: None,
                initial_password: false,
                created_at: Utc::now(),
            },
        )
        .await
        .unwrap();
    }
    for (member_id, account_id, id) in &sessions {
        SessionRepo::put(
            &mut tx,
            &SessionRecord {
                id: id.clone(),
                household_id: hid,
                member_id: *member_id,
                account_id: *account_id,
                data: vec![],
                user_agent: None,
                created_at: Utc::now(),
                last_seen: Utc::now(),
                expires_at: Utc::now() + Dur::hours(8),
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(hid).await.unwrap();
    fa_meals::ensure_defaults(&mut tx).await.unwrap();
    tx.commit().await.unwrap();

    let devices = DeviceService::new(store.clone(), "http://localhost");
    let plain = devices.pair_system(hid, "Kitchen").await.unwrap();
    let elevated = devices.pair_system(hid, "Hall").await.unwrap();

    let auth = AuthService::new(store.clone(), AuthConfig::default());
    let state = AppState::new(
        store.clone(),
        auth,
        services(&store),
        ApiConfig {
            cookie_secure: fa_config::CookieSecure::Never,
            heartbeat_interval: Duration::from_secs(60),
            // Debug-build argon2 is slow when many tests run at once.
            request_timeout: Duration::from_secs(120),
            ..ApiConfig::default()
        },
        CancellationToken::new(),
    );
    let ready = Arc::new(tokio::sync::Notify::new());
    tokio::spawn(run_bridge_signalled(
        store.clone(),
        state.hub.clone(),
        CancellationToken::new(),
        Some(ready.clone()),
    ));
    tokio::time::timeout(Duration::from_secs(5), ready.notified())
        .await
        .expect("bridge listening");
    let app = router(state);

    let fx = Fx {
        app,
        store,
        hid,
        adult: adult.id,
        adult2: adult2.id,
        child: child.id,
        device: plain.device.id,
        device_token: plain.token,
        adult_cookie: "fa_session=test-session-alex".into(),
        child_cookie: "fa_session=test-session-sam".into(),
        elevated_token: elevated.token,
    };
    // An adult has confirmed their PIN on the second device.
    let mut tx = fx.store.tx_for(hid).await.unwrap();
    DeviceElevationRepo::create(
        &mut tx,
        &DeviceElevation {
            device_id: elevated.device.id,
            member_id: fx.adult,
            expires_at: Utc::now() + Dur::minutes(10),
        },
        hid,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    fx
}

async fn call(app: &Router, req: Request<Body>) -> Resp {
    let res = app.clone().oneshot(req).await.unwrap();
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

async fn send_raw(
    app: &Router,
    m: Method,
    uri: &str,
    hdrs: &[(&str, &str)],
    body: Vec<u8>,
) -> Resp {
    let mut b = Request::builder().method(m.clone()).uri(uri);
    if m != Method::GET {
        b = b.header("x-fa-request", "1");
    }
    for (k, v) in hdrs {
        b = b.header(*k, *v);
    }
    call(app, b.body(Body::from(body)).unwrap()).await
}

async fn send(
    app: &Router,
    m: Method,
    uri: &str,
    hdrs: &[(&str, &str)],
    body: Option<Value>,
) -> Resp {
    match body {
        Some(v) => {
            let mut h = hdrs.to_vec();
            h.push(("content-type", "application/json"));
            send_raw(app, m, uri, &h, v.to_string().into_bytes()).await
        }
        None => send_raw(app, m, uri, hdrs, Vec::new()).await,
    }
}

#[derive(Clone, Copy, Debug)]
enum Who {
    Anon,
    Adult,
    Child,
    Device,
    Elevated,
}

fn auth_of(fx: &Fx, who: Who) -> Vec<(String, String)> {
    let bearer = |t: &str| vec![("authorization".to_owned(), format!("Bearer {t}"))];
    match who {
        Who::Anon => vec![],
        Who::Adult => vec![("cookie".into(), fx.adult_cookie.clone())],
        Who::Child => vec![("cookie".into(), fx.child_cookie.clone())],
        Who::Device => bearer(&fx.device_token),
        Who::Elevated => bearer(&fx.elevated_token),
    }
}

async fn as_(fx: &Fx, who: Who, m: Method, uri: &str, body: Option<Value>) -> Resp {
    let h = auth_of(fx, who);
    let h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    send(&fx.app, m, uri, &h, body).await
}

async fn insert_job(fx: &Fx, hid: HouseholdId, kind: &str, status: &str, attempts: i32, err: &str) {
    let mut tx = fx.store.tx_system().await.unwrap();
    sqlx::query(
        "INSERT INTO job (id, household_id, kind, payload, status, attempts, max_attempts, last_error)
         VALUES ($1, $2, $3, $4, $5, $6, 5, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(hid.as_uuid())
    .bind(kind)
    .bind(json!({"message_id": "m-1", "body": "SECRET MAIL BODY"}))
    .bind(status)
    .bind(attempts)
    .bind(err)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn failed_jobs_are_household_scoped_adult_only_and_hide_payloads(pool: PgPool) {
    let fx = seed(pool).await;
    // A second household whose jobs must not leak.
    let other = HouseholdId::new();
    let mut tx = fx.store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'Other', 'UTC')")
        .bind(other.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    insert_job(
        &fx,
        fx.hid,
        "mail.extract",
        "dead",
        5,
        "llm timeout\nsecond line",
    )
    .await;
    insert_job(
        &fx,
        fx.hid,
        "mail.extract",
        "dead",
        5,
        "llm timeout\nother tail",
    )
    .await;
    insert_job(&fx, fx.hid, "mail.sync", "queued", 2, "401 unauthorized").await;
    insert_job(&fx, fx.hid, "mail.sync", "queued", 0, "never ran").await;
    insert_job(&fx, other, "mail.extract", "dead", 5, "foreign error").await;

    let uri = "/api/v1/status/jobs";
    assert_eq!(
        as_(&fx, Who::Child, Method::GET, uri, None).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        as_(&fx, Who::Anon, Method::GET, uri, None).await.status,
        StatusCode::UNAUTHORIZED
    );
    let r = as_(&fx, Who::Adult, Method::GET, uri, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let v = r.json();
    assert_eq!(
        (v["dead"].as_i64(), v["retrying"].as_i64()),
        (Some(2), Some(1))
    );
    assert_eq!(v["jobs"].as_array().unwrap().len(), 3);
    let groups = v["groups"].as_array().unwrap();
    assert_eq!(groups[0]["count"], 2);
    assert_eq!(groups[0]["error"], "llm timeout");
    assert_eq!(v["jobs"][0]["refs"]["message_id"], "m-1");
    let text = r.text();
    assert!(!text.contains("SECRET MAIL BODY") && !text.contains("foreign error"));

    // Retry requeues only this household's dead jobs of the kind.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        "/api/v1/status/jobs/retry",
        Some(json!({"kind": "mail.extract"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["requeued"], 2);
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        "/api/v1/status/jobs/retry",
        Some(json!({"kind": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let mut tx = fx.store.tx_system().await.unwrap();
    let dead: i64 = sqlx::query_scalar("SELECT count(*) FROM job WHERE status = 'dead'")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(dead, 1, "the other household's dead job stays");
}
