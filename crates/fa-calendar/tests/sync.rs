use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use fa_calendar::{CalendarError, CalendarService, ConnectorConfig, enqueue_due};
use fa_domain::api::{CalendarPatch, NewSource};
use fa_domain::calendar::{HouseholdFlags, Viewer};
use fa_domain::*;
use fa_store::*;
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgListener;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

struct FakeTokens(Mutex<Result<String, TokenError>>);

#[async_trait]
impl AccessTokenProvider for FakeTokens {
    async fn access_token(&self, _: HouseholdId, _: OAuthAccountId) -> Result<String, TokenError> {
        self.0.lock().unwrap().clone()
    }
}

struct Env {
    store: Store,
    svc: CalendarService,
    tokens: Arc<FakeTokens>,
    h: HouseholdId,
    adult: MemberId,
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

async fn env(pool: PgPool, server: Option<&MockServer>) -> Env {
    let store = Store::from_pool(pool.clone());
    let h = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
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
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let adult = member(h.id, "Adult", Role::Adult);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    MemberRepo::insert(&mut tx, &adult, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let tokens = Arc::new(FakeTokens(Mutex::new(Ok("tok".into()))));
    let cfg = ConnectorConfig {
        allow_http: true,
        allow_private_hosts: true,
        google_api_base: server
            .map(|s| s.uri())
            .unwrap_or_else(|| "http://127.0.0.1:1".into()),
    };
    let svc = CalendarService::new(
        store.clone(),
        MasterKey::from_bytes(&[7u8; 32], 1).unwrap(),
        cfg,
        tokens.clone(),
    )
    .unwrap();
    Env {
        store,
        svc,
        tokens,
        h: h.id,
        adult: adult.id,
    }
}

fn stamp(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn ics_feed(events: &[(&str, i64)]) -> String {
    let mut s = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Example//EN\r\n");
    for (uid, days) in events {
        let st = Utc::now() + Duration::days(*days);
        s += &format!(
            "BEGIN:VEVENT\r\nUID:{uid}@example.com\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{uid}\r\nEND:VEVENT\r\n",
            stamp(st),
            stamp(st + Duration::hours(1))
        );
    }
    s + "END:VCALENDAR\r\n"
}

async fn titles(e: &Env) -> Vec<String> {
    let mut v: Vec<String> = e
        .svc
        .events(
            e.h,
            Viewer::AdultUser,
            HouseholdFlags {
                private_events_as_busy: true,
            },
            Utc::now() - Duration::days(30),
            Utc::now() + Duration::days(300),
            None,
        )
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.title)
        .collect();
    v.sort();
    v
}

async fn source_of(e: &Env, id: CalendarSourceId) -> CalendarSource {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    CalendarSourceRepo::get(&mut tx, id)
        .await
        .unwrap()
        .unwrap()
        .value
}

async fn mount_ics(server: &MockServer, etag: &str, body: String) {
    Mock::given(method("GET"))
        .and(path("/feed.ics"))
        .and(header("if-none-match", etag))
        .respond_with(ResponseTemplate::new(304))
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/feed.ics"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", etag)
                .set_body_string(body),
        )
        .with_priority(5)
        .mount(server)
        .await;
}

