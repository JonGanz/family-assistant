---
id: T-05
title: "fa-tasks: scheduling, strategies, completion, routines, producers, jobs"
milestone: M2
wave: 3
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-tasks/**", "crates/fa-store/src/repo/ext_t05.rs"]
---

# T-05: fa-tasks: scheduling, strategies, completion, routines, producers, jobs

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Time semantics*, *Materialisation*, *Strategies*, *Completion, skip, undo*, *Routines*, *Now score*, *Notifications* (the producer table), *Service APIs* (`TaskService`), *Jobs*, *HTTP API v1 additions* (what fa-api will need from you)
- `arch/Tasks.md` §3 to §5
- `crates/fa-domain` (T-01's `tasks`, `routines`, `notify` helpers and DTOs), `crates/fa-store` (T-04's repos), `crates/fa-calendar/src/jobs.rs` and `service.rs` (job and service patterns, iterating households), `crates/fa-jobs/src/lib.rs`

## You own
- `crates/fa-tasks/**` (the crate exists, empty, from W0; add workspace dependencies you need to its `Cargo.toml`: `fa-store`, `fa-jobs`, `chrono`, `chrono-tz`, `rrule`, `serde`, `serde_json`, `thiserror`, `tracing`, `uuid`, `sqlx`, and `tokio` for tests)
- `crates/fa-store/src/repo/ext_t05.rs` (already registered in `repo/mod.rs`): add repo methods you need that T-04 didn't provide, as `impl XxxRepo { … }` blocks. **Don't run `cargo sqlx prepare`**: build online against the dev DB (`DATABASE_URL` from `.env.dev`); the main session regenerates `.sqlx` after the wave.

## Work
1. `schedule.rs`: next-due computation for both schedule kinds, RRULE parsing and expansion with `rrule 0.14` in the household tz (`validate_rrule(&str) -> Result<(), String>`, `preview_dates(schedule, start_date, due_time, tz, from, n)`), season moves, condition gating. Exactly the plan's *Time semantics*.
2. `strategy.rs`: `AssignmentStrategy` trait with `Fixed`, `Rotation`, `Pool` (plan *Strategies*), using `MemberAwayRepo::away_on`.
3. `service.rs` — `TaskService` with the methods in the plan's *Service APIs* (the names are the contract for T-09). It returns fa-domain DTOs (`TaskView` with `score` via `now_score`, `assignee_away`, `undo_until = completed_at + 10 min` when applicable; `TaskDefinitionView` with `next_assignee`). Every mutation publishes `tasks.updated` (or `routines.updated{date}`) through `ChangeBus` inside its transaction. Authorisation is **not** done here (fa-api does it), but the service enforces data rules: completion only for `open` tasks, claim only on pool tasks by candidates (adults may claim), skip and accept only on the right statuses (return typed errors that fa-api maps to 409/422).
   - `tasks.materialize` first initialises definitions with `next_due_at IS NULL` (written that way by the config import).
   - Creating a definition sets `next_due_at` (plan *Materialisation*, last paragraph) and defaults `on_missed` with `fa_domain::tasks::default_on_missed`. Updating its schedule recomputes `next_due_at`.
   - `list_tasks` implements `view=now|mine|all`, the status filter, and the `from`/`to` calendar rule, and the child scope: a child sees tasks assigned to them, plus unassigned tasks.
4. `jobs.rs`: handlers for `tasks.materialize`, `tasks.tick`, `tasks.digest`, `routines.materialize`, `tasks.from_weather` (payload `{household, rule, date}`: load the rule's `task_definition` and `task_mode`; create a task with `origin = weather`, `weather_rule_id`, `due_at` = due instant of `date` with the definition's due_time, status `suggested` when the mode is `suggest`, assignees by the definition's strategy, and skip it if the rule has no definition). `default_schedules()` per the plan. Cron jobs iterate households like `calendar.sync_due`.
5. **Producers:** insert outbox rows exactly per the plan's table (titles in plain language: "New task: Clean the toilet", "Overdue: …", "Good morning: 3 tasks today" with the titles in the body, "Routine: 2 steps left"). `click_path` = `/tasks/{id}` or `/routines`. Digest body lists today's open tasks and routines for the member, and is skipped (not inserted) when there is nothing.
6. **Tests** (`#[sqlx::test]` plus pure unit tests), at least:
   - after_done: completion moves `next_due_at`; roll_over keeps one occurrence; skip moves it from now;
   - rrule: weekly on Saturday with auto_skip creates one per week and skips the missed one; roll_over consumes instances; DST week in Europe/London;
   - lead_days and visible_from; season window wrapping the year end; condition_ok false defers, stale counts as true;
   - rotation: any_completion vs assignee_completion, skip_advances, absences;
   - pool claims and expiry; fixed with an away member → `assignee_away`;
   - undo within 10 minutes restores cursor, `last_done_at`, `next_due_at`, and deletes the untouched follow-up; after 10 minutes only reopens;
   - routines: weekdays, school_days with an all-day "No school" event, run done on last tick, yesterday's run skipped;
   - Now list order and size; the calendar range rule (done `always` included, done `until_done` excluded);
   - producers: assigned, overdue (once), due_soon, nudge, digest once per day after quiet hours;
   - `tasks.from_weather` idempotent per rule and day, `suggest` mode.

## Acceptance
- [ ] `cargo test -p fa-tasks` passes (dev DB online), and on a fresh throwaway Postgres (port 55405) with `DATABASE_URL` pointed at it (online macros against the fresh DB after `sqlx migrate run --source crates/fa-store/migrations` or by letting `#[sqlx::test]` migrate)
- [ ] `cargo clippy -p fa-tasks --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] Report the exact public API of `TaskService` (signatures) for T-09

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
