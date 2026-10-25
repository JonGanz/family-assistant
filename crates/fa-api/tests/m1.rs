//! M1 routes: RBAC matrix, event masking, avatars, OAuth redirects, dashboard, weather rules,
//! calendar sources and the NOTIFY to SSE bridge.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use chrono::{DateTime, Duration as Dur, Utc};
use fa_api::{ApiConfig, AppState, Services, router, run_bridge_signalled};
use fa_auth::{AuthConfig, AuthService, DeviceService, hash_password};
use fa_connectors::google::oauth::GoogleOAuth;
use fa_domain::*;
use fa_store::*;
use futures::StreamExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PW: &str = "adult password 123";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n0000000000000000";

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
    adult2_cookie: String,
    child_cookie: String,
    /// A device token with an active adult PIN elevation.
    elevated_token: String,
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
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn key() -> MasterKey {
    MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
}

fn services(store: &Store, google: bool, allow_http: bool) -> Services {
    let oauth = fa_auth::OAuthService::new(
        store.clone(),
        key(),
        google.then(|| GoogleOAuth::new("client-id", "client-secret")),
        "https://fa.example.com",
        fa_connectors::Http::new(allow_http).unwrap(),
    );
    Services {
        calendar: Arc::new(
            fa_calendar::CalendarService::new(
                store.clone(),
                key(),
                fa_calendar::ConnectorConfig {
                    allow_http,
                    ..Default::default()
                },
                Arc::new(oauth.clone()),
            )
            .unwrap(),
        ),
        weather: fa_weather::WeatherService::new(
            store.clone(),
            fa_weather::WeatherConfig::default(),
        )
        .unwrap(),
        oauth,
        tasks: Arc::new(fa_tasks::TaskService::new(store.clone())),
        notify: fa_notify::NotifyService::new(
            store.clone(),
            fa_notify::NotifyConfig {
                ntfy_url: None,
                ntfy_token: None,
                public_base_url: "https://fa.example.com".into(),
            },
        ),
    }
}

async fn seed(pool: PgPool) -> Fx {
    seed_with(pool, true, false).await
}