fn new_ics(server: &MockServer) -> NewSource {
    NewSource::Ics {
        name: "School".into(),
        url: format!("{}/feed.ics?token=s3cret", server.uri()),
        owner: None,
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn ics_full_sync_304_and_removal(pool: PgPool) {
    let server = MockServer::start().await;
    mount_ics(&server, "\"v1\"", ics_feed(&[("a", 1), ("b", 2)])).await;
    let e = env(pool.clone(), Some(&server)).await;
    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen(CHANNEL).await.unwrap();

    let (view, cals) = e
        .svc
        .create_source(e.h, None, new_ics(&server))
        .await
        .unwrap();
    assert_eq!(cals.len(), 1);
    assert!(cals[0].enabled, "an ICS calendar is enabled");
    let masked = view.masked_url.unwrap();
    assert!(!masked.contains("s3cret") && masked.starts_with("http://"));

    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["a", "b"]);
    let s = source_of(&e, view.id).await;
    assert_eq!(s.state, SourceState::Ok);
    assert!(s.last_synced_at.is_some());
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let cal = CalendarRepo::list(&mut tx, Some(view.id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(cal.value.remote_id, "ics");
    drop(tx);

    // calendar.updated arrives on commit (source.updated may come first)
    let mut kinds = vec![];
    while let Ok(Ok(n)) = tokio::time::timeout(StdDuration::from_millis(500), listener.recv()).await
    {
        let m: ChangeMessage = serde_json::from_str(n.payload()).unwrap();
        kinds.push(m.kind);
    }
    assert!(kinds.contains(&"calendar.updated".to_string()), "{kinds:?}");
    assert!(kinds.contains(&"source.updated".to_string()), "{kinds:?}");

    // second run: the server answers 304 and nothing changes
    e.svc.sync_source(e.h, view.id).await.unwrap();
    let reqs = server.received_requests().await.unwrap();
    assert!(reqs.last().unwrap().headers.contains_key("if-none-match"));
    assert_eq!(titles(&e).await, ["a", "b"]);

    // the feed changes: "a" is gone, "c" is new
    server.reset().await;
    mount_ics(&server, "\"v2\"", ics_feed(&[("b", 2), ("c", 3)])).await;
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["b", "c"]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn ics_unauthorized_and_server_error(pool: PgPool) {
    let server = MockServer::start().await;
    mount_ics(&server, "\"v1\"", ics_feed(&[("a", 1)])).await;
    let e = env(pool, Some(&server)).await;
    let (view, _) = e
        .svc
        .create_source(e.h, None, new_ics(&server))
        .await
        .unwrap();

    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(source_of(&e, view.id).await.state, SourceState::NeedsReauth);

    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let err = e.svc.sync_source(e.h, view.id).await.unwrap_err();
    assert!(matches!(err, CalendarError::Remote(_)));
    let s = source_of(&e, view.id).await;
    assert_eq!(s.state, SourceState::Error);
    let le = s.last_error.unwrap();
    assert!(
        le.contains("500") && !le.contains("s3cret") && !le.contains("://"),
        "{le}"
    );

    // a credential change resets the state and queues a sync
    server.reset().await;
    mount_ics(&server, "\"v1\"", ics_feed(&[("a", 1)])).await;
    let v = e
        .svc
        .update_source(
            e.h,
            view.id,
            &fa_domain::api::SourcePatch {
                credentials: Some(fa_domain::api::SourceCredentials {
                    url: Some(format!("{}/feed.ics?token=new", server.uri())),
                    ..Default::default()
                }),
                ..Default::default()
            },
            current_version(&e, view.id).await,
        )
        .await
        .unwrap();
    assert_eq!(v.state, SourceState::Ok);
}

async fn current_version(e: &Env, id: CalendarSourceId) -> i32 {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    CalendarSourceRepo::get(&mut tx, id)
        .await
        .unwrap()
        .unwrap()
        .version
}

async fn caldav_server() -> MockServer {
    let server = MockServer::start().await;
    let fx = |f: &str| {
        std::fs::read_to_string(format!("{}/tests/fixtures/{f}", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
    };
    let dav = |body: String| {
        ResponseTemplate::new(207)
            .insert_header("Content-Type", "application/xml; charset=utf-8")
            .set_body_string(body)
    };
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/"))
        .respond_with(dav(fx("caldav_principal.xml")))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/alex/"))
        .and(wiremock::matchers::body_string_contains(
            "calendar-home-set",
        ))
        .respond_with(dav(fx("caldav_home.xml")))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/alex/"))
        .and(wiremock::matchers::body_string_contains("resourcetype"))
        .respond_with(dav(fx("caldav_collections.xml")))
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path("/alex/family/"))
        .respond_with(dav(fx("caldav_report.xml")))
        .mount(&server)
        .await;
    server
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn caldav_discovery_disabled_then_ctag_skip(pool: PgPool) {
    let server = caldav_server().await;
    let e = env(pool, None).await;
    let (view, cals) = e
        .svc
        .create_source(
            e.h,
            Some(e.adult),
            NewSource::Caldav {
                name: "Cloud".into(),
                url: server.uri(),
                username: "alex".into(),
                password: "pw".into(),
                owner: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(cals.len(), 2);
    assert!(cals.iter().all(|c| !c.enabled));
    assert_eq!(view.owner, Some(e.adult));

    // nothing enabled: nothing fetched
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert!(titles(&e).await.is_empty());

    let fam = cals.iter().find(|c| c.name.starts_with("Family")).unwrap();
    e.svc
        .update_calendar(
            e.h,
            fam.id,
            &CalendarPatch {
                enabled: Some(true),
                ..Default::default()
            },
            fam.version,
        )
        .await
        .unwrap();
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["Dentist & checkup", "School trip"]);
    let reports = |reqs: &[Request]| {
        reqs.iter()
            .filter(|r| r.method.as_str() == "REPORT")
            .count()
    };
    assert_eq!(reports(&server.received_requests().await.unwrap()), 1);

    // same ctag: no REPORT
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(reports(&server.received_requests().await.unwrap()), 1);
    assert_eq!(titles(&e).await.len(), 2);
}

fn g_events(items: serde_json::Value, token: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({"items": items, "nextSyncToken": token}))
}

fn g_item(id: &str, title: &str, days: i64) -> serde_json::Value {
    let s = Utc::now() + Duration::days(days);
    json!({"id": id, "summary": title,
           "start": {"dateTime": s.to_rfc3339()}, "end": {"dateTime": (s + Duration::hours(1)).to_rfc3339()}})
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn google_incremental_cancel_gone_and_reauth(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, Some(&server)).await;
    let acc = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let a =
            OAuthAccountRepo::upsert(&mut tx, e.h, "google", e.adult, "me@example.com", &[], None)
                .await
                .unwrap();
        tx.commit().await.unwrap();
        a.value.id
    };
    let events = "/calendar/v3/calendars/primary/events";
    let calendar_list = || async {
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"items": [{"id": "primary", "summary": "Me"}]})),
            )
            .mount(&server)
            .await;
    };
    calendar_list().await;
    Mock::given(method("GET"))
        .and(path(events))
        .and(header_exists("authorization"))
        .and(wiremock::matchers::query_param_is_missing("syncToken"))
        .respond_with(g_events(
            json!([g_item("e1", "One", 1), g_item("e2", "Two", 2)]),
            "s1",
        ))
        .mount(&server)
        .await;

    let (view, cals) = e
        .svc
        .create_source(
            e.h,
            None,
            NewSource::Google {
                oauth_account: acc,
                name: "Mine".into(),
                owner: Some(e.adult),
            },
        )
        .await
        .unwrap();
    assert!(!cals[0].enabled);
    e.svc
        .update_calendar(
            e.h,
            cals[0].id,
            &CalendarPatch {
                enabled: Some(true),
                ..Default::default()
            },
            cals[0].version,
        )
        .await
        .unwrap();
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["One", "Two"]);

    // incremental: e1 cancelled, e2 renamed
    server.reset().await;
    calendar_list().await;
    Mock::given(method("GET"))
        .and(path(events))
        .and(query_param("syncToken", "s1"))
        .respond_with(g_events(
            json!([{"id": "e1", "status": "cancelled"}, g_item("e2", "Two!", 2)]),
            "s2",
        ))
        .mount(&server)
        .await;
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["Two!"]);

    // 410 Gone: everything is dropped and refetched in full
    server.reset().await;
    calendar_list().await;
    Mock::given(method("GET"))
        .and(path(events))
        .and(query_param("syncToken", "s2"))
        .respond_with(ResponseTemplate::new(410))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(events))
        .and(wiremock::matchers::query_param_is_missing("syncToken"))
        .respond_with(g_events(json!([g_item("e3", "Three", 3)]), "s3"))
        .mount(&server)
        .await;
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(titles(&e).await, ["Three"]);

    // the token provider reports a revoked account
    *e.tokens.0.lock().unwrap() = Err(TokenError::NeedsReauth);
    e.svc.sync_source(e.h, view.id).await.unwrap();
    assert_eq!(source_of(&e, view.id).await.state, SourceState::NeedsReauth);
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let a = OAuthAccountRepo::get(&mut tx, acc).await.unwrap().unwrap();
    assert_eq!(a.value.state, SourceState::NeedsReauth);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn due_enqueue_is_idempotent(pool: PgPool) {
    let server = MockServer::start().await;
    mount_ics(&server, "\"v1\"", ics_feed(&[("a", 1)])).await;
    let e = env(pool, Some(&server)).await;
    e.svc
        .create_source(e.h, None, new_ics(&server))
        .await
        .unwrap();
    let now = Utc::now();
    let first = enqueue_due(&e.store, now).await.unwrap();
    assert!(first <= 1);
    assert_eq!(enqueue_due(&e.store, now).await.unwrap(), 0);
    let mut tx = e.store.tx_system().await.unwrap();
    assert_eq!(JobRepo::counts(&mut tx).await.unwrap().ready, 1);
}

