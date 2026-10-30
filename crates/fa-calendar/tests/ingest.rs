use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, SubsecRound, Utc};
use fa_calendar::{CalendarService, ConnectorConfig};
use fa_domain::calendar::{HouseholdFlags, Viewer};
use fa_domain::mail_api::EventApproval;
use fa_domain::*;
use fa_store::*;
use serde_json::json;
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

struct Tokens;

#[async_trait]
impl AccessTokenProvider for Tokens {
    async fn access_token(&self, _: HouseholdId, _: OAuthAccountId) -> Result<String, TokenError> {
        Ok("tok".into())
    }
}

struct Env {
    store: Store,
    svc: CalendarService,
    h: HouseholdId,
    adult: MemberId,
    kid: MemberId,
    /// Google calendar on a writable source (not a default target).
    gcal: CalendarId,
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

fn source(h: HouseholdId, kind: SourceKind, acc: Option<OAuthAccountId>) -> CalendarSource {
    CalendarSource {
        id: CalendarSourceId::new(),
        household_id: h,
        kind,
        name: "Src".into(),
        owner_member: None,
        oauth_account: acc,
        url: None,
        username: None,
        secret_id: None,
        writable: true,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

async fn add_calendar(
    e: &Env,
    src: &CalendarSource,
    remote_id: &str,
    default: bool,
    members: &[MemberId],
) -> CalendarId {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let (c, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, remote_id, remote_id)
        .await
        .unwrap();
    sqlx::query("UPDATE calendar SET enabled = true, is_default_target = $2 WHERE id = $1")
        .bind(c.value.id.as_uuid())
        .bind(default)
        .execute(&mut *tx)
        .await
        .unwrap();
    if !members.is_empty() {
        CalendarRepo::set_members(&mut tx, c.value.id, members)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    c.value.id
}

async fn add_source(e: &Env, s: &CalendarSource) {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    CalendarSourceRepo::insert(&mut tx, s).await.unwrap();
    tx.commit().await.unwrap();
}

async fn env(pool: PgPool, server: &MockServer, scopes: &[&str], auto: bool) -> Env {
    let store = Store::from_pool(pool);
    let h = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
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
        quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        currency: fa_domain::DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: EventsSettings {
            auto_write: auto,
            ..Default::default()
        },
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let adult = member(h.id, "Adult", Role::Adult);
    let kid = member(h.id, "Kid", Role::Child);
    let acc = {
        let mut tx = store.tx_system().await.unwrap();
        HouseholdRepo::insert(&mut tx, &h).await.unwrap();
        MemberRepo::insert(&mut tx, &adult, None, None)
            .await
            .unwrap();
        MemberRepo::insert(&mut tx, &kid, None, None).await.unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.tx_for(h.id).await.unwrap();
        let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
        let a = OAuthAccountRepo::upsert(
            &mut tx,
            h.id,
            "google",
            adult.id,
            "me@example.com",
            &scopes,
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        a.value.id
    };
    let svc = CalendarService::new(
        store.clone(),
        MasterKey::from_bytes(&[7u8; 32], 1).unwrap(),
        ConnectorConfig {
            allow_http: true,
            allow_private_hosts: true,
            google_api_base: server.uri(),
        },
        Arc::new(Tokens),
    )
    .unwrap();
    let mut e = Env {
        store,
        svc,
        h: h.id,
        adult: adult.id,
        kid: kid.id,
        gcal: CalendarId::new(),
    };
    let src = source(h.id, SourceKind::Google, Some(acc));
    add_source(&e, &src).await;
    e.gcal = add_calendar(&e, &src, "primary", false, &[]).await;
    e
}

fn start(days: i64) -> DateTime<Utc> {
    (Utc::now() + Duration::days(days)).trunc_subsecs(0)
}

fn new_event(e: &Env, title: &str, at: DateTime<Utc>) -> NewExtractedEvent {
    NewExtractedEvent {
        household_id: e.h,
        message_id: None,
        org_id: None,
        title: title.into(),
        description: None,
        location: None,
        start_at: at,
        end_at: at + Duration::hours(1),
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
    }
}

async fn insert(e: &Env, ev: &NewExtractedEvent) -> EventId {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let id = EventRepo::insert_extracted(&mut tx, ev).await.unwrap();
    tx.commit().await.unwrap();
    id
}

async fn get(e: &Env, id: EventId) -> Option<EventFull> {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    EventRepo::get_full(&mut tx, id).await.unwrap()
}

async fn route(e: &Env, ev: &NewExtractedEvent) -> EventId {
    let id = insert(e, ev).await;
    e.svc.route_event(e.h, id).await.unwrap();
    id
}

async fn count(e: &Env, sql: &str) -> i64 {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap()
}

async fn jobs(e: &Env, kind: &str) -> i64 {
    count(
        e,
        &format!("SELECT count(*) FROM job WHERE kind = '{kind}'"),
    )
    .await
}

fn g_ok(id: &str, uid: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id": id, "iCalUID": uid, "etag": "\"e1\"", "htmlLink": "https://example.com/ev"
    }))
}

fn ea(calendar: Option<CalendarId>) -> EventApproval {
    EventApproval {
        calendar,
        title: None,
        start_at: None,
        end_at: None,
        all_day: None,
        members: None,
    }
}

const EVENTS: &str = "/calendar/v3/calendars/primary/events";

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn dedupe_by_key_similarity_and_uid(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let at = start(3);
    let first = route(&e, &new_event(&e, "Spring concert", at)).await;
    assert_eq!(
        get(&e, first).await.unwrap().event.status,
        Some(EventStatus::Proposed)
    );

    // same key
    let dup = route(&e, &new_event(&e, "Spring concert", at)).await;
    assert!(get(&e, dup).await.is_none());

    // similar title, 30 minutes apart
    let mut s = new_event(&e, "Spring Concert!", at + Duration::minutes(30));
    s.dedupe_key = "other".into();
    assert!(get(&e, route(&e, &s).await).await.is_none());

    // far apart in time: kept
    let mut far = new_event(&e, "Spring Concert", at + Duration::hours(5));
    far.dedupe_key = "far".into();
    assert!(get(&e, route(&e, &far).await).await.is_some());

    // different title, same time: kept
    let mut other = new_event(&e, "Dentist", at);
    other.dedupe_key = "dentist".into();
    assert!(get(&e, route(&e, &other).await).await.is_some());

    // same remote uid, different title
    let mut a = new_event(&e, "Zebra picnic", start(6));
    a.remote_uid = Some("uid-1@example.com".into());
    let kept = route(&e, &a).await;
    let mut b = new_event(&e, "Totally different", start(6) + Duration::hours(2));
    b.remote_uid = Some("uid-1@example.com".into());
    b.dedupe_key = "b".into();
    assert!(get(&e, route(&e, &b).await).await.is_none());
    assert!(get(&e, kept).await.is_some());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn dedupe_links_the_message(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let at = start(3);
    let first = route(&e, &new_event(&e, "Picture day", at)).await;
    // a dup with no message links nothing, but must not error
    route(&e, &new_event(&e, "Picture day", at)).await;
    assert_eq!(count(&e, "SELECT count(*) FROM event_message").await, 0);
    assert!(get(&e, first).await.is_some());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn cancellation_of_proposed_and_written(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let at = start(3);
    let p = route(&e, &new_event(&e, "Book fair", at)).await;
    let mut c = new_event(&e, "Book fair", at);
    c.cancelled = true;
    let cid = route(&e, &c).await;
    assert!(get(&e, cid).await.is_none(), "cancellation row deleted");
    assert_eq!(
        get(&e, p).await.unwrap().event.status,
        Some(EventStatus::Expired)
    );

    // a written event gets a pending cancel
    let at2 = start(4);
    let w = route(&e, &new_event(&e, "Field trip", at2)).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(g_ok("g1", "g1@google.com"))
        .mount(&server)
        .await;
    e.svc
        .approve_event(e.h, w, ea(None), Some(e.adult))
        .await
        .unwrap_err();
    e.svc
        .approve_event(e.h, w, ea(Some(e.gcal)), Some(e.adult))
        .await
        .unwrap();
    e.svc.write_event(e.h, w).await.unwrap();
    assert_eq!(
        get(&e, w).await.unwrap().event.status,
        Some(EventStatus::Written)
    );
    let mut c = new_event(&e, "Field trip", at2);
    c.cancelled = true;
    c.dedupe_key = "c2".into();
    route(&e, &c).await;
    let row = get(&e, w).await.unwrap();
    assert!(row.pending_cancel);
    assert_eq!(e.svc.proposals(e.h).await.unwrap().len(), 1);
    assert_eq!(jobs(&e, "calendar.delete").await, 0);

    // rejecting the cancel keeps it; approving deletes it at the provider
    e.svc.reject_cancel(e.h, w, Some(e.adult)).await.unwrap();
    assert!(!get(&e, w).await.unwrap().pending_cancel);
    let mut c = new_event(&e, "Field trip", at2);
    c.cancelled = true;
    c.dedupe_key = "c3".into();
    route(&e, &c).await;
    Mock::given(method("DELETE"))
        .and(path(format!("{EVENTS}/g1")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    e.svc.approve_cancel(e.h, w, Some(e.adult)).await.unwrap();
    assert_eq!(jobs(&e, "calendar.delete").await, 1);
    e.svc.delete_event(e.h, w).await.unwrap();
    assert!(get(&e, w).await.is_none());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn cancellation_with_auto_write_enqueues_delete(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], true).await;
    let at = start(3);
    let w = insert(&e, &new_event(&e, "Recital", at)).await;
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        EventRepo::set_written(&mut tx, w, e.gcal, "x@google.com", None, None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let mut c = new_event(&e, "Recital", at);
    c.cancelled = true;
    c.dedupe_key = "c".into();
    route(&e, &c).await;
    assert_eq!(jobs(&e, "calendar.delete").await, 1);
    assert!(!get(&e, w).await.unwrap().pending_cancel);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn routing_confidences(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let src = source(e.h, SourceKind::Google, None);
    add_source(&e, &src).await;
    let household_cal = add_calendar(&e, &src, "household", true, &[]).await;
    let member_cal = add_calendar(&e, &src, "kid", true, &[e.kid]).await;
    // a default target on a read-only source is never used
    let mut ro = source(e.h, SourceKind::Ics, None);
    ro.writable = false;
    add_source(&e, &ro).await;
    let _ro_cal = add_calendar(&e, &ro, "ro", true, &[e.kid]).await;

    // household default only
    let id = route(&e, &new_event(&e, "A", start(3))).await;
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.target_calendar_id, Some(household_cal));
    assert!((r.route_confidence.unwrap() - 0.7).abs() < 1e-6);

    // linked member's default
    let mut m = new_event(&e, "B", start(4));
    m.members = vec![e.kid];
    let r = get(&e, route(&e, &m).await).await.unwrap();
    assert_eq!(r.target_calendar_id, Some(member_cal));
    assert!((r.route_confidence.unwrap() - 0.9).abs() < 1e-6);

    // org default wins
    let org = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let o = Org {
            id: OrgId::new(),
            household_id: e.h,
            kind: OrgKind::School,
            name: "School".into(),
            domains: vec![],
            senders: vec![],
            ics_calendar_id: None,
            default_calendar_id: Some(e.gcal),
            contact_mode: ContactMode::Suggest,
            bill_amount_regex: None,
            bill_due_regex: None,
            bill_account_regex: None,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let o = OrgRepo::insert(&mut tx, &o).await.unwrap();
        tx.commit().await.unwrap();
        o.value.id
    };
    let mut o = new_event(&e, "C", start(5));
    o.org_id = Some(org);
    o.members = vec![e.kid];
    let r = get(&e, route(&e, &o).await).await.unwrap();
    assert_eq!(r.target_calendar_id, Some(e.gcal));
    assert!((r.route_confidence.unwrap() - 0.95).abs() < 1e-6);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn gate_off_proposes_and_notifies(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let src = source(e.h, SourceKind::Google, None);
    add_source(&e, &src).await;
    add_calendar(&e, &src, "household", true, &[]).await;
    let id = route(&e, &new_event(&e, "Concert", start(3))).await;
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Proposed));
    assert_eq!(jobs(&e, "calendar.write").await, 0);
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM notification WHERE trigger = 'approval_needed'"
        )
        .await,
        1,
        "one adult"
    );
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM notification WHERE dedupe_key LIKE 'approval:event:%'"
        )
        .await,
        1
    );
    let p = e.svc.proposals(e.h).await.unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].id, id);
    assert!(p[0].suggested_calendar.is_some());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn gate_on_approves_and_enqueues_write(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], true).await;
    let src = source(e.h, SourceKind::Google, None);
    add_source(&e, &src).await;
    // 0.7 household default is below the 0.9 route threshold
    add_calendar(&e, &src, "household", true, &[]).await;
    let id = route(&e, &new_event(&e, "Low", start(3))).await;
    assert_eq!(
        get(&e, id).await.unwrap().event.status,
        Some(EventStatus::Proposed)
    );

    // member default (0.9) passes
    let mcal = add_calendar(&e, &src, "kid", true, &[e.kid]).await;
    let mut m = new_event(&e, "High", start(4));
    m.members = vec![e.kid];
    let id = route(&e, &m).await;
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Approved));
    assert_eq!(r.target_calendar_id, Some(mcal));
    assert_eq!(jobs(&e, "calendar.write").await, 1);

    // low extraction confidence blocks auto write
    let mut low = new_event(&e, "Unsure", start(5));
    low.members = vec![e.kid];
    low.confidence = 0.5;
    let id = route(&e, &low).await;
    assert_eq!(
        get(&e, id).await.unwrap().event.status,
        Some(EventStatus::Proposed)
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn approve_with_edits_writes_to_google_and_sync_does_not_duplicate(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let at = start(3);
    let id = route(&e, &new_event(&e, "Concert", at)).await;
    let edited = start(3) + Duration::hours(2);
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(g_ok("g1", "g1@google.com"))
        .expect(1)
        .mount(&server)
        .await;
    e.svc
        .approve_event(
            e.h,
            id,
            EventApproval {
                title: Some("Winter concert".into()),
                start_at: Some(edited),
                end_at: Some(edited + Duration::hours(2)),
                members: Some(vec![e.kid]),
                ..ea(Some(e.gcal))
            },
            Some(e.adult),
        )
        .await
        .unwrap();
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Approved));
    assert_eq!(r.decided_by, Some(e.adult));
    assert_eq!(r.members, vec![e.kid]);
    assert_eq!(jobs(&e, "calendar.write").await, 1);
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM decision WHERE is_final AND source = 'human'"
        )
        .await,
        1
    );

    e.svc.write_event(e.h, id).await.unwrap();
    // a second run is a no-op (the mock expects exactly one POST)
    e.svc.write_event(e.h, id).await.unwrap();
    let body: serde_json::Value = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .find(|r| r.method.as_str() == "POST")
        .map(|r| serde_json::from_slice(&r.body).unwrap())
        .unwrap();
    assert_eq!(body["summary"], "Winter concert");
    assert_eq!(
        body["start"]["dateTime"],
        edited.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Written));
    assert_eq!(r.event.calendar, Some(e.gcal));
    assert_eq!(r.event.remote_uid.as_deref(), Some("g1"));
    assert!(r.write_error.is_none());
    assert_eq!(
        r.event.source_ref.as_ref().unwrap()["external_url"],
        "https://example.com/ev"
    );

    // the next sync returns the same event and must not duplicate it
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"items": [{"id": "primary", "summary": "Me"}]})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(EVENTS))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"id": "g1", "iCalUID": "g1@google.com", "summary": "Winter concert",
                       "start": {"dateTime": edited.to_rfc3339()},
                       "end": {"dateTime": (edited + Duration::hours(2)).to_rfc3339()}}],
            "nextSyncToken": "s1"})))
        .mount(&server)
        .await;
    let src = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        CalendarSourceRepo::list(&mut tx, false).await.unwrap()
    };
    for s in src {
        e.svc.sync_source(e.h, s.value.id).await.ok();
    }
    let n = e
        .svc
        .events(
            e.h,
            Viewer::AdultUser,
            HouseholdFlags {
                private_events_as_busy: true,
            },
            Utc::now() - Duration::days(1),
            Utc::now() + Duration::days(30),
            None,
        )
        .await
        .unwrap()
        .into_iter()
        .filter(|x| x.title == "Winter concert")
        .count();
    assert_eq!(n, 1);
    assert_eq!(
        get(&e, id).await.unwrap().event.origin,
        EventOrigin::Extracted
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn caldav_put(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let mut src = source(e.h, SourceKind::Caldav, None);
    src.url = Some(server.uri());
    src.username = Some("me".into());
    add_source(&e, &src).await;
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let sid = SecretRepo::put(
            &mut tx,
            &MasterKey::from_bytes(&[7u8; 32], 1).unwrap(),
            e.h,
            "calendar_source",
            src.id.as_uuid(),
            SecretKind::Other,
            b"pw",
            None,
        )
        .await
        .unwrap();
        CalendarSourceRepo::set_secret(&mut tx, src.id, sid)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let cal = add_calendar(&e, &src, "/dav/cal/", true, &[]).await;
    let id = route(&e, &new_event(&e, "Swim meet", start(3))).await;
    e.svc
        .approve_event(e.h, id, ea(Some(cal)), None)
        .await
        .unwrap();
    let uid = format!("fa-{}@family-assistant", id.as_uuid());
    Mock::given(method("PUT"))
        .and(path(format!("/dav/cal/{uid}.ics")))
        .respond_with(ResponseTemplate::new(201).insert_header("ETag", "\"c1\""))
        .expect(1)
        .mount(&server)
        .await;
    e.svc.write_event(e.h, id).await.unwrap();
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Written));
    assert_eq!(r.event.calendar, Some(cal));
    assert_eq!(r.event.remote_uid.as_deref(), Some(uid.as_str()));
    assert_eq!(r.event.remote_etag.as_deref(), Some("\"c1\""));
    let put = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.method.as_str() == "PUT")
        .unwrap();
    let body = String::from_utf8(put.body).unwrap();
    assert!(
        body.contains("SUMMARY:Swim meet") && body.contains(&uid),
        "{body}"
    );

    // delete
    Mock::given(method("DELETE"))
        .and(path(format!("/dav/cal/{uid}.ics")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    e.svc.delete_event(e.h, id).await.unwrap();
    assert!(get(&e, id).await.is_none());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn write_error_is_kept_on_the_row(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let id = route(&e, &new_event(&e, "Concert", start(3))).await;
    e.svc
        .approve_event(e.h, id, ea(Some(e.gcal)), None)
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert!(
        e.svc.write_event(e.h, id).await.is_err(),
        "transient: retried by the job policy"
    );
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Approved));
    let msg = r.write_error.unwrap();
    assert!(msg.contains("500") && !msg.contains("://"), "{msg}");

    // success clears it
    server.reset().await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(g_ok("g9", "g9@google.com"))
        .mount(&server)
        .await;
    e.svc.write_event(e.h, id).await.unwrap();
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Written));
    assert!(r.write_error.is_none());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn missing_scope_is_reported_without_a_call(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(
        pool,
        &server,
        &["https://www.googleapis.com/auth/calendar.readonly"],
        false,
    )
    .await;
    let id = route(&e, &new_event(&e, "Concert", start(3))).await;
    e.svc
        .approve_event(e.h, id, ea(Some(e.gcal)), None)
        .await
        .unwrap();
    e.svc.write_event(e.h, id).await.unwrap();
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Approved));
    assert_eq!(
        r.write_error.as_deref(),
        Some("reconnect with calendar write access")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn reject_and_state_checks(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let id = route(&e, &new_event(&e, "Concert", start(3))).await;
    e.svc.reject_event(e.h, id, Some(e.adult)).await.unwrap();
    let r = get(&e, id).await.unwrap();
    assert_eq!(r.event.status, Some(EventStatus::Rejected));
    assert_eq!(r.decided_by, Some(e.adult));
    assert!(e.svc.proposals(e.h).await.unwrap().is_empty());
    assert!(e.svc.reject_event(e.h, id, None).await.is_err());
    assert!(e.svc.approve_event(e.h, id, ea(None), None).await.is_err());
    // an approval naming a calendar that is not writable is refused
    let id2 = route(&e, &new_event(&e, "Other", start(4))).await;
    let bogus = CalendarId::new();
    let err = e.svc.approve_event(e.h, id2, ea(Some(bogus)), None).await;
    assert!(err.is_err());
    assert_eq!(
        get(&e, id2).await.unwrap().event.status,
        Some(EventStatus::Proposed)
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn failed_writes_are_proposals_and_retry_by_approving(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server, &[SCOPE], false).await;
    let id = route(&e, &new_event(&e, "Concert", start(3))).await;
    e.svc
        .approve_event(e.h, id, ea(Some(e.gcal)), None)
        .await
        .unwrap();
    assert!(e.svc.proposals(e.h).await.unwrap().is_empty());
    assert!(e.svc.failed_writes(e.h).await.unwrap().is_empty());

    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert!(e.svc.write_event(e.h, id).await.is_err());
    let p = e.svc.proposals(e.h).await.unwrap();
    assert_eq!(p.len(), 1);
    assert!(p[0].write_error.as_deref().unwrap().contains("500"));
    assert_eq!(e.svc.failed_writes(e.h).await.unwrap().len(), 1);

    // Approving again retries: the error is cleared and a new write job is queued.
    let jobs_before = jobs(&e, "calendar.write").await;
    e.svc
        .approve_event(e.h, id, ea(Some(e.gcal)), None)
        .await
        .unwrap();
    assert!(e.svc.proposals(e.h).await.unwrap().is_empty());
    assert!(get(&e, id).await.unwrap().write_error.is_none());
    assert_eq!(jobs(&e, "calendar.write").await, jobs_before + 1);

    // A state conflict is its own error (HTTP 409), not a validation failure.
    let done = route(&e, &new_event(&e, "Other", start(4))).await;
    e.svc.reject_event(e.h, done, None).await.unwrap();
    assert!(matches!(
        e.svc.approve_event(e.h, done, ea(None), None).await,
        Err(fa_calendar::CalendarError::State(_))
    ));
    assert!(matches!(
        e.svc.reject_event(e.h, done, None).await,
        Err(fa_calendar::CalendarError::State(_))
    ));
}
