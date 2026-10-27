use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use fa_domain::Schedule;
use fa_domain::api::HouseholdPatch;
use fa_domain::tasks_api::TaskListPatch;
use fa_domain::*;
use fa_store::*;
use sqlx::{Executor, PgPool};

fn empty_patch() -> TaskListPatch {
    serde_json::from_value(serde_json::json!({})).unwrap()
}

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

fn at(day: u32, h: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, h, min, 0).unwrap()
}

fn tm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn household(name: &str) -> Household {
    Household {
        id: HouseholdId::new(),
        name: name.into(),
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
        quiet_from: tm(21, 0),
        quiet_until: tm(7, 0),
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

struct Seed {
    h: HouseholdId,
    adult: MemberId,
    child: MemberId,
}

async fn seed(store: &Store, name: &str) -> Seed {
    let hh = household(name);
    let adult = member(hh.id, "Ada Example", Role::Adult);
    let child = member(hh.id, "Kit Example", Role::Child);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    MemberRepo::insert(&mut tx, &adult, None, None)
        .await
        .unwrap();
    MemberRepo::insert(&mut tx, &child, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    Seed {
        h: hh.id,
        adult: adult.id,
        child: child.id,
    }
}

fn list(h: HouseholdId, name: &str) -> TaskList {
    TaskList {
        id: TaskListId::new(),
        household_id: h,
        name: name.into(),
        kind: TaskListKind::Chores,
        completion_policy: ListCompletionPolicy::Assignee,
        child_can_add: ChildCanAdd::Own,
        dashboard_add: false,
        calendar_default: CalendarDisplay::UntilDone,
        claim_ttl_minutes: Some(30),
        visible_on_dashboard: true,
        color: None,
        sort: 0,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn definition(h: HouseholdId, l: TaskListId, title: &str) -> TaskDefinition {
    TaskDefinition {
        id: TaskDefinitionId::new(),
        household_id: h,
        list_id: l,
        title: title.into(),
        notes: None,
        steps_template: vec!["one".into(), "two".into()],
        schedule: Schedule::Rrule {
            rrule: "FREQ=WEEKLY;BYDAY=SA".into(),
        },
        start_date: d(10, 3),
        due_time: None,
        lead_days: 1,
        on_missed: OnMissed::AutoSkip,
        strategy: Strategy::Rotation,
        advance_on: AdvanceOn::AnyCompletion,
        skip_advances: true,
        rotation_cursor: 0,
        priority_flag: PriorityFlag::Normal,
        season: None,
        season_condition: None,
        condition_ok: None,
        condition_checked_at: None,
        last_done_at: None,
        next_due_at: Some(at(31, 23, 59)),
        paused: false,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn new_task(
    h: HouseholdId,
    l: TaskListId,
    def: Option<TaskDefinitionId>,
    due: Option<DateTime<Utc>>,
) -> NewTaskRow {
    NewTaskRow {
        id: TaskId::new(),
        household_id: h,
        list_id: l,
        definition_id: def,
        weather_rule_id: None,
        title: "Bins".into(),
        notes: None,
        due_at: due,
        visible_from: None,
        status: TaskStatus::Open,
        origin: if def.is_some() {
            TaskOrigin::System
        } else {
            TaskOrigin::Manual
        },
        source_ref: None,
        priority_flag: PriorityFlag::Normal,
        calendar_display: CalendarDisplay::Hidden,
        created_by: None,
    }
}

fn routine(h: HouseholdId, name: &str) -> Routine {
    Routine {
        id: RoutineId::new(),
        household_id: h,
        name: name.into(),
        starts_at: tm(7, 0),
        done_by: tm(8, 0),
        day_filter: DayFilter::Weekdays,
        school_calendar_id: None,
        no_school_match: vec!["holiday".into()],
        show_on_dashboard: true,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn spec(text: &str) -> RoutineStepSpec {
    RoutineStepSpec {
        text: text.into(),
        target_time: None,
    }
}

fn notification(s: &Seed, key: &str, not_before: Option<DateTime<Utc>>) -> NewNotification {
    NewNotification {
        household_id: s.h,
        member_id: s.child,
        trigger: NotifyTrigger::Assigned,
        subject_type: "task".into(),
        subject_id: None,
        title: key.into(),
        body: String::new(),
        click_path: Some("/tasks".into()),
        priority: 3,
        dedupe_key: key.into(),
        not_before,
    }
}

const TABLES: &[&str] = &[
    "task_list",
    "task_definition",
    "task_definition_candidate",
    "task",
    "task_assignee",
    "task_step",
    "member_away",
    "routine",
    "routine_member",
    "routine_step",
    "routine_run",
    "routine_run_step",
    "notification",
];

async fn count(tx: &mut Tx, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_reverts_and_reapplies(pool: PgPool) {
    let down = include_str!("../migrations/20261026000001_m2_tasks.down.sql");
    let up = include_str!("../migrations/20261026000001_m2_tasks.up.sql");
    let tables = "('task_list','task','routine','notification','member_away')";
    let n = |pool: PgPool| async move {
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN {tables}"
        ))
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    assert_eq!(n(pool.clone()).await, 5);
    let down3 = include_str!("../migrations/20261027000001_m3_ingest.down.sql");
    let up3 = include_str!("../migrations/20261027000001_m3_ingest.up.sql");
    pool.execute(down3).await.unwrap();
    pool.execute(down).await.unwrap();
    assert_eq!(n(pool.clone()).await, 0);
    let cols: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns WHERE table_name = 'household' AND column_name = 'now_list_size'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cols, 0);
    pool.execute(up).await.unwrap();
    pool.execute(up3).await.unwrap();
    assert_eq!(n(pool).await, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn rls_isolates_every_new_table(pool: PgPool) {
    let store = Store::from_pool(pool);
    let a = seed(&store, "A").await;
    let b = seed(&store, "B").await;

    let mut tx = store.tx_for(a.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(a.h, "Chores"))
        .await
        .unwrap();
    let def = TaskDefinitionRepo::insert(
        &mut tx,
        &definition(a.h, l.value.id, "Bins"),
        &[a.adult, a.child],
    )
    .await
    .unwrap();
    TaskRepo::insert(
        &mut tx,
        &new_task(a.h, l.value.id, Some(def.value.id), Some(at(31, 12, 0))),
        &[a.child],
        &["x".into()],
    )
    .await
    .unwrap()
    .unwrap();
    MemberAwayRepo::insert(
        &mut tx,
        &MemberAway {
            id: MemberAwayId::new(),
            household_id: a.h,
            member_id: a.child,
            from_date: d(10, 1),
            until_date: d(10, 3),
            note: None,
            created_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    let r = routine(a.h, "Morning");
    let full = RoutineRepo::insert(&mut tx, &r, &[a.child], &[spec("Dress"), spec("Teeth")])
        .await
        .unwrap();
    let steps: Vec<_> = full.steps.iter().map(|s| s.id).collect();
    assert!(
        RoutineRunRepo::ensure(&mut tx, r.id, a.child, d(10, 26), &steps)
            .await
            .unwrap()
    );
    assert!(
        NotificationRepo::insert_dedupe(&mut tx, &notification(&a, "k1", None))
            .await
            .unwrap()
    );
    for t in TABLES {
        assert!(count(&mut tx, t).await > 0, "{t} should have rows for A");
    }
    tx.commit().await.unwrap();

    let mut tx = store.tx_for(b.h).await.unwrap();
    for t in TABLES {
        assert_eq!(count(&mut tx, t).await, 0, "{t} leaks across households");
    }
    assert!(TaskListRepo::list(&mut tx, true).await.unwrap().is_empty());
    assert!(RoutineRepo::list(&mut tx, true).await.unwrap().is_empty());
    tx.commit().await.unwrap();

    // The tenant policy also blocks inserting into another household.
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert!(
        TaskListRepo::insert(&mut tx, &list(a.h, "Sneaky"))
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn household_weather_and_pref_extensions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let h = HouseholdRepo::get(&mut tx, s.h).await.unwrap().unwrap();
    assert_eq!(h.value.now_list_size, 7);
    assert_eq!(h.value.quiet_from, tm(21, 0));
    let v = HouseholdRepo::update_settings(
        &mut tx,
        s.h,
        &HouseholdPatch {
            now_list_size: Some(5),
            now_weights: Some(fa_domain::tasks::NowWeights {
                due: 2.0,
                flag: 0.25,
                weather: 0.0,
            }),
            quiet_from: Some(tm(22, 30)),
            quiet_until: Some(tm(6, 0)),
            ..Default::default()
        },
        h.version,
    )
    .await
    .unwrap();
    assert_eq!(v, h.version + 1);
    let h = HouseholdRepo::list(&mut tx).await.unwrap().remove(0);
    assert_eq!(h.value.now_list_size, 5);
    assert_eq!(h.value.now_weights.due, 2.0);
    assert_eq!(h.value.quiet_until, tm(6, 0));
    let mut hv = h.value.clone();
    hv.now_list_size = 9;
    HouseholdRepo::update(&mut tx, &hv, h.version)
        .await
        .unwrap();
    assert_eq!(
        HouseholdRepo::get(&mut tx, s.h)
            .await
            .unwrap()
            .unwrap()
            .value
            .now_list_size,
        9
    );

    let rule = WeatherRule {
        id: WeatherRuleId::new(),
        household_id: s.h,
        name: "Wet".into(),
        condition: "rain > 1".into(),
        action: RuleAction::Task,
        severity: Severity::Info,
        message: "Bring washing in".into(),
        task_definition: None,
        status: RuleStatus::Active,
        suggested_by: SuggestedBy::User,
        task_mode: TaskMode::Suggest,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let r = WeatherRuleRepo::insert(&mut tx, &rule).await.unwrap();
    assert_eq!(r.value.task_mode, TaskMode::Suggest);
    let mut upd = r.value.clone();
    upd.task_mode = TaskMode::Create;
    WeatherRuleRepo::update(&mut tx, &upd, r.version)
        .await
        .unwrap();
    let r = WeatherRuleRepo::get(&mut tx, rule.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.value.task_mode, TaskMode::Create);

    let pref = NotifyPref {
        id: uuid::Uuid::now_v7(),
        household_id: s.h,
        member_id: s.child,
        channel: "ntfy".into(),
        topic: "abc".into(),
        triggers: vec!["assigned".into()],
        due_soon_minutes: 90,
        nudge_minutes: 5,
        quiet_from: Some(tm(20, 0)),
        quiet_until: None,
        copy_to_member_id: Some(s.adult),
        approval_mode: Default::default(),
        archived_at: None,
    };
    let p = NotifyPrefRepo::upsert_for_member(&mut tx, &pref, None)
        .await
        .unwrap();
    assert_eq!(p.value.copy_to_member_id, Some(s.adult));
    let mut p2 = p.value.clone();
    p2.nudge_minutes = 15;
    let p2 = NotifyPrefRepo::upsert_for_member(&mut tx, &p2, Some(p.version))
        .await
        .unwrap();
    assert_eq!(p2.value.id, p.value.id);
    assert_eq!(p2.value.nudge_minutes, 15);
    assert_eq!(p2.version, p.version + 1);
    assert!(matches!(
        NotifyPrefRepo::upsert_for_member(&mut tx, &p2.value, Some(p.version)).await,
        Err(StoreError::Conflict)
    ));
    let got = NotifyPrefRepo::get_for_member(&mut tx, s.child)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value.due_soon_minutes, 90);
    assert_eq!(NotifyPrefRepo::list_all(&mut tx).await.unwrap().len(), 1);
    assert!(
        NotifyPrefRepo::get_for_member(&mut tx, s.adult)
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(MemberRepo::adults(&mut tx).await.unwrap(), vec![s.adult]);
}

#[sqlx::test(migrations = "./migrations")]
async fn lists_and_versions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    assert_eq!(l.value.completion_policy, ListCompletionPolicy::Assignee);
    assert_eq!(l.value.claim_ttl_minutes, Some(30));
    let adult_only = TaskList {
        completion_policy: ListCompletionPolicy::AdultOnly,
        ..list(s.h, "Homework")
    };
    let hw = TaskListRepo::insert(&mut tx, &adult_only).await.unwrap();
    assert_eq!(hw.value.completion_policy, ListCompletionPolicy::AdultOnly);

    let v = TaskListRepo::update(
        &mut tx,
        l.value.id,
        &TaskListPatch {
            name: Some("Jobs".into()),
            claim_ttl_minutes: Some(None),
            color: Some(Some("#fff".into())),
            ..empty_patch()
        },
        l.version,
    )
    .await
    .unwrap();
    assert_eq!(v, l.version + 1);
    let got = TaskListRepo::get_by_name(&mut tx, "Jobs")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value.claim_ttl_minutes, None);
    assert_eq!(got.value.color.as_deref(), Some("#fff"));
    assert!(matches!(
        TaskListRepo::update(&mut tx, l.value.id, &empty_patch(), l.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        TaskListRepo::update(&mut tx, TaskListId::new(), &empty_patch(), 1).await,
        Err(StoreError::NotFound)
    ));
    TaskListRepo::archive(&mut tx, hw.value.id).await.unwrap();
    assert_eq!(TaskListRepo::list(&mut tx, false).await.unwrap().len(), 1);
    assert_eq!(TaskListRepo::list(&mut tx, true).await.unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn definitions_candidates_lock_and_state(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    let mut def = definition(s.h, l.value.id, "Bins");
    def.season = Some((401, 1031));
    let ins = TaskDefinitionRepo::insert(&mut tx, &def, &[s.child, s.adult])
        .await
        .unwrap();
    assert_eq!(ins.value.season, Some((401, 1031)));
    assert_eq!(
        ins.value.schedule,
        Schedule::Rrule {
            rrule: "FREQ=WEEKLY;BYDAY=SA".into()
        }
    );
    assert_eq!(
        TaskDefinitionRepo::candidates(&mut tx, def.id)
            .await
            .unwrap(),
        vec![(s.child, 0), (s.adult, 1)]
    );

    // after_done schedule and candidate replacement.
    let mut upd = ins.value.clone();
    upd.schedule = Schedule::AfterDone { interval_days: 7 };
    upd.next_due_at = None;
    upd.title = "Bins out".into();
    let v = TaskDefinitionRepo::update(&mut tx, &upd, Some(&[s.adult]), ins.version)
        .await
        .unwrap();
    assert_eq!(v, ins.version + 1);
    assert_eq!(
        TaskDefinitionRepo::candidates(&mut tx, def.id)
            .await
            .unwrap(),
        vec![(s.adult, 0)]
    );
    assert!(matches!(
        TaskDefinitionRepo::update(&mut tx, &upd, None, ins.version).await,
        Err(StoreError::Conflict)
    ));
    let got = TaskDefinitionRepo::get(&mut tx, def.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value.schedule, Schedule::AfterDone { interval_days: 7 });
    assert_eq!(got.value.next_due_at, None);
    assert!(
        TaskDefinitionRepo::get_by_title(&mut tx, l.value.id, "Bins out")
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        TaskDefinitionRepo::list(&mut tx, Some(l.value.id), false)
            .await
            .unwrap()
            .len(),
        1
    );

    // due_for_materialize: next_due_at unset counts; far-future does not.
    let now = at(26, 12, 0);
    let due = TaskDefinitionRepo::due_for_materialize(&mut tx, now)
        .await
        .unwrap();
    assert_eq!(due, vec![(s.h, def.id)]);
    TaskDefinitionRepo::set_schedule_state(
        &mut tx,
        def.id,
        1,
        Some(at(20, 9, 0)),
        Some(at(31, 23, 59)),
    )
    .await
    .unwrap();
    let after = TaskDefinitionRepo::get(&mut tx, def.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.version, got.version,
        "system state doesn't bump the version"
    );
    assert_eq!(after.value.rotation_cursor, 1);
    // lead_days 1: visible-from window starts 2 days before 31 Oct 23:59.
    assert!(
        TaskDefinitionRepo::due_for_materialize(&mut tx, at(28, 12, 0))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        TaskDefinitionRepo::due_for_materialize(&mut tx, at(30, 0, 0))
            .await
            .unwrap()
            .len(),
        1
    );

    // conditions
    assert!(
        TaskDefinitionRepo::with_condition(&mut tx)
            .await
            .unwrap()
            .is_empty()
    );
    let mut cond = after.value.clone();
    cond.season_condition = Some("temp_min < 0".into());
    TaskDefinitionRepo::update(&mut tx, &cond, None, after.version)
        .await
        .unwrap();
    assert_eq!(
        TaskDefinitionRepo::with_condition(&mut tx).await.unwrap(),
        vec![(def.id, "temp_min < 0".to_string())]
    );
    TaskDefinitionRepo::set_condition_result(&mut tx, def.id, false, at(26, 11, 0))
        .await
        .unwrap();
    let c = TaskDefinitionRepo::get(&mut tx, def.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.value.condition_ok, Some(false));
    assert_eq!(c.value.condition_checked_at, Some(at(26, 11, 0)));
    tx.commit().await.unwrap();

    // archived and paused definitions are not due.
    let mut tx = store.tx_system().await.unwrap();
    TaskDefinitionRepo::archive(&mut tx, def.id).await.unwrap();
    assert!(
        TaskDefinitionRepo::due_for_materialize(&mut tx, at(30, 0, 0))
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn lock_skips_a_row_locked_elsewhere(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    let def = definition(s.h, l.value.id, "Bins");
    TaskDefinitionRepo::insert(&mut tx, &def, &[s.child])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut t1 = store.tx_for(s.h).await.unwrap();
    assert!(
        TaskDefinitionRepo::lock(&mut t1, def.id)
            .await
            .unwrap()
            .is_some()
    );
    let mut t2 = store.tx_for(s.h).await.unwrap();
    assert!(
        TaskDefinitionRepo::lock(&mut t2, def.id)
            .await
            .unwrap()
            .is_none(),
        "locked by t1"
    );
    t2.rollback().await.unwrap();
    t1.commit().await.unwrap();
    let mut t3 = store.tx_for(s.h).await.unwrap();
    assert!(
        TaskDefinitionRepo::lock(&mut t3, def.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn task_dedupe_status_and_filters(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    let def = TaskDefinitionRepo::insert(&mut tx, &definition(s.h, l.value.id, "Bins"), &[s.child])
        .await
        .unwrap();
    let did = def.value.id;
    let due = at(31, 23, 59);

    let first = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, Some(did), Some(due)),
        &[s.child],
        &["a".into(), "b".into()],
    )
    .await
    .unwrap()
    .unwrap();
    // Same (definition, origin, due_at): a no-op that writes nothing.
    let dup = new_task(s.h, l.value.id, Some(did), Some(due));
    assert!(
        TaskRepo::insert(&mut tx, &dup, &[s.adult], &["z".into()])
            .await
            .unwrap()
            .is_none()
    );
    assert!(TaskRepo::get(&mut tx, dup.id).await.unwrap().is_none());
    // Another due date, and manual tasks with no due date, are fine.
    assert!(
        TaskRepo::insert(
            &mut tx,
            &new_task(s.h, l.value.id, Some(did), Some(at(30, 23, 59))),
            &[],
            &[]
        )
        .await
        .unwrap()
        .is_some()
    );
    for _ in 0..2 {
        assert!(
            TaskRepo::insert(
                &mut tx,
                &new_task(s.h, l.value.id, None, None),
                &[s.adult],
                &[]
            )
            .await
            .unwrap()
            .is_some()
        );
    }

    let rows = TaskRepo::get_view_rows(
        &mut tx,
        &TaskFilter {
            ids: Some(vec![first.value.id]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].list_name, "Chores");
    assert_eq!(rows[0].completion_policy, ListCompletionPolicy::Assignee);
    assert_eq!(rows[0].assignees, vec![s.child]);
    assert_eq!(
        rows[0]
            .steps
            .iter()
            .map(|x| x.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );

    let by_member = TaskRepo::get_view_rows(
        &mut tx,
        &TaskFilter {
            member: Some(s.adult),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_member.len(), 2);
    let ranged = TaskRepo::get_view_rows(
        &mut tx,
        &TaskFilter {
            due_from: Some(at(31, 0, 0)),
            due_to: Some(at(31, 23, 59) + Duration::seconds(1)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(ranged.len(), 1);
    assert_eq!(ranged[0].task.value.id, first.value.id);
    assert!(
        TaskRepo::get_view_rows(
            &mut tx,
            &TaskFilter {
                calendar_display_not_hidden: true,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        TaskRepo::open_for_definition(&mut tx, did)
            .await
            .unwrap()
            .len(),
        2
    );

    // status transitions
    let id = first.value.id;
    TaskRepo::set_claim(&mut tx, id, Some(s.child), Some(at(26, 10, 0)))
        .await
        .unwrap();
    assert_eq!(
        TaskRepo::clear_expired_claims(&mut tx, at(26, 9, 0))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        TaskRepo::clear_expired_claims(&mut tx, at(26, 10, 0))
            .await
            .unwrap(),
        1
    );
    TaskRepo::set_claim(&mut tx, id, Some(s.child), Some(at(26, 10, 0)))
        .await
        .unwrap();
    let undo = serde_json::json!({"rotation_cursor": 0});
    TaskRepo::set_status(
        &mut tx,
        id,
        TaskStatus::Done,
        Some(s.child),
        None,
        Some(undo.clone()),
    )
    .await
    .unwrap();
    let done = TaskRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(done.value.status, TaskStatus::Done);
    assert_eq!(done.value.completed_by, Some(s.child));
    assert!(done.value.completed_at.is_some());
    assert_eq!(done.value.completion_undo, Some(undo));
    assert_eq!(done.value.claimed_by, None, "completing clears the claim");
    assert!(done.version > first.version);
    TaskRepo::reopen(&mut tx, id).await.unwrap();
    let re = TaskRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(re.value.status, TaskStatus::Open);
    assert!(re.value.completed_at.is_none() && re.value.completion_undo.is_none());
    TaskRepo::set_status(&mut tx, id, TaskStatus::Skipped, None, None, None)
        .await
        .unwrap();
    let sk = TaskRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(sk.value.status, TaskStatus::Skipped);
    assert_eq!(sk.value.skipped_by, None, "None = skipped by the system");
    assert!(matches!(
        TaskRepo::set_status(&mut tx, TaskId::new(), TaskStatus::Done, None, None, None).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn step_replacement_keeps_ticks_and_versions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    let t = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, None, None),
        &[s.child],
        &["wash".into(), "dry".into(), "put away".into()],
    )
    .await
    .unwrap()
    .unwrap();
    let id = t.value.id;
    let rows = TaskRepo::get_view_rows(&mut tx, &TaskFilter::default())
        .await
        .unwrap();
    let dry = rows[0].steps[1].id;
    TaskRepo::tick_step(&mut tx, id, dry, true, Some(s.child))
        .await
        .unwrap();
    assert!(matches!(
        TaskRepo::tick_step(&mut tx, TaskId::new(), dry, true, None).await,
        Err(StoreError::NotFound)
    ));

    let mut upd = t.value.clone();
    upd.title = "Laundry".into();
    let v = TaskRepo::update(
        &mut tx,
        &upd,
        Some(&[s.adult]),
        Some(&["fold".into(), "dry".into(), "wash".into()]),
        t.version,
    )
    .await
    .unwrap();
    assert_eq!(v, t.version + 1);
    let rows = TaskRepo::get_view_rows(&mut tx, &TaskFilter::default())
        .await
        .unwrap();
    let r = &rows[0];
    assert_eq!(r.task.value.title, "Laundry");
    assert_eq!(r.assignees, vec![s.adult]);
    let texts: Vec<_> = r
        .steps
        .iter()
        .map(|x| (x.ord, x.text.as_str(), x.done_at.is_some()))
        .collect();
    assert_eq!(
        texts,
        [(0, "fold", false), (1, "dry", true), (2, "wash", false)]
    );
    assert_eq!(
        r.steps[1].id, dry,
        "the unchanged step keeps its id and tick"
    );
    assert_eq!(r.steps[1].done_by, Some(s.child));

    // untick, then drop all steps
    TaskRepo::tick_step(&mut tx, id, dry, false, None)
        .await
        .unwrap();
    TaskRepo::update(&mut tx, &upd, None, Some(&[]), v)
        .await
        .unwrap();
    let rows = TaskRepo::get_view_rows(&mut tx, &TaskFilter::default())
        .await
        .unwrap();
    assert!(rows[0].steps.is_empty());
    assert!(matches!(
        TaskRepo::update(&mut tx, &upd, None, None, t.version).await,
        Err(StoreError::Conflict)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_created_after_and_notification_queries(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let l = TaskListRepo::insert(&mut tx, &list(s.h, "Chores"))
        .await
        .unwrap();
    let def = TaskDefinitionRepo::insert(&mut tx, &definition(s.h, l.value.id, "Bins"), &[s.child])
        .await
        .unwrap();
    let did = def.value.id;
    let old = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, Some(did), Some(at(1, 9, 0))),
        &[],
        &[],
    )
    .await
    .unwrap()
    .unwrap();
    // Created "after": move the old one's created_at back, keep the new ones fresh.
    sqlx::query("UPDATE task SET created_at = now() - interval '1 hour' WHERE id = $1")
        .bind(old.value.id.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    let ticked = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, Some(did), Some(at(2, 9, 0))),
        &[],
        &["s".into()],
    )
    .await
    .unwrap()
    .unwrap();
    let rows = TaskRepo::get_view_rows(
        &mut tx,
        &TaskFilter {
            ids: Some(vec![ticked.value.id]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    TaskRepo::tick_step(&mut tx, ticked.value.id, rows[0].steps[0].id, true, None)
        .await
        .unwrap();
    let fresh = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, Some(did), Some(at(3, 9, 0))),
        &[],
        &[],
    )
    .await
    .unwrap()
    .unwrap();
    let n = TaskRepo::delete_created_after(&mut tx, did, Utc::now() - Duration::minutes(10))
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert!(
        TaskRepo::get(&mut tx, fresh.value.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        TaskRepo::get(&mut tx, ticked.value.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        TaskRepo::get(&mut tx, old.value.id)
            .await
            .unwrap()
            .is_some()
    );

    // overdue / due soon
    let a = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, None, Some(at(26, 8, 0))),
        &[s.child],
        &[],
    )
    .await
    .unwrap()
    .unwrap();
    let b = TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, None, Some(at(26, 13, 0))),
        &[s.adult, s.child],
        &[],
    )
    .await
    .unwrap()
    .unwrap();
    TaskRepo::insert(
        &mut tx,
        &new_task(s.h, l.value.id, None, Some(at(26, 8, 0))),
        &[],
        &[],
    )
    .await
    .unwrap();
    let now = at(26, 12, 0);
    let over = TaskRepo::overdue_open(&mut tx, now).await.unwrap();
    assert!(
        over.iter()
            .any(|(id, m)| *id == a.value.id && m == &vec![s.child])
    );
    assert!(over.iter().all(|(id, _)| *id != b.value.id));
    let soon = TaskRepo::due_soon_candidates(&mut tx, now, Duration::minutes(30))
        .await
        .unwrap();
    assert!(soon.is_empty());
    let soon = TaskRepo::due_soon_candidates(&mut tx, now, Duration::hours(2))
        .await
        .unwrap();
    assert_eq!(soon.len(), 1);
    assert_eq!(soon[0].0, b.value.id);
    assert_eq!(soon[0].1, at(26, 13, 0));
    assert_eq!(soon[0].2.len(), 2);
    assert_eq!(soon[0].3, "Bins");

    // notifications
    let n1 = notification(&s, "k1", Some(at(26, 10, 0)));
    assert!(NotificationRepo::insert_dedupe(&mut tx, &n1).await.unwrap());
    assert!(
        !NotificationRepo::insert_dedupe(&mut tx, &n1).await.unwrap(),
        "idempotent"
    );
    assert!(
        NotificationRepo::insert_dedupe(&mut tx, &notification(&s, "k2", Some(at(26, 9, 0))))
            .await
            .unwrap()
    );
    assert!(
        NotificationRepo::insert_dedupe(&mut tx, &notification(&s, "k3", Some(at(26, 15, 0))))
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();

    let mut sys = store.tx_system().await.unwrap();
    let pend = NotificationRepo::pending(&mut sys, now, 10).await.unwrap();
    // oldest first = insertion order, and k3 is not due yet
    assert_eq!(
        pend.iter()
            .map(|n| n.dedupe_key.as_str())
            .collect::<Vec<_>>(),
        ["k1", "k2"]
    );
    assert_eq!(
        NotificationRepo::pending(&mut sys, now, 1)
            .await
            .unwrap()
            .len(),
        1
    );
    let k1 = pend[0].id;
    let k2 = pend[1].id;
    NotificationRepo::defer(&mut sys, k1, at(26, 14, 0))
        .await
        .unwrap();
    NotificationRepo::mark_dropped(&mut sys, k2, "not configured")
        .await
        .unwrap();
    assert!(
        NotificationRepo::pending(&mut sys, now, 10)
            .await
            .unwrap()
            .is_empty()
    );
    let n = NotificationRepo::pending(&mut sys, at(26, 14, 0), 10)
        .await
        .unwrap();
    assert_eq!(n.len(), 1, "k1 is due again at 14:00, k3 only at 15:00");
    NotificationRepo::record_failure(&mut sys, k1, "boom", at(26, 14, 2), false)
        .await
        .unwrap();
    let f = NotificationRepo::get(&mut sys, k1).await.unwrap().unwrap();
    assert_eq!(
        (f.attempts, f.status, f.not_before),
        (1, NotificationStatus::Pending, at(26, 14, 2))
    );
    NotificationRepo::record_failure(&mut sys, k1, "boom", at(26, 14, 6), true)
        .await
        .unwrap();
    let f = NotificationRepo::get(&mut sys, k1).await.unwrap().unwrap();
    assert_eq!((f.attempts, f.status), (2, NotificationStatus::Failed));
    let k3 = NotificationRepo::pending(&mut sys, at(26, 16, 0), 10)
        .await
        .unwrap();
    assert_eq!(k3.len(), 1);
    NotificationRepo::mark_sent(&mut sys, &[k3[0].id], at(26, 16, 0))
        .await
        .unwrap();
    let k3 = NotificationRepo::get(&mut sys, k3[0].id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (k3.status, k3.sent_at),
        (NotificationStatus::Sent, Some(at(26, 16, 0)))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn routines_runs_and_ticks(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let r = routine(s.h, "Morning");
    let full = RoutineRepo::insert(
        &mut tx,
        &r,
        &[s.child],
        &[spec("Dress"), spec("Teeth"), spec("Bag")],
    )
    .await
    .unwrap();
    assert_eq!(full.members, vec![s.child]);
    assert_eq!(full.steps.len(), 3);
    assert_eq!(
        full.routine.value.no_school_match,
        vec!["holiday".to_string()]
    );
    assert_eq!(full.routine.value.day_filter, DayFilter::Weekdays);
    let ids: Vec<_> = full.steps.iter().map(|x| x.id).collect();

    let day = d(10, 26);
    assert!(
        RoutineRunRepo::ensure(&mut tx, r.id, s.child, day, &ids)
            .await
            .unwrap()
    );
    assert!(
        !RoutineRunRepo::ensure(&mut tx, r.id, s.child, day, &ids)
            .await
            .unwrap()
    );
    let runs = RoutineRunRepo::for_date(&mut tx, day, None).await.unwrap();
    assert_eq!(runs.len(), 1);
    let run = runs[0].run.id;
    assert_eq!(runs[0].routine_name, "Morning");
    assert_eq!(
        runs[0]
            .steps
            .iter()
            .map(|x| x.text.as_str())
            .collect::<Vec<_>>(),
        ["Dress", "Teeth", "Bag"]
    );
    assert!(
        RoutineRunRepo::for_date(&mut tx, day, Some(s.adult))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        RoutineRunRepo::open_with_steps_left(&mut tx, day)
            .await
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        RoutineRunRepo::tick(&mut tx, run, ids[0], true, Some(s.child), None)
            .await
            .unwrap(),
        RunStatus::Open
    );
    assert_eq!(
        RoutineRunRepo::tick(&mut tx, run, ids[1], true, Some(s.child), None)
            .await
            .unwrap(),
        RunStatus::Open
    );
    assert_eq!(
        RoutineRunRepo::tick(&mut tx, run, ids[2], true, Some(s.child), None)
            .await
            .unwrap(),
        RunStatus::Done
    );
    let done = RoutineRunRepo::for_date(&mut tx, day, None).await.unwrap();
    assert_eq!(done[0].run.status, RunStatus::Done);
    assert!(done[0].run.completed_at.is_some());
    assert!(
        RoutineRunRepo::open_with_steps_left(&mut tx, day)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        RoutineRunRepo::tick(&mut tx, run, ids[1], false, None, None)
            .await
            .unwrap(),
        RunStatus::Open
    );
    let reopened = RoutineRunRepo::for_date(&mut tx, day, None).await.unwrap();
    assert!(reopened[0].run.completed_at.is_none());
    assert!(reopened[0].steps[1].done_at.is_none());
    assert!(matches!(
        RoutineRunRepo::tick(&mut tx, run, RoutineStepId::new(), true, None, None).await,
        Err(StoreError::NotFound)
    ));

    // steps replacement keeps ids for the same ord + text
    let mut upd = full.routine.value.clone();
    upd.name = "Morning rush".into();
    let v = RoutineRepo::update(
        &mut tx,
        &upd,
        Some(&[s.child, s.adult]),
        Some(&[
            spec("Dress"),
            spec("Brush hair"),
            RoutineStepSpec {
                text: "Bag".into(),
                target_time: Some(tm(7, 45)),
            },
        ]),
        full.routine.version,
    )
    .await
    .unwrap();
    assert_eq!(v, full.routine.version + 1);
    let got = RoutineRepo::get(&mut tx, r.id).await.unwrap().unwrap();
    assert_eq!(got.members.len(), 2);
    assert_eq!(
        got.steps
            .iter()
            .map(|x| x.text.as_str())
            .collect::<Vec<_>>(),
        ["Dress", "Brush hair", "Bag"]
    );
    assert_eq!(got.steps[0].id, ids[0], "same ord and text keeps the id");
    assert_ne!(got.steps[1].id, ids[1]);
    assert_eq!(got.steps[2].id, ids[2]);
    assert_eq!(got.steps[2].target_time, Some(tm(7, 45)));
    assert!(matches!(
        RoutineRepo::update(&mut tx, &upd, None, None, full.routine.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(
        RoutineRepo::get_by_name(&mut tx, "Morning rush")
            .await
            .unwrap()
            .is_some()
    );

    // skip_before: tomorrow's date skips today's open run, not done ones
    let day2 = d(10, 27);
    assert!(
        RoutineRunRepo::ensure(
            &mut tx,
            r.id,
            s.adult,
            day2,
            &got.steps.iter().map(|x| x.id).collect::<Vec<_>>(),
        )
        .await
        .unwrap()
    );
    assert_eq!(RoutineRunRepo::skip_before(&mut tx, day2).await.unwrap(), 1);
    let first = RoutineRunRepo::for_date(&mut tx, day, None).await.unwrap();
    assert_eq!(first[0].run.status, RunStatus::Skipped);
    assert_eq!(RoutineRunRepo::skip_before(&mut tx, day2).await.unwrap(), 0);

    RoutineRepo::archive(&mut tx, r.id).await.unwrap();
    assert!(RoutineRepo::list(&mut tx, false).await.unwrap().is_empty());
    assert!(
        RoutineRunRepo::for_date(&mut tx, day2, None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn member_away_ranges(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let a = MemberAwayRepo::insert(
        &mut tx,
        &MemberAway {
            id: MemberAwayId::new(),
            household_id: s.h,
            member_id: s.child,
            from_date: d(10, 26),
            until_date: d(10, 28),
            note: Some("trip".into()),
            created_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        MemberAwayRepo::away_on(&mut tx, d(10, 25)).await.unwrap(),
        vec![]
    );
    assert_eq!(
        MemberAwayRepo::away_on(&mut tx, d(10, 26)).await.unwrap(),
        vec![s.child]
    );
    assert_eq!(
        MemberAwayRepo::away_on(&mut tx, d(10, 28)).await.unwrap(),
        vec![s.child]
    );
    assert_eq!(
        MemberAwayRepo::away_on(&mut tx, d(10, 29)).await.unwrap(),
        vec![]
    );
    assert_eq!(
        MemberAwayRepo::list(&mut tx, Some(s.adult))
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(MemberAwayRepo::list(&mut tx, None).await.unwrap().len(), 1);
    let bad = MemberAwayRepo::insert(
        &mut tx,
        &MemberAway {
            id: MemberAwayId::new(),
            from_date: d(10, 5),
            until_date: d(10, 4),
            ..a.clone()
        },
    )
    .await;
    assert!(bad.is_err(), "until before from violates the CHECK");
    drop(tx);
    let mut tx = store.tx_for(s.h).await.unwrap();
    MemberAwayRepo::insert(&mut tx, &a).await.unwrap();
    MemberAwayRepo::delete(&mut tx, a.id).await.unwrap();
    assert!(matches!(
        MemberAwayRepo::delete(&mut tx, a.id).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn all_day_titles_cover_multi_day_events(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let src = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: s.h,
        kind: SourceKind::Ics,
        name: "School".into(),
        owner_member: Some(s.adult),
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: false,
        poll_minutes: 15,
        state: SourceState::NeedsReauth,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    CalendarSourceRepo::insert(&mut tx, &src).await.unwrap();
    let (cal, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, "remote", "School", None)
        .await
        .unwrap();
    let cal = cal.value.id;
    let allday =
        |uid: &str, title: &str, from: NaiveDate, to: NaiveDate, cancelled: bool| NewOccurrence {
            remote_uid: uid.into(),
            instance_key: String::new(),
            title: title.into(),
            description: None,
            location: None,
            start_at: Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0).unwrap()),
            end_at: Utc.from_utc_datetime(&to.and_hms_opt(0, 0, 0).unwrap()),
            all_day: true,
            start_date: Some(from),
            end_date: Some(to),
            source_tz: None,
            is_private: false,
            cancelled,
            remote_etag: None,
            external_url: None,
        };
    let mut timed = allday("u4", "Timed", d(10, 26), d(10, 26), false);
    timed.all_day = false;
    timed.start_date = None;
    timed.end_date = None;
    EventRepo::upsert_synced(
        &mut tx,
        cal,
        &[
            allday("u1", "Half term", d(10, 26), d(10, 31), false),
            allday("u2", "Inset day", d(10, 28), d(10, 29), false),
            allday("u3", "No school (cancelled)", d(10, 26), d(10, 27), true),
            timed,
        ],
    )
    .await
    .unwrap();
    macro_rules! titles {
        ($date:expr) => {
            EventRepo::all_day_titles(&mut tx, cal, $date)
        };
    }
    assert_eq!(titles!(d(10, 25)).await.unwrap(), Vec::<String>::new());
    assert_eq!(titles!(d(10, 26)).await.unwrap(), ["Half term"]);
    assert_eq!(
        titles!(d(10, 28)).await.unwrap(),
        ["Half term", "Inset day"]
    );
    assert_eq!(titles!(d(10, 29)).await.unwrap(), ["Half term"]);
    assert_eq!(titles!(d(10, 30)).await.unwrap(), ["Half term"]);
    assert_eq!(
        titles!(d(10, 31)).await.unwrap(),
        Vec::<String>::new(),
        "end_date is exclusive"
    );

    let re = CalendarSourceRepo::needing_reauth(&mut tx).await.unwrap();
    assert_eq!(re, vec![(src.id, Some(s.adult), None)]);
}
