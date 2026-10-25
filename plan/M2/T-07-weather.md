---
id: T-07
title: "fa-weather: task rules fire, season conditions"
milestone: M2
wave: 3
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-weather/**", "crates/fa-store/src/repo/ext_t07.rs"]
---

# T-07: fa-weather: task rules fire, season conditions

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Time semantics* (Condition), *Jobs* (`tasks.from_weather`), *Service APIs* (fa-weather)
- `arch/Tasks.md` §4.5; `arch/Feeds.md` §4 (the DSL and evaluation)
- `crates/fa-weather/src` (service.rs `evaluate`, eval.rs, dsl/), `crates/fa-jobs/src/lib.rs` (`enqueue`), T-04's `TaskDefinitionRepo::with_condition` / `set_condition_result` and `WeatherRuleRepo` `task_mode`

## You own
- `crates/fa-weather/**`
- `crates/fa-store/src/repo/ext_t07.rs` (already registered) for any missing repo methods. **Don't run `cargo sqlx prepare`**.

## Work
1. In `evaluate`, rules with `action = task` now fire: when a rule matches with a `key_date`, enqueue `tasks.from_weather` with payload `{household, rule, date}` and idempotency key `tasks.from_weather:{rule}:{date}` (use `fa_jobs::enqueue`). Rules without a `task_definition` are logged at debug and skipped. No advisory is created for task rules.
2. After the rules, evaluate every `task_definition.season_condition` of the household against the same snapshot and write `condition_ok` and `condition_checked_at = now` (unparsable conditions: `condition_ok = NULL`, warn once per evaluation). The condition's truth is the evaluation's `matches`.
3. Export `pub fn validate_condition(s: &str) -> Result<(), ParseError>` (thin wrapper over `parse`), for fa-api and the config import.
4. `WeatherService` rule CRUD (if it lives here) carries `task_mode` and validates that a `task` rule's `task_definition` exists in the household.
5. **Tests:** a matching task rule enqueues exactly one job per rule and date across two evaluations; a non-matching one enqueues none; season conditions set true/false/NULL; validate_condition.

## Acceptance
- [ ] `cargo test -p fa-weather` passes (dev DB online; fresh Postgres on port 55407)
- [ ] `cargo clippy -p fa-weather --all-targets -- -D warnings` and `cargo fmt --check` are clean

## Wave 2 outcomes (read before starting)
T-04 is done (migration `20261026000001_m2_tasks`, applied to the dev DB; repos in `crates/fa-store/src/repo/{task_list,task_definition,task,routine,notification}.rs`). Differences from the plan's repo list:
- `TaskListRepo::update(conn, id, &TaskListPatch, expected_version)`.
- `TaskRepo::update(conn, &Task, assignees: Option<&[MemberId]>, steps: Option<&[String]>, expected_version) -> i32`.
- `RoutineRepo::get/get_by_name/list` return `RoutineFull { routine: Versioned<Routine>, members, steps }`; `members()` and `steps()` exist too. `RoutineRunRepo::ensure(conn, routine, member, date, &[RoutineStepId])`. `RunRow` includes `show_on_dashboard`; `for_date` and `open_with_steps_left` skip archived routines.
- `TaskRepo::set_claim(conn, id, Option<MemberId>, Option<DateTime>)` (None releases). `due_soon_candidates(conn, now, horizon: chrono::Duration)`. `overdue_open` and `due_soon_candidates` return only tasks with assignees.
- `NotifyPrefRepo::upsert_for_member(conn, &NotifyPref, expected_version: Option<i32>)`. `NotificationRepo::mark_sent(&[NotificationId])`; `get` exists. `MemberAwayRepo::get` exists.
- Versions: `TaskRepo::set_status`, `reopen`, `set_claim`, `clear_expired_claims` bump the task version; `tick_step` doesn't. `TaskDefinitionRepo::set_schedule_state` and `set_condition_result` don't bump it. `TaskDefinitionRepo::update` writes `next_due_at` from the struct (pass `None` on a schedule change so materialize re-initialises it) and doesn't touch `rotation_cursor`, `last_done_at`, or condition columns.
- `TaskRepo::set_status` stamps `completed_at` with the DB `now()`.
- `all_day_titles` matches `start_date <= date < end_date` (end exclusive). Step and candidate `ord` are 0-based (matching `rotation_cursor`).
- Your extension file in `crates/fa-store/src/repo/` exists and is registered in `mod.rs`.
- **The M2 migration is now applied to the dev DB: never edit it.** If you need a schema change, stop and report it (the main session adds a new migration).
