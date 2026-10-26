mod common;

use chrono::Utc;
use common::*;
use fa_domain::*;
use fa_store::*;
use fa_weather::{WeatherConfig, WeatherService, parse, validate_condition};
use serde_json::json;
use sqlx::PgPool;

fn now() -> chrono::DateTime<Utc> {
    utc(2026, 10, 24, 10, 0)
}

async fn seed(store: &Store) -> HouseholdId {
    let h = Household {
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
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    tx.commit().await.unwrap();
    h.id
}

fn definition(h: HouseholdId, l: TaskListId, title: &str, cond: &str) -> TaskDefinition {
    TaskDefinition {
        id: TaskDefinitionId::new(),
        household_id: h,
        list_id: l,
        title: title.into(),
        notes: None,
        steps_template: vec![],
        schedule: fa_domain::Schedule::Rrule {
            rrule: "FREQ=WEEKLY;BYDAY=SA".into(),
        },
        start_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        due_time: None,
        lead_days: 1,
        on_missed: OnMissed::AutoSkip,
        strategy: Strategy::Rotation,
        advance_on: AdvanceOn::AnyCompletion,
        skip_advances: true,
        rotation_cursor: 0,
        priority_flag: PriorityFlag::Normal,
        season: None,
        season_condition: Some(cond.into()),
        condition_ok: None,
        condition_checked_at: None,
        last_done_at: None,
        next_due_at: None,
        paused: false,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

async fn setup(pool: &PgPool, temp: f64) -> (Store, WeatherService, HouseholdId) {
    let store = Store::from_pool(pool.clone());
    let svc = WeatherService::new(store.clone(), WeatherConfig::default()).unwrap();
    let h = seed(&store).await;
    let synth = Synth::new(now(), 48).set("temperature_2m", |_| Some(temp));
    let mut tx = store.tx_for(h).await.unwrap();
    WeatherSnapshotRepo::insert(
        &mut tx,
        "open-meteo",
        &synth.current(),
        &synth.hourly(),
        &synth.daily(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (store, svc, h)
}

async fn task_rule(store: &Store, h: HouseholdId, cond: &str, def: bool) -> WeatherRuleId {
    let r = WeatherRule {
        id: WeatherRuleId::new(),
        household_id: h,
        name: "Gutters".into(),
        condition: cond.into(),
        action: RuleAction::Task,
        severity: Severity::Info,
        message: "x".into(),
        task_definition: def.then(|| json!({"title": "Clear the gutters"})),
        task_mode: TaskMode::Create,
        status: RuleStatus::Active,
        suggested_by: SuggestedBy::User,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = store.tx_for(h).await.unwrap();
    WeatherRuleRepo::insert(&mut tx, &r).await.unwrap();
    tx.commit().await.unwrap();
    r.id
}

async fn jobs(pool: &PgPool) -> Vec<(String, serde_json::Value)> {
    sqlx::query_as("SELECT idempotency_key, payload FROM job WHERE kind = 'tasks.from_weather'")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn matching_task_rule_enqueues_once(pool: PgPool) {
    let (store, svc, h) = setup(&pool, -3.0).await;
    let hot = task_rule(&store, h, "min(temp_c, today) < 0", true).await;
    task_rule(&store, h, "min(temp_c, today) < -50", true).await;
    task_rule(&store, h, "min(temp_c, today) < 0", false).await;
    svc.evaluate_at(h, now()).await.unwrap();
    svc.evaluate_at(h, now()).await.unwrap();
    let j = jobs(&pool).await;
    assert_eq!(j.len(), 1);
    assert_eq!(j[0].0, format!("tasks.from_weather:{hot}:2026-10-24"));
    assert_eq!(j[0].1["rule"], json!(hot));
    assert_eq!(j[0].1["date"], "2026-10-24");
    assert_eq!(j[0].1["household"], json!(h));
    let adv: i64 = sqlx::query_scalar("SELECT count(*) FROM advisory")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(adv, 0);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn season_conditions_set_true_false_null(pool: PgPool) {
    let (store, svc, h) = setup(&pool, -3.0).await;
    let list = TaskList {
        id: TaskListId::new(),
        household_id: h,
        name: "Chores".into(),
        kind: TaskListKind::Chores,
        completion_policy: fa_domain::authz::ListCompletionPolicy::Assignee,
        child_can_add: ChildCanAdd::Own,
        dashboard_add: false,
        calendar_default: CalendarDisplay::UntilDone,
        claim_ttl_minutes: None,
        visible_on_dashboard: true,
        color: None,
        sort: 0,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = store.tx_for(h).await.unwrap();
    TaskListRepo::insert(&mut tx, &list).await.unwrap();
    let mut ids = vec![];
    for (t, c) in [
        ("a", "min(temp_c, today) < 0"),
        ("b", "min(temp_c, today) > 30"),
        ("c", "nonsense"),
    ] {
        let d = definition(h, list.id, t, c);
        ids.push(d.id);
        TaskDefinitionRepo::insert(&mut tx, &d, &[]).await.unwrap();
    }
    tx.commit().await.unwrap();

    svc.evaluate_at(h, now()).await.unwrap();
    let mut tx = store.tx_for(h).await.unwrap();
    for (id, want) in ids.iter().zip([Some(true), Some(false), None]) {
        let d = TaskDefinitionRepo::get(&mut tx, *id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(d.value.condition_ok, want);
        assert_eq!(d.value.condition_checked_at, Some(now()));
    }
}

#[test]
fn validate_condition_wraps_parse() {
    assert!(validate_condition("min(temp_c, today) < 0").is_ok());
    assert!(validate_condition("nonsense").is_err());
    assert_eq!(
        validate_condition("nonsense").is_err(),
        parse("nonsense").is_err()
    );
}
