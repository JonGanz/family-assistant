use fa_config::*;
use fa_store::Store;
use sqlx::PgPool;

const EXAMPLE: &str = include_str!("../../../config/examples/household.example.toml");

const TASKS: &str = r##"
[household]
name = "The Example Family"
tz = "America/New_York"
now_list_size = 5
now_weights = { due = 1.0, flag = 0.75, weather = 0.5 }
quiet_from = "20:30"
quiet_until = "06:45"

[[members]]
key = "alex"
display_name = "Alex"
role = "adult"

[[members]]
key = "riley"
display_name = "Riley"
role = "child"

[members.notify]
triggers = ["assigned", "due_soon"]
due_soon_minutes = 90
nudge_minutes = 5
quiet_from = "21:30"
quiet_until = "07:30"
copy_to = "alex"

[[calendar_sources]]
key = "school"
kind = "ics"
name = "School"
[[calendar_sources.calendars]]
remote_id = "term"
name = "Term dates"
enabled = true

[[task_lists]]
name = "Chores"
kind = "chores"
claim_ttl_minutes = 60
color = "#44aa88"

[[task_lists]]
name = "Homework"
kind = "homework"

[[task_definitions]]
list = "Chores"
title = "Take out the bins"
steps = ["Recycling", "Rubbish"]
rrule = "FREQ=WEEKLY;BYDAY=TU"
start_date = "2026-01-06"
due_time = "19:00"
lead_days = 1
candidates = ["alex", "riley"]
strategy = "rotation"
season_from = "04-01"
season_until = "10-31"
season_condition = "temp_min > 5"

[[task_definitions]]
list = "Chores"
title = "Water the plants"
every_days = 3
start_date = "2026-01-01"
candidates = ["riley"]

[[routines]]
name = "Morning"
members = ["riley"]
starts_at = "07:00"
done_by = "08:00"
day_filter = "school_days"
school_calendar = "School/Term dates"
no_school_match = ["holiday", "inset"]
steps = [{ text = "Brush teeth", target_time = "07:10" }, { text = "Pack bag" }]

[[weather_rules]]
name = "Bins in the rain"
condition = "rain_mm > 5"
action = "task"
severity = "info"
message = "Rain tomorrow"
task_mode = "suggest"
task_definition = "Take out the bins"
"##;

#[test]
fn parses_plan_examples_with_defaults() {
    let c = HouseholdConfig::from_toml(TASKS).unwrap();
    assert_eq!(c.task_lists.len(), 2);
    assert_eq!(c.task_lists[1].child_can_add, None); // filled from the kind defaults on import
    assert_eq!(c.task_definitions[0].candidates, ["alex", "riley"]);
    assert_eq!(c.routines[0].steps.len(), 2);
    assert_eq!(
        c.members[1].notify.as_ref().unwrap().copy_to.as_deref(),
        Some("alex")
    );
    assert_eq!(c.rrules().count(), 1);
    assert_eq!(c.conditions().count(), 2);
}

#[test]
fn example_household_has_no_task_data() {
    let c = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    assert!(c.task_lists.is_empty() && c.task_definitions.is_empty() && c.routines.is_empty());
    assert!(c.members.iter().all(|m| m.notify.is_none()));
    assert!(c.weather_rules.iter().all(|r| r.task_definition.is_none()));
}

fn err(toml: &str) -> String {
    HouseholdConfig::from_toml(toml).unwrap_err().to_string()
}

