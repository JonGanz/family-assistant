use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveTime, TimeZone, Utc};
use fa_domain::*;
use fa_notify::{Notifier, NotifyConfig, NotifyError, NotifyService, NtfyMessage};
use fa_store::*;
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Fake {
    sent: Mutex<Vec<(String, NtfyMessage)>>,
    fail: Mutex<bool>,
}

#[async_trait::async_trait]
impl Notifier for Fake {
    async fn publish(&self, topic: &str, msg: &NtfyMessage) -> Result<(), NotifyError> {
        if *self.fail.lock().unwrap() {
            return Err(NotifyError::Publish("boom".into()));
        }
        self.sent.lock().unwrap().push((topic.into(), msg.clone()));
        Ok(())
    }
}

fn utc(d: u32, h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap()
}

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn config(url: Option<String>) -> NotifyConfig {
    NotifyConfig {
        ntfy_url: url,
        ntfy_token: None,
        public_base_url: "https://fa.example.com/".into(),
        subscribe_base: None,
    }
}

struct Env {
    store: Store,
    svc: NotifyService,
    fake: Arc<Fake>,
    h: HouseholdId,
    adult: MemberId,
    child: MemberId,
}

async fn member(store: &Store, h: HouseholdId, name: &str, role: Role) -> MemberId {
    let m = Member {
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
    };
    let mut tx = store.tx_for(h).await.unwrap();
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    tx.commit().await.unwrap();
    m.id
}

async fn env(pool: &PgPool, url: Option<&str>) -> Env {
    let store = Store::from_pool(pool.clone());
    let hh = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
        tz: "Europe/Berlin".into(),
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
        quiet_from: hm(21, 0),
        quiet_until: hm(7, 0),
        currency: fa_domain::DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    tx.commit().await.unwrap();
    let adult = member(&store, hh.id, "Alex", Role::Adult).await;
    let child = member(&store, hh.id, "Sam", Role::Child).await;
    let fake = Arc::new(Fake::default());
    let svc =
        NotifyService::with_notifier(store.clone(), config(url.map(Into::into)), fake.clone());
    Env {
        store,
        svc,
        fake,
        h: hh.id,
        adult,
        child,
    }
}

impl Env {
    /// Inserts a row that is old enough to leave the batch window at `now`.
    async fn row(
        &self,
        who: MemberId,
        trigger: NotifyTrigger,
        title: &str,
        priority: i32,
        now: DateTime<Utc>,
    ) -> NotificationId {
        let key = format!("t:{title}:{who}");
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        NotificationRepo::insert_dedupe(
            &mut tx,
            &NewNotification {
                household_id: self.h,
                member_id: who,
                trigger,
                subject_type: "task".into(),
                subject_id: None,
                title: title.into(),
                body: format!("{title} body"),
                click_path: Some("/tasks/1".into()),
                priority,
                dedupe_key: key.clone(),
                not_before: None,
            },
        )
        .await
        .unwrap();
        let id: Uuid = sqlx::query_scalar("SELECT id FROM notification WHERE dedupe_key = $1")
            .bind(&key)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE notification SET created_at = $2, not_before = $2 WHERE id = $1")
            .bind(id)
            .bind(now - Duration::minutes(5))
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        id.into()
    }

    async fn get(&self, id: NotificationId) -> Notification {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        NotificationRepo::get(&mut tx, id).await.unwrap().unwrap()
    }

    fn sent(&self) -> Vec<(String, NtfyMessage)> {
        self.fake.sent.lock().unwrap().clone()
    }
}

