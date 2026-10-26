use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use chrono::Utc;
use fa_api::{ApiConfig, AppState, Services, router, run_bridge_signalled};
use fa_auth::{AuthConfig, AuthService, hash_password};
use fa_domain::*;
use fa_store::*;
use futures::StreamExt;
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
    fa_api::MailServices::new(std::sync::Arc::new(fa_mail::MailDeps {
        store: store.clone(),
        http: fa_connectors::Http::new(true).unwrap(),
        tokens: std::sync::Arc::new(oauth.clone()),
        key: std::sync::Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        cfg: fa_mail::MailConfig::default(),
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
            id: h,
            name: "Example".into(),
            tz: "UTC".into(),
            location: None,
            meal_plan_horizon_weeks: 2,
            dashboard_completion_override: false,
            units: fa_domain::Units::Metric,
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
                fa_calendar::ConnectorConfig::default(),
                std::sync::Arc::new(oauth.clone()),
            )
            .unwrap(),
        ),
        weather: fa_weather::WeatherService::new(
            store.clone(),
            fa_weather::WeatherConfig::default(),
        )
        .unwrap(),
        tasks: std::sync::Arc::new(fa_tasks::TaskService::new(store.clone())),
        notify: fa_notify::NotifyService::new(
            store.clone(),
            fa_notify::NotifyConfig {
                ntfy_url: None,
                ntfy_token: None,
                public_base_url: "https://fa.example.com".into(),
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
            cookie_secure: false,
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

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn login_and_me(pool: PgPool) {
    let fx = seed(pool).await;
    let bad = send(
        &fx.app,
        Method::POST,
        "/api/v1/auth/login",
        &[],
        Some(json!({"username": "alex", "password": "nope nope nope"})),
    )
    .await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        bad.headers.get("content-type").unwrap(),
        "application/problem+json"
    );

    let adult = login(&fx, "alex", ADULT_PW).await;
    let me = send(
        &fx.app,
        Method::GET,
        "/api/v1/me",
        &[("cookie", &adult)],
        None,
    )
    .await;
    assert_eq!(me.status, StatusCode::OK);
    let j = me.json();
    assert_eq!(j["actor"]["kind"], "user");
    assert_eq!(j["actor"]["member"]["id"], fx.adult.to_string());
    assert_eq!(j["actor"]["member"]["role"], "adult");
    assert_eq!(j["household"]["name"], "Example");

    let child = login(&fx, "sam", CHILD_PW).await;
    let me = send(
        &fx.app,
        Method::GET,
        "/api/v1/me",
        &[("cookie", &child)],
        None,
    )
    .await;
    assert_eq!(me.json()["actor"]["member"]["role"], "child");

    let anon = send(&fx.app, Method::GET, "/api/v1/me", &[], None).await;
    assert_eq!(anon.status, StatusCode::UNAUTHORIZED);

    let out = send(
        &fx.app,
        Method::POST,
        "/api/v1/auth/logout",
        &[("cookie", &child)],
        None,
    )
    .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    let me = send(
        &fx.app,
        Method::GET,
        "/api/v1/me",
        &[("cookie", &child)],
        None,
    )
    .await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn member_policy_and_versions(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let child = login(&fx, "sam", CHILD_PW).await;
    let new = json!({"display_name": "Kit", "role": "child"});

    let denied = send(
        &fx.app,
        Method::POST,
        "/api/v1/members",
        &[("cookie", &child)],
        Some(new.clone()),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);

    let created = send(
        &fx.app,
        Method::POST,
        "/api/v1/members",
        &[("cookie", &adult)],
        Some(new),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let kit = created.json();
    let id = kit["id"].as_str().unwrap().to_owned();
    let v = kit["version"].as_i64().unwrap();

    // Adults see everything, children only the public fields.
    let full = send(
        &fx.app,
        Method::GET,
        "/api/v1/members",
        &[("cookie", &adult)],
        None,
    )
    .await;
    assert_eq!(full.json().as_array().unwrap().len(), 4);
    assert!(full.json()[0].get("has_pin").is_some());
    let public = send(
        &fx.app,
        Method::GET,
        "/api/v1/members",
        &[("cookie", &child)],
        None,
    )
    .await;
    assert!(public.json()[0].get("has_pin").is_none());
    assert!(public.json()[0].get("display_name").is_some());

    let uri = format!("/api/v1/members/{id}");
    let no_version = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &adult)],
        Some(json!({"grade": "3"})),
    )
    .await;
    assert_eq!(no_version.status, StatusCode::PRECONDITION_REQUIRED);

    let ifm = format!("\"{v}\"");
    let ok = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &adult), ("if-match", &ifm)],
        Some(json!({"grade": "3"})),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.json()["grade"], "3");
    assert_eq!(ok.json()["version"], v + 1);

    let stale = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &adult), ("if-match", &ifm)],
        Some(json!({"grade": "4"})),
    )
    .await;
    assert_eq!(stale.status, StatusCode::CONFLICT);

    // Explicit null clears; an adult can edit themself but never another adult.
    let me_uri = format!("/api/v1/members/{}", fx.adult);
    let list = full.json();
    let my_v = list
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == fx.adult.to_string())
        .unwrap()["version"]
        .as_i64()
        .unwrap();
    let mine = send(
        &fx.app,
        Method::PATCH,
        &me_uri,
        &[("cookie", &adult), ("if-match", &my_v.to_string())],
        Some(json!({"avatar": null, "color": "#fff"})),
    )
    .await;
    assert_eq!(mine.status, StatusCode::OK);
    let other = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/members/{}", fx.adult2),
        &[("cookie", &adult), ("if-match", "1")],
        Some(json!({"grade": "x"})),
    )
    .await;
    assert_eq!(other.status, StatusCode::FORBIDDEN);
    let by_child = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &child), ("if-match", "1")],
        Some(json!({"grade": "x"})),
    )
    .await;
    assert_eq!(by_child.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn accounts_and_passwords(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let kit = send(
        &fx.app,
        Method::POST,
        "/api/v1/members",
        &[("cookie", &adult)],
        Some(json!({"display_name": "Kit", "role": "child"})),
    )
    .await
    .json();
    let uri = format!("/api/v1/members/{}/account", kit["id"].as_str().unwrap());
    let short = send(
        &fx.app,
        Method::POST,
        &uri,
        &[("cookie", &adult)],
        Some(json!({"username": "kit", "password": "abc"})),
    )
    .await;
    assert_eq!(short.status, StatusCode::UNPROCESSABLE_ENTITY);
    let ok = send(
        &fx.app,
        Method::POST,
        &uri,
        &[("cookie", &adult)],
        Some(json!({"username": "kit", "password": "abcd"})),
    )
    .await;
    assert_eq!(ok.status, StatusCode::CREATED);
    login(&fx, "kit", "abcd").await;
    let reset = send(
        &fx.app,
        Method::POST,
        &format!("{uri}/password"),
        &[("cookie", &adult)],
        Some(json!({"password": "wxyz1"})),
    )
    .await;
    assert_eq!(reset.status, StatusCode::NO_CONTENT);
    login(&fx, "kit", "wxyz1").await;
    // A child may not reset an adult's password.
    let child = login(&fx, "sam", CHILD_PW).await;
    let r = send(
        &fx.app,
        Method::POST,
        &format!("/api/v1/members/{}/account/password", fx.adult),
        &[("cookie", &child)],
        Some(json!({"password": "a long enough one"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn device_pairing_and_pin_lockout(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;

    let pin = send(
        &fx.app,
        Method::PUT,
        &format!("/api/v1/members/{}/pin", fx.adult),
        &[("cookie", &adult)],
        Some(json!({"pin": "4321"})),
    )
    .await;
    assert_eq!(pin.status, StatusCode::NO_CONTENT);

    let paired = send(
        &fx.app,
        Method::POST,
        "/api/v1/devices",
        &[("cookie", &adult)],
        Some(json!({"name": "Kitchen"})),
    )
    .await;
    assert_eq!(paired.status, StatusCode::CREATED);
    let token = paired.json()["token"].as_str().unwrap().to_owned();
    let device_id = paired.json()["id"].as_str().unwrap().to_owned();
    assert!(
        paired.json()["pair_url"]
            .as_str()
            .unwrap()
            .contains("/dashboard?device=")
    );
    let bearer = format!("Bearer {token}");
    let dev = [("authorization", bearer.as_str())];

    let me = send(&fx.app, Method::GET, "/api/v1/me", &dev, None).await;
    assert_eq!(me.json()["actor"]["kind"], "device");
    // Devices see public fields and cannot manage anything.
    let members = send(&fx.app, Method::GET, "/api/v1/members", &dev, None).await;
    assert!(members.json()[0].get("has_pin").is_none());
    let denied = send(&fx.app, Method::GET, "/api/v1/devices", &dev, None).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);

    // Correct PIN elevates; /me shows the elevated device.
    let el_body = |p: &str| Some(json!({"member_id": fx.adult, "pin": p}));
    let ok = send(
        &fx.app,
        Method::POST,
        "/api/v1/device/elevate",
        &dev,
        el_body("4321"),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
    let me = send(&fx.app, Method::GET, "/api/v1/me", &dev, None).await;
    assert_eq!(me.json()["actor"]["kind"], "elevated_device");

    // Five wrong PINs lock the adult's PIN, even for the right one.
    for _ in 0..5 {
        let r = send(
            &fx.app,
            Method::POST,
            "/api/v1/device/elevate",
            &dev,
            el_body("0000"),
        )
        .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    }
    let locked = send(
        &fx.app,
        Method::POST,
        "/api/v1/device/elevate",
        &dev,
        el_body("4321"),
    )
    .await;
    assert_eq!(locked.status, StatusCode::LOCKED);

    // Revocation ends the device's access.
    let rev = send(
        &fx.app,
        Method::DELETE,
        &format!("/api/v1/devices/{device_id}"),
        &[("cookie", &adult)],
        None,
    )
    .await;
    assert_eq!(rev.status, StatusCode::NO_CONTENT);
    let me = send(&fx.app, Method::GET, "/api/v1/me", &dev, None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn sessions_listing_and_revocation(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let _child = login(&fx, "sam", CHILD_PW).await;
    let uri = format!("/api/v1/sessions?member={}", fx.child);
    let list = send(&fx.app, Method::GET, &uri, &[("cookie", &adult)], None).await;
    assert_eq!(list.status, StatusCode::OK);
    let handle = list.json()[0]["id"].as_str().unwrap().to_owned();
    let del = send(
        &fx.app,
        Method::DELETE,
        &format!("/api/v1/sessions/{handle}?member={}", fx.child),
        &[("cookie", &adult)],
        None,
    )
    .await;
    assert_eq!(del.status, StatusCode::NO_CONTENT);
    let list = send(&fx.app, Method::GET, &uri, &[("cookie", &adult)], None).await;
    assert!(list.json().as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn csrf_header_required(pool: PgPool) {
    let fx = seed(pool).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"username": "alex", "password": ADULT_PW}).to_string(),
        ))
        .unwrap();
    let res = fx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        res.headers().get("content-type").unwrap(),
        "application/problem+json"
    );
}

async fn read_until(
    stream: &mut (impl futures::Stream<Item = Result<axum::body::Bytes, axum::Error>> + Unpin),
    buf: &mut String,
    needle: &str,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !buf.contains(needle) {
            let chunk = stream.next().await.expect("stream ended").unwrap();
            buf.push_str(&String::from_utf8_lossy(&chunk));
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {needle:?}; got {buf:?}"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn sse_heartbeat_and_member_updated(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let res = fx
        .app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v1/stream",
            &[("cookie", &adult)],
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    let mut stream = res.into_body().into_data_stream();
    let mut buf = String::new();
    read_until(&mut stream, &mut buf, "event: heartbeat").await;

    let list = send(
        &fx.app,
        Method::GET,
        "/api/v1/members",
        &[("cookie", &adult)],
        None,
    )
    .await
    .json();
    let me = list
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == fx.adult.to_string())
        .unwrap();
    let v = me["version"].as_i64().unwrap().to_string();
    let p = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/members/{}", fx.adult),
        &[("cookie", &adult), ("if-match", &v)],
        Some(json!({"grade": "x"})),
    )
    .await;
    assert_eq!(p.status, StatusCode::OK);
    read_until(&mut stream, &mut buf, "event: member.updated").await;
    assert!(buf.contains("id: 1"));

    // Reconnecting with Last-Event-ID replays what was missed.
    let res = fx
        .app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v1/stream",
            &[("cookie", &adult), ("last-event-id", "0")],
            None,
        ))
        .await
        .unwrap();
    let mut stream = res.into_body().into_data_stream();
    let mut buf = String::new();
    read_until(&mut stream, &mut buf, "event: member.updated").await;
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn health_and_openapi(pool: PgPool) {
    let fx = seed(pool).await;
    let r = send(&fx.app, Method::GET, "/healthz", &[], None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.headers.get("x-request-id").is_some());
    let r = send(&fx.app, Method::GET, "/readyz", &[], None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = send(&fx.app, Method::GET, "/api/v1/openapi.json", &[], None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json()["paths"]["/api/v1/members/{id}"].is_object());
    let r = send(&fx.app, Method::GET, "/api/v1/nope", &[], None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

/// The home-network dashboard listener as seen from `peer` (D-064).
fn lan_app(fx: &Fx, peer: &str, allow_pin: bool) -> Router {
    let cfg = fa_api::LanConfig {
        allowed: fa_api::LanConfig::default_allowed(),
        allow_pin,
    };
    fa_api::lan_router(fx.state.clone(), cfg).layer(axum::extract::connect_info::MockConnectInfo(
        peer.parse::<std::net::SocketAddr>().unwrap(),
    ))
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn lan_listener_serves_devices_only(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let pin = send(
        &fx.app,
        Method::PUT,
        &format!("/api/v1/members/{}/pin", fx.adult),
        &[("cookie", &adult)],
        Some(json!({"pin": "4321"})),
    )
    .await;
    assert_eq!(pin.status, StatusCode::NO_CONTENT);
    let paired = send(
        &fx.app,
        Method::POST,
        "/api/v1/devices",
        &[("cookie", &adult)],
        Some(json!({"name": "Kitchen"})),
    )
    .await;
    // Without FA_LAN_DASHBOARD_URL there is no second link.
    assert!(paired.json()["lan_pair_url"].is_null());
    let bearer = format!("Bearer {}", paired.json()["token"].as_str().unwrap());
    let dev = [("authorization", bearer.as_str())];
    let lan = lan_app(&fx, "192.168.1.50:40000", false);

    // A device reads its dashboard.
    let dash = send(&lan, Method::GET, "/api/v1/dashboard?days=3", &dev, None).await;
    assert_eq!(
        dash.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&dash.body)
    );
    let me = send(&lan, Method::GET, "/api/v1/me", &dev, None).await;
    assert_eq!(me.json()["actor"]["kind"], "device");
    assert_eq!(
        send(&lan, Method::GET, "/healthz", &[], None).await.status,
        StatusCode::OK
    );

    // Anything a dashboard doesn't call is not routed, even for a device.
    for path in [
        "/api/v1/members",
        "/api/v1/devices",
        "/api/v1/messages",
        "/api/v1/openapi.json",
    ] {
        let r = send(&lan, Method::GET, path, &dev, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
    }
    let login_lan = send(
        &lan,
        Method::POST,
        "/api/v1/auth/login",
        &[],
        Some(json!({"username": "alex", "password": ADULT_PW})),
    )
    .await;
    assert_eq!(login_lan.status, StatusCode::NOT_FOUND);

    // A user's session works neither as a cookie nor as a bearer token.
    let r = send(&lan, Method::GET, "/api/v1/me", &[("cookie", &adult)], None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let session = format!("Bearer {}", adult.split_once('=').unwrap().1);
    let r = send(
        &lan,
        Method::GET,
        "/api/v1/me",
        &[("authorization", &session)],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // PIN elevation is off unless allowed.
    let el = Some(json!({"member_id": fx.adult, "pin": "4321"}));
    let r = send(
        &lan,
        Method::POST,
        "/api/v1/device/elevate",
        &dev,
        el.clone(),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let lan_pin = lan_app(&fx, "192.168.1.50:40000", true);
    let r = send(&lan_pin, Method::POST, "/api/v1/device/elevate", &dev, el).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );

    // Clients outside the allowed networks are refused, and X-Forwarded-For is ignored.
    let outside = lan_app(&fx, "203.0.113.9:40000", false);
    let r = send(
        &outside,
        Method::GET,
        "/api/v1/dashboard?days=3",
        &dev,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let spoof = [
        ("authorization", bearer.as_str()),
        ("x-forwarded-for", "192.168.1.5"),
    ];
    let r = send(
        &outside,
        Method::GET,
        "/api/v1/dashboard?days=3",
        &spoof,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}
