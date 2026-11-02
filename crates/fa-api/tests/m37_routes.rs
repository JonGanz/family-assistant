//! M3.7 C-13: `/devices/elevate` and `/members/{member_id}/away/{id}` and their deprecated aliases.
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
    fa_api::MailServices::new(std::sync::Arc::new(fa_mail::MailDeps {
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

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn elevate_new_path_and_deprecated_alias(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let bearer = pin_and_device(&fx, &adult).await;
    let dev = [("authorization", bearer.as_str())];
    let body = || Some(json!({"member_id": fx.adult, "pin": "4321"}));

    for uri in ["/api/v1/devices/elevate", "/api/v1/device/elevate"] {
        let r = send(&fx.app, Method::POST, uri, &dev, body()).await;
        assert_eq!(r.status, StatusCode::OK, "{uri}");
        assert!(r.json()["expires_at"].is_string(), "{uri}");
        let me = send(&fx.app, Method::GET, "/api/v1/me", &dev, None).await;
        assert_eq!(me.json()["actor"]["kind"], "elevated_device", "{uri}");
    }
    // a signed-in user is not a device, on both paths
    for uri in ["/api/v1/devices/elevate", "/api/v1/device/elevate"] {
        let r = send(&fx.app, Method::POST, uri, &[("cookie", &adult)], body()).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn away_delete_under_member_and_deprecated_alias(pool: PgPool) {
    let fx = seed(pool).await;
    let adult = login(&fx, "alex", ADULT_PW).await;
    let c = [("cookie", adult.as_str())];
    let add = |member: MemberId| {
        let uri = format!("/api/v1/members/{member}/away");
        let fx = &fx;
        async move {
            let r = send(
                &fx.app,
                Method::POST,
                &uri,
                &c,
                Some(json!({"from": "2030-01-01", "until": "2030-01-05", "note": "trip"})),
            )
            .await;
            assert_eq!(r.status, StatusCode::CREATED);
            r.json()["id"].as_str().unwrap().to_owned()
        }
    };
    let list = |member: MemberId| {
        let uri = format!("/api/v1/members/{member}/away");
        let fx = &fx;
        async move {
            send(&fx.app, Method::GET, &uri, &c, None)
                .await
                .json()
                .as_array()
                .unwrap()
                .len()
        }
    };

    // new path: the row must belong to the member in the path
    let a1 = add(fx.child).await;
    let wrong = send(
        &fx.app,
        Method::DELETE,
        &format!("/api/v1/members/{}/away/{a1}", fx.adult2),
        &c,
        None,
    )
    .await;
    assert_eq!(wrong.status, StatusCode::NOT_FOUND);
    assert_eq!(list(fx.child).await, 1);
    let ok = send(
        &fx.app,
        Method::DELETE,
        &format!("/api/v1/members/{}/away/{a1}", fx.child),
        &c,
        None,
    )
    .await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT);
    assert_eq!(list(fx.child).await, 0);

    // deprecated alias still works
    let a2 = add(fx.child).await;
    let old = send(
        &fx.app,
        Method::DELETE,
        &format!("/api/v1/away/{a2}"),
        &c,
        None,
    )
    .await;
    assert_eq!(old.status, StatusCode::NO_CONTENT);
    assert_eq!(list(fx.child).await, 0);

    // both are adult-only
    let child = login(&fx, "sam", CHILD_PW).await;
    let a3 = add(fx.child).await;
    for uri in [
        format!("/api/v1/members/{}/away/{a3}", fx.child),
        format!("/api/v1/away/{a3}"),
    ] {
        let r = send(&fx.app, Method::DELETE, &uri, &[("cookie", &child)], None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[test]
fn openapi_marks_the_aliases_deprecated() {
    let doc = serde_json::to_value(fa_api::openapi()).unwrap();
    let p = &doc["paths"];
    assert_eq!(p["/api/v1/device/elevate"]["post"]["deprecated"], true);
    assert_eq!(p["/api/v1/away/{id}"]["delete"]["deprecated"], true);
    assert!(p["/api/v1/devices/elevate"]["post"]["deprecated"].is_null());
    assert!(p["/api/v1/members/{member_id}/away/{id}"]["delete"].is_object());
}
