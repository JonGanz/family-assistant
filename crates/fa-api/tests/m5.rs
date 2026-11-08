//! M5 routes: recipes, the meal plan, templates and the grocery list over HTTP.
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

// ---- helpers ----

const API: &str = "/api/v1";

fn today() -> chrono::NaiveDate {
    Utc::now().date_naive()
}

fn day(n: i64) -> String {
    (today() + Dur::days(n)).format("%Y-%m-%d").to_string()
}

fn id(v: &Value) -> String {
    sid(v, "id")
}

fn sid(v: &Value, key: &str) -> String {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} in {v}"))
        .to_owned()
}

fn recipe_body(title: &str, lines: &[&str]) -> Value {
    json!({"title": title, "import_method": "manual", "servings": 4, "tags": [],
           "lines": lines, "steps": ["Cook it."]})
}

async fn mk_recipe(fx: &Fx, title: &str, lines: &[&str]) -> Value {
    let r = as_(
        fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/recipes"),
        Some(recipe_body(title, lines)),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    r.json()
}

/// The id of the visible Dinner slot.
async fn dinner(fx: &Fx) -> String {
    let r = as_(
        fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/meals/slots"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let slots = r.json();
    let d = slots
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Dinner")
        .expect("seeded Dinner slot");
    sid(d, "id")
}

async fn plan_add(fx: &Fx, date: &str, slot: &str, body: Value) -> Value {
    let mut b = json!({"date": date, "slot_id": slot});
    for (k, v) in body.as_object().unwrap() {
        b[k] = v.clone();
    }
    let r = as_(
        fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-plan"),
        Some(b),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    r.json()
}

async fn plan_range(fx: &Fx, from: &str, to: &str) -> Vec<Value> {
    let r = as_(
        fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/meal-plan?from={from}&to={to}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    r.json().as_array().unwrap().clone()
}

async fn grocery(fx: &Fx) -> Value {
    let r = as_(fx, Who::Adult, Method::GET, &format!("{API}/grocery"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    r.json()
}

// ---- tests ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn authz_matrix_sample(pool: PgPool) {
    let fx = seed(pool).await;
    let items = format!("{API}/grocery/items");

    // A child adds and checks, but cannot delete or edit.
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        &items,
        Some(json!({"text": "milk"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let milk = id(&r.json());
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        &format!("{items}/{milk}/check"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.json()["checked_at"].is_string());
    let r = as_(
        &fx,
        Who::Child,
        Method::DELETE,
        &format!("{items}/{milk}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(
        &fx,
        Who::Child,
        Method::PUT,
        &format!("{items}/{milk}"),
        Some(json!({"text": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // A device adds without a PIN; only an elevated device deletes.
    let r = as_(
        &fx,
        Who::Device,
        Method::POST,
        &items,
        Some(json!({"text": "eggs"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let eggs = id(&r.json());
    let r = as_(
        &fx,
        Who::Device,
        Method::DELETE,
        &format!("{items}/{eggs}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(
        &fx,
        Who::Elevated,
        Method::DELETE,
        &format!("{items}/{eggs}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    // The replay of a delete is a 404, not an error that matters.
    let r = as_(
        &fx,
        Who::Elevated,
        Method::DELETE,
        &format!("{items}/{eggs}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    // A client-generated id is idempotent (D-118): 200 the second time.
    let cid = Uuid::new_v4().to_string();
    for want in [StatusCode::CREATED, StatusCode::OK] {
        let r = as_(
            &fx,
            Who::Device,
            Method::POST,
            &items,
            Some(json!({"id": cid, "text": "bread"})),
        )
        .await;
        assert_eq!(r.status, want, "{}", r.text());
    }

    // Recipes and the plan: adults write, a child and a device read.
    let recipe = mk_recipe(&fx, "Plain Soup", &["2 cups water"]).await;
    let rid = id(&recipe);
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        &format!("{API}/recipes"),
        Some(recipe_body("Nope", &[])),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    for who in [Who::Child, Who::Device] {
        let r = as_(&fx, who, Method::GET, &format!("{API}/recipes/{rid}"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{who:?}");
        let r = as_(
            &fx,
            who,
            Method::GET,
            &format!("{API}/meal-plan?from={}&to={}", day(0), day(6)),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{who:?}");
        let r = as_(
            &fx,
            who,
            Method::POST,
            &format!("{API}/meal-plan"),
            Some(json!({"date": day(1), "slot_id": Uuid::new_v4(), "free_text": "x"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{who:?}");
    }
    // A child rates their own; a device cannot rate.
    let rating = format!("{API}/recipes/{rid}/rating");
    let r = as_(
        &fx,
        Who::Child,
        Method::PUT,
        &rating,
        Some(json!({"stars": 4})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["mine"], 4);
    assert_eq!(r.json()["by_member"][0]["member_id"], json!(fx.child));
    let r = as_(
        &fx,
        Who::Device,
        Method::PUT,
        &rating,
        Some(json!({"stars": 5})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(&fx, Who::Anon, Method::GET, &format!("{API}/recipes"), None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn if_match_428_and_409(pool: PgPool) {
    let fx = seed(pool).await;
    let r = mk_recipe(&fx, "Plain Soup", &["2 cups water"]).await;
    let rid = id(&r);
    let v = r["version"].as_i64().unwrap();
    let uri = format!("{API}/recipes/{rid}");
    let body = recipe_body("Plain Soup 2", &["2 cups water"]);
    let r = as_(&fx, Who::Adult, Method::PUT, &uri, Some(body.clone())).await;
    assert_eq!(r.status, StatusCode::PRECONDITION_REQUIRED, "{}", r.text());

    let send_v = |ver: String| {
        let h = [("cookie", fx.adult_cookie.clone()), ("if-match", ver)];
        let body = body.clone();
        let uri = uri.clone();
        let app = fx.app.clone();
        async move {
            let h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (*a, b.as_str())).collect();
            send(&app, Method::PUT, &uri, &h, Some(body)).await
        }
    };
    let r = send_v((v + 40).to_string()).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.text());
    let r = send_v(format!("\"{v}\"")).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["title"], "Plain Soup 2");
    assert_eq!(r.header("etag"), format!("\"{}\"", v + 1));
    // Stale now.
    let r = send_v(v.to_string()).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
}

const JSONLD_PAGE: &str = r#"<html><head><script type="application/ld+json">
{"@context":"https://schema.org","@type":"Recipe","name":"Stub Soup","recipeIngredient":["2 cups water","1 tsp salt"],
 "recipeInstructions":[{"@type":"HowToStep","text":"Boil the water."}],"prepTime":"PT5M"}
</script></head><body><h1>Stub Soup</h1></body></html>"#;

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_a_local_page_and_the_alias(pool: PgPool) {
    let fx = seed(pool).await;
    let app = Router::new().route(
        "/soup.html",
        axum::routing::get(|| async { ([("content-type", "text/html")], JSONLD_PAGE) }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/soup.html", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });

    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/recipes/import"),
        Some(json!({"url": url})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], "Stub Soup");
    assert_eq!(r.json()["origin"], "jsonld");
    // Nothing is stored until Save.
    let list = as_(
        &fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/recipes"),
        None,
    )
    .await;
    assert_eq!(list.json()["items"].as_array().unwrap().len(), 0);
    // The alias runs the same code.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/ai/recipes/extract"),
        Some(json!({"url": url})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], "Stub Soup");
    // Errors: both fields, none, no endpoint for text, a child.
    let imp = format!("{API}/recipes/import");
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &imp,
        Some(json!({"url": url, "text": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &imp,
        Some(json!({"text": "2 eggs, beat them"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.json()["code"], "llm_unavailable");
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &imp,
        Some(json!({"url": "http://127.0.0.1:1/none.html"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json()["code"], "fetch_failed");
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        &imp,
        Some(json!({"url": url})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn two_entries_in_one_slot(pool: PgPool) {
    let fx = seed(pool).await;
    let slot = dinner(&fx).await;
    let recipe = mk_recipe(&fx, "Plain Soup", &["2 cups water"]).await;
    plan_add(&fx, &day(1), &slot, json!({"recipe_id": id(&recipe)})).await;
    plan_add(&fx, &day(1), &slot, json!({"free_text": "Leftovers"})).await;
    let list = plan_range(&fx, &day(0), &day(3)).await;
    assert_eq!(list.len(), 2, "{list:?}");
    let ords: Vec<i64> = list.iter().map(|e| e["ord"].as_i64().unwrap()).collect();
    assert!(ords[0] < ords[1], "{ords:?}");
    assert_eq!(list[0]["title"], "Plain Soup");
    assert_eq!(list[1]["title"], "Leftovers");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn template_preview_apply_from_plan_and_duplicate(pool: PgPool) {
    let fx = seed(pool).await;
    let slot = dinner(&fx).await;
    let recipe = mk_recipe(&fx, "Plain Soup", &["2 cups water"]).await;
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-templates"),
        Some(json!({"name": "Two days", "days": 2, "cells": [
            {"day": 1, "slot_id": slot, "ord": 0, "recipe_id": id(&recipe)},
            {"day": 2, "slot_id": slot, "ord": 0, "free_text": "Pizza night"}]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let tid = id(&r.json());
    assert!(!r.header("etag").is_empty());

    // A free-text entry already sits in day 1's dinner.
    plan_add(&fx, &day(1), &slot, json!({"free_text": "Takeaway"})).await;
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-templates/{tid}/preview"),
        Some(json!({"start": day(1)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let targets = r.json()["targets"].as_array().unwrap().clone();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0]["date"], day(1));
    assert_eq!(targets[0]["existing"].as_array().unwrap().len(), 1);
    assert_eq!(targets[1]["existing"].as_array().unwrap().len(), 0);

    // `add_slots` keeps the existing entry; the item is appended.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-templates/{tid}/apply"),
        Some(json!({"start": day(1), "add_slots": [{"date": day(1), "slot_id": slot}]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let change = sid(&r.json(), "change_id");
    let list = plan_range(&fx, &day(1), &day(2)).await;
    let titles: Vec<&str> = list.iter().map(|e| e["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["Takeaway", "Plain Soup", "Pizza night"]);
    // Undo restores the plan.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meals/changes/{change}/undo"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(plan_range(&fx, &day(1), &day(2)).await.len(), 1);

    // From the plan, then duplicate.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-templates/from-plan"),
        Some(json!({"name": "Saved", "from": day(1), "to": day(2), "slot_ids": [slot]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let saved = id(&r.json());
    assert_eq!(r.json()["cells"].as_array().unwrap().len(), 1);
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meal-templates/{saved}/duplicate"),
        Some(json!({"name": "Saved copy"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_ne!(id(&r.json()), saved);
    let r = as_(
        &fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/meal-templates"),
        None,
    )
    .await;
    assert_eq!(r.json().as_array().unwrap().len(), 3);
    // Templates are for adults.
    let r = as_(
        &fx,
        Who::Child,
        Method::GET,
        &format!("{API}/meal-templates"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn a_stores_aisle_order_round_trips(pool: PgPool) {
    let fx = seed(pool).await;
    let r = as_(
        &fx,
        Who::Adult,
        Method::PUT,
        &format!("{API}/meals/stores"),
        Some(json!([{"name": "Corner Shop", "ord": 0}])),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let store = sid(&r.json()[0], "id");
    let uri = format!("{API}/meals/stores/{store}/aisles");
    let r = as_(&fx, Who::Child, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let mut list = r.json().as_array().unwrap().clone();
    assert!(list.len() > 3, "the seeded aisles are listed");
    list.reverse();
    for (i, a) in list.iter_mut().enumerate() {
        a["ord"] = json!(i);
        a["hidden"] = json!(i == 1);
    }
    let r = as_(&fx, Who::Child, Method::PUT, &uri, Some(json!(list))).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(&fx, Who::Adult, Method::PUT, &uri, Some(json!(list))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let r = as_(&fx, Who::Device, Method::GET, &uri, None).await;
    assert_eq!(r.json(), json!(list));
    // Another store and the household order are untouched.
    let aisles = as_(
        &fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/meals/aisles"),
        None,
    )
    .await;
    assert_ne!(aisles.json()[0]["id"], list[0]["aisle_id"]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn generate_then_undo(pool: PgPool) {
    let fx = seed(pool).await;
    let slot = dinner(&fx).await;
    let recipe = mk_recipe(&fx, "Tomato Pasta", &["2 tomatoes", "200 g spaghetti"]).await;
    let e = plan_add(&fx, &day(1), &slot, json!({"recipe_id": id(&recipe)})).await;
    let entry = id(&e["entries"][0]);
    let gen_uri = format!("{API}/grocery/generate");
    let r = as_(
        &fx,
        Who::Child,
        Method::POST,
        &gen_uri,
        Some(json!({"entry_ids": [entry]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &gen_uri,
        Some(json!({"entry_ids": [entry]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["added"], 2);
    let change = sid(&r.json(), "change_id");
    assert_eq!(grocery(&fx).await["items"].as_array().unwrap().len(), 2);
    // Twice adds nothing.
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &gen_uri,
        Some(json!({"entry_ids": [entry]})),
    )
    .await;
    assert_eq!(r.json()["added"], 0, "{}", r.text());
    assert_eq!(grocery(&fx).await["items"].as_array().unwrap().len(), 2);

    let undo = format!("{API}/meals/changes/{change}/undo");
    let r = as_(&fx, Who::Adult, Method::POST, &undo, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(grocery(&fx).await["items"].as_array().unwrap().len(), 0);
    let r = as_(&fx, Who::Adult, Method::POST, &undo, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.json()["code"], "already_undone");
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/meals/changes/{}/undo", Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn plan_delete_reports_the_impact_and_reconcile_applies_it(pool: PgPool) {
    let fx = seed(pool).await;
    let slot = dinner(&fx).await;
    let recipe = mk_recipe(&fx, "Tomato Pasta", &["2 tomatoes", "200 g spaghetti"]).await;
    let e = plan_add(&fx, &day(1), &slot, json!({"recipe_id": id(&recipe)})).await;
    let entry = id(&e["entries"][0]);
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/grocery/generate"),
        Some(json!({"entry_ids": [entry]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());

    let r = as_(
        &fx,
        Who::Adult,
        Method::DELETE,
        &format!("{API}/meal-plan/{entry}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let impact = r.json()["grocery_impact"].as_array().unwrap().clone();
    assert_eq!(impact.len(), 2, "{impact:?}");
    assert!(
        impact
            .iter()
            .all(|i| i["remove"] == true && i["entry_id"] == json!(entry))
    );
    // The list is untouched until the adult agrees.
    assert_eq!(grocery(&fx).await["items"].as_array().unwrap().len(), 2);
    let r = as_(
        &fx,
        Who::Adult,
        Method::POST,
        &format!("{API}/grocery/reconcile"),
        Some(json!({"entry_ids": [entry]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["changed"], 2);
    assert_eq!(grocery(&fx).await["items"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn the_dashboard_carries_todays_meals(pool: PgPool) {
    let fx = seed(pool).await;
    let slot = dinner(&fx).await;
    plan_add(&fx, &day(0), &slot, json!({"free_text": "Soup night"})).await;
    plan_add(&fx, &day(1), &slot, json!({"free_text": "Tomorrow"})).await;
    let r = as_(
        &fx,
        Who::Device,
        Method::GET,
        &format!("{API}/dashboard"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let meals = r.json()["meals"].as_array().unwrap().clone();
    assert_eq!(meals.len(), 1, "{meals:?}");
    assert_eq!(meals[0]["slot_name"], "Dinner");
    assert_eq!(meals[0]["time"], "18:00");
    assert_eq!(meals[0]["title"], "Soup night");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn settings_horizon_is_written_to_the_household(pool: PgPool) {
    let fx = seed(pool).await;
    let uri = format!("{API}/meals/settings");
    let r = as_(&fx, Who::Child, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["meal_plan_horizon_weeks"], 2);
    let r = as_(
        &fx,
        Who::Child,
        Method::PUT,
        &uri,
        Some(json!({"meal_plan_horizon_weeks": 4})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = as_(
        &fx,
        Who::Adult,
        Method::PUT,
        &uri,
        Some(json!({"meal_plan_horizon_weeks": 99})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json()["field"], "meal_plan_horizon_weeks");
    let r = as_(
        &fx,
        Who::Adult,
        Method::PUT,
        &uri,
        Some(json!({"meal_plan_horizon_weeks": 4})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["meal_plan_horizon_weeks"], 4);
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let h = HouseholdRepo::get(&mut tx, fx.hid).await.unwrap().unwrap();
    assert_eq!(h.value.meal_plan_horizon_weeks, 4);
    // A slot in use cannot be removed.
    let slot = dinner(&fx).await;
    plan_add(&fx, &day(1), &slot, json!({"free_text": "x"})).await;
    let slots = as_(
        &fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/meals/slots"),
        None,
    )
    .await
    .json();
    let kept: Vec<Value> = slots
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["name"] != "Dinner")
        .cloned()
        .collect();
    let r = as_(
        &fx,
        Who::Adult,
        Method::PUT,
        &format!("{API}/meals/slots"),
        Some(json!(kept)),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.text());
    assert_eq!(r.json()["code"], "slot_in_use");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn image_upload_round_trip_with_a_304(pool: PgPool) {
    let fx = seed(pool).await;
    let recipe = mk_recipe(&fx, "Plain Soup", &["2 cups water"]).await;
    let rid = id(&recipe);
    let uri = format!("{API}/recipes/{rid}/image");
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([x as u8 * 3, y as u8 * 4, 90]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let png = png.into_inner();

    let r = as_(&fx, Who::Adult, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let post = |who: Who, ct: &'static str, body: Vec<u8>| {
        let h = auth_of(&fx, who);
        let app = fx.app.clone();
        let uri = uri.clone();
        async move {
            let mut h: Vec<(&str, &str)> =
                h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            h.push(("content-type", ct));
            send_raw(&app, Method::POST, &uri, &h, body).await
        }
    };
    assert_eq!(
        post(Who::Child, "image/png", png.clone()).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(Who::Adult, "text/plain", png.clone()).await.status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        post(Who::Adult, "image/png", b"not an image".to_vec())
            .await
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    let r = post(Who::Adult, "image/png", png).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());

    let r = as_(&fx, Who::Device, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.header("content-type").starts_with("image/"));
    assert_eq!(r.header("cache-control"), "private, max-age=86400");
    assert_eq!(r.header("x-content-type-options"), "nosniff");
    assert!(!r.body.is_empty());
    let tag = r.header("etag");
    assert!(tag.starts_with('"') && tag.len() > 10, "{tag}");
    let h = auth_of(&fx, Who::Device);
    let mut h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    h.push(("if-none-match", &tag));
    let r = send(&fx.app, Method::GET, &uri, &h, None).await;
    assert_eq!(r.status, StatusCode::NOT_MODIFIED);
    assert!(r.body.is_empty());
    // The summary and the page know about it.
    let r = as_(
        &fx,
        Who::Adult,
        Method::GET,
        &format!("{API}/recipes/{rid}"),
        None,
    )
    .await;
    assert_eq!(r.json()["has_image"], true);

    let r = as_(&fx, Who::Adult, Method::DELETE, &uri, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = as_(&fx, Who::Adult, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn setup_seeds_the_catalogue(pool: PgPool) {
    let store = Store::from_pool(pool);
    let key_path = std::env::temp_dir().join(format!("fa-m5-{}", Uuid::new_v4()));
    std::fs::write(&key_path, [7u8; 32]).unwrap();
    let state = AppState::new(
        store.clone(),
        AuthService::new(store.clone(), AuthConfig::default()),
        services(&store),
        ApiConfig {
            master_key_path: key_path.clone(),
            request_timeout: Duration::from_secs(120),
            ..ApiConfig::default()
        },
        CancellationToken::new(),
    );
    let app = router(state);
    let r = send(
        &app,
        Method::POST,
        "/api/v1/setup/household",
        &[],
        Some(
            json!({"name": "Example", "tz": "UTC", "location": null, "units": "metric",
                    "currency": "USD", "base_url": "http://fa.example.com"}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let hid: HouseholdId = serde_json::from_value(r.json()["household_id"].clone()).unwrap();
    let mut tx = store.tx_for(hid).await.unwrap();
    let ingredients: i64 = sqlx::query_scalar("SELECT count(*) FROM ingredient")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let slots: i64 = sqlx::query_scalar("SELECT count(*) FROM meal_slot WHERE visible")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let lists: i64 = sqlx::query_scalar("SELECT count(*) FROM grocery_list")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(ingredients > 200, "{ingredients}");
    assert_eq!((slots, lists), (1, 1));
    let _ = std::fs::remove_file(key_path);
}
