mod common;

use chrono::Utc;
use common::*;
use fa_domain::*;
use fa_store::*;
use fa_weather::{WeatherConfig, WeatherService, ensure_default_rules};
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn now() -> chrono::DateTime<Utc> {
    utc(2026, 10, 24, 10, 0)
}

/// Frost tonight (idx 6..) and nothing else.
fn frosty() -> Synth {
    Synth::new(now(), 48).set("temperature_2m", |i| Some(if i >= 6 { -3.0 } else { 8.0 }))
}

async fn household(store: &Store, located: bool, units: Units) -> HouseholdId {
    let h = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
        tz: "Europe/Berlin".into(),
        location: located.then(|| Location {
            lat: 52.52,
            lon: 13.42,
            label: "Example".into(),
        }),
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units,
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
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(h.id).await.unwrap();
    ensure_default_rules(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    h.id
}

async fn service(pool: &PgPool, server: &MockServer) -> (Store, WeatherService) {
    let store = Store::from_pool(pool.clone());
    let svc = WeatherService::new(
        store.clone(),
        WeatherConfig {
            open_meteo_url: server.uri(),
            allow_http: true,
        },
    )
    .unwrap();
    (store, svc)
}

async fn serve(body: Option<serde_json::Value>) -> MockServer {
    let server = MockServer::start().await;
    let resp = match body {
        Some(b) => ResponseTemplate::new(200).set_body_json(b),
        None => ResponseTemplate::new(500),
    };
    Mock::given(method("GET"))
        .and(path("/v1/forecast"))
        .respond_with(resp)
        .mount(&server)
        .await;
    server
}

async fn active(store: &Store, h: HouseholdId) -> Vec<Advisory> {
    let mut tx = store.tx_for(h).await.unwrap();
    AdvisoryRepo::active(&mut tx, now()).await.unwrap()
}

async fn advisory_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM advisory")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn fetch_evaluate_dedupe_dismiss(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Metric).await;

    svc.fetch(h).await.unwrap();
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM job WHERE kind = 'weather.evaluate'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
    let (status, _) = {
        let mut tx = store.tx_system().await.unwrap();
        SystemStatusRepo::get(&mut tx, "weather")
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(status["ok"], true);

    svc.evaluate_at(h, now()).await.unwrap();
    let adv = active(&store, h).await;
    assert_eq!(adv.len(), 1);
    assert_eq!(adv[0].text, "Frost tonight, low of -3°C");
    assert_eq!(adv[0].severity, Severity::Warning);
    assert_eq!(adv[0].source, AdvisorySource::Rule);
    assert_eq!(adv[0].valid_from, utc(2026, 10, 24, 16, 0));
    assert_eq!(adv[0].valid_until, utc(2026, 10, 25, 7, 0));
    assert_eq!(
        adv[0].dedupe_key.len(),
        "rule:".len() + 36 + ":2026-10-24".len()
    );
    assert!(adv[0].dedupe_key.ends_with(":2026-10-24"));

    svc.evaluate_at(h, now()).await.unwrap();
    assert_eq!(advisory_rows(&pool).await, 1);

    let mut tx = store.tx_for(h).await.unwrap();
    AdvisoryRepo::dismiss(&mut tx, adv[0].id, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    svc.evaluate_at(h, now()).await.unwrap();
    assert!(active(&store, h).await.is_empty());
    assert_eq!(advisory_rows(&pool).await, 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn imperial_message_and_rule_filters(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Imperial).await;
    svc.fetch(h).await.unwrap();

    let mut tx = store.tx_for(h).await.unwrap();
    let mk = |name: &str, action, status| WeatherRule {
        id: WeatherRuleId::new(),
        household_id: h,
        name: name.into(),
        condition: "max(temp_c, today) > -100".into(),
        action,
        severity: Severity::Info,
        message: name.into(),
        task_definition: None,
        status,
        suggested_by: SuggestedBy::User,
        task_mode: fa_domain::TaskMode::Create,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    WeatherRuleRepo::insert(
        &mut tx,
        &mk("Task rule", RuleAction::Task, RuleStatus::Active),
    )
    .await
    .unwrap();
    WeatherRuleRepo::insert(
        &mut tx,
        &mk("Off rule", RuleAction::Alert, RuleStatus::Disabled),
    )
    .await
    .unwrap();
    let mut bad = mk("Broken rule", RuleAction::Alert, RuleStatus::Active);
    bad.condition = "nonsense".into();
    WeatherRuleRepo::insert(&mut tx, &bad).await.unwrap();
    tx.commit().await.unwrap();

    svc.evaluate_at(h, now()).await.unwrap();
    let adv = active(&store, h).await;
    assert_eq!(adv.len(), 1);
    assert_eq!(adv[0].text, "Frost tonight, low of 27°F");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn fetch_failure_sets_status_and_errors(pool: PgPool) {
    let server = serve(None).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Metric).await;
    assert!(svc.fetch(h).await.is_err());
    let mut tx = store.tx_system().await.unwrap();
    let (status, _) = SystemStatusRepo::get(&mut tx, "weather")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status["ok"], false);
    assert!(status["error"].as_str().unwrap().contains("500"));
    let mut tx = store.tx_for(h).await.unwrap();
    assert!(
        WeatherSnapshotRepo::latest(&mut tx)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn fetch_skips_household_without_location(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, false, Units::Metric).await;
    svc.fetch(h).await.unwrap();
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(svc.households_with_location().await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn retention_deletes_old_snapshots_and_advisories(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Metric).await;
    svc.fetch(h).await.unwrap();
    svc.evaluate_at(h, now()).await.unwrap();

    assert_eq!(svc.retention_at(h, now()).await.unwrap(), (0, 0));
    let later = Utc::now() + chrono::Duration::days(9);
    assert_eq!(svc.retention_at(h, later).await.unwrap(), (1, 1));
    assert_eq!(advisory_rows(&pool).await, 0);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn default_rules_are_idempotent(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = household(&store, false, Units::Metric).await;
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(ensure_default_rules(&mut tx).await.unwrap(), 0);
    let rules = WeatherRuleRepo::list(&mut tx, None).await.unwrap();
    assert_eq!(rules.len(), 8);
    drop(tx);

    // An edited default arriving from another install (suggested_by = user) is not duplicated.
    let mut tx = store.tx_for(h).await.unwrap();
    let frost = rules
        .iter()
        .find(|r| r.value.name == "Frost tonight")
        .unwrap();
    let mut edited = frost.value.clone();
    edited.id = WeatherRuleId::new();
    edited.name = "Custom".into();
    WeatherRuleRepo::insert(&mut tx, &edited).await.unwrap();
    WeatherRuleRepo::archive(&mut tx, frost.value.id)
        .await
        .unwrap();
    assert_eq!(ensure_default_rules(&mut tx).await.unwrap(), 0);
    assert_eq!(WeatherRuleRepo::list(&mut tx, None).await.unwrap().len(), 8);
    assert!(rules.iter().all(|r| r.value.action == RuleAction::Alert
        && r.value.status == RuleStatus::Active
        && r.value.suggested_by == SuggestedBy::Default));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn validate_previews_against_latest_snapshot(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Metric).await;

    let bad = svc
        .validate(h, "min(temp_c, soon) < 0", None)
        .await
        .unwrap();
    assert!(!bad.ok);
    assert_eq!(bad.error.unwrap().position, 12);

    let none = svc
        .validate(h, "min(temp_c, today) < 0", None)
        .await
        .unwrap();
    assert!(none.ok && none.preview.is_none());

    svc.fetch(h).await.unwrap();
    let v = svc
        .validate(h, "min(temp_c, next 48h) < 0", Some("low {value}"))
        .await
        .unwrap();
    assert!(v.ok);
    let p = v.preview.unwrap();
    assert!(p.matches || p.value.is_some());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn latest_view_trims_hourly_and_daily(pool: PgPool) {
    let server = serve(Some(frosty().api_body())).await;
    let (store, svc) = service(&pool, &server).await;
    let h = household(&store, true, Units::Metric).await;
    assert!(svc.latest_view_at(h, now()).await.unwrap().is_none());
    svc.fetch(h).await.unwrap();

    let v = svc.latest_view_at(h, now()).await.unwrap().unwrap();
    assert_eq!(v.hourly["time"].as_array().unwrap().len(), 48);
    assert_eq!(v.daily["time"].as_array().unwrap().len(), 7);
    assert_eq!(v.sunrise_today, Some(utc(2026, 10, 24, 5, 18)));
    assert_eq!(v.sunset_today, Some(utc(2026, 10, 24, 15, 39)));

    let later = svc
        .latest_view_at(h, now() + chrono::Duration::hours(5))
        .await
        .unwrap()
        .unwrap();
    let times = later.hourly["time"].as_array().unwrap();
    assert_eq!(times.len(), 43);
    assert_eq!(times[0], "2026-10-24T17:00");
    assert_eq!(later.hourly["temperature_2m"].as_array().unwrap().len(), 43);
}

#[test]
fn jobs_are_declared() {
    let kinds: Vec<_> = fa_weather::handlers(WeatherConfig::default())
        .unwrap()
        .iter()
        .map(|h| h.kind())
        .collect();
    assert_eq!(
        kinds,
        ["weather.fetch", "weather.evaluate", "weather.retention"]
    );
    assert_eq!(
        fa_weather::default_schedules(),
        [
            ("weather.fetch", "*/30 * * * *"),
            ("weather.retention", "45 3 * * *")
        ]
    );
}
