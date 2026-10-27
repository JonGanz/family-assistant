//! M3 routes: mail source visibility, mail actions, rules, the approval inbox, bills and the
//! calendar write toggles over HTTP.

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
use uuid::Uuid;

const PW: &str = "adult password 123";

struct Fx {
    app: Router,
    store: Store,
    hid: HouseholdId,
    adult: MemberId,
    adult2: MemberId,
    child: MemberId,
    device_token: String,
    adult_cookie: String,
    adult2_cookie: String,
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

fn key() -> MasterKey {
    MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
}

fn services(store: &Store) -> Services {
    let oauth = fa_auth::OAuthService::new(
        store.clone(),
        key(),
        Some(fa_connectors::google::oauth::GoogleOAuth::new(
            "client-id",
            "client-secret",
        )),
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
    mail_defaults::ensure(&mut tx, hid).await.unwrap();
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
    Adult2,
    Child,
    Device,
    Elevated,
}

fn auth_of(fx: &Fx, who: Who) -> Vec<(String, String)> {
    let bearer = |t: &str| vec![("authorization".to_owned(), format!("Bearer {t}"))];
    match who {
        Who::Anon => vec![],
        Who::Adult => vec![("cookie".into(), fx.adult_cookie.clone())],
        Who::Adult2 => vec![("cookie".into(), fx.adult2_cookie.clone())],
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

// ---- fixtures ----

fn id(v: &Value) -> String {
    match &v["id"] {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

async fn get(fx: &Fx, who: Who, uri: &str) -> Resp {
    as_(fx, who, Method::GET, uri, None).await
}

async fn post(fx: &Fx, who: Who, uri: &str, body: Option<Value>) -> Resp {
    as_(fx, who, Method::POST, uri, body).await
}

async fn sql(fx: &Fx, q: &str) {
    sqlx::query(q).execute(fx.store.pool()).await.unwrap();
}

async fn scalar<T>(fx: &Fx, q: &str) -> T
where
    T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
{
    sqlx::query_scalar(q)
        .fetch_one(fx.store.pool())
        .await
        .unwrap()
}

struct Mx {
    /// Owned by `adult`, owners-only, writable.
    owned: MailSourceId,
    /// Owned by `adult`, owners-only, read-only.
    ro: MailSourceId,
    /// Owned by `adult2`, visible to the household.
    house: MailSourceId,
    /// Owned by `adult2`, owners-only.
    hidden: MailSourceId,
    m_owned: MessageId,
    m_ro: MessageId,
    m_house: MessageId,
    m_hidden: MessageId,
}

async fn add_source(
    fx: &Fx,
    name: &str,
    owner: MemberId,
    visibility: MailVisibility,
    read_only: bool,
) -> MailSourceId {
    let now = Utc::now();
    let src = MailSource {
        id: MailSourceId::new(),
        household_id: fx.hid,
        provider: MailProvider::Imap,
        name: name.into(),
        address: format!("{name}@example.com"),
        oauth_account_id: None,
        imap_host: Some("imap.example.com".into()),
        imap_port: Some(993),
        imap_username: Some(format!("{name}@example.com")),
        secret_id: None,
        folders: vec!["INBOX".into()],
        archive_folder: None,
        gmail_ext: false,
        read_only,
        visibility,
        backfill_days: 30,
        query: None,
        poll_minutes: 5,
        history_id: None,
        state: MailSourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    MailSourceRepo::insert(&mut tx, &src, &[owner])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    src.id
}

async fn add_message(fx: &Fx, source: MailSourceId, remote: &str) -> MessageId {
    let m = NewMessage {
        household_id: fx.hid,
        source_id: source,
        remote_id: remote.into(),
        folder: Some("INBOX".into()),
        thread_id: None,
        rfc822_id: Some(format!("<{remote}@example.com>")),
        received_at: Utc::now(),
        from_addr: "teacher@school.example.edu".into(),
        from_name: Some("Ms Rivera".into()),
        to_addrs: vec!["parent@example.com".into()],
        cc_addrs: vec![],
        subject: format!("Subject {remote}"),
        gmail_labels: vec![],
        has_attachments: false,
        body_text: Some("Hello".into()),
        body_html: None,
        snippet: Some("Hello".into()),
        headers: Some(json!({})),
        attachments: Some(json!([])),
    };
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let id = MessageRepo::insert_parsed(&mut tx, &m)
        .await
        .unwrap()
        .expect("new message");
    tx.commit().await.unwrap();
    id
}

async fn seed_mail(fx: &Fx) -> Mx {
    let owned = add_source(fx, "owned", fx.adult, MailVisibility::Owners, false).await;
    let ro = add_source(fx, "readonly", fx.adult, MailVisibility::Owners, true).await;
    let house = add_source(fx, "house", fx.adult2, MailVisibility::Household, false).await;
    let hidden = add_source(fx, "hidden", fx.adult2, MailVisibility::Owners, false).await;
    Mx {
        m_owned: add_message(fx, owned, "o1").await,
        m_ro: add_message(fx, ro, "r1").await,
        m_house: add_message(fx, house, "h1").await,
        m_hidden: add_message(fx, hidden, "x1").await,
        owned,
        ro,
        house,
        hidden,
    }
}

fn ids(v: &Value, key: &str) -> Vec<String> {
    let mut out: Vec<String> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x[key].as_str().unwrap().to_owned())
        .collect();
    out.sort();
    out
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// ---- mail sources and messages: visibility ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn sources_and_messages_follow_visibility(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;

    // Owner: own sources plus the household source; never another adult's private one.
    let r = get(&fx, Adult, "/api/v1/mail-sources").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(
        ids(&r.json(), "id"),
        sorted(vec![
            mx.owned.to_string(),
            mx.ro.to_string(),
            mx.house.to_string()
        ])
    );
    let owned = r
        .json()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == json!(mx.owned))
        .cloned()
        .unwrap();
    assert_eq!(owned["can_manage"], json!(true));
    let house = r
        .json()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == json!(mx.house))
        .cloned()
        .unwrap();
    assert_eq!(house["can_manage"], json!(false));

    // The other adult: the household source and their own private one.
    let r = get(&fx, Adult2, "/api/v1/mail-sources").await;
    assert_eq!(
        ids(&r.json(), "id"),
        sorted(vec![mx.house.to_string(), mx.hidden.to_string()])
    );

    // Children and devices (elevated or not) never see mail.
    for who in [Child, Device, Elevated, Anon] {
        let want = if matches!(who, Anon) { 401 } else { 403 };
        for uri in ["/api/v1/mail-sources", "/api/v1/messages"] {
            assert_eq!(
                get(&fx, who, uri).await.status.as_u16(),
                want,
                "{who:?} {uri}"
            );
        }
        let uri = format!("/api/v1/messages/{}", mx.m_house);
        assert_eq!(get(&fx, who, &uri).await.status.as_u16(), want, "{who:?}");
    }

    // Messages: only those of readable sources.
    let r = get(&fx, Adult, "/api/v1/messages").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let subjects: Vec<String> = sorted(
        r.json()["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_owned())
            .collect(),
    );
    assert_eq!(
        subjects,
        sorted(vec![
            mx.m_owned.to_string(),
            mx.m_ro.to_string(),
            mx.m_house.to_string()
        ])
    );

    // A message of an unreadable source is a 404, not a 403: its existence does not leak.
    let hidden = format!("/api/v1/messages/{}", mx.m_hidden);
    assert_eq!(get(&fx, Adult, &hidden).await.status, StatusCode::NOT_FOUND);
    assert_eq!(get(&fx, Adult2, &hidden).await.status, StatusCode::OK);
    let own = format!("/api/v1/messages/{}", mx.m_owned);
    assert_eq!(get(&fx, Adult2, &own).await.status, StatusCode::NOT_FOUND);
    let house = format!("/api/v1/messages/{}", mx.m_house);
    let r = get(&fx, Adult, &house).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["summary"]["id"], json!(mx.m_house));
    let missing = format!("/api/v1/messages/{}", MessageId::new());
    assert_eq!(
        get(&fx, Adult, &missing).await.status,
        StatusCode::NOT_FOUND
    );

    // Labels, eval samples and tasks are 404 for hidden messages too.
    for (suffix, body) in [
        (
            "labels",
            json!({"decision_point": "email.is_bill", "value": true}),
        ),
        ("eval-sample", json!({"in_sample": true})),
        ("actions", json!({"action": "archive"})),
    ] {
        let uri = format!("/api/v1/messages/{}/{suffix}", mx.m_hidden);
        assert_eq!(
            post(&fx, Adult, &uri, Some(body)).await.status,
            StatusCode::NOT_FOUND,
            "{suffix}"
        );
    }
    let uri = format!("/api/v1/messages/{}/eval-sample", mx.m_owned);
    let r = post(&fx, Adult, &uri, Some(json!({"in_sample": true}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());

    // Managing a source: only owners. A reader of a household source gets 403, a stranger 404.
    let patch = |who, src: MailSourceId| {
        let fx = &fx;
        async move {
            let uri = format!("/api/v1/mail-sources/{src}");
            as_(
                fx,
                who,
                Method::PATCH,
                &uri,
                Some(json!({"name": "Renamed"})),
            )
            .await
        }
    };
    assert_eq!(patch(Adult, mx.house).await.status, StatusCode::FORBIDDEN);
    assert_eq!(patch(Adult, mx.hidden).await.status, StatusCode::NOT_FOUND);
    // The version is required.
    assert_eq!(patch(Adult2, mx.house).await.status.as_u16(), 428);
    let uri = format!("/api/v1/mail-sources/{}", mx.house);
    let r = as_(
        &fx,
        Adult2,
        Method::PATCH,
        &uri,
        Some(json!({"name": "Shared"})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 428);
    let h = auth_of(&fx, Adult2);
    let h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let mut hv = h.clone();
    hv.push(("if-match", "\"1\""));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"name": "Shared"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], json!("Shared"));
    // A stale version is a 409.
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"name": "Again"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // Sync and delete follow the same rule.
    let sync = |src: MailSourceId| format!("/api/v1/mail-sources/{src}/sync");
    assert_eq!(
        post(&fx, Adult, &sync(mx.house), None).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&fx, Adult2, &sync(mx.house), None).await.status,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        post(&fx, Adult, &sync(mx.hidden), None).await.status,
        StatusCode::NOT_FOUND
    );
    let del = |who, src: MailSourceId| {
        let fx = &fx;
        async move {
            as_(
                fx,
                who,
                Method::DELETE,
                &format!("/api/v1/mail-sources/{src}"),
                None,
            )
            .await
        }
    };
    assert_eq!(del(Adult, mx.house).await.status, StatusCode::FORBIDDEN);
    assert_eq!(del(Adult2, mx.house).await.status, StatusCode::NO_CONTENT);
    let r = get(&fx, Adult, "/api/v1/mail-sources").await;
    assert!(!ids(&r.json(), "id").contains(&mx.house.to_string()));
    assert!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM audit_entry WHERE action = 'mail_source.archive'"
        )
        .await
            >= 1
    );
}

// ---- mail actions ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn mail_actions_and_undo_errors(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;

    // A read-only source refuses actions with 409.
    let uri = format!("/api/v1/messages/{}/actions", mx.m_ro);
    let r = post(&fx, Adult, &uri, Some(json!({"action": "archive"}))).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.text());
    // Hidden: 404. Children and devices: 403.
    let uri = format!("/api/v1/messages/{}/actions", mx.m_hidden);
    assert_eq!(
        post(&fx, Adult, &uri, Some(json!({"action": "archive"})))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let uri = format!("/api/v1/messages/{}/actions", mx.m_owned);
    for who in [Child, Device, Elevated] {
        assert_eq!(
            post(&fx, who, &uri, Some(json!({"action": "archive"})))
                .await
                .status,
            StatusCode::FORBIDDEN,
            "{who:?}"
        );
    }
    // A label action needs a label (422).
    let r = post(&fx, Adult, &uri, Some(json!({"action": "label"}))).await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());