fn occ(uid: &str, title: &str, private: bool) -> NewOccurrence {
    let s = Utc::now() + Duration::days(1);
    NewOccurrence {
        remote_uid: uid.into(),
        instance_key: String::new(),
        title: title.into(),
        description: Some("notes".into()),
        location: Some("Somewhere".into()),
        start_at: s,
        end_at: s + Duration::hours(1),
        all_day: false,
        start_date: None,
        end_date: None,
        source_tz: None,
        is_private: private,
        cancelled: false,
        remote_etag: None,
        external_url: None,
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn events_apply_visibility_and_scope(pool: PgPool) {
    let e = env(pool, None).await;
    let child = member(e.h, "Kid", Role::Child);
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    MemberRepo::insert(&mut tx, &child, None, None)
        .await
        .unwrap();
    let mk = |_tx: &mut Tx, name: &str, members: Vec<MemberId>, owner: Option<MemberId>| {
        let src = CalendarSource {
            id: CalendarSourceId::new(),
            household_id: e.h,
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
        (src, members)
    };
    let specs = [
        mk(&mut tx, "adult", vec![e.adult], Some(e.adult)),
        mk(&mut tx, "child", vec![child.id], None),
        mk(&mut tx, "house", vec![], None),
    ];
    let evs = [
        vec![
            occ("n1", "Adult plain", false),
            occ("p1", "Adult private", true),
        ],
        vec![occ("p2", "Child private", true)],
        vec![occ("h1", "Household", false)],
    ];
    for ((src, members), evs) in specs.iter().zip(&evs) {
        CalendarSourceRepo::insert(&mut tx, src).await.unwrap();
        let (c, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, "r", &src.name, None)
            .await
            .unwrap();
        CalendarRepo::update(
            &mut tx,
            c.value.id,
            &CalendarPatch {
                enabled: Some(true),
                ..Default::default()
            },
            c.version,
        )
        .await
        .unwrap();
        CalendarRepo::set_members(&mut tx, c.value.id, members)
            .await
            .unwrap();
        EventRepo::upsert_synced(&mut tx, c.value.id, evs)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();

    let get = |viewer, busy, member| {
        let svc = &e.svc;
        let h = e.h;
        async move {
            let mut v: Vec<(String, Visibility)> = svc
                .events(
                    h,
                    viewer,
                    HouseholdFlags {
                        private_events_as_busy: busy,
                    },
                    Utc::now(),
                    Utc::now() + Duration::days(3),
                    member,
                )
                .await
                .unwrap()
                .into_iter()
                .map(|x| {
                    if x.visibility == Visibility::Busy {
                        assert!(x.location.is_none() && x.description.is_none());
                    }
                    (x.title, x.visibility)
                })
                .collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        }
    };
    use Visibility::{Busy, Full};
    let all_full = get(Viewer::AdultUser, true, None).await;
    assert_eq!(all_full.len(), 4);
    assert!(all_full.iter().all(|(_, v)| *v == Full));

    let child_view = get(Viewer::ChildUser(child.id), true, None).await;
    assert_eq!(
        child_view,
        [("Child private".into(), Full), ("Household".into(), Full)]
    );

    let device = get(Viewer::Device, true, None).await;
    assert_eq!(
        device,
        [
            ("Adult plain".into(), Full),
            ("Busy".into(), Busy),
            ("Child private".into(), Full),
            ("Household".into(), Full)
        ]
    );
    assert!(
        get(Viewer::Device, false, None)
            .await
            .iter()
            .all(|(_, v)| *v == Full)
    );
    assert_eq!(
        get(Viewer::AdultUser, true, Some(child.id)).await,
        [("Child private".into(), Full)]
    );

    let too_long = e
        .svc
        .events(
            e.h,
            Viewer::AdultUser,
            HouseholdFlags {
                private_events_as_busy: true,
            },
            Utc::now(),
            Utc::now() + Duration::days(401),
            None,
        )
        .await;
    assert!(matches!(too_long, Err(CalendarError::RangeTooLarge(_))));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn prune_removes_old_events(pool: PgPool) {
    let e = env(pool, None).await;
    let src = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: e.h,
        kind: SourceKind::Ics,
        name: "x".into(),
        owner_member: None,
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
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    CalendarSourceRepo::insert(&mut tx, &src).await.unwrap();
    let (c, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, "r", "x", None)
        .await
        .unwrap();
    let mut old = occ("old", "Old", false);
    old.start_at = Utc::now() - Duration::days(60);
    old.end_at = old.start_at + Duration::hours(1);
    EventRepo::upsert_synced(&mut tx, c.value.id, &[old, occ("new", "New", false)])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(fa_calendar::prune(&e.store, Utc::now()).await.unwrap(), 1);
}
