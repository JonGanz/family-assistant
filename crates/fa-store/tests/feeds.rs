use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use fa_domain::api::{CalendarPatch, HouseholdPatch, SourcePatch};
use fa_domain::*;
use fa_store::*;
use sqlx::postgres::PgListener;
use sqlx::{Executor, PgPool};

fn household(name: &str) -> Household {
    Household {
        id: HouseholdId::new(),
        name: name.into(),
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
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
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

async fn seed(store: &Store, name: &str) -> (HouseholdId, MemberId) {
    let h = household(name);
    let m = member(h.id, &format!("{name} adult"), Role::Adult);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    tx.commit().await.unwrap();
    (h.id, m.id)
}

fn source(h: HouseholdId, owner: Option<MemberId>, poll: i32) -> CalendarSource {
    CalendarSource {
        id: CalendarSourceId::new(),
        household_id: h,
        kind: SourceKind::Ics,
        name: "Feed".into(),
        owner_member: owner,
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: false,
        poll_minutes: poll,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 26, h, 0, 0).unwrap()
}

fn occ(uid: &str, key: &str, title: &str, start: u32, end: u32) -> NewOccurrence {
    NewOccurrence {
        remote_uid: uid.into(),
        instance_key: key.into(),
        title: title.into(),
        description: None,
        location: None,
        start_at: t(start),
        end_at: t(end),
        all_day: false,
        start_date: None,
        end_date: None,
        source_tz: None,
        is_private: false,
        cancelled: false,
        remote_etag: None,
        external_url: None,
    }
}

/// Source with one enabled calendar, committed.
async fn seed_calendar(
    store: &Store,
    h: HouseholdId,
    owner: Option<MemberId>,
) -> (CalendarSourceId, CalendarId) {
    let s = source(h, owner, 15);
    let mut tx = store.tx_for(h).await.unwrap();
    CalendarSourceRepo::insert(&mut tx, &s).await.unwrap();
    let (cal, created) = CalendarRepo::upsert_discovered(&mut tx, s.id, "remote-1", "Family")
        .await
        .unwrap();
    assert!(created);
    assert!(!cal.value.enabled, "discovered calendars start disabled");
    let v = CalendarRepo::update(
        &mut tx,
        cal.value.id,
        &CalendarPatch {
            enabled: Some(true),
            ..Default::default()
        },
        cal.version,
    )
    .await
    .unwrap();
    assert_eq!(v, cal.version + 1);
    tx.commit().await.unwrap();
    (s.id, cal.value.id)
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_reverts_and_reapplies(pool: PgPool) {
    let down = include_str!("../migrations/20261025000001_m1_feeds.down.sql");
    let up = include_str!("../migrations/20261025000001_m1_feeds.up.sql");
    // M2 depends on M1 tables: revert it first and reapply it last.
    let down2 = include_str!("../migrations/20261026000001_m2_tasks.down.sql");
    let up2 = include_str!("../migrations/20261026000001_m2_tasks.up.sql");
    let down3 = include_str!("../migrations/20261027000001_m3_ingest.down.sql");
    let up3 = include_str!("../migrations/20261027000001_m3_ingest.up.sql");
    pool.execute(down3).await.unwrap();
    pool.execute(down2).await.unwrap();
    pool.execute(down).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.tables WHERE table_name IN ('event','advisory','calendar')")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    pool.execute(up).await.unwrap();
    pool.execute(up2).await.unwrap();
    pool.execute(up3).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.tables WHERE table_name IN ('event','advisory','calendar')")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn rls_isolates_new_tables(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (a, ma) = seed(&store, "A").await;
    let (b, _mb) = seed(&store, "B").await;
    let (src, cal) = seed_calendar(&store, a, Some(ma)).await;

    let mut tx = store.tx_for(a).await.unwrap();
    MemberAvatarRepo::put(&mut tx, ma, "image/png", b"png", "abc")
        .await
        .unwrap();
    OAuthAccountRepo::upsert(
        &mut tx,
        a,
        "google",
        ma,
        "x@example.com",
        &["calendar.readonly".into()],
        None,
    )
    .await
    .unwrap();
    OAuthPendingRepo::insert(&mut tx, "st1", "sess", ma, "ver", None, t(12))
        .await
        .unwrap();
    CalendarRepo::set_members(&mut tx, cal, &[ma])
        .await
        .unwrap();
    EventRepo::upsert_synced(&mut tx, cal, &[occ("u1", "", "Event", 9, 10)])
        .await
        .unwrap();
    WeatherSnapshotRepo::insert(
        &mut tx,
        "open-meteo",
        &serde_json::json!({}),
        &serde_json::json!([]),
        &serde_json::json!([]),
    )
    .await
    .unwrap();
    assert!(
        WeatherRuleRepo::ensure_default(
            &mut tx,
            "Frost",
            "min(temp_c, tonight) <= 0",
            RuleAction::Alert,
            Severity::Warning,
            "Frost"
        )
        .await
        .unwrap()
    );
    AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "x",
        Severity::Info,
        t(0),
        t(23),
        AdvisorySource::Rule,
        "k",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Household B sees none of it; a raw count per table as fa_app.
    let mut tx = store.tx_for(b).await.unwrap();
    for table in [
        "member_avatar",
        "oauth_account",
        "oauth_pending",
        "calendar_source",
        "calendar",
        "calendar_member",
        "event",
        "weather_snapshot",
        "weather_rule",
        "advisory",
    ] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(n, 0, "{table} leaked");
    }
    assert!(
        CalendarSourceRepo::get(&mut tx, src)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        WeatherSnapshotRepo::latest(&mut tx)
            .await
            .unwrap()
            .is_none()
    );
    // Writing a row for another household is rejected.
    let r = sqlx::query("INSERT INTO weather_snapshot (id, household_id, provider, \"current\", hourly, daily) VALUES (gen_random_uuid(), $1, 'x', '{}', '[]', '[]')")
        .bind(a.as_uuid())
        .execute(&mut *tx)
        .await;
    assert!(r.is_err());
    tx.rollback().await.unwrap();

    // A sees all of it.
    let mut tx = store.tx_for(a).await.unwrap();
    assert!(
        WeatherSnapshotRepo::latest(&mut tx)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        EventRepo::range(&mut tx, t(0), t(23), None)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn event_upsert_idempotent_and_window_delete(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, ma) = seed(&store, "A").await;
    let (_, cal) = seed_calendar(&store, h, Some(ma)).await;
    let (_, other) = seed_calendar(&store, h, Some(ma)).await;

    let evs = vec![
        occ("u1", "", "One", 9, 10),
        occ("u2", "2026-10-26T11:00:00Z", "Two", 11, 12),
        occ("u3", "", "Three", 13, 14),
    ];
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(
        EventRepo::upsert_synced(&mut tx, cal, &evs).await.unwrap(),
        3
    );
    assert_eq!(
        EventRepo::upsert_synced(&mut tx, cal, &evs).await.unwrap(),
        0,
        "idempotent"
    );
    EventRepo::upsert_synced(&mut tx, other, &[occ("u1", "", "Other cal", 9, 10)])
        .await
        .unwrap();

    let mut changed = evs.clone();
    changed[0].title = "One (moved)".into();
    assert_eq!(
        EventRepo::upsert_synced(&mut tx, cal, &changed)
            .await
            .unwrap(),
        1
    );
    let v: i32 = sqlx::query_scalar(
        "SELECT version FROM event WHERE calendar_id = $1 AND remote_uid = 'u1'",
    )
    .bind(cal.as_uuid())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(v, 2);

    // Window [10:30, 12:30) overlaps only "Two" (11-12); "u2" unseen -> deleted. Other rows and the other calendar stay.
    let n = EventRepo::delete_unseen_in_window(
        &mut tx,
        cal,
        t(10) + chrono::Duration::minutes(30),
        t(12) + chrono::Duration::minutes(30),
        &[("u1".into(), "".into())],
    )
    .await
    .unwrap();
    assert_eq!(n, 1);
    let titles: Vec<String> = EventRepo::range(&mut tx, t(0), t(23), None)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.event.title)
        .collect();
    assert_eq!(titles, vec!["One (moved)", "Other cal", "Three"]);

    // Seen rows inside the window survive.
    let n = EventRepo::delete_unseen_in_window(
        &mut tx,
        cal,
        t(0),
        t(23),
        &[("u1".into(), "".into()), ("u3".into(), "".into())],
    )
    .await
    .unwrap();
    assert_eq!(n, 0);

    assert_eq!(
        EventRepo::delete_by_uid(&mut tx, cal, "u3", Some(""))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        EventRepo::delete_by_uid(&mut tx, cal, "u1", None)
            .await
            .unwrap(),
        1
    );
    assert_eq!(EventRepo::prune_before(&mut tx, t(23)).await.unwrap(), 1);
    assert_eq!(
        EventRepo::delete_calendar_events(&mut tx, other)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn range_returns_members_and_owner_role(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, adult) = seed(&store, "A").await;
    let kid = member(h, "Kid", Role::Child);
    let mut tx = store.tx_for(h).await.unwrap();
    MemberRepo::insert(&mut tx, &kid, None, None).await.unwrap();
    tx.commit().await.unwrap();
    let (src, cal) = seed_calendar(&store, h, Some(kid.id)).await;
    let (_, household_cal) = seed_calendar(&store, h, None).await;

    let mut tx = store.tx_for(h).await.unwrap();
    CalendarRepo::set_members(&mut tx, cal, &[adult, kid.id])
        .await
        .unwrap();
    assert_eq!(CalendarRepo::members(&mut tx, cal).await.unwrap().len(), 2);
    let mut private = occ("p1", "", "Secret", 9, 10);
    private.is_private = true;
    EventRepo::upsert_synced(&mut tx, cal, &[private])
        .await
        .unwrap();
    EventRepo::upsert_synced(&mut tx, household_cal, &[occ("h1", "", "Household", 9, 10)])
        .await
        .unwrap();

    let rows = EventRepo::range(&mut tx, t(0), t(23), None).await.unwrap();
    let a = rows.iter().find(|r| r.event.title == "Secret").unwrap();
    assert!(a.event.is_private);
    assert_eq!(a.owner_role, Some(Role::Child));
    let mut roles: Vec<Role> = a.members.iter().map(|m| m.1).collect();
    roles.sort_by_key(|r| *r == Role::Child);
    assert_eq!(roles, vec![Role::Adult, Role::Child]);
    let b = rows.iter().find(|r| r.event.title == "Household").unwrap();
    assert!(b.members.is_empty());
    assert_eq!(b.owner_role, None);

    // Calendar filter, disabled calendar and archived source are excluded.
    let only = EventRepo::range(&mut tx, t(0), t(23), Some(&[household_cal]))
        .await
        .unwrap();
    assert_eq!(only.len(), 1);
    CalendarSourceRepo::archive(&mut tx, src).await.unwrap();
    assert_eq!(
        EventRepo::range(&mut tx, t(0), t(23), None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        CalendarSourceRepo::owner_role(&mut tx, src).await.unwrap(),
        Some(Role::Child)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn due_respects_poll_minutes_and_archive(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h1, _) = seed(&store, "A").await;
    let (h2, _) = seed(&store, "B").await;
    let now = Utc::now();
    let mut ids = vec![];
    for (h, poll) in [(h1, 15), (h1, 15), (h1, 60), (h2, 15)] {
        let s = source(h, None, poll);
        let mut tx = store.tx_for(h).await.unwrap();
        CalendarSourceRepo::insert(&mut tx, &s).await.unwrap();
        tx.commit().await.unwrap();
        ids.push((h, s.id));
    }
    let mut tx = store.tx_system().await.unwrap();
    // [0] synced 10 min ago (poll 15: not due); [1] 20 min ago (due); [2] 20 min ago, poll 60 (not due).
    CalendarSourceRepo::set_sync_result(
        &mut tx,
        ids[0].1,
        SourceState::Ok,
        None,
        Some(now - chrono::Duration::minutes(10)),
    )
    .await
    .unwrap();
    CalendarSourceRepo::set_sync_result(
        &mut tx,
        ids[1].1,
        SourceState::Ok,
        None,
        Some(now - chrono::Duration::minutes(20)),
    )
    .await
    .unwrap();
    CalendarSourceRepo::set_sync_result(
        &mut tx,
        ids[2].1,
        SourceState::Error,
        Some("boom"),
        Some(now - chrono::Duration::minutes(20)),
    )
    .await
    .unwrap();
    let due = CalendarSourceRepo::due(&mut tx, now).await.unwrap();
    assert_eq!(due.len(), 2);
    assert!(due.contains(&ids[1]) && due.contains(&ids[3]), "{due:?}");
    CalendarSourceRepo::archive(&mut tx, ids[3].1)
        .await
        .unwrap();
    CalendarSourceRepo::set_enabled(&mut tx, ids[1].1, false)
        .await
        .unwrap();
    assert!(
        CalendarSourceRepo::due(&mut tx, now)
            .await
            .unwrap()
            .is_empty()
    );
    CalendarSourceRepo::set_enabled(&mut tx, ids[1].1, true)
        .await
        .unwrap();
    assert_eq!(
        CalendarSourceRepo::due(&mut tx, now + chrono::Duration::minutes(30))
            .await
            .unwrap()
            .len(),
        2
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn calendars_discovery_archive_and_conflicts(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, _) = seed(&store, "A").await;
    let s = source(h, None, 15);
    let mut tx = store.tx_for(h).await.unwrap();
    let src = CalendarSourceRepo::insert(&mut tx, &s).await.unwrap();
    let (c1, _) = CalendarRepo::upsert_discovered(&mut tx, s.id, "a", "A")
        .await
        .unwrap();
    let (c1b, created) = CalendarRepo::upsert_discovered(&mut tx, s.id, "a", "A renamed")
        .await
        .unwrap();
    assert!(!created);
    assert_eq!(c1b.value.id, c1.value.id);
    assert_eq!(c1b.value.name, "A renamed");
    CalendarRepo::upsert_discovered(&mut tx, s.id, "b", "B")
        .await
        .unwrap();
    let archived = CalendarRepo::archive_missing(&mut tx, s.id, &["a".to_string()])
        .await
        .unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(
        CalendarRepo::list(&mut tx, Some(s.id)).await.unwrap().len(),
        1
    );
    CalendarRepo::set_sync_state(&mut tx, c1.value.id, Some("tok"), Some(t(1)))
        .await
        .unwrap();
    let c = CalendarRepo::get(&mut tx, c1.value.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.value.sync_state.as_deref(), Some("tok"));

    // Version conflicts.
    let patch = SourcePatch {
        name: Some("Renamed".into()),
        ..Default::default()
    };
    let v = CalendarSourceRepo::update(&mut tx, s.id, &patch, src.version)
        .await
        .unwrap();
    assert_eq!(v, src.version + 1);
    assert!(matches!(
        CalendarSourceRepo::update(&mut tx, s.id, &patch, src.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        CalendarRepo::update(&mut tx, c1.value.id, &CalendarPatch::default(), 99).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        CalendarRepo::update(&mut tx, CalendarId::new(), &CalendarPatch::default(), 1).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn advisories_dedupe_and_no_resurrection(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, _ma) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let (id, changed) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Frost",
        Severity::Warning,
        t(0),
        t(20),
        AdvisorySource::Rule,
        "rule:1:d",
    )
    .await
    .unwrap();
    assert!(changed);
    let (id2, changed) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Frost",
        Severity::Warning,
        t(0),
        t(20),
        AdvisorySource::Rule,
        "rule:1:d",
    )
    .await
    .unwrap();
    assert_eq!(id, id2);
    assert!(!changed);
    let (_, changed) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Hard frost",
        Severity::Severe,
        t(0),
        t(20),
        AdvisorySource::Rule,
        "rule:1:d",
    )
    .await
    .unwrap();
    assert!(changed);
    assert_eq!(AdvisoryRepo::active(&mut tx, t(5)).await.unwrap().len(), 1);
    assert!(
        AdvisoryRepo::active(&mut tx, t(21))
            .await
            .unwrap()
            .is_empty()
    );

    let (s2, _) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Model note",
        Severity::Info,
        t(0),
        t(20),
        AdvisorySource::S2,
        "s2:1",
    )
    .await
    .unwrap();
    let (other, _) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Wind",
        Severity::Info,
        t(0),
        t(20),
        AdvisorySource::Rule,
        "rule:2:d",
    )
    .await
    .unwrap();
    // keeps the listed rule key and every s2 advisory, drops the other rule advisory
    let removed = AdvisoryRepo::retain_rule_advisories(&mut tx, &["rule:1:d".to_string()])
        .await
        .unwrap();
    assert_eq!(removed, 1);
    let ids: Vec<_> = AdvisoryRepo::active(&mut tx, t(5))
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.id)
        .collect();
    assert!(ids.contains(&id) && ids.contains(&s2) && !ids.contains(&other));
    // an empty keep list removes every rule advisory
    assert_eq!(
        AdvisoryRepo::retain_rule_advisories(&mut tx, &[])
            .await
            .unwrap(),
        1
    );
    assert_eq!(AdvisoryRepo::active(&mut tx, t(5)).await.unwrap().len(), 1);
    let (id3, changed) = AdvisoryRepo::upsert_by_dedupe(
        &mut tx,
        None,
        "Frost again",
        Severity::Info,
        t(0),
        t(22),
        AdvisorySource::Rule,
        "rule:1:d",
    )
    .await
    .unwrap();
    assert!(changed);
    let _ = id3;
    assert_eq!(
        AdvisoryRepo::delete_expired(&mut tx, t(19)).await.unwrap(),
        0
    );
    assert_eq!(
        AdvisoryRepo::delete_expired(&mut tx, t(23) + chrono::Duration::days(1))
            .await
            .unwrap(),
        2
    );
}

async fn ensure(tx: &mut Tx) -> Result<bool> {
    WeatherRuleRepo::ensure_default(
        tx,
        "Frost tonight",
        "min(temp_c, tonight) <= 0",
        RuleAction::Alert,
        Severity::Warning,
        "Frost",
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
async fn weather_rules_and_snapshots(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, _) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    assert!(ensure(&mut tx).await.unwrap());
    assert!(!ensure(&mut tx).await.unwrap());
    let rules = WeatherRuleRepo::list(&mut tx, None).await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].value.suggested_by, SuggestedBy::Default);
    // An archived default is not re-seeded.
    WeatherRuleRepo::archive(&mut tx, rules[0].value.id)
        .await
        .unwrap();
    assert!(!ensure(&mut tx).await.unwrap());
    assert!(
        WeatherRuleRepo::list(&mut tx, None)
            .await
            .unwrap()
            .is_empty()
    );

    let mut r = rules[0].value.clone();
    r.archived_at = None;
    r.status = RuleStatus::Disabled;
    let cur = WeatherRuleRepo::get(&mut tx, r.id).await.unwrap().unwrap();
    let v = WeatherRuleRepo::update(&mut tx, &r, cur.version)
        .await
        .unwrap();
    assert_eq!(v, cur.version + 1);
    assert!(matches!(
        WeatherRuleRepo::update(&mut tx, &r, cur.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(
        WeatherRuleRepo::list(&mut tx, Some(&[RuleStatus::Active]))
            .await
            .unwrap()
            .is_empty()
    );

    WeatherSnapshotRepo::insert(
        &mut tx,
        "open-meteo",
        &serde_json::json!({"a": 1}),
        &serde_json::json!([]),
        &serde_json::json!([]),
    )
    .await
    .unwrap();
    WeatherSnapshotRepo::insert(
        &mut tx,
        "open-meteo",
        &serde_json::json!({"a": 2}),
        &serde_json::json!([]),
        &serde_json::json!([]),
    )
    .await
    .unwrap();
    let latest = WeatherSnapshotRepo::latest(&mut tx).await.unwrap().unwrap();
    assert_eq!(latest.current["a"], 2);
    assert_eq!(
        WeatherSnapshotRepo::delete_older_than(&mut tx, Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap(),
        2
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn oauth_pending_is_single_use_and_accounts_upsert(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, ma) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    OAuthPendingRepo::insert(
        &mut tx,
        "state-1",
        "sess",
        ma,
        "verifier",
        Some("/settings"),
        Utc::now() + chrono::Duration::minutes(10),
    )
    .await
    .unwrap();
    OAuthPendingRepo::insert(
        &mut tx,
        "state-2",
        "sess",
        ma,
        "verifier",
        None,
        Utc::now() - chrono::Duration::minutes(1),
    )
    .await
    .unwrap();
    let p = OAuthPendingRepo::take(&mut tx, "state-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.pkce_verifier, "verifier");
    assert_eq!(p.return_to.as_deref(), Some("/settings"));
    assert_eq!(p.household_id, h);
    assert!(
        OAuthPendingRepo::take(&mut tx, "state-1")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(OAuthPendingRepo::delete_expired(&mut tx).await.unwrap(), 1);

    let a = OAuthAccountRepo::upsert(
        &mut tx,
        h,
        "google",
        ma,
        "x@example.com",
        &["s1".into()],
        None,
    )
    .await
    .unwrap();
    OAuthAccountRepo::set_state(
        &mut tx,
        a.value.id,
        SourceState::NeedsReauth,
        Some("invalid_grant"),
    )
    .await
    .unwrap();
    let b = OAuthAccountRepo::upsert(
        &mut tx,
        h,
        "google",
        ma,
        "x@example.com",
        &["s1".into(), "s2".into()],
        None,
    )
    .await
    .unwrap();
    assert_eq!(a.value.id, b.value.id);
    assert_eq!(b.value.state, SourceState::Ok);
    assert_eq!(b.value.scopes.len(), 2);
    assert_eq!(
        OAuthAccountRepo::list(&mut tx, None).await.unwrap().len(),
        1
    );
    assert_eq!(
        OAuthAccountRepo::list(&mut tx, Some(&[MemberId::new()]))
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        OAuthAccountRepo::list(&mut tx, Some(&[ma]))
            .await
            .unwrap()
            .len(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn household_settings_device_settings_avatar_member_jobs(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, ma) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();

    let cur = HouseholdRepo::get(&mut tx, h).await.unwrap().unwrap();
    assert_eq!(cur.value.calendar_future_days, 400);
    let patch = HouseholdPatch {
        units: Some(Units::Imperial),
        calendar_future_days: Some(90),
        private_events_as_busy: Some(false),
        ..Default::default()
    };
    let v = HouseholdRepo::update_settings(&mut tx, h, &patch, cur.version)
        .await
        .unwrap();
    assert!(matches!(
        HouseholdRepo::update_settings(&mut tx, h, &patch, cur.version).await,
        Err(StoreError::Conflict)
    ));
    let now = HouseholdRepo::get(&mut tx, h).await.unwrap().unwrap();
    assert_eq!(now.version, v);
    assert_eq!(now.value.units, Units::Imperial);
    assert_eq!(now.value.calendar_future_days, 90);
    assert_eq!(now.value.calendar_past_days, 30);
    assert!(!now.value.private_events_as_busy);

    let d = Device {
        id: DeviceId::new(),
        household_id: h,
        kind: DeviceKind::Dashboard,
        name: "Hall".into(),
        token_hash: "hash".into(),
        last_seen: None,
        revoked_at: None,
        created_at: Utc::now(),
    };
    let dv = DeviceRepo::insert(&mut tx, &d, Some(ma)).await.unwrap();
    assert_eq!(
        DeviceRepo::get_settings(&mut tx, d.id).await.unwrap(),
        serde_json::json!({})
    );
    let v = DeviceRepo::set_settings(
        &mut tx,
        d.id,
        &serde_json::json!({"theme": "dark"}),
        dv.version,
    )
    .await
    .unwrap();
    assert_eq!(
        DeviceRepo::get_settings(&mut tx, d.id).await.unwrap()["theme"],
        "dark"
    );
    assert!(matches!(
        DeviceRepo::set_settings(&mut tx, d.id, &serde_json::json!({}), dv.version).await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(v, dv.version + 1);

    assert!(MemberAvatarRepo::get(&mut tx, ma).await.unwrap().is_none());
    MemberAvatarRepo::put(&mut tx, ma, "image/png", b"one", "h1")
        .await
        .unwrap();
    MemberAvatarRepo::put(&mut tx, ma, "image/webp", b"two", "h2")
        .await
        .unwrap();
    let (mime, bytes, sha) = MemberAvatarRepo::get(&mut tx, ma).await.unwrap().unwrap();
    assert_eq!(
        (mime.as_str(), bytes.as_slice(), sha.as_str()),
        ("image/webp", &b"two"[..], "h2")
    );
    MemberAvatarRepo::delete(&mut tx, ma).await.unwrap();
    assert!(MemberAvatarRepo::get(&mut tx, ma).await.unwrap().is_none());

    MemberRepo::archive(&mut tx, ma).await.unwrap();
    assert!(MemberRepo::list(&mut tx, false).await.unwrap().is_empty());
    assert_eq!(
        MemberRepo::list_including_archived(&mut tx)
            .await
            .unwrap()
            .len(),
        1
    );
    MemberRepo::unarchive(&mut tx, ma).await.unwrap();
    assert!(
        MemberRepo::get(&mut tx, ma)
            .await
            .unwrap()
            .unwrap()
            .value
            .archived_at
            .is_none()
    );
    tx.commit().await.unwrap();

    let mut tx = store.tx_system().await.unwrap();
    let before = JobRepo::counts(&mut tx).await.unwrap();
    JobRepo::enqueue(&mut tx, &NewJob::new("x")).await.unwrap();
    let after = JobRepo::counts(&mut tx).await.unwrap();
    assert_eq!(after.ready, before.ready + 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn change_bus_delivers_on_commit_only(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let (h, _) = seed(&store, "A").await;
    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen(CHANNEL).await.unwrap();

    let mut tx = store.tx_for(h).await.unwrap();
    ChangeBus::publish(
        &mut tx,
        h,
        "calendar.updated",
        &serde_json::json!({"from": "2026-10-26"}),
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(300), listener.recv())
            .await
            .is_err(),
        "rollback must not notify"
    );

    let mut tx = store.tx_for(h).await.unwrap();
    ChangeBus::publish(
        &mut tx,
        h,
        "calendar.updated",
        &serde_json::json!({"from": "2026-10-26"}),
    )
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(300), listener.recv())
            .await
            .is_err(),
        "not before commit"
    );
    tx.commit().await.unwrap();
    let n = tokio::time::timeout(Duration::from_secs(5), listener.recv())
        .await
        .unwrap()
        .unwrap();
    let msg: ChangeMessage = serde_json::from_str(n.payload()).unwrap();
    assert_eq!(msg.household, h);
    assert_eq!(msg.kind, "calendar.updated");
    assert_eq!(msg.data["from"], "2026-10-26");

    let mut tx = store.tx_for(h).await.unwrap();
    let big = serde_json::json!({"blob": "x".repeat(2000)});
    assert!(matches!(
        ChangeBus::publish(&mut tx, h, "k", &big).await,
        Err(StoreError::PayloadTooLarge(_))
    ));
}