const URL: &str = "http://ntfy.example.com";

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn disabled_trigger_and_missing_pref_are_dropped(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let now = utc(26, 12, 0);
    e.svc.get_prefs(e.h, e.adult).await.unwrap();
    let a = e.row(e.adult, NotifyTrigger::DueSoon, "soon", 3, now).await;
    let b = e
        .row(e.child, NotifyTrigger::Assigned, "nopref", 3, now)
        .await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(a).await.status, NotificationStatus::Dropped);
    assert_eq!(e.get(b).await.status, NotificationStatus::Dropped);
    assert!(e.sent().is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn quiet_hours_defer_household_and_override(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    e.svc.get_prefs(e.h, e.adult).await.unwrap();
    // 22:00 UTC is 23:00 CET: inside the household window 21:00-07:00.
    let now = utc(26, 22, 0);
    let a = e
        .row(e.adult, NotifyTrigger::Assigned, "late", 3, now)
        .await;
    e.svc.run_send_at(now).await.unwrap();
    let r = e.get(a).await;
    assert_eq!(r.status, NotificationStatus::Pending);
    assert_eq!(r.not_before, utc(27, 6, 0));
    assert!(e.sent().is_empty());

    // Member override 22:00-06:00 (across midnight). 04:00 UTC = 05:00 CET is inside it.
    let v = e.svc.get_prefs(e.h, e.adult).await.unwrap();
    e.svc
        .put_prefs(
            e.h,
            e.adult,
            NotifyPrefPatch {
                quiet_from: Some(Some(hm(22, 0))),
                quiet_until: Some(Some(hm(6, 0))),
                ..empty()
            },
            v.version,
        )
        .await
        .unwrap();
    let now = utc(27, 4, 0);
    let b = e
        .row(e.adult, NotifyTrigger::Assigned, "early", 3, now)
        .await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(b).await.not_before, utc(27, 5, 0));
    // 20:45 UTC = 21:45 CET: household-quiet, but outside the override, so it goes out.
    let now = utc(27, 20, 45);
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(a).await.status, NotificationStatus::Sent);
    assert_eq!(e.get(b).await.status, NotificationStatus::Sent);
    assert_eq!(e.sent().len(), 1, "both rows batched into one message");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn batching_merges_and_respects_window(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let now = utc(26, 12, 0);
    let topic = e.svc.get_prefs(e.h, e.adult).await.unwrap().topic.unwrap();
    let ids = [
        e.row(e.adult, NotifyTrigger::Assigned, "one", 3, now).await,
        e.row(e.adult, NotifyTrigger::Overdue, "two", 4, now).await,
        e.row(e.adult, NotifyTrigger::Digest, "three", 2, now).await,
    ];
    // Created just now: stays pending.
    let young = e
        .row(e.adult, NotifyTrigger::Assigned, "young", 3, now)
        .await;
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        sqlx::query("UPDATE notification SET created_at = $2 WHERE id = $1")
            .bind(young.as_uuid())
            .bind(now - Duration::seconds(10))
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    e.svc.run_send_at(now).await.unwrap();
    let sent = e.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, topic);
    let m = &sent[0].1;
    assert_eq!(m.title, "3 updates");
    assert_eq!(m.body, "one\ntwo\nthree");
    assert_eq!(m.priority, 4);
    assert_eq!(m.click.as_deref(), Some("https://fa.example.com/tasks"));
    for id in ids {
        assert_eq!(e.get(id).await.status, NotificationStatus::Sent);
    }
    assert_eq!(e.get(young).await.status, NotificationStatus::Pending);

    // A single row keeps its own content, click link and trigger tag.
    e.svc.run_send_at(now + Duration::minutes(2)).await.unwrap();
    let sent = e.sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].1.title, "young");
    assert_eq!(sent[1].1.body, "young body");
    assert_eq!(
        sent[1].1.click.as_deref(),
        Some("https://fa.example.com/tasks/1")
    );
    assert_eq!(sent[1].1.tags, vec!["memo".to_string()]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn failures_back_off_and_give_up(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let mut now = utc(26, 12, 0);
    e.svc.get_prefs(e.h, e.adult).await.unwrap();
    let id = e.row(e.adult, NotifyTrigger::Assigned, "x", 3, now).await;
    *e.fake.fail.lock().unwrap() = true;
    for attempt in 1..=5 {
        e.svc.run_send_at(now).await.unwrap();
        let r = e.get(id).await;
        assert_eq!(r.attempts, attempt);
        assert_eq!(r.last_error.as_deref(), Some("publish failed: boom"));
        if attempt < 5 {
            assert_eq!(r.status, NotificationStatus::Pending);
            assert_eq!(r.not_before, now + Duration::minutes(1 << attempt));
            now = r.not_before;
        } else {
            assert_eq!(r.status, NotificationStatus::Failed);
        }
    }
    // Not retried once failed.
    *e.fake.fail.lock().unwrap() = false;
    e.svc.run_send_at(now + Duration::hours(1)).await.unwrap();
    assert!(e.sent().is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn child_copies_go_to_the_adult_and_follow_their_quiet_hours(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let now = utc(26, 12, 0);
    let c = e.svc.get_prefs(e.h, e.child).await.unwrap();
    e.svc.get_prefs(e.h, e.adult).await.unwrap();
    // The adult must be a valid copy target; a child is rejected.
    let bad = e
        .svc
        .put_prefs(
            e.h,
            e.child,
            NotifyPrefPatch {
                copy_to_member: Some(Some(e.child)),
                ..empty()
            },
            c.version,
        )
        .await;
    assert!(matches!(bad, Err(NotifyError::Invalid(_))));
    e.svc
        .put_prefs(
            e.h,
            e.child,
            NotifyPrefPatch {
                copy_to_member: Some(Some(e.adult)),
                ..empty()
            },
            c.version,
        )
        .await
        .unwrap();
    let r = e
        .row(e.child, NotifyTrigger::Assigned, "Tidy room", 3, now)
        .await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(r).await.status, NotificationStatus::Sent);
    assert_eq!(e.sent().len(), 1);

    // The copy exists, prefixed with the child's name, and is not sent in the same pass.
    let copy_key = format!("copy:{r}");
    let (id, title, member): (Uuid, String, Uuid) = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        sqlx::query_as("SELECT id, title, member_id FROM notification WHERE dedupe_key = $1")
            .bind(&copy_key)
            .fetch_one(&mut *tx)
            .await
            .unwrap()
    };
    assert_eq!(title, "Sam: Tidy room");
    assert_eq!(member, e.adult.as_uuid());
    let id: NotificationId = id.into();
    // The adult's own quiet hours apply: 22:30 UTC is 23:30 CET.
    let late = utc(26, 22, 30);
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        sqlx::query("UPDATE notification SET created_at = $2, not_before = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(late - Duration::minutes(5))
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    e.svc.run_send_at(late).await.unwrap();
    assert_eq!(e.get(id).await.not_before, utc(27, 6, 0));
    assert_eq!(e.sent().len(), 1);
    e.svc.run_send_at(utc(27, 6, 0)).await.unwrap();
    let sent = e.sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].1.title, "Sam: Tidy room");
    // The copy is not copied again (no chain).
    assert_eq!(e.get(id).await.status, NotificationStatus::Sent);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn not_configured_drops_rows_and_test_errors(pool: PgPool) {
    let e = env(&pool, None).await;
    let now = utc(26, 12, 0);
    let v = e.svc.get_prefs(e.h, e.adult).await.unwrap();
    assert!(!v.server_configured);
    assert!(v.subscribe_url.is_none());
    let id = e.row(e.adult, NotifyTrigger::Assigned, "x", 3, now).await;
    e.svc.run_send_at(now).await.unwrap();
    let r = e.get(id).await;
    assert_eq!(r.status, NotificationStatus::Dropped);
    assert_eq!(r.last_error.as_deref(), Some("not configured"));
    assert!(matches!(
        e.svc.send_test(e.h, e.adult).await,
        Err(NotifyError::NotConfigured)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn reauth_rows_for_every_adult_once(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let second = member(&e.store, e.h, "Jo", Role::Adult).await;
    let now = utc(26, 12, 0);
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let src = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO calendar_source (id, household_id, kind, name, state)
             VALUES ($1, $2, 'google', 'Example', 'needs_reauth')",
        )
        .bind(src)
        .bind(e.h.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    e.svc.run_send_at(now).await.unwrap();
    e.svc.run_send_at(now).await.unwrap();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notification WHERE trigger = 'source_reauth' AND member_id = ANY($1)",
    )
    .bind([e.adult.as_uuid(), second.as_uuid()])
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(n, 2);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn prefs_topic_is_stable_and_validated(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let a = e.svc.get_prefs(e.h, e.adult).await.unwrap();
    let topic = a.topic.clone().unwrap();
    assert_eq!(topic.len(), 24);
    assert!(
        topic
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    );
    assert_eq!(
        a.subscribe_url.as_deref(),
        Some(format!("{URL}/{topic}").as_str())
    );
    assert!(a.server_configured);
    assert_eq!(
        a.enabled_triggers,
        vec![
            NotifyTrigger::Assigned,
            NotifyTrigger::Overdue,
            NotifyTrigger::Digest,
            NotifyTrigger::ApprovalNeeded,
            NotifyTrigger::MailTimeSensitive
        ]
    );
    assert_eq!(a.approval_mode, ApprovalMode::Batched);
    let c = e.svc.get_prefs(e.h, e.child).await.unwrap();
    assert_eq!(
        c.enabled_triggers,
        vec![
            NotifyTrigger::Assigned,
            NotifyTrigger::Overdue,
            NotifyTrigger::Digest
        ]
    );
    assert_eq!((a.due_soon_minutes, a.nudge_minutes), (120, 10));
    let b = e.svc.get_prefs(e.h, e.adult).await.unwrap();
    assert_eq!(b.topic, a.topic);
    assert_eq!(b.version, a.version);

    let patch = |f: fn(&mut NotifyPrefPatch)| {
        let mut p = empty();
        f(&mut p);
        p
    };
    for p in [
        patch(|p| p.due_soon_minutes = Some(4)),
        patch(|p| p.due_soon_minutes = Some(1441)),
        patch(|p| p.nudge_minutes = Some(0)),
        patch(|p| p.nudge_minutes = Some(121)),
        patch(|p| p.quiet_from = Some(Some(hm(22, 0)))),
    ] {
        assert!(matches!(
            e.svc.put_prefs(e.h, e.adult, p, a.version).await,
            Err(NotifyError::Invalid(_))
        ));
    }
    let ok = e
        .svc
        .put_prefs(
            e.h,
            e.adult,
            patch(|p| {
                p.due_soon_minutes = Some(5);
                p.nudge_minutes = Some(120);
            }),
            a.version,
        )
        .await
        .unwrap();
    assert_eq!(ok.version, a.version + 1);
    assert_eq!(ok.topic, a.topic);
    // Stale version.
    assert!(matches!(
        e.svc.put_prefs(e.h, e.adult, empty(), a.version).await,
        Err(NotifyError::Store(StoreError::Conflict))
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn real_client_sends_headers_and_token(pool: PgPool) {
    let server = MockServer::start().await;
    let store = Store::from_pool(pool.clone());
    let e = env(&pool, Some(&server.uri())).await;
    let svc = NotifyService::new(
        store,
        NotifyConfig {
            ntfy_url: Some(server.uri()),
            ntfy_token: Some("tk_secret".into()),
            public_base_url: "https://fa.example.com".into(),
            subscribe_base: None,
        },
    );
    let topic = svc.get_prefs(e.h, e.adult).await.unwrap().topic.unwrap();
    Mock::given(method("POST"))
        .and(path(format!("/{topic}")))
        .and(header("Authorization", "Bearer tk_secret"))
        .and(header("Title", "Alert"))
        .and(header("Priority", "4"))
        .and(header("Tags", "warning"))
        .and(header("Click", "https://fa.example.com/tasks/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "m1"})))
        .expect(1)
        .mount(&server)
        .await;
    let now = utc(26, 12, 0);
    let id = e
        .row(e.adult, NotifyTrigger::Overdue, "Alert", 4, now)
        .await;
    svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(id).await.status, NotificationStatus::Sent);

    // The test message bypasses the outbox and quiet hours, and publish errors surface.
    let test = Mock::given(method("POST"))
        .and(header("Title", "Test notification"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "m2"})))
        .expect(1)
        .mount_as_scoped(&server)
        .await;
    svc.send_test(e.h, e.adult).await.unwrap();
    drop(test);
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert!(matches!(
        svc.send_test(e.h, e.adult).await,
        Err(NotifyError::Publish(_))
    ));
}

fn empty() -> NotifyPrefPatch {
    NotifyPrefPatch {
        enabled_triggers: None,
        due_soon_minutes: None,
        nudge_minutes: None,
        quiet_from: None,
        quiet_until: None,
        copy_to_member: None,
        approval_mode: None,
    }
}

async fn set_mode(e: &Env, m: ApprovalMode) {
    let v = e.svc.get_prefs(e.h, e.adult).await.unwrap();
    let mut p = empty();
    p.approval_mode = Some(m);
    let out = e.svc.put_prefs(e.h, e.adult, p, v.version).await.unwrap();
    assert_eq!(out.approval_mode, m);
    assert_eq!(
        e.svc.get_prefs(e.h, e.adult).await.unwrap().approval_mode,
        m
    );
}

async fn age(e: &Env, id: NotificationId, created: DateTime<Utc>) {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    sqlx::query("UPDATE notification SET created_at = $2 WHERE id = $1")
        .bind(id.as_uuid())
        .bind(created)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn drop_reason(e: &Env, id: NotificationId) -> Option<String> {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    sqlx::query_scalar("SELECT last_error FROM notification WHERE id = $1")
        .bind(id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn approval_modes(pool: PgPool) {
    let e = env(&pool, Some(URL)).await;
    let now = utc(26, 12, 0);
    let young = now - Duration::seconds(10);
    e.svc.get_prefs(e.h, e.adult).await.unwrap();

    // batched: waits for the window.
    let a = e
        .row(e.adult, NotifyTrigger::ApprovalNeeded, "appr1", 3, now)
        .await;
    age(&e, a, young).await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(a).await.status, NotificationStatus::Pending);
    assert!(e.sent().is_empty());

    // immediate: sent on the next pass, together with a due row.
    set_mode(&e, ApprovalMode::Immediate).await;
    let due = e.row(e.adult, NotifyTrigger::Assigned, "due", 3, now).await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(a).await.status, NotificationStatus::Sent);
    assert_eq!(e.get(due).await.status, NotificationStatus::Sent);
    assert_eq!(e.sent().len(), 1);

    // a young non-approval row still waits in immediate mode.
    let y = e
        .row(e.adult, NotifyTrigger::Assigned, "young", 3, now)
        .await;
    age(&e, y, young).await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(y).await.status, NotificationStatus::Pending);

    // digest and off drop with their reason.
    for (mode, reason) in [(ApprovalMode::Digest, "digest"), (ApprovalMode::Off, "off")] {
        set_mode(&e, mode).await;
        let r = e
            .row(e.adult, NotifyTrigger::ApprovalNeeded, reason, 3, now)
            .await;
        e.svc.run_send_at(now).await.unwrap();
        assert_eq!(e.get(r).await.status, NotificationStatus::Dropped);
        assert_eq!(drop_reason(&e, r).await.as_deref(), Some(reason));
    }

    // other triggers are unaffected by the mode.
    let o = e
        .row(e.adult, NotifyTrigger::MailTimeSensitive, "mail", 3, now)
        .await;
    e.svc.run_send_at(now).await.unwrap();
    assert_eq!(e.get(o).await.status, NotificationStatus::Sent);
}