#[test]
fn validation_errors() {
    let e = err(&TASKS.replace("name = \"Homework\"", "name = \"Chores\""));
    assert!(e.contains("duplicate task list name 'Chores'"), "{e}");
    let e = err(&TASKS.replace(
        "list = \"Chores\"\ntitle = \"Water",
        "list = \"Nope\"\ntitle = \"Water",
    ));
    assert!(e.contains("unknown task list 'Nope'"), "{e}");
    let e = err(&TASKS.replace("candidates = [\"riley\"]", "candidates = [\"ghost\"]"));
    assert!(e.contains("unknown member key 'ghost'"), "{e}");
    let e = err(&TASKS.replace("every_days = 3", "every_days = 3\nrrule = \"FREQ=DAILY\""));
    assert!(e.contains("exactly one of every_days and rrule"), "{e}");
    let e = err(&TASKS.replace("every_days = 3\n", ""));
    assert!(e.contains("exactly one of every_days and rrule"), "{e}");
    let e = err(&TASKS.replace("strategy = \"rotation\"", "strategy = \"fixed\""));
    assert!(e.contains("fixed strategy needs exactly one"), "{e}");
    let e = err(&TASKS.replace("candidates = [\"riley\"]", "candidates = []"));
    assert!(e.contains("at least one candidate"), "{e}");
    let e = err(&TASKS.replace("season_until = \"10-31\"", "season_until = \"13-31\""));
    assert!(e.contains("season_until '13-31' must be MM-DD"), "{e}");
    let e = err(&TASKS.replace("season_until = \"10-31\"\n", ""));
    assert!(e.contains("both season_from and season_until"), "{e}");
    let e = err(&TASKS.replace("members = [\"riley\"]", "members = [\"ghost\"]"));
    assert!(
        e.contains("routine 'Morning': unknown member key 'ghost'"),
        "{e}"
    );
    let e = err(&TASKS.replace("School/Term dates", "School"));
    assert!(e.contains("school_calendar"), "{e}");
    let e = err(&TASKS.replace("quiet_from = \"21:30\"", "quiet_from = \"25:30\""));
    assert!(e.contains("quiet_from '25:30' must be HH:MM"), "{e}");
    let e = err(&TASKS.replace("copy_to = \"alex\"", "copy_to = \"riley\""));
    assert!(e.contains("must be an adult"), "{e}");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_export_round_trip_and_idempotent(pool: PgPool) {
    let store = Store::from_pool(pool);
    let cfg = HouseholdConfig::from_toml(TASKS).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge, false)
        .await
        .unwrap();
    assert_eq!(r.task_lists_created, ["Chores", "Homework"]);
    assert_eq!(r.task_definitions_created.len(), 2);
    assert_eq!(r.routines_created, ["Morning"]);
    assert_eq!(r.notify_created, ["riley"]);

    let out = export(&store, r.household_id).await.unwrap();
    assert_eq!(out.task_lists.len(), 2);
    assert_eq!(out.task_lists[0].claim_ttl_minutes, Some(60));
    assert_eq!(
        out.task_lists[1].child_can_add,
        Some(fa_domain::ChildCanAdd::Own)
    );
    assert_eq!(out.task_definitions[0].title, "Take out the bins");
    assert_eq!(out.task_definitions[0].candidates, ["alex", "riley"]);
    assert_eq!(
        out.task_definitions[0].season_from.as_deref(),
        Some("04-01")
    );
    assert_eq!(
        out.task_definitions[1].on_missed,
        Some(fa_domain::OnMissed::RollOver)
    );
    assert_eq!(
        out.routines[0].school_calendar.as_deref(),
        Some("School/Term dates")
    );
    assert_eq!(out.routines[0].steps[1].target_time, None);
    let n = out.members[1].notify.as_ref().unwrap();
    assert_eq!(n.due_soon_minutes, Some(90));
    assert_eq!(n.copy_to.as_deref(), Some("alex"));
    assert_eq!(
        out.weather_rules[0].task_definition.as_deref(),
        Some("Take out the bins")
    );
    assert_eq!(
        out.weather_rules[0].task_mode,
        Some(fa_domain::TaskMode::Suggest)
    );
    assert_eq!(out.household.now_list_size, Some(5));
    assert_eq!(out.household.quiet_from.as_deref(), Some("20:30"));

    let first = out.to_toml().unwrap();
    assert!(!first.contains("topic"), "{first}");
    let again = HouseholdConfig::from_toml(&first).unwrap();
    let r2 = import(
        &store,
        Some(r.household_id),
        &again,
        ImportMode::Merge,
        false,
    )
    .await
    .unwrap();
    assert!(r2.task_lists_unchanged.len() == 2 && r2.task_lists_updated.is_empty());
    assert!(r2.task_definitions_unchanged.len() == 2 && r2.task_definitions_updated.is_empty());
    assert_eq!(r2.routines_unchanged, ["Morning"]);
    assert_eq!(r2.notify_unchanged, ["riley"]);
    assert!(r2.rules_unchanged.len() == 1 && !r2.household_updated);
    let second = export(&store, r.household_id)
        .await
        .unwrap()
        .to_toml()
        .unwrap();
    assert_eq!(first, second);

    // Topics are kept across imports; a schedule change resets next_due_at.
    let mut tx = store.tx_for(r.household_id).await.unwrap();
    let before = fa_store::TaskDefinitionRepo::list(&mut tx, None, false)
        .await
        .unwrap();
    assert!(before.iter().all(|d| d.value.next_due_at.is_none()));
    tx.rollback().await.unwrap();

    let mut changed = cfg.clone();
    changed.task_definitions[1].every_days = Some(5);
    changed.routines[0].steps.push(RoutineStepConfig {
        text: "Shoes".into(),
        target_time: None,
    });
    let r3 = import(
        &store,
        Some(r.household_id),
        &changed,
        ImportMode::Merge,
        false,
    )
    .await
    .unwrap();
    assert_eq!(r3.task_definitions_updated, ["Water the plants"]);
    assert_eq!(r3.routines_updated, ["Morning"]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_rejects_unknown_weather_definition(pool: PgPool) {
    let store = Store::from_pool(pool);
    let mut cfg = HouseholdConfig::from_toml(TASKS).unwrap();
    cfg.weather_rules[0].task_definition = Some("Missing".into());
    let e = import(&store, None, &cfg, ImportMode::Merge, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("'Missing' not found"), "{e}");
}