async fn seed_with(pool: PgPool, google: bool, allow_http: bool) -> Fx {
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
            id: hid,
            name: "Example".into(),
            tz: "UTC".into(),
            location: None,
            meal_plan_horizon_weeks: 2,
            dashboard_completion_override: false,
            units: Units::Metric,
            calendar_past_days: 30,
            calendar_future_days: 400,
            weather_retention_days: 7,
            private_events_as_busy: true,
            now_list_size: 7,
            now_weights: fa_domain::tasks::NowWeights::default(),
            quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).expect("time"),
            quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
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

    let devices = DeviceService::new(store.clone(), "http://localhost");
    let plain = devices.pair_system(hid, "Kitchen").await.unwrap();
    let elevated = devices.pair_system(hid, "Hall").await.unwrap();

    let auth = AuthService::new(store.clone(), AuthConfig::default());
    let state = AppState::new(
        store.clone(),
        auth,
        services(&store, google, allow_http),
        ApiConfig {
            cookie_secure: false,
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
        adult2_cookie: "fa_session=test-session-jo".into(),
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

fn range_query(from: DateTime<Utc>, to: DateTime<Utc>) -> String {
    let f = |t: DateTime<Utc>| t.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    format!("from={}&to={}", f(from), f(to))
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rbac_matrix(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let now = Utc::now();
    let events = format!("/api/v1/events?{}", range_query(now, now + Dur::days(2)));
    let device = format!("/api/v1/devices/{}", fx.device);
    let dismiss = format!("/api/v1/advisories/{}/dismiss", AdvisoryId::new());
    let ver = as_(&fx, Adult, Method::GET, "/api/v1/household", None)
        .await
        .json()["version"]
        .to_string();
    let patch_household = json!({"name": "Home"});
    // (method, path, body, expected for Anon, Adult, Child, Device, Elevated)
    type Row<'a> = (Method, &'a str, Option<Value>, [u16; 5]);
    let rows: Vec<Row> = vec![
        (
            Method::GET,
            "/api/v1/household",
            None,
            [401, 200, 200, 200, 200],
        ),
        (
            Method::GET,
            "/api/v1/calendar-sources",
            None,
            [401, 200, 403, 403, 403],
        ),
        (
            Method::GET,
            "/api/v1/calendars",
            None,
            [401, 200, 403, 403, 403],
        ),
        (
            Method::GET,
            "/api/v1/oauth/accounts",
            None,
            [401, 200, 403, 403, 403],
        ),
        (
            Method::GET,
            "/api/v1/weather-rules",
            None,
            [401, 200, 403, 403, 403],
        ),
        (
            Method::POST,
            "/api/v1/weather-rules/validate",
            Some(json!({"condition": "min(temp_c, tonight) < 2"})),
            [401, 200, 403, 403, 403],
        ),
        (Method::GET, &events, None, [401, 200, 200, 200, 200]),
        (
            Method::GET,
            "/api/v1/weather",
            None,
            [401, 404, 404, 404, 404],
        ),
        (
            Method::GET,
            "/api/v1/advisories",
            None,
            [401, 200, 200, 200, 200],
        ),
        (Method::POST, &dismiss, None, [401, 404, 404, 404, 404]),
        (
            Method::GET,
            "/api/v1/status",
            None,
            [401, 200, 403, 403, 403],
        ),
        (Method::GET, &device, None, [401, 200, 403, 403, 403]),
        (
            Method::GET,
            "/api/v1/dashboard",
            None,
            [401, 403, 403, 200, 200],
        ),
        (
            Method::POST,
            "/api/v1/calendar-sources/test",
            Some(json!({"kind": "ics", "name": "x", "url": "https://example.com/a.ics"})),
            [401, 200, 403, 403, 403],
        ),
        (
            Method::POST,
            "/api/v1/calendar-sources/00000000-0000-0000-0000-000000000000/sync",
            None,
            [401, 404, 403, 403, 403],
        ),
        (
            Method::GET,
            "/api/v1/oauth/google/start",
            None,
            [401, 302, 403, 403, 403],
        ),
    ];
    for (m, uri, body, expect) in &rows {
        for (who, want) in [Anon, Adult, Child, Device, Elevated]
            .into_iter()
            .zip(expect)
        {
            let r = as_(&fx, who, m.clone(), uri, body.clone()).await;
            assert_eq!(
                r.status.as_u16(),
                *want,
                "{m} {uri} as {who:?}: {}",
                r.text()
            );
        }
    }
    // Mutations of household settings: adults only (denial comes before the version check).
    for who in [Anon, Child, Device, Elevated] {
        let r = as_(
            &fx,
            who,
            Method::PATCH,
            "/api/v1/household",
            Some(patch_household.clone()),
        )
        .await;
        assert_eq!(
            r.status.as_u16(),
            if matches!(who, Anon) { 401 } else { 403 },
            "{who:?}"
        );
    }
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie), ("if-match", &ver)],
        Some(patch_household),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], "Home");
    assert_eq!(r.json()["google_oauth_enabled"], true);
    // Stale version is a conflict and a missing one is 428.
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie), ("if-match", &ver)],
        Some(json!({"name": "Again"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie)],
        Some(json!({"name": "Again"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::PRECONDITION_REQUIRED);
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie), ("if-match", "99")],
        Some(json!({"tz": "Mars/Base"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}

struct Cals {
    adult_private: &'static str,
    child_private: &'static str,
    house_private: &'static str,
}

const TITLES: Cals = Cals {
    adult_private: "Adult dentist",
    child_private: "Child secret",
    house_private: "Household therapist",
};

async fn add_calendar(
    fx: &Fx,
    name: &str,
    owner: Option<MemberId>,
    members: &[MemberId],
    events: &[(&str, bool)],
) {
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let src = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: fx.hid,
        kind: SourceKind::Ics,
        name: name.into(),
        owner_member: owner,
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: false,
        poll_minutes: 60,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let src = CalendarSourceRepo::insert(&mut tx, &src).await.unwrap();
    let (cal, _) = CalendarRepo::upsert_discovered(&mut tx, src.value.id, "cal", name, None)
        .await
        .unwrap();
    CalendarRepo::update(
        &mut tx,
        cal.value.id,
        &fa_domain::api::CalendarPatch {
            enabled: Some(true),
            ..Default::default()
        },
        cal.version,
    )
    .await
    .unwrap();
    CalendarRepo::set_members(&mut tx, cal.value.id, members)
        .await
        .unwrap();
    let start = Utc::now() + Dur::hours(2);
    let occ: Vec<NewOccurrence> = events
        .iter()
        .enumerate()
        .map(|(i, (title, private))| NewOccurrence {
            remote_uid: format!("{name}-{i}"),
            instance_key: String::new(),
            title: (*title).into(),
            description: private.then(|| format!("{title} notes")),
            location: private.then(|| format!("{title} place")),
            start_at: start,
            end_at: start + Dur::hours(1),
            all_day: false,
            start_date: None,
            end_date: None,
            source_tz: None,
            is_private: *private,
            cancelled: false,
            remote_etag: None,
            external_url: None,
        })
        .collect();
    EventRepo::upsert_synced(&mut tx, cal.value.id, &occ)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn seed_events(fx: &Fx) {
    add_calendar(
        fx,
        "Alex",
        Some(fx.adult),
        &[fx.adult],
        &[(TITLES.adult_private, true), ("Adult standup", false)],
    )
    .await;
    add_calendar(
        fx,
        "Sam",
        Some(fx.child),
        &[fx.child],
        &[(TITLES.child_private, true)],
    )
    .await;
    add_calendar(
        fx,
        "Home",
        None,
        &[],
        &[(TITLES.house_private, true), ("Bin day", false)],
    )
    .await;
}

fn titles(r: &Resp) -> Vec<String> {
    let mut t: Vec<String> = r
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["title"].as_str().unwrap().to_owned())
        .collect();
    t.sort();
    t
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn events_are_scoped_and_masked(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    seed_events(&fx).await;
    let now = Utc::now();
    let uri = format!("/api/v1/events?{}", range_query(now, now + Dur::days(2)));
    let leaks = |r: &Resp, who: Who| {
        // A busy event reveals nothing but its time block and members.
        let body = r.text();
        for secret in [TITLES.adult_private, TITLES.house_private] {
            assert!(!body.contains(secret), "{who:?} sees '{secret}': {body}");
        }
    };

    // Flag on (the default).
    let r = as_(&fx, Adult, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(titles(&r).len(), 5);
    assert!(titles(&r).contains(&TITLES.adult_private.to_owned()));
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["visibility"] == "full")
    );

    let r = as_(&fx, Child, Method::GET, &uri, None).await;
    // Own calendar in full, the household's public event, and the household's private one as busy.
    assert_eq!(titles(&r), ["Bin day", "Busy", TITLES.child_private]);
    assert!(!r.text().contains("Adult standup"));
    let busy = r
        .json()
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["title"] == "Busy")
        .unwrap()
        .clone();
    assert_eq!(busy["visibility"], "busy");
    assert!(busy["location"].is_null() && busy["description"].is_null());
    assert!(!r.text().contains(TITLES.house_private));

    for who in [Device, Elevated] {
        let r = as_(&fx, who, Method::GET, &uri, None).await;
        assert_eq!(
            titles(&r),
            [
                "Adult standup",
                "Bin day",
                "Busy",
                "Busy",
                TITLES.child_private
            ],
            "{who:?}"
        );
        leaks(&r, who);
    }

    // Member filter.
    let r = as_(
        &fx,
        Adult,
        Method::GET,
        &format!("{uri}&member={}", fx.child),
        None,
    )
    .await;
    assert_eq!(titles(&r), [TITLES.child_private]);

    // Flag off: everything is shown in full to children and devices.
    let ver = as_(&fx, Adult, Method::GET, "/api/v1/household", None)
        .await
        .json()["version"]
        .to_string();
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie), ("if-match", &ver)],
        Some(json!({"private_events_as_busy": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = as_(&fx, Device, Method::GET, &uri, None).await;
    assert_eq!(titles(&r).len(), 5);
    assert!(titles(&r).contains(&TITLES.adult_private.to_owned()));
    let r = as_(&fx, Child, Method::GET, &uri, None).await;
    assert_eq!(
        titles(&r),
        ["Bin day", TITLES.child_private, TITLES.house_private]
    );

    // Bad ranges.
    let r = as_(
        &fx,
        Adult,
        Method::GET,
        &format!("/api/v1/events?{}", range_query(now, now + Dur::days(500))),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = as_(&fx, Adult, Method::GET, "/api/v1/events", None).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn avatars_limits_types_and_caching(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let uri = format!("/api/v1/members/{}/avatar", fx.child);
    let put = |ct: &'static str, body: Vec<u8>, cookie: String| {
        let app = fx.app.clone();
        let uri = uri.clone();
        async move {
            send_raw(
                &app,
                Method::PUT,
                &uri,
                &[("cookie", &cookie), ("content-type", ct)],
                body,
            )
            .await
        }
    };

    // Too big: 413 before anything is stored.
    let mut big = PNG.to_vec();
    big.resize(512 * 1024 + 1, 0);
    assert_eq!(
        put("image/png", big, fx.adult_cookie.clone()).await.status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // The limit itself passes.
    let mut max = PNG.to_vec();
    max.resize(512 * 1024, 0);
    assert_eq!(
        put("image/png", max, fx.adult_cookie.clone()).await.status,
        StatusCode::NO_CONTENT
    );
    // Wrong declared type, or right type with other bytes: 415.
    assert_eq!(
        put("text/plain", PNG.to_vec(), fx.adult_cookie.clone())
            .await
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        put(
            "image/png",
            b"<svg onload=alert(1)/>".to_vec(),
            fx.adult_cookie.clone()
        )
        .await
        .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        put("image/svg+xml", b"<svg/>".to_vec(), fx.adult_cookie.clone())
            .await
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    // A child cannot change an adult's avatar; an adult cannot change another adult's.
    let adult_uri = format!("/api/v1/members/{}/avatar", fx.adult);
    for cookie in [&fx.child_cookie, &fx.adult2_cookie] {
        let r = send_raw(
            &fx.app,
            Method::PUT,
            &adult_uri,
            &[("cookie", cookie), ("content-type", "image/png")],
            PNG.to_vec(),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN);
    }
    // Devices cannot upload.
    let r = send_raw(
        &fx.app,
        Method::PUT,
        &uri,
        &[
            ("authorization", &format!("Bearer {}", fx.device_token)),
            ("content-type", "image/png"),
        ],
        PNG.to_vec(),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // A JPEG declared as PNG is stored as what it is.
    let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3, 4].to_vec();
    assert_eq!(
        put("image/png", jpeg.clone(), fx.adult_cookie.clone())
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let r = as_(&fx, Device, Method::GET, &uri, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers["content-type"], "image/jpeg");
    assert_eq!(r.body, jpeg);
    assert_eq!(r.headers["cache-control"], "private, max-age=86400");
    let etag = r.headers["etag"].to_str().unwrap().to_owned();
    assert_eq!(etag.len(), 64 + 2);
    let r = send(
        &fx.app,
        Method::GET,
        &uri,
        &[
            ("authorization", &format!("Bearer {}", fx.device_token)),
            ("if-none-match", &etag),
        ],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_MODIFIED);

    // The members list carries the cache-busting URL, only for members with an avatar.
    let list = as_(&fx, Child, Method::GET, "/api/v1/members", None)
        .await
        .json();
    let sam = list
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == fx.child.to_string())
        .unwrap();
    let url = sam["avatar_url"].as_str().unwrap();
    assert!(
        url.starts_with(&format!("/api/v1/members/{}/avatar?v=", fx.child)),
        "{url}"
    );
    assert_eq!(url.split("?v=").nth(1).unwrap(), &etag[1..13]);
    let alex = list
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == fx.adult.to_string())
        .unwrap();
    assert!(alex["avatar_url"].is_null());

    // Delete.
    let r = as_(&fx, Adult, Method::DELETE, &uri, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        as_(&fx, Adult, Method::GET, &uri, None).await.status,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn oauth_redirects_stay_on_site(pool: PgPool) {
    let fx = seed(pool).await;
    let start = |q: &str| format!("/api/v1/oauth/google/start{q}");
    let get = |uri: String, cookie: String| {
        let app = fx.app.clone();
        async move { send(&app, Method::GET, &uri, &[("cookie", &cookie)], None).await }
    };

    for bad in [
        "https://evil.example.com/steal",
        "//evil.example.com",
        "/\\evil",
        "evil.example.com",
        "javascript:alert(1)",
    ] {
        let r = get(
            start(&format!("?return_to={}", urlenc(bad))),
            fx.adult_cookie.clone(),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
    }
    let r = get(
        start("?return_to=/settings/calendars"),
        fx.adult_cookie.clone(),
    )
    .await;
    assert_eq!(r.status, StatusCode::FOUND);
    let loc = r.headers["location"].to_str().unwrap();
    assert!(loc.starts_with("https://accounts.google.com/"), "{loc}");
    assert!(loc.contains("code_challenge_method=S256") && loc.contains("access_type=offline"));
    assert!(loc.contains(
        "redirect_uri=https%3A%2F%2Ffa.example.com%2Fapi%2Fv1%2Foauth%2Fgoogle%2Fcallback"
    ));

    // The callback only ever redirects to /oauth/done with an encoded error or account.
    let cb = |q: &str| format!("/api/v1/oauth/google/callback{q}");
    let r = get(cb("?error=access_denied"), fx.adult_cookie.clone()).await;
    assert_eq!(r.status, StatusCode::FOUND);
    assert_eq!(r.headers["location"], "/oauth/done?error=access_denied");
    let r = get(
        cb("?error=https://evil.example.com"),
        fx.adult_cookie.clone(),
    )
    .await;
    // Only lowercase letters and underscores survive from Google's `error` value.
    assert_eq!(
        r.headers["location"],
        "/oauth/done?error=httpsevilexamplecom"
    );
    let r = get(cb("?code=abc&state=forged"), fx.adult_cookie.clone()).await;
    assert_eq!(r.headers["location"], "/oauth/done?error=invalid_state");
    let r = send(&fx.app, Method::GET, &cb("?code=abc&state=x"), &[], None).await;
    assert_eq!(r.status, StatusCode::FOUND);
    assert_eq!(r.headers["location"], "/oauth/done?error=session_required");
    let r = get(cb(""), fx.adult_cookie.clone()).await;
    assert_eq!(r.headers["location"], "/oauth/done?error=invalid_request");
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn oauth_without_google_is_404(pool: PgPool) {
    let fx = seed_with(pool, false, false).await;
    let r = as_(
        &fx,
        Who::Adult,
        Method::GET,
        "/api/v1/oauth/google/start",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let h = as_(&fx, Who::Adult, Method::GET, "/api/v1/household", None).await;
    assert_eq!(h.json()["google_oauth_enabled"], false);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn dashboard_shape_and_device_settings(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    seed_events(&fx).await;
    let r = as_(&fx, Device, Method::GET, "/api/v1/dashboard", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let j = r.json();
    for k in [
        "household",
        "members",
        "events",
        "weather",
        "advisories",
        "settings",
    ] {
        assert!(j.get(k).is_some(), "missing {k}: {j}");
    }
    assert_eq!(j["household"]["name"], "Example");
    assert_eq!(j["household"]["tz"], "UTC");
    assert_eq!(j["household"]["units"], "metric");
    assert!(j["weather"].is_null());
    assert_eq!(j["advisories"], json!([]));
    assert_eq!(j["members"].as_array().unwrap().len(), 3);
    assert!(j["members"][0].get("avatar_url").is_some());
    assert!(
        j["members"][0].get("birthdate").is_none(),
        "public fields only"
    );
    assert_eq!(j["settings"]["members"], "all");
    assert_eq!(j["settings"]["agenda_days"], 3);
    // Already masked for the device.
    assert!(!r.text().contains(TITLES.adult_private));
    assert!(r.text().contains("Adult standup"));

    for days in ["0", "15"] {
        let r = as_(
            &fx,
            Device,
            Method::GET,
            &format!("/api/v1/dashboard?days={days}"),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "days={days}");
    }

    // Adults edit the device settings; the dashboard follows.
    let uri = format!("/api/v1/devices/{}", fx.device);
    let dev = as_(&fx, Adult, Method::GET, &uri, None).await;
    assert_eq!(dev.json()["settings"]["theme"], "auto");
    let v = dev.json()["version"].to_string();
    let settings = json!({"orientation": "portrait", "theme": "dark", "agenda_days": 5,
        "members": [fx.child], "panels": [{"kind": "clock"}, {"kind": "hologram", "depth": 3}]});
    let bad = json!({"settings": {"orientation": "portrait", "theme": "dark", "agenda_days": 20, "members": "all", "panels": []}});
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(bad),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &fx.child_cookie), ("if-match", &v)],
        Some(json!({"settings": settings})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(json!({"name": "Fridge", "settings": settings})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], "Fridge");
    let r = as_(&fx, Device, Method::GET, "/api/v1/dashboard", None).await;
    assert_eq!(r.json()["settings"]["agenda_days"], 5);
    assert_eq!(r.json()["settings"]["members"], json!([fx.child]));
    assert_eq!(r.json()["settings"]["panels"][1]["depth"], 3);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn weather_rules_validate_and_422(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let bad = json!({"name": "Bad", "condition": "temp_c <<< 3", "action": "alert",
        "severity": "info", "message": "x"});
    let r = as_(&fx, Adult, Method::POST, "/api/v1/weather-rules", Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.headers["content-type"], "application/problem+json");
    let j = r.json();
    assert!(j["position"].is_u64() && j["message"].is_string(), "{j}");

    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/weather-rules/validate",
        Some(json!({"condition": "temp_c <<< 3"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["ok"], false);
    assert!(r.json()["error"]["position"].is_u64());

    let good = json!({"name": "Frost", "condition": "min(temp_c, tonight) < 2", "action": "alert",
        "severity": "warning", "message": "Frost tonight"});
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/weather-rules",
        Some(good),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let id = r.json()["id"].as_str().unwrap().to_owned();
    assert_eq!(r.json()["suggested_by"], "user");
    let v = r.json()["version"].to_string();

    let uri = format!("/api/v1/weather-rules/{id}");
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(json!({"condition": "nonsense !!"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(json!({"status": "disabled"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "disabled");
    assert_eq!(
        as_(&fx, Adult, Method::DELETE, &uri, None).await.status,
        StatusCode::NO_CONTENT
    );
    let list = as_(&fx, Adult, Method::GET, "/api/v1/weather-rules", None)
        .await
        .json();
    assert!(list.as_array().unwrap().iter().all(|r| r["id"] != id));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn advisories_list_and_dismiss(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let (id, _) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Frost tonight",
        Severity::Warning,
        Utc::now() - Dur::hours(1),
        Utc::now() + Dur::hours(5),
        AdvisorySource::Rule,
        "k1",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let r = as_(&fx, Device, Method::GET, "/api/v1/advisories", None).await;
    assert_eq!(r.json().as_array().unwrap().len(), 1);
    assert_eq!(r.json()[0]["text"], "Frost tonight");
    // Dashboards dismiss without a PIN.
    let r = as_(
        &fx,
        Device,
        Method::POST,
        &format!("/api/v1/advisories/{id}/dismiss"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = as_(&fx, Child, Method::GET, "/api/v1/advisories", None).await;
    assert_eq!(r.json(), json!([]));
}

fn ics(uid: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//example//EN\r\nBEGIN:VEVENT\r\nUID:{uid}@example.com\r\n\
         DTSTAMP:20261001T000000Z\r\nDTSTART:20261101T100000Z\r\nDTEND:20261101T110000Z\r\n\
         SUMMARY:Fixture\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    )
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn calendar_sources_never_leak_secrets(pool: PgPool) {
    use Who::*;
    let feed = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feeds/s3cr3t-token/basic.ics"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ics("a")))
        .mount(&feed)
        .await;
    let fx = seed_with(pool, true, true).await;
    let url = format!("{}/feeds/s3cr3t-token/basic.ics", feed.uri());
    let body = json!({"kind": "ics", "name": "School", "url": url, "owner": null});

    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/calendar-sources/test",
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["ok"], true, "{}", r.text());
    assert!(!r.text().contains("s3cr3t-token"));

    for who in [Child, Device, Elevated] {
        let r = as_(
            &fx,
            who,
            Method::POST,
            "/api/v1/calendar-sources",
            Some(body.clone()),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{who:?}");
    }
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/calendar-sources",
        Some(body),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert!(!r.text().contains("s3cr3t-token"), "{}", r.text());
    let j = r.json();
    assert_eq!(j["source"]["kind"], "ics");
    assert!(j["source"]["masked_url"].as_str().unwrap().ends_with("/…"));
    assert_eq!(j["calendars"].as_array().unwrap().len(), 1);
    let sid = j["source"]["id"].as_str().unwrap().to_owned();
    let cid = j["calendars"][0]["id"].as_str().unwrap().to_owned();

    let list = as_(&fx, Adult, Method::GET, "/api/v1/calendar-sources", None).await;
    assert_eq!(list.json().as_array().unwrap().len(), 1);
    assert!(!list.text().contains("s3cr3t-token"));
    // Another adult may not see or touch this adult's personal sources.
    let mut mine = json!({"kind": "ics", "name": "Mine", "url": url, "owner": fx.adult});
    let r = send(
        &fx.app,
        Method::POST,
        "/api/v1/calendar-sources",
        &[("cookie", &fx.adult2_cookie)],
        Some(mine.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    mine["owner"] = json!(fx.adult2);
    let r = send(
        &fx.app,
        Method::POST,
        "/api/v1/calendar-sources",
        &[("cookie", &fx.adult2_cookie)],
        Some(mine),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let theirs = r.json()["source"]["id"].as_str().unwrap().to_owned();
    let list = send(
        &fx.app,
        Method::GET,
        "/api/v1/calendar-sources",
        &[("cookie", &fx.adult_cookie)],
        None,
    )
    .await;
    assert_eq!(
        list.json().as_array().unwrap().len(),
        1,
        "other adult's source is hidden"
    );
    let r = as_(
        &fx,
        Adult,
        Method::DELETE,
        &format!("/api/v1/calendar-sources/{theirs}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Calendar edit needs the version.
    let cal = as_(&fx, Adult, Method::GET, "/api/v1/calendars", None)
        .await
        .json();
    let v = cal
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == cid)
        .unwrap()["version"]
        .to_string();
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/calendars/{cid}"),
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(json!({"color": "#3366ff", "members": [fx.child]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["members"], json!([fx.child]));
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/calendars/{cid}"),
        &[("cookie", &fx.adult_cookie), ("if-match", &v)],
        Some(json!({"color": "#000000"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    let r = as_(
        &fx,
        Adult,
        Method::POST,
        &format!("/api/v1/calendar-sources/{sid}/sync"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    let r = as_(
        &fx,
        Adult,
        Method::DELETE,
        &format!("/api/v1/calendar-sources/{sid}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let list = as_(&fx, Adult, Method::GET, "/api/v1/calendar-sources", None).await;
    assert_eq!(list.json(), json!([]));
}

async fn read_until(
    stream: &mut (impl futures::Stream<Item = Result<axum::body::Bytes, axum::Error>> + Unpin),
    buf: &mut String,
    needle: &str,
) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !buf.contains(needle) {
            let chunk = stream.next().await.expect("stream ended").unwrap();
            buf.push_str(&String::from_utf8_lossy(&chunk));
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {needle:?}; got {buf:?}"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn notify_from_another_connection_reaches_sse_clients(pool: PgPool) {
    let fx = seed(pool).await;
    let res = fx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/stream")
                .header("authorization", format!("Bearer {}", fx.device_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mut stream = res.into_body().into_data_stream();
    let mut buf = String::new();

    // A separate connection, as the worker process would use.
    let other = fx.store.pool().clone();
    let notify = |payload: String| {
        let other = other.clone();
        async move {
            sqlx::query("SELECT pg_notify($1, $2)")
                .bind(CHANNEL)
                .bind(payload)
                .execute(&other)
                .await
                .unwrap();
        }
    };
    // Garbage is dropped, and so are messages of another household.
    notify("not json".into()).await;
    notify(
        json!({"household": HouseholdId::new(), "kind": "weather.updated", "data": {}}).to_string(),
    )
    .await;
    notify(
        json!({"household": fx.hid, "kind": "calendar.updated", "data": {"from": "a", "to": "b"}})
            .to_string(),
    )
    .await;
    read_until(&mut stream, &mut buf, "event: calendar.updated").await;
    assert!(buf.contains(r#"data: {"from":"a","to":"b"}"#), "{buf}");
    assert!(!buf.contains("weather.updated"), "{buf}");

    // After the listener's connection dies, it reconnects and tells every client to refetch.
    let killed: Vec<bool> = sqlx::query_scalar(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity
         WHERE datname = current_database() AND pid <> pg_backend_pid() AND query ILIKE 'listen%'",
    )
    .fetch_all(&other)
    .await
    .unwrap();
    assert!(!killed.is_empty(), "found no LISTEN connection");
    read_until(&mut stream, &mut buf, "event: resync").await;
    // And it keeps delivering.
    notify(json!({"household": fx.hid, "kind": "weather.updated", "data": {}}).to_string()).await;
    read_until(&mut stream, &mut buf, "event: weather.updated").await;
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn writes_publish_through_the_change_bus(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let res = fx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/stream")
                .header("cookie", &fx.adult_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut stream = res.into_body().into_data_stream();
    let mut buf = String::new();
    let ver = as_(&fx, Adult, Method::GET, "/api/v1/household", None)
        .await
        .json()["version"]
        .to_string();
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &[("cookie", &fx.adult_cookie), ("if-match", &ver)],
        Some(json!({"name": "Renamed"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    read_until(&mut stream, &mut buf, "event: household.updated").await;
    let r = send_raw(
        &fx.app,
        Method::PUT,
        &format!("/api/v1/members/{}/avatar", fx.child),
        &[("cookie", &fx.adult_cookie), ("content-type", "image/png")],
        PNG.to_vec(),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    read_until(&mut stream, &mut buf, "event: member.updated").await;
}
