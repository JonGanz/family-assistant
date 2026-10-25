//! M2 routes: the authz matrix for tasks, routines and notification preferences over HTTP.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use chrono::{DateTime, Duration as Dur, Utc};
use fa_api::{ApiConfig, AppState, Services, router, run_bridge_signalled};
use fa_auth::{AuthConfig, AuthService, DeviceService, hash_password};
use fa_domain::*;
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

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
        services(&store),
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
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    Resp { status, body }
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

// ---- helpers ----

const J: &str = "application/json";

fn id(v: &Value) -> String {
    match &v["id"] {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

async fn set_override(fx: &Fx, on: bool) {
    let mut tx = fx.store.tx_system().await.unwrap();
    let h = HouseholdRepo::get(&mut tx, fx.hid).await.unwrap().unwrap();
    let mut v = h.value;
    v.dashboard_completion_override = on;
    HouseholdRepo::update(&mut tx, &v, h.version).await.unwrap();
    tx.commit().await.unwrap();
}

async fn add_child(fx: &Fx) -> MemberId {
    let m = member(fx.hid, "Pat", Role::Child);
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    tx.commit().await.unwrap();
    m.id
}

async fn mk_list(
    fx: &Fx,
    name: &str,
    policy: &str,
    child_can_add: &str,
    dashboard_add: bool,
) -> String {
    let r = as_(
        fx,
        Who::Adult,
        Method::POST,
        "/api/v1/task-lists",
        Some(json!({
            "name": name, "kind": "chores", "completion_policy": policy,
            "child_can_add": child_can_add, "dashboard_add": dashboard_add,
            "calendar_default": "hidden", "claim_ttl_minutes": null,
            "visible_on_dashboard": true, "color": null, "sort": 0
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    id(&r.json())
}

async fn mk_task(fx: &Fx, list: &str, title: &str, assignees: &[MemberId], extra: Value) -> Value {
    let mut body = json!({"list": list, "title": title, "assignees": assignees});
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        body[k] = v;
    }
    let r = as_(fx, Who::Adult, Method::POST, "/api/v1/tasks", Some(body)).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    r.json()
}

async fn status_of(fx: &Fx, who: Who, m: Method, uri: &str, body: Option<Value>) -> u16 {
    as_(fx, who, m, uri, body).await.status.as_u16()
}

fn complete(t: &Value) -> String {
    format!("/api/v1/tasks/{}/complete", id(t))
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn complete_add_edit_skip_authz(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let assignee = mk_list(&fx, "Assignee", "assignee", "own", true).await;
    let adult_only = mk_list(&fx, "Adult only", "adult_only", "none", false).await;
    let t_child = mk_task(&fx, &assignee, "Tidy", &[fx.child], json!({})).await;
    let t_adult = mk_task(&fx, &assignee, "Taxes", &[fx.adult], json!({})).await;
    let t_locked = mk_task(&fx, &adult_only, "Boiler", &[fx.child], json!({})).await;
    let post = Method::POST;

    // complete: assignee policy, adult-only lists, devices and the household override
    assert_eq!(
        status_of(&fx, Anon, post.clone(), &complete(&t_child), None).await,
        401
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &complete(&t_adult), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &complete(&t_locked), None).await,
        403
    );
    assert_eq!(
        status_of(
            &fx,
            Device,
            post.clone(),
            &complete(&t_child),
            Some(json!({"member": fx.child}))
        )
        .await,
        403
    );
    let r = as_(
        &fx,
        Child,
        post.clone(),
        &complete(&t_child),
        Some(json!({})),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["status"], "done");
    assert_eq!(r.json()["completed_by"], json!(fx.child));
    // D-013: the completion is audited with who did it
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let audit = fa_store::AuditRepo::list(&mut tx, fx.hid, 20)
        .await
        .unwrap();
    assert!(audit.iter().any(|e| e.action == "task.complete"
        && e.subject_id.as_deref() == Some(id(&t_child).as_str())
        && e.actor_member == Some(fx.child)));
    tx.rollback().await.unwrap();
    // a child may undo their own completion
    let reopen = format!("/api/v1/tasks/{}/reopen", id(&t_child));
    assert_eq!(
        status_of(&fx, Child, post.clone(), &reopen, None).await,
        200
    );
    assert_eq!(
        status_of(&fx, Adult, post.clone(), &complete(&t_locked), None).await,
        200
    );
    assert_eq!(
        status_of(&fx, Adult, post.clone(), &complete(&t_locked), None).await,
        409
    );

    // devices with the override on must say who; the member must be in the household
    set_override(&fx, true).await;
    assert_eq!(
        status_of(&fx, Device, post.clone(), &complete(&t_child), None).await,
        422
    );
    let r = as_(
        &fx,
        Device,
        post.clone(),
        &complete(&t_child),
        Some(json!({"member": MemberId::new()})),
    )
    .await;
    assert_eq!(r.status, 422);
    assert_eq!(r.json()["field"], "member");
    let r = as_(
        &fx,
        Device,
        post.clone(),
        &complete(&t_child),
        Some(json!({"member": fx.child})),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["completed_by"], json!(fx.child));
    // the override does not open adult-only lists
    let t_locked2 = mk_task(&fx, &adult_only, "Fuse box", &[], json!({})).await;
    assert_eq!(
        status_of(
            &fx,
            Device,
            post.clone(),
            &complete(&t_locked2),
            Some(json!({"member": fx.child}))
        )
        .await,
        403
    );
    // an elevated device is an adult: no member needed
    assert_eq!(
        status_of(&fx, Elevated, post.clone(), &complete(&t_locked2), None).await,
        200
    );
    set_override(&fx, false).await;

    // add
    let new = |list: &str, extra: Value| {
        let mut b = json!({"list": list, "title": "New"});
        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
            b[k] = v;
        }
        Some(b)
    };
    let tasks = "/api/v1/tasks";
    assert_eq!(
        status_of(&fx, Anon, post.clone(), tasks, new(&assignee, json!({}))).await,
        401
    );
    let r = as_(&fx, Child, post.clone(), tasks, new(&assignee, json!({}))).await;
    assert_eq!(r.status, 201, "{}", r.text());
    let own = r.json();
    assert_eq!(own["assignees"], json!([fx.child]));
    assert_eq!(
        status_of(
            &fx,
            Child,
            post.clone(),
            tasks,
            new(&assignee, json!({"assignees": [fx.adult]}))
        )
        .await,
        403
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), tasks, new(&adult_only, json!({}))).await,
        403
    );
    assert_eq!(
        status_of(
            &fx,
            Device,
            post.clone(),
            tasks,
            new(&assignee, json!({"member": fx.child}))
        )
        .await,
        201
    );
    assert_eq!(
        status_of(&fx, Device, post.clone(), tasks, new(&assignee, json!({}))).await,
        422
    );
    assert_eq!(
        status_of(
            &fx,
            Device,
            post.clone(),
            tasks,
            new(&adult_only, json!({"member": fx.child}))
        )
        .await,
        403
    );
    assert_eq!(
        status_of(
            &fx,
            Elevated,
            post.clone(),
            tasks,
            new(&adult_only, json!({}))
        )
        .await,
        201
    );

    // edit: a child edits only what they created; devices never
    let edit = |t: &Value| format!("/api/v1/tasks/{}", id(t));
    let hdr = |t: &Value| t["version"].to_string();
    for (who, task, want) in [
        (Child, &own, 200),
        (Child, &t_adult, 403),
        (Device, &own, 403),
        (Anon, &own, 401),
        (Adult, &t_adult, 200),
    ] {
        let h = auth_of(&fx, who);
        let mut hdrs: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let v = hdr(task);
        hdrs.push(("if-match", &v));
        let r = send(
            &fx.app,
            Method::PATCH,
            &edit(task),
            &hdrs,
            Some(json!({"title": "Renamed"})),
        )
        .await;
        assert_eq!(r.status.as_u16(), want, "{who:?}: {}", r.text());
    }
    let stale = as_(
        &fx,
        Adult,
        Method::PATCH,
        &edit(&t_adult),
        Some(json!({"title": "x"})),
    )
    .await;
    assert_eq!(stale.status, 428);
    assert_eq!(
        status_of(&fx, Child, Method::DELETE, &edit(&t_adult), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Adult, Method::DELETE, &edit(&t_adult), None).await,
        204
    );
    assert_eq!(
        status_of(&fx, Adult, Method::GET, &edit(&t_adult), None).await,
        404
    );

    // skip and accept are adult-user only
    let skip = |t: &Value| format!("/api/v1/tasks/{}/skip", id(t));
    for who in [Child, Device, Elevated, Anon] {
        let want = if matches!(who, Anon) { 401 } else { 403 };
        assert_eq!(
            status_of(&fx, who, post.clone(), &skip(&own), None).await,
            want,
            "{who:?}"
        );
        let accept = format!("/api/v1/tasks/{}/accept", id(&own));
        assert_eq!(
            status_of(&fx, who, post.clone(), &accept, None).await,
            want,
            "{who:?}"
        );
    }
    let r = as_(&fx, Adult, post.clone(), &skip(&own), None).await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["status"], "skipped");
    let accept = format!("/api/v1/tasks/{}/accept", id(&own));
    assert_eq!(status_of(&fx, Adult, post, &accept, None).await, 409);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn claim_and_step_ticks(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let list = mk_list(&fx, "Chores", "assignee", "own", false).await;
    let t_child = mk_task(
        &fx,
        &list,
        "Tidy",
        &[fx.child],
        json!({"steps": ["Toys", "Books"]}),
    )
    .await;
    let t_adult = mk_task(&fx, &list, "Taxes", &[fx.adult], json!({"steps": ["File"]})).await;
    let post = Method::POST;
    let claim = |t: &Value| format!("/api/v1/tasks/{}/claim", id(t));
    let release = |t: &Value| format!("/api/v1/tasks/{}/release", id(t));

    // claim: authorized for assignees and adults; devices never. These tasks are not pool tasks,
    // so a passing policy shows up as the service's 422.
    assert_eq!(
        status_of(&fx, Anon, post.clone(), &claim(&t_child), None).await,
        401
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &claim(&t_adult), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Device, post.clone(), &claim(&t_child), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Elevated, post.clone(), &claim(&t_child), None).await,
        422
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &claim(&t_child), None).await,
        422
    );
    assert_eq!(
        status_of(&fx, Adult, post.clone(), &claim(&t_adult), None).await,
        422
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &release(&t_adult), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Device, post.clone(), &release(&t_child), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Child, post.clone(), &release(&t_child), None).await,
        200
    );

    // step ticks
    let step =
        |t: &Value, i: usize| format!("/api/v1/tasks/{}/steps/{}", id(t), id(&t["steps"][i]));
    let tick = |done: bool| Some(json!({"done": done}));
    assert_eq!(
        status_of(&fx, Anon, Method::PATCH, &step(&t_child, 0), tick(true)).await,
        401
    );
    assert_eq!(
        status_of(&fx, Child, Method::PATCH, &step(&t_adult, 0), tick(true)).await,
        403
    );
    let r = as_(&fx, Child, Method::PATCH, &step(&t_child, 0), tick(true)).await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert!(r.json()["steps"][0]["done_at"].is_string());
    assert_eq!(
        r.json()["status"],
        "open",
        "ticking every step does not complete the task"
    );
    let r = as_(&fx, Child, Method::PATCH, &step(&t_child, 0), tick(false)).await;
    assert!(r.json()["steps"][0]["done_at"].is_null());
    // devices: override off = 403; on = a member is required
    let dev = |m: Option<MemberId>| Some(json!({"done": true, "member": m}));
    assert_eq!(
        status_of(
            &fx,
            Device,
            Method::PATCH,
            &step(&t_child, 1),
            dev(Some(fx.child))
        )
        .await,
        403
    );
    set_override(&fx, true).await;
    assert_eq!(
        status_of(&fx, Device, Method::PATCH, &step(&t_child, 1), dev(None)).await,
        422
    );
    let r = as_(
        &fx,
        Device,
        Method::PATCH,
        &step(&t_child, 1),
        dev(Some(fx.child)),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["steps"][1]["done_by"], json!(fx.child));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn children_only_see_their_own_tasks_and_routines(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let child2 = add_child(&fx).await;
    let list = mk_list(&fx, "Chores", "assignee", "own", false).await;
    let a = mk_task(&fx, &list, "Mine", &[fx.child], json!({})).await;
    let b = mk_task(&fx, &list, "Adult", &[fx.adult], json!({})).await;
    let c = mk_task(&fx, &list, "Anyone", &[], json!({})).await;
    let d = mk_task(&fx, &list, "Other child", &[child2], json!({})).await;
    let titles = |r: &Resp| {
        let mut t: Vec<String> = r
            .json()
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap().to_owned())
            .collect();
        t.sort();
        t
    };
    for uri in [
        "/api/v1/tasks",
        &format!("/api/v1/tasks?member={child2}"),
        "/api/v1/tasks?view=mine&status=any",
        "/api/v1/tasks?view=now",
    ] {
        let r = as_(&fx, Child, Method::GET, uri, None).await;
        assert_eq!(r.status, 200, "{uri}");
        assert!(
            !titles(&r).contains(&"Other child".to_owned()),
            "{uri}: {:?}",
            titles(&r)
        );
        assert!(
            !titles(&r).contains(&"Adult".to_owned()),
            "{uri}: {:?}",
            titles(&r)
        );
    }
    let r = as_(&fx, Child, Method::GET, "/api/v1/tasks", None).await;
    assert_eq!(titles(&r), vec!["Anyone", "Mine"]);
    let r = as_(&fx, Adult, Method::GET, "/api/v1/tasks", None).await;
    assert_eq!(titles(&r).len(), 4);
    let r = as_(
        &fx,
        Adult,
        Method::GET,
        &format!("/api/v1/tasks?view=mine&member={}", fx.adult),
        None,
    )
    .await;
    assert_eq!(titles(&r), vec!["Adult"]);
    for (t, want) in [(&a, 200), (&b, 404), (&c, 200), (&d, 404)] {
        assert_eq!(
            status_of(
                &fx,
                Child,
                Method::GET,
                &format!("/api/v1/tasks/{}", id(t)),
                None
            )
            .await,
            want
        );
    }
    assert_eq!(
        status_of(&fx, Anon, Method::GET, "/api/v1/tasks", None).await,
        401
    );
    assert_eq!(
        status_of(&fx, Device, Method::GET, "/api/v1/tasks", None).await,
        200
    );

    // routines: each child's run, ticking only their own
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/routines",
        Some(json!({
            "name": "Morning", "members": [fx.child, child2],
            "steps": [{"text": "Teeth"}, {"text": "Shoes", "target_time": "07:30"}],
            "starts_at": "00:00", "done_by": "23:59", "day_filter": "daily"
        })),
    )
    .await;
    assert_eq!(r.status, 201, "{}", r.text());
    fa_tasks::TaskService::new(fx.store.clone())
        .routines_materialize(fx.hid, Utc::now())
        .await
        .unwrap();
    let all = as_(&fx, Adult, Method::GET, "/api/v1/routines/today", None)
        .await
        .json();
    assert_eq!(all.as_array().unwrap().len(), 2, "{all}");
    let mine = as_(&fx, Child, Method::GET, "/api/v1/routines/today", None)
        .await
        .json();
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["member"], json!(fx.child));
    let other = as_(
        &fx,
        Child,
        Method::GET,
        &format!("/api/v1/routines/today?member={child2}"),
        None,
    )
    .await
    .json();
    assert_eq!(other.as_array().unwrap().len(), 1);
    assert_eq!(
        other[0]["member"],
        json!(fx.child),
        "a child cannot ask for another child's runs"
    );
    let theirs = all
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["member"] == json!(child2))
        .unwrap()
        .clone();
    let run_step = |run: &Value| {
        format!(
            "/api/v1/routine-runs/{}/steps/{}",
            id(run),
            run["steps"][0]["step"].as_str().unwrap()
        )
    };
    let tick = Some(json!({"done": true}));
    assert_eq!(
        status_of(&fx, Child, Method::PATCH, &run_step(&theirs), tick.clone()).await,
        403
    );
    let r = as_(&fx, Child, Method::PATCH, &run_step(&mine[0]), tick.clone()).await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["done"], 1);
    assert_eq!(
        status_of(&fx, Adult, Method::PATCH, &run_step(&theirs), tick.clone()).await,
        200
    );
    assert_eq!(
        status_of(&fx, Anon, Method::PATCH, &run_step(&theirs), tick).await,
        401
    );
    // devices: override off = 403; on = a member is required and must exist
    let dev = |m: Option<MemberId>| Some(json!({"done": false, "member": m}));
    assert_eq!(
        status_of(
            &fx,
            Device,
            Method::PATCH,
            &run_step(&mine[0]),
            dev(Some(fx.child))
        )
        .await,
        403
    );
    set_override(&fx, true).await;
    assert_eq!(
        status_of(&fx, Device, Method::PATCH, &run_step(&mine[0]), dev(None)).await,
        422
    );
    assert_eq!(
        status_of(
            &fx,
            Device,
            Method::PATCH,
            &run_step(&mine[0]),
            dev(Some(MemberId::new()))
        )
        .await,
        422
    );
    assert_eq!(
        status_of(
            &fx,
            Device,
            Method::PATCH,
            &run_step(&mine[0]),
            dev(Some(fx.child))
        )
        .await,
        200
    );
    // routine management is adult-only
    for who in [Child, Device, Elevated] {
        assert_eq!(
            status_of(
                &fx,
                who,
                Method::POST,
                "/api/v1/routines",
                Some(json!({"name": "x", "starts_at": "06:00", "done_by": "07:00"}))
            )
            .await,
            403,
            "{who:?}"
        );
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn now_view_and_calendar_range(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let list = mk_list(&fx, "Chores", "assignee", "own", false).await;
    let now = Utc::now();
    let tomorrow = (now + Dur::days(1)).to_rfc3339();
    mk_task(
        &fx,
        &list,
        "Soon",
        &[],
        json!({"due_at": (now + Dur::hours(2)).to_rfc3339()}),
    )
    .await;
    mk_task(&fx, &list, "Flagged", &[], json!({"priority_flag": "high"})).await;
    mk_task(
        &fx,
        &list,
        "Cal always",
        &[],
        json!({"due_at": tomorrow, "calendar_display": "always"}),
    )
    .await;
    mk_task(
        &fx,
        &list,
        "Cal until done",
        &[],
        json!({"due_at": tomorrow, "calendar_display": "until_done"}),
    )
    .await;
    mk_task(&fx, &list, "Hidden", &[], json!({"due_at": tomorrow})).await;

    let r = as_(&fx, Adult, Method::GET, "/api/v1/tasks?view=now", None).await;
    assert_eq!(r.status, 200);
    assert_eq!(r.json().as_array().unwrap().len(), 5);
    // the household sets the length of the Now list
    for bad in [
        json!({"now_list_size": 0}),
        json!({"now_list_size": 51}),
        json!({"now_weights": {"due": 11.0, "flag": 1.0, "weather": 1.0}}),
        json!({"now_weights": {"due": -1.0, "flag": 1.0, "weather": 1.0}}),
    ] {
        let v = as_(&fx, Adult, Method::GET, "/api/v1/household", None)
            .await
            .json()["version"]
            .to_string();
        let h = auth_of(&fx, Adult);
        let hdrs = [(h[0].0.as_str(), h[0].1.as_str()), ("if-match", v.as_str())];
        let r = send(
            &fx.app,
            Method::PATCH,
            "/api/v1/household",
            &hdrs,
            Some(bad.clone()),
        )
        .await;
        assert_eq!(r.status, 422, "{bad}");
    }
    let v = as_(&fx, Adult, Method::GET, "/api/v1/household", None)
        .await
        .json()["version"]
        .to_string();
    let h = auth_of(&fx, Adult);
    let hdrs = [(h[0].0.as_str(), h[0].1.as_str()), ("if-match", v.as_str())];
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &hdrs,
        Some(json!({"now_list_size": 2, "now_weights": {"due": 2.0, "flag": 1.0, "weather": 0.5}})),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["now_list_size"], 2);
    for who in [Adult, Device] {
        let r = as_(&fx, who, Method::GET, "/api/v1/tasks?view=now", None).await;
        assert_eq!(r.json().as_array().unwrap().len(), 2, "{who:?}");
    }

    // calendar range: hidden tasks stay out
    let q = range_query(now, now + Dur::days(3));
    let r = as_(&fx, Adult, Method::GET, &format!("/api/v1/tasks?{q}"), None).await;
    assert_eq!(r.status, 200, "{}", r.text());
    let body = r.json();
    let mut titles: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap())
        .collect();
    titles.sort_unstable();
    assert_eq!(titles, vec!["Cal always", "Cal until done"]);
    // a done `until_done` task leaves the calendar, a done `always` task stays
    for t in body.as_array().unwrap() {
        let done = as_(&fx, Adult, Method::POST, &complete(t), None).await;
        assert_eq!(done.status, 200);
    }
    let r = as_(&fx, Adult, Method::GET, &format!("/api/v1/tasks?{q}"), None).await;
    let body = r.json();
    let titles: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, vec!["Cal always"]);
    let _ = Child;
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn definitions_validate_rrule_and_condition(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let list = mk_list(&fx, "Chores", "assignee", "own", false).await;
    let def = |extra: Value| {
        let mut b = json!({
            "list": list, "title": "Bins",
            "schedule": {"kind": "rrule", "rrule": "FREQ=WEEKLY;BYDAY=SA"},
            "start_date": "2026-01-03", "strategy": "fixed", "candidates": [fx.adult]
        });
        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
            b[k] = v;
        }
        Some(b)
    };
    let url = "/api/v1/task-definitions";
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        url,
        def(json!({"schedule": {"kind": "rrule", "rrule": "FREQ=SOMETIMES"}})),
    )
    .await;
    assert_eq!(r.status, 422, "{}", r.text());
    assert_eq!(r.json()["field"], "rrule");
    assert!(r.json()["message"].is_string());
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        url,
        def(json!({"season_condition": "temperature >>> 3"})),
    )
    .await;
    assert_eq!(r.status, 422, "{}", r.text());
    assert_eq!(r.json()["field"], "season_condition");
    assert!(r.json()["position"].is_number());
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        url,
        def(json!({"season_condition": "min(temp_c, tonight) < 2"})),
    )
    .await;
    assert_eq!(r.status, 201, "{}", r.text());
    let created = r.json();
    assert_eq!(created["season_condition"], "min(temp_c, tonight) < 2");
    let one = format!("{url}/{}", id(&created));
    let hdrs = |who: Who| auth_of(&fx, who);
    let h = hdrs(Adult);
    let v = created["version"].to_string();
    let hh = [(h[0].0.as_str(), h[0].1.as_str()), ("if-match", v.as_str())];
    let r = send(
        &fx.app,
        Method::PATCH,
        &one,
        &hh,
        Some(json!({"schedule": {"kind": "rrule", "rrule": "nonsense"}})),
    )
    .await;
    assert_eq!(r.status, 422);
    let r = send(
        &fx.app,
        Method::PATCH,
        &one,
        &hh,
        Some(json!({"season_condition": "((("})),
    )
    .await;
    assert_eq!(r.status, 422);
    assert_eq!(r.json()["field"], "season_condition");
    let r = send(
        &fx.app,
        Method::PATCH,
        &one,
        &hh,
        Some(json!({"title": "Bins out"})),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());

    // /validate
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/task-definitions/validate",
        Some(json!({"rrule": "FREQ=DAILY", "season_condition": "min(temp_c, tonight) < 2"})),
    )
    .await;
    assert_eq!(r.status, 200);
    assert_eq!(r.json()["ok"], true);
    assert_eq!(r.json()["next_dates"].as_array().unwrap().len(), 5);
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        "/api/v1/task-definitions/validate",
        Some(json!({"rrule": "FREQ=NEVER", "season_condition": "a >>> b"})),
    )
    .await;
    let v = r.json();
    assert_eq!(v["ok"], false);
    assert_eq!(v["next_dates"], json!([]));
    let fields: Vec<&str> = v["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, vec!["rrule", "season_condition"]);
    assert!(v["errors"][1]["position"].is_number());

    // only adults manage definitions, lists and absences
    let list_body = json!({
        "name": "x", "kind": "chores", "completion_policy": "assignee", "child_can_add": "none",
        "dashboard_add": false, "calendar_default": "hidden", "claim_ttl_minutes": null,
        "visible_on_dashboard": true, "color": null, "sort": 0
    });
    for who in [Child, Device, Elevated] {
        for (m, uri) in [
            (Method::GET, url.to_string()),
            (Method::POST, "/api/v1/task-definitions/validate".into()),
            (Method::POST, "/api/v1/task-lists".into()),
            (Method::GET, format!("/api/v1/members/{}/away", fx.child)),
        ] {
            assert_eq!(
                status_of(&fx, who, m, &uri, Some(list_body.clone())).await,
                403,
                "{who:?} {uri}"
            );
        }
    }
    assert_eq!(status_of(&fx, Anon, Method::GET, url, None).await, 401);

    // a task rule must name an existing definition; an alert must not
    let rule = |action: &str, def: Value| {
        Some(json!({
            "name": format!("rule {action}"), "condition": "min(temp_c, tonight) < 2",
            "action": action, "severity": "info", "message": "m", "task_definition": def
        }))
    };
    let rules = "/api/v1/weather-rules";
    assert_eq!(
        status_of(&fx, Adult, Method::POST, rules, rule("task", Value::Null)).await,
        422
    );
    assert_eq!(
        status_of(
            &fx,
            Adult,
            Method::POST,
            rules,
            rule("task", json!({"definition": TaskDefinitionId::new()}))
        )
        .await,
        422
    );
    assert_eq!(
        status_of(
            &fx,
            Adult,
            Method::POST,
            rules,
            rule("alert", json!({"definition": id(&created)}))
        )
        .await,
        422
    );
    let r = as_(
        &fx,
        Adult,
        Method::POST,
        rules,
        rule("task", json!({"definition": id(&created)})),
    )
    .await;
    assert_eq!(r.status, 201, "{}", r.text());
    assert_eq!(r.json()["task_definition"]["definition"], id(&created));
    assert_eq!(r.json()["task_mode"], "create");
    assert_eq!(
        status_of(&fx, Adult, Method::POST, rules, rule("alert", Value::Null)).await,
        201
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn dashboard_carries_tasks_and_routines(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let child2 = add_child(&fx).await;
    let list = mk_list(&fx, "Chores", "assignee", "own", false).await;
    mk_task(&fx, &list, "Open", &[fx.child], json!({})).await;
    mk_task(&fx, &list, "Adults only", &[fx.adult], json!({})).await;
    let r = as_(&fx, Adult, Method::POST, "/api/v1/routines", Some(json!({
        "name": "Morning", "members": [fx.child, child2],
        "steps": [{"text": "Teeth"}], "starts_at": "00:00", "done_by": "23:59", "day_filter": "daily"
    }))).await;
    assert_eq!(r.status, 201, "{}", r.text());
    fa_tasks::TaskService::new(fx.store.clone())
        .routines_materialize(fx.hid, Utc::now())
        .await
        .unwrap();

    for who in [Device, Elevated] {
        let r = as_(&fx, who, Method::GET, "/api/v1/dashboard", None).await;
        assert_eq!(r.status, 200, "{}", r.text());
        assert_eq!(r.json()["tasks"].as_array().unwrap().len(), 2, "{who:?}");
        assert_eq!(r.json()["routines"].as_array().unwrap().len(), 2);
    }
    // the device's `members` setting narrows both
    let dev = as_(
        &fx,
        Adult,
        Method::GET,
        &format!("/api/v1/devices/{}", fx.device),
        None,
    )
    .await
    .json();
    let mut settings = dev["settings"].clone();
    settings["members"] = json!([fx.child]);
    let h = auth_of(&fx, Adult);
    let v = dev["version"].to_string();
    let hh = [(h[0].0.as_str(), h[0].1.as_str()), ("if-match", v.as_str())];
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/devices/{}", fx.device),
        &hh,
        Some(json!({"settings": settings})),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    let r = as_(&fx, Device, Method::GET, "/api/v1/dashboard", None)
        .await
        .json();
    let titles: Vec<&str> = r["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, vec!["Open"]);
    let runs = r["routines"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["member"], json!(fx.child));
    let _ = (Child, J);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn notify_prefs_authz_and_test_endpoint(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let prefs = |m: MemberId| format!("/api/v1/members/{m}/notify-prefs");
    // (who, member, expected)
    for (who, m, want) in [
        (Anon, fx.child, 401),
        (Adult, fx.child, 200),
        (Adult, fx.adult, 200),
        (Adult, fx.adult2, 403),
        (Child, fx.child, 200),
        (Child, fx.adult, 403),
        (Device, fx.child, 403),
        (Elevated, fx.child, 403),
    ] {
        assert_eq!(
            status_of(&fx, who, Method::GET, &prefs(m), None).await,
            want,
            "{who:?} {m}"
        );
    }
    let r = as_(&fx, Child, Method::GET, &prefs(fx.child), None).await;
    let own = r.json();
    assert!(own["topic"].is_string());
    assert!(
        own["subscribe_url"].is_null(),
        "no ntfy server, no subscribe URL"
    );
    assert_eq!(own["server_configured"], false);
    assert_eq!(
        status_of(&fx, Adult, Method::GET, &prefs(MemberId::new()), None).await,
        404
    );

    // PUT needs the version; a child sets their own quiet hours
    assert_eq!(
        status_of(
            &fx,
            Child,
            Method::PUT,
            &prefs(fx.child),
            Some(json!({"due_soon_minutes": 30}))
        )
        .await,
        428
    );
    let put = |who: Who, m: MemberId, v: &str, body: Value| {
        let fx = &fx;
        let v = v.to_owned();
        async move {
            let h = auth_of(fx, who);
            let hh = [
                (
                    h.first().map(|x| x.0.as_str()).unwrap_or("x-none"),
                    h.first().map(|x| x.1.as_str()).unwrap_or(""),
                ),
                ("if-match", v.as_str()),
            ];
            send(&fx.app, Method::PUT, &prefs(m), &hh, Some(body)).await
        }
    };
    let ver = own["version"].to_string();
    let r = put(
        Child,
        fx.child,
        &ver,
        json!({"due_soon_minutes": 30, "quiet_from": "20:30", "quiet_until": "06:30"}),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.text());
    assert_eq!(r.json()["due_soon_minutes"], 30);
    assert_eq!(r.json()["quiet_from"], "20:30");
    assert_eq!(
        put(Child, fx.child, &ver, json!({"due_soon_minutes": 45}))
            .await
            .status,
        409
    );
    assert_eq!(put(Child, fx.adult, &ver, json!({})).await.status, 403);
    assert_eq!(put(Device, fx.child, &ver, json!({})).await.status, 403);
    assert_eq!(
        put(
            Adult,
            fx.child,
            &r.json()["version"].to_string(),
            json!({"due_soon_minutes": 0})
        )
        .await
        .status,
        422
    );

    // test: 503 without ntfy, for the member and their adult; the rest is forbidden
    let test = |m: MemberId| format!("{}/test", prefs(m));
    assert_eq!(
        status_of(&fx, Child, Method::POST, &test(fx.child), None).await,
        503
    );
    assert_eq!(
        status_of(&fx, Adult, Method::POST, &test(fx.child), None).await,
        503
    );
    assert_eq!(
        status_of(&fx, Adult, Method::POST, &test(fx.adult2), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Device, Method::POST, &test(fx.child), None).await,
        403
    );
    assert_eq!(
        status_of(&fx, Anon, Method::POST, &test(fx.child), None).await,
        401
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn household_patch_toggles_dashboard_override(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let list = mk_list(&fx, "Assignee", "assignee", "own", true).await;
    let t = mk_task(&fx, &list, "Tidy", &[fx.child], json!({})).await;
    let body = json!({"member": fx.child});

    let patch = |who: Who, on: bool, version: i64| {
        let fx = &fx;
        async move {
            let h = auth_of(fx, who);
            let mut hdrs: Vec<(&str, &str)> =
                h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            let v = version.to_string();
            hdrs.push(("if-match", &v));
            send(
                &fx.app,
                Method::PATCH,
                "/api/v1/household",
                &hdrs,
                Some(json!({"dashboard_completion_override": on})),
            )
            .await
        }
    };

    let view = as_(&fx, Adult, Method::GET, "/api/v1/household", None).await;
    assert_eq!(view.json()["dashboard_completion_override"], false);

    // Only adults may change it.
    let r = patch(Child, true, view.json()["version"].as_i64().unwrap()).await;
    assert_eq!(r.status.as_u16(), 403);

    let r = patch(Adult, true, view.json()["version"].as_i64().unwrap()).await;
    assert_eq!(r.status.as_u16(), 200, "{}", r.text());
    assert_eq!(r.json()["dashboard_completion_override"], true);
    let v = r.json()["version"].as_i64().unwrap();
    let ok = status_of(&fx, Device, Method::POST, &complete(&t), Some(body.clone())).await;
    assert_eq!(ok, 200);

    let r = patch(Adult, false, v).await;
    assert_eq!(r.status.as_u16(), 200, "{}", r.text());
    assert_eq!(r.json()["dashboard_completion_override"], false);
    let t2 = mk_task(&fx, &list, "Sweep", &[fx.child], json!({})).await;
    let denied = status_of(&fx, Device, Method::POST, &complete(&t2), Some(body)).await;
    assert_eq!(denied, 403);
}