    // Undo after 30 days is gone (410); a fresh action of a hidden message is a 404.
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let old = MailActionRepo::insert(
        &mut tx,
        &NewMailAction {
            household_id: fx.hid,
            message_id: mx.m_owned,
            action: MailActionKind::Archive,
            label: None,
            automatic: false,
            done_by: Some(fx.adult),
            error: None,
        },
    )
    .await
    .unwrap();
    let hidden = MailActionRepo::insert(
        &mut tx,
        &NewMailAction {
            household_id: fx.hid,
            message_id: mx.m_hidden,
            action: MailActionKind::Archive,
            label: None,
            automatic: false,
            done_by: Some(fx.adult2),
            error: None,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    sql(
        &fx,
        &format!(
            "UPDATE mail_action SET done_at = now() - interval '31 days' WHERE id = '{}'",
            old.id
        ),
    )
    .await;
    let undo = |a: MailActionId| format!("/api/v1/mail-actions/{a}/undo");
    let r = post(&fx, Adult, &undo(old.id), None).await;
    assert_eq!(r.status, StatusCode::GONE, "{}", r.text());
    assert_eq!(
        post(&fx, Adult, &undo(hidden.id), None).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        post(&fx, Child, &undo(old.id), None).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&fx, Adult, &undo(MailActionId::new()), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

// ---- rules ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rule_validation_is_422_with_a_field(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let ok = json!({"field": "from_domain", "op": "is", "values": ["school.example.edu"]});
    let rule = |conditions: Value, value: Value| {
        json!({"decision_point": "email.category", "name": "School mail",
               "conditions": conditions, "value": value})
    };

    // A header condition without a header name.
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules",
        Some(rule(
            json!([{"field": "header", "op": "is", "values": ["x"]}]),
            json!("school"),
        )),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
    assert_eq!(r.json()["field"], json!("conditions"));
    assert!(r.json()["message"].as_str().unwrap().contains("header"));
    // An invalid regular expression.
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules",
        Some(rule(
            json!([{"field": "subject", "op": "matches", "values": ["("]}]),
            json!("school"),
        )),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422);
    assert_eq!(r.json()["field"], json!("conditions"));
    // No conditions; an unknown category.
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules",
        Some(rule(json!([]), json!("school"))),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422);
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules",
        Some(rule(json!([ok.clone()]), json!("no_such_category"))),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
    assert_eq!(r.json()["field"], json!("value"));
    // The test endpoint validates the same way.
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules/test",
        Some(json!({"rule": rule(json!([]), json!("school"))})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422);

    // A valid rule is created, listed, patched with If-Match and deleted.
    let r = post(
        &fx,
        Adult,
        "/api/v1/mail-rules",
        Some(rule(json!([ok]), json!("school"))),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let rid = id(&r.json());
    assert_eq!(
        get(&fx, Adult, "/api/v1/mail-rules")
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let uri = format!("/api/v1/mail-rules/{rid}");
    let h = auth_of(&fx, Adult);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"enabled": false})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 428);
    hv.push(("if-match", "1"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"enabled": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["enabled"], json!(false));
    // Children and devices cannot touch setup; devices cannot even read it.
    for who in [Child, Device, Elevated] {
        assert_eq!(
            get(&fx, who, "/api/v1/mail-rules").await.status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            post(
                &fx,
                who,
                "/api/v1/orgs",
                Some(json!({"kind": "school", "name": "X"}))
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
    }
    let r = as_(&fx, Adult, Method::DELETE, &uri, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
}

// ---- inbox ----

struct Items {
    event: EventId,
    bill: BillId,
    task: TaskId,
    contact: ContactSuggestionId,
    calendar: CalendarId,
}

async fn writable_calendar(fx: &Fx) -> (CalendarSourceId, CalendarId) {
    let src = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: fx.hid,
        kind: SourceKind::Caldav,
        name: "Family".into(),
        owner_member: None,
        oauth_account: None,
        url: Some("https://cal.example.com/dav".into()),
        username: Some("u".into()),
        secret_id: None,
        writable: true,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    CalendarSourceRepo::insert(&mut tx, &src).await.unwrap();
    let (c, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, "fam", "Family", None)
        .await
        .unwrap();
    sqlx::query("UPDATE calendar SET enabled = true WHERE id = $1")
        .bind(c.value.id.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (src.id, c.value.id)
}

async fn new_event(fx: &Fx, title: &str, message: Option<MessageId>, status: &str) -> EventId {
    let at = (Utc::now() + Dur::days(3)).trunc_subsecs_secs();
    let ev = NewExtractedEvent {
        household_id: fx.hid,
        message_id: message,
        org_id: None,
        title: title.into(),
        description: None,
        location: None,
        start_at: at,
        end_at: at + Dur::hours(1),
        all_day: false,
        start_date: None,
        end_date: None,
        source_tz: None,
        cancelled: false,
        confidence: 0.95,
        source_ref: Some(json!({"kind": "email"})),
        dedupe_key: format!("{title}|{at}"),
        remote_uid: None,
        members: vec![],
    };
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let id = EventRepo::insert_extracted(&mut tx, &ev).await.unwrap();
    tx.commit().await.unwrap();
    sql(
        fx,
        &format!("UPDATE event SET status = '{status}' WHERE id = '{id}'"),
    )
    .await;
    id
}

trait Trunc {
    fn trunc_subsecs_secs(self) -> Self;
}

impl Trunc for DateTime<Utc> {
    fn trunc_subsecs_secs(self) -> Self {
        DateTime::from_timestamp(self.timestamp(), 0).unwrap()
    }
}

async fn new_bill(fx: &Fx, payee: &str, status: BillStatus, message: Option<MessageId>) -> BillId {
    let b = Bill {
        id: BillId::new(),
        household_id: fx.hid,
        message_id: message,
        org_id: None,
        payee: payee.into(),
        amount_minor: 12_345,
        currency: "USD".into(),
        due_on: chrono::NaiveDate::from_ymd_opt(2026, 11, 20),
        account_hint: None,
        statement_period: None,
        status,
        confidence: 0.8,
        task_id: None,
        source_ref: json!({"external_url": "https://mail.example.com/x"}),
        dedupe_key: format!("{payee}|1"),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let v = BillRepo::insert_if_new(&mut tx, &b).await.unwrap().unwrap();
    tx.commit().await.unwrap();
    v.value.id
}

async fn seed_inbox(fx: &Fx, mx: &Mx) -> Items {
    let (_, calendar) = writable_calendar(fx).await;
    let org = post(
        fx,
        Who::Adult,
        "/api/v1/orgs",
        Some(json!({"kind": "school", "name": "Example Elementary", "domains": ["school.example.edu"]})),
    )
    .await;
    assert_eq!(org.status, StatusCode::CREATED, "{}", org.text());
    let org: OrgId = serde_json::from_value(org.json()["id"].clone()).unwrap();
    let contact = ContactSuggestionId::new();
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    ContactSuggestionRepo::insert_if_new(
        &mut tx,
        &ContactSuggestion {
            id: contact,
            household_id: fx.hid,
            org_id: org,
            email: "coach@school.example.edu".into(),
            name: Some("Coach".into()),
            member_id: None,
            message_id: Some(mx.m_owned),
            status: SuggestionStatus::Proposed,
            decided_by: None,
            decided_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let task = fa_tasks::TaskService::new(fx.store.clone())
        .from_mail(fa_tasks::MailTaskRequest {
            household: Some(fx.hid),
            message: mx.m_owned,
            kind: None,
            title: "Sign the form".into(),
            due_at: None,
            assignees: vec![fx.adult],
            mode: fa_tasks::MailTaskMode::Suggest,
            external_url: None,
        })
        .await
        .unwrap()
        .expect("task suggested");
    Items {
        event: new_event(fx, "Concert", Some(mx.m_owned), "proposed").await,
        bill: new_bill(fx, "Power Co", BillStatus::Proposed, Some(mx.m_owned)).await,
        task,
        contact,
        calendar,
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn inbox_lists_items_and_devices_get_counts_only(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;
    let it = seed_inbox(&fx, &mx).await;

    let r = get(&fx, Adult, "/api/v1/inbox").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let v = r.json();
    assert_eq!(
        v["counts"],
        json!({"event": 1, "bill": 1, "task": 1, "contact": 1, "total": 4})
    );
    let kinds = sorted(
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["kind"].as_str().unwrap().to_owned())
            .collect(),
    );
    assert_eq!(kinds, vec!["bill", "contact", "event", "task"]);
    let r = get(&fx, Adult, "/api/v1/inbox?kind=bill").await;
    assert_eq!(r.json()["items"].as_array().unwrap().len(), 1);
    assert_eq!(r.json()["items"][0]["bill"]["id"], json!(it.bill));

    // D-068: the inbox is for signed-in adults; devices get 403, elevated or not.
    for who in [Device, Elevated] {
        assert_eq!(
            get(&fx, who, "/api/v1/inbox").await.status,
            StatusCode::FORBIDDEN,
            "{who:?}"
        );
    }
    assert_eq!(
        get(&fx, Child, "/api/v1/inbox").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&fx, Anon, "/api/v1/inbox").await.status,
        StatusCode::UNAUTHORIZED
    );

    // The message link of an item is shown only to those who can read the message.
    let bill_msg = |v: &Value| v["items"][0]["bill"]["message"].clone();
    assert_eq!(
        bill_msg(&get(&fx, Adult, "/api/v1/inbox?kind=bill").await.json()),
        json!(mx.m_owned)
    );
    assert_eq!(
        bill_msg(&get(&fx, Adult2, "/api/v1/inbox?kind=bill").await.json()),
        Value::Null
    );

    // The dashboard carries per-adult counts, not the inbox: the owner sees all four, the other
    // adult only what their sources allow (the household source holds none of them).
    let r = get(&fx, Device, "/api/v1/dashboard").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.json().get("inbox").is_none());
    let pending = r.json()["pending_approvals"].clone();
    let count_of = |m: MemberId| {
        pending
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["member_id"] == json!(m))
            .map(|p| p["count"].clone())
    };
    assert_eq!(count_of(fx.adult), Some(json!(4)));
    assert_eq!(count_of(fx.adult2), Some(json!(0)));

    // Approval is for adults: not children, not plain devices.
    let uri = format!("/api/v1/inbox/bill/{}/approve", it.bill);
    for who in [Child, Device] {
        assert_eq!(
            post(&fx, who, &uri, None).await.status,
            StatusCode::FORBIDDEN,
            "{who:?}"
        );
    }
    assert_eq!(
        post(&fx, Anon, &uri, None).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn inbox_approve_and_reject_each_kind(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;
    let it = seed_inbox(&fx, &mx).await;
    let inbox = |kind: &str, id: &dyn std::fmt::Display, verb: &str| {
        format!("/api/v1/inbox/{kind}/{id}/{verb}")
    };

    // Event: approve with edits queues the write; approving twice is a 409.
    let uri = inbox("event", &it.event, "approve");
    let r = post(
        &fx,
        Adult,
        &uri,
        Some(json!({"calendar": it.calendar, "title": "Spring concert"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    let st: String = scalar(
        &fx,
        &format!("SELECT status FROM event WHERE id = '{}'", it.event),
    )
    .await;
    let title: String = scalar(
        &fx,
        &format!("SELECT title FROM event WHERE id = '{}'", it.event),
    )
    .await;
    assert_eq!(
        (st.as_str(), title.as_str()),
        ("approved", "Spring concert")
    );
    assert_eq!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM job WHERE kind = 'calendar.write'"
        )
        .await,
        1
    );
    assert_eq!(
        post(&fx, Adult, &uri, None).await.status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(&fx, Adult, &inbox("event", &it.event, "reject"), None)
            .await
            .status,
        StatusCode::CONFLICT
    );
    // Event without a writable choice: 422. Unknown id: 404.
    let e2 = new_event(&fx, "Meeting", None, "proposed").await;
    sql(&fx, "UPDATE calendar SET is_default_target = false").await;
    let r = post(
        &fx,
        Adult,
        &inbox("event", &e2, "approve"),
        Some(json!({"calendar": CalendarId::new()})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
    let r = post(&fx, Adult, &inbox("event", &e2, "reject"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<String>(&fx, &format!("SELECT status FROM event WHERE id = '{e2}'")).await,
        "rejected"
    );
    assert_eq!(
        post(
            &fx,
            Adult,
            &inbox("event", &EventId::new(), "approve"),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // A cancellation of a written event: approve deletes, reject keeps.
    let c1 = new_event(&fx, "Old", None, "written").await;
    let c2 = new_event(&fx, "Keep", None, "written").await;
    sql(
        &fx,
        &format!("UPDATE event SET pending_cancel = true WHERE id IN ('{c1}', '{c2}')"),
    )
    .await;
    assert_eq!(
        get(&fx, Adult, "/api/v1/inbox?kind=event").await.json()["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let r = post(&fx, Adult, &inbox("event", &c1, "approve"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM job WHERE kind = 'calendar.delete'"
        )
        .await,
        1
    );
    let r = post(&fx, Adult, &inbox("event", &c2, "reject"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert!(
        !scalar::<bool>(
            &fx,
            &format!("SELECT pending_cancel FROM event WHERE id = '{c2}'")
        )
        .await
    );

    // Bill: approve with edits opens it and queues the pay task; reject dismisses.
    let r = post(
        &fx,
        Adult,
        &inbox("bill", &it.bill, "approve"),
        Some(json!({"amount_minor": 20_000})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    let row: (String, i64) = sqlx::query_as(&format!(
        "SELECT status, amount_minor FROM bill WHERE id = '{}'",
        it.bill
    ))
    .fetch_one(fx.store.pool())
    .await
    .unwrap();
    assert_eq!(row, ("open".to_owned(), 20_000));
    assert_eq!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM job WHERE kind = 'tasks.from_bill'"
        )
        .await,
        1
    );
    let b2 = new_bill(&fx, "Water Co", BillStatus::Proposed, None).await;
    let r = post(&fx, Adult, &inbox("bill", &b2, "reject"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<String>(&fx, &format!("SELECT status FROM bill WHERE id = '{b2}'")).await,
        "dismissed"
    );
    // Rejecting an open bill is a conflict.
    assert_eq!(
        post(&fx, Adult, &inbox("bill", &it.bill, "reject"), None)
            .await
            .status,
        StatusCode::CONFLICT
    );

    // Task: approve applies the edits first and opens it; reject skips.
    let r = post(
        &fx,
        Adult,
        &inbox("task", &it.task, "approve"),
        Some(json!({"title": "Sign and return the form", "assignees": [fx.adult2]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    let row: (String, String) = sqlx::query_as(&format!(
        "SELECT status, title FROM task WHERE id = '{}'",
        it.task
    ))
    .fetch_one(fx.store.pool())
    .await
    .unwrap();
    assert_eq!(
        row,
        ("open".to_owned(), "Sign and return the form".to_owned())
    );
    assert_eq!(
        post(&fx, Adult, &inbox("task", &it.task, "approve"), None)
            .await
            .status,
        StatusCode::CONFLICT
    );
    let t2 = fa_tasks::TaskService::new(fx.store.clone())
        .from_mail(fa_tasks::MailTaskRequest {
            household: Some(fx.hid),
            message: mx.m_owned,
            kind: None,
            title: "Bring snacks".into(),
            due_at: None,
            assignees: vec![fx.adult],
            mode: fa_tasks::MailTaskMode::Suggest,
            external_url: None,
        })
        .await
        .unwrap()
        .unwrap();
    let r = post(&fx, Adult, &inbox("task", &t2, "reject"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<String>(&fx, &format!("SELECT status FROM task WHERE id = '{t2}'")).await,
        "skipped"
    );
    // A task that did not come from email is not an inbox item.
    let list = post(
        &fx,
        Adult,
        "/api/v1/task-lists",
        Some(json!({"name": "Chores", "kind": "chores", "completion_policy": "assignee",
                    "child_can_add": "none", "dashboard_add": false, "calendar_default": "hidden",
                    "claim_ttl_minutes": null, "visible_on_dashboard": true, "color": null, "sort": 0})),
    )
    .await;
    let r = post(
        &fx,
        Adult,
        "/api/v1/tasks",
        Some(json!({"list": id(&list.json()), "title": "Mow", "assignees": [fx.adult]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_eq!(
        post(&fx, Adult, &inbox("task", &id(&r.json()), "approve"), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    // Contact: approve creates the org contact; reject is remembered.
    let r = post(
        &fx,
        Adult,
        &inbox("contact", &it.contact, "approve"),
        Some(json!({"name": "Coach Lee", "role": "coach"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    let name: String = scalar(
        &fx,
        "SELECT name FROM org_contact WHERE email = 'coach@school.example.edu'",
    )
    .await;
    assert_eq!(name, "Coach Lee");
    assert_eq!(
        post(&fx, Adult, &inbox("contact", &it.contact, "approve"), None)
            .await
            .status,
        StatusCode::CONFLICT
    );
    let org: Uuid = scalar(&fx, "SELECT id FROM org LIMIT 1").await;
    let c2 = ContactSuggestionId::new();
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    ContactSuggestionRepo::insert_if_new(
        &mut tx,
        &ContactSuggestion {
            id: c2,
            household_id: fx.hid,
            org_id: org.into(),
            email: "aide@school.example.edu".into(),
            name: None,
            member_id: None,
            message_id: None,
            status: SuggestionStatus::Proposed,
            decided_by: None,
            decided_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let r = post(&fx, Adult, &inbox("contact", &c2, "reject"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<String>(
            &fx,
            &format!("SELECT status FROM contact_suggestion WHERE id = '{c2}'")
        )
        .await,
        "rejected"
    );

    // Every decision is in the audit log; the inbox is empty now.
    assert!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM audit_entry WHERE action = 'inbox.approve'"
        )
        .await
            >= 4
    );
    assert!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM audit_entry WHERE action = 'inbox.reject'"
        )
        .await
            >= 3
    );
    // Only the approved cancellation remains, until its `calendar.delete` job has run.
    let r = get(&fx, Adult, "/api/v1/inbox").await;
    assert_eq!(
        r.json()["counts"],
        json!({"event": 1, "bill": 0, "task": 0, "contact": 0, "total": 1}),
        "{}",
        r.text()
    );
    // An unknown kind is a bad request.
    assert_eq!(
        post(
            &fx,
            Adult,
            &format!("/api/v1/inbox/nope/{}/approve", it.bill),
            None
        )
        .await
        .status
        .as_u16(),
        400
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn failed_event_writes_are_listed_and_can_be_retried(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let (_, cal) = writable_calendar(&fx).await;
    let e = new_event(&fx, "Recital", None, "approved").await;
    sql(
        &fx,
        &format!("UPDATE event SET write_error = 'the calendar refused it', target_calendar_id = '{cal}' WHERE id = '{e}'"),
    )
    .await;
    let r = get(&fx, Adult, "/api/v1/inbox?kind=event").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let items = r.json()["items"].as_array().unwrap().clone();
    assert_eq!(items.len(), 1, "{}", r.text());
    assert_eq!(
        items[0]["event"]["write_error"],
        json!("the calendar refused it")
    );

    // Approving again retries: the error is cleared and a write is queued.
    let uri = format!("/api/v1/inbox/event/{e}/approve");
    let r = post(&fx, Adult, &uri, Some(json!({}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM job WHERE kind = 'calendar.write'"
        )
        .await,
        1
    );
    let r = get(&fx, Adult, "/api/v1/inbox?kind=event").await;
    assert_eq!(r.json()["items"].as_array().unwrap().len(), 0);

    // Or reject it.
    let e2 = new_event(&fx, "Rehearsal", None, "approved").await;
    sql(
        &fx,
        &format!("UPDATE event SET write_error = 'nope' WHERE id = '{e2}'"),
    )
    .await;
    let r = post(
        &fx,
        Adult,
        &format!("/api/v1/inbox/event/{e2}/reject"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn adult_cannot_decide_another_adults_owners_only_item(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;
    let _ = seed_inbox(&fx, &mx).await;
    let bill = new_bill(&fx, "Water Co", BillStatus::Proposed, Some(mx.m_hidden)).await;
    let event = new_event(&fx, "Hidden concert", Some(mx.m_hidden), "proposed").await;
    let task = fa_tasks::TaskService::new(fx.store.clone())
        .from_mail(fa_tasks::MailTaskRequest {
            household: Some(fx.hid),
            message: mx.m_hidden,
            kind: None,
            title: "Hidden form".into(),
            due_at: None,
            assignees: vec![fx.adult2],
            mode: fa_tasks::MailTaskMode::Suggest,
            external_url: None,
        })
        .await
        .unwrap()
        .expect("task suggested");
    for (kind, id) in [
        ("bill", bill.to_string()),
        ("event", event.to_string()),
        ("task", task.to_string()),
    ] {
        for action in ["approve", "reject"] {
            let uri = format!("/api/v1/inbox/{kind}/{id}/{action}");
            assert_eq!(
                post(&fx, Adult, &uri, None).await.status,
                StatusCode::FORBIDDEN,
                "{kind} {action}"
            );
        }
    }
    // The suggested-task routes outside the inbox are scoped the same way.
    for action in ["accept", "skip"] {
        let uri = format!("/api/v1/tasks/{task}/{action}");
        assert_eq!(
            post(&fx, Adult, &uri, None).await.status,
            StatusCode::FORBIDDEN,
            "{action}"
        );
    }
    // The adult who owns the source may.
    let r = post(
        &fx,
        Adult2,
        &format!("/api/v1/inbox/bill/{bill}/reject"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
}

// ---- bills ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn bills_list_and_patch(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let mx = seed_mail(&fx).await;
    let open = new_bill(&fx, "Power Co", BillStatus::Open, Some(mx.m_owned)).await;
    let proposed = new_bill(&fx, "Water Co", BillStatus::Proposed, Some(mx.m_hidden)).await;

    let r = get(&fx, Adult, "/api/v1/bills").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json().as_array().unwrap().len(), 2);
    let r = get(&fx, Adult, "/api/v1/bills?status=open").await;
    assert_eq!(ids(&r.json(), "id"), vec![open.to_string()]);
    assert_eq!(r.json()[0]["message"], json!(mx.m_owned));
    assert_eq!(r.json()[0]["amount_minor"], json!(12_345));
    // The hidden message's link is withheld from the adult who cannot read it.
    let r = get(&fx, Adult, "/api/v1/bills?status=proposed").await;
    assert_eq!(r.json()[0]["message"], Value::Null);
    assert_eq!(r.json()[0]["external_url"], Value::Null);
    for who in [Child, Device, Elevated] {
        assert_eq!(
            get(&fx, who, "/api/v1/bills").await.status,
            StatusCode::FORBIDDEN,
            "{who:?}"
        );
    }

    let h = auth_of(&fx, Adult);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let uri = format!("/api/v1/bills/{open}");
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"payee": "Power & Light"})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 428);
    hv.push(("if-match", "1"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"payee": "Power & Light", "amount_minor": 999, "due_on": null})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["payee"], json!("Power & Light"));
    assert_eq!(r.json()["amount_minor"], json!(999));
    assert_eq!(r.json()["due_on"], Value::Null);
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"payee": "Stale"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    let mut hv2: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv2.push(("if-match", "2"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv2,
        Some(json!({"amount_minor": 0})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422);
    assert_eq!(r.json()["field"], json!("amount_minor"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv2,
        Some(json!({"status": "dismissed"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["status"], json!("dismissed"));
    // Dismissing a proposal goes through the pipeline decision.
    let mut hv3: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv3.push(("if-match", "1"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/bills/{proposed}"),
        &hv3,
        Some(json!({"status": "dismissed"})),
    )
    .await;
    // The proposal came from another adult's private source: this adult may not decide it (D-078).
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.text());
    let h2 = auth_of(&fx, Adult2);
    let mut hv4: Vec<(&str, &str)> = h2.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv4.push(("if-match", "1"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/bills/{proposed}"),
        &hv4,
        Some(json!({"status": "dismissed"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["status"], json!("dismissed"));
    assert_eq!(post(&fx, Child, &uri, None).await.status.as_u16(), 405);
    assert_eq!(
        as_(&fx, Child, Method::PATCH, &uri, Some(json!({"payee": "x"})))
            .await
            .status
            .as_u16(),
        403
    );
}

// ---- calendar toggles and OAuth purpose ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn calendar_write_toggles(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let (src, cal) = writable_calendar(&fx).await;
    sql(
        &fx,
        &format!("UPDATE calendar_source SET writable = false WHERE id = '{src}'"),
    )
    .await;

    let list = get(&fx, Adult, "/api/v1/calendar-sources").await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.text());
    let s = list.json()[0].clone();
    assert_eq!(s["writable"], json!(false));
    assert_eq!(s["name"], json!("Family"));
    let version = s["version"].as_i64().unwrap().to_string();

    let h = auth_of(&fx, Adult);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", version.as_str()));
    let uri = format!("/api/v1/calendar-sources/{src}");
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"writable": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["writable"], json!(true));
    assert_eq!(
        get(&fx, Adult, "/api/v1/calendar-sources").await.json()[0]["writable"],
        json!(true)
    );

    let cals = get(&fx, Adult, "/api/v1/calendars").await;
    assert_eq!(cals.json()[0]["is_default_target"], json!(false));
    let v = cals.json()[0]["version"].as_i64().unwrap().to_string();
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", v.as_str()));
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/calendars/{cal}"),
        &hv,
        Some(json!({"is_default_target": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["is_default_target"], json!(true));
    assert_eq!(
        get(&fx, Adult, "/api/v1/calendars").await.json()[0]["is_default_target"],
        json!(true)
    );

    // A Google source needs the account to have granted calendar write access.
    let mut tx = fx.store.tx_for(fx.hid).await.unwrap();
    let acct = OAuthAccountRepo::upsert(
        &mut tx,
        fx.hid,
        "google",
        fx.adult,
        "alex@example.com",
        &["https://www.googleapis.com/auth/calendar.readonly".to_owned()],
        None,
    )
    .await
    .unwrap()
    .value;
    let g = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: fx.hid,
        kind: SourceKind::Google,
        name: "Google".into(),
        owner_member: Some(fx.adult),
        oauth_account: Some(acct.id),
        url: None,
        username: None,
        secret_id: None,
        writable: false,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    CalendarSourceRepo::insert(&mut tx, &g).await.unwrap();
    tx.commit().await.unwrap();
    let uri = format!("/api/v1/calendar-sources/{}", g.id);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", "1"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"writable": true})),
    )
    .await;
    assert_eq!(r.status.as_u16(), 422, "{}", r.text());
    assert!(
        r.json()["detail"]
            .as_str()
            .unwrap()
            .contains("calendar write access")
    );
    sql(
        &fx,
        &format!("UPDATE oauth_account SET scopes = scopes || ARRAY['https://www.googleapis.com/auth/calendar.events'] WHERE id = '{}'", acct.id),
    )
    .await;
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", "2"));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"writable": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["writable"], json!(true));
    // Plain edits still work without the new fields.
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", r.json()["version"].to_string().leak()));
    let r = send(
        &fx.app,
        Method::PATCH,
        &uri,
        &hv,
        Some(json!({"name": "Google (Alex)"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["name"], json!("Google (Alex)"));
    assert_eq!(r.json()["writable"], json!(true));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn oauth_start_requests_the_purpose_scope(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    for (purpose, scope) in [
        ("gmail_read", "gmail.readonly"),
        ("gmail_modify", "gmail.modify"),
        ("calendar_write", "calendar.events"),
    ] {
        let h = auth_of(&fx, Adult);
        let h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let mut b = Request::builder().method(Method::GET).uri(format!(
            "/api/v1/oauth/google/start?purpose={purpose}&return_to=/settings"
        ));
        for (k, v) in &h {
            b = b.header(*k, *v);
        }
        let res = fx
            .app
            .clone()
            .oneshot(b.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(res.status().is_redirection(), "{purpose}: {}", res.status());
        let loc = res
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let url = url::Url::parse(&loc).unwrap();
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert!(q["scope"].contains(scope), "{purpose}: {}", q["scope"]);
        assert!(q["scope"].contains("calendar.readonly"));
        assert_eq!(q["include_granted_scopes"], "true");
    }
    // The default asks for the calendar scopes only; an unknown purpose is a bad request.
    let h = auth_of(&fx, Adult);
    let h: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let r = send(
        &fx.app,
        Method::GET,
        "/api/v1/oauth/google/start?purpose=everything",
        &h,
        None,
    )
    .await;
    assert_eq!(r.status.as_u16(), 400, "{}", r.text());
    let r = send(&fx.app, Method::GET, "/api/v1/oauth/google/start", &h, None).await;
    assert!(r.status.is_redirection(), "{}", r.text());
    assert_eq!(
        get(&fx, Child, "/api/v1/oauth/google/start?purpose=gmail_read")
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

// ---- household, OpenAPI ----

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn household_m3_fields_are_validated(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let r = get(&fx, Adult, "/api/v1/household").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let v = r.json();
    assert_eq!(v["currency"], json!("USD"));
    let ver = v["version"].as_i64().unwrap().to_string();
    let h = auth_of(&fx, Adult);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    hv.push(("if-match", ver.as_str()));
    for bad in [
        json!({"currency": "US"}),
        json!({"bill_lead_days": 61}),
        json!({"events": {"auto_write": false, "extract_threshold": 1.5, "route_threshold": 0.9}}),
        json!({"inbox": {"task_expiry_days": 0, "contact_expiry_days": 14}}),
    ] {
        let r = send(
            &fx.app,
            Method::PATCH,
            "/api/v1/household",
            &hv,
            Some(bad.clone()),
        )
        .await;
        assert_eq!(r.status.as_u16(), 422, "{bad}: {}", r.text());
    }
    let r = send(
        &fx.app,
        Method::PATCH,
        "/api/v1/household",
        &hv,
        Some(json!({"currency": "eur", "bill_lead_days": 5})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["currency"], json!("EUR"));
    assert_eq!(r.json()["bill_lead_days"], json!(5));
}

#[test]
fn openapi_lists_the_m3_routes() {
    let doc = serde_json::to_value(fa_api::openapi()).unwrap();
    let paths = doc["paths"].as_object().unwrap();
    for p in [
        "/api/v1/mail-sources",
        "/api/v1/mail-sources/{id}",
        "/api/v1/mail-sources/{id}/sync",
        "/api/v1/mail-sources/imap/test",
        "/api/v1/messages",
        "/api/v1/messages/{id}",
        "/api/v1/messages/{id}/labels",
        "/api/v1/messages/{id}/eval-sample",
        "/api/v1/messages/{id}/actions",
        "/api/v1/messages/{id}/task",
        "/api/v1/mail-actions/{id}/undo",
        "/api/v1/labelling/next",
        "/api/v1/orgs",
        "/api/v1/orgs/{id}",
        "/api/v1/orgs/{id}/contacts",
        "/api/v1/org-contacts/{id}",
        "/api/v1/members/{id}/enrollments",
        "/api/v1/enrollments/{id}",
        "/api/v1/mail-rules",
        "/api/v1/mail-rules/{id}",
        "/api/v1/mail-rules/test",
        "/api/v1/mail-signals",
        "/api/v1/mail-signals/{key}",
        "/api/v1/categories",
        "/api/v1/categories/{id}",
        "/api/v1/inbox",
        "/api/v1/inbox/{kind}/{id}/approve",
        "/api/v1/inbox/{kind}/{id}/reject",
        "/api/v1/bills",
        "/api/v1/bills/{id}",
    ] {
        assert!(paths.contains_key(p), "missing {p}");
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn labelling_orgs_and_enrollments_over_http(pool: PgPool) {
    use Who::*;
    let fx = seed(pool).await;
    let _mx = seed_mail(&fx).await;

    // Nothing to label yet: 204.
    let r = get(&fx, Adult, "/api/v1/labelling/next").await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text());
    assert_eq!(
        get(&fx, Child, "/api/v1/labelling/next").await.status,
        StatusCode::FORBIDDEN
    );

    // Organisations: any adult reads, writing is settings-like.
    let r = post(
        &fx,
        Adult2,
        "/api/v1/orgs",
        Some(
            json!({"kind": "school", "name": "Example School", "domains": ["School.Example.edu"]}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let org = id(&r.json());
    assert_eq!(r.json()["domains"], json!(["school.example.edu"]));
    let r = post(
        &fx,
        Adult,
        &format!("/api/v1/orgs/{org}/contacts"),
        Some(json!({"name": "Ms Rivera", "email": "rivera@school.example.edu", "role": "teacher"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let contact = id(&r.json());
    let r = get(&fx, Adult2, "/api/v1/orgs").await;
    assert_eq!(r.json()[0]["contacts"].as_array().unwrap().len(), 1);

    // Enrollments of a member, with the contact linked.
    let r = post(
        &fx,
        Adult,
        &format!("/api/v1/members/{}/enrollments", fx.child),
        Some(json!({"org": org, "grade": "3", "from": "2026-09-01", "contacts": [contact]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    assert_eq!(r.json()["from"], json!("2026-09-01"));
    let r = get(
        &fx,
        Adult,
        &format!("/api/v1/members/{}/enrollments", fx.child),
    )
    .await;
    assert_eq!(r.json().as_array().unwrap().len(), 1);
    assert_eq!(
        get(
            &fx,
            Child,
            &format!("/api/v1/members/{}/enrollments", fx.child)
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Signals and categories.
    let r = get(&fx, Adult, "/api/v1/mail-signals").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    let key = r.json()[0]["key"].as_str().unwrap().to_owned();
    let r = as_(
        &fx,
        Adult,
        Method::PATCH,
        &format!("/api/v1/mail-signals/{key}"),
        Some(json!({"enabled": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["enabled"], json!(false));
    let r = as_(
        &fx,
        Adult,
        Method::PATCH,
        "/api/v1/mail-signals/no_such_signal",
        Some(json!({"enabled": true})),
    )
    .await;
    assert!(matches!(r.status.as_u16(), 404 | 422), "{}", r.text());
    let r = get(&fx, Adult, "/api/v1/categories").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["key"] == json!("school"))
    );
    let r = post(
        &fx,
        Adult,
        "/api/v1/categories",
        Some(json!({"key": "pets", "label": "Pets", "behaviour": "keep"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text());
    let cat = r.json();
    let h = auth_of(&fx, Adult);
    let mut hv: Vec<(&str, &str)> = h.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let v = cat["version"].as_i64().unwrap().to_string();
    hv.push(("if-match", v.as_str()));
    let r = send(
        &fx.app,
        Method::PATCH,
        &format!("/api/v1/categories/{}", id(&cat)),
        &hv,
        Some(json!({"archived": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert_eq!(r.json()["archived"], json!(true));
    let r = get(&fx, Adult, "/api/v1/categories").await;
    assert!(
        !r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["key"] == json!("pets"))
    );

    // Make a task from a message.
    let r = post(
        &fx,
        Adult,
        &format!("/api/v1/messages/{}/task", _mx.m_owned),
        Some(json!({"title": "Reply"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.text());
    let r = post(
        &fx,
        Adult,
        &format!("/api/v1/messages/{}/task", _mx.m_owned),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.text());
    assert_eq!(
        scalar::<i64>(
            &fx,
            "SELECT count(*) FROM job WHERE kind = 'tasks.from_mail'"
        )
        .await,
        2
    );
}
