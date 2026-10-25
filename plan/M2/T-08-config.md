---
id: T-08
title: "fa-config: ntfy env, TOML sections, round trip"
milestone: M2
wave: 3
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-config/**", "crates/fa-store/src/repo/ext_t08m2.rs"]
---

# T-08: fa-config: ntfy env, TOML sections, round trip

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Process config*, *Household TOML additions*, *Migration*
- `arch/Tasks.md` §8; decision `decisions/D-018-hybrid-config.md`, `D-047-no-starter-templates.md`
- `crates/fa-config/src` (process.rs, household.rs, feeds.rs: how M1 added sections, import modes, export, validation), `config/examples/household.example.toml`, T-04's repos

## You own
- `crates/fa-config/**`
- `crates/fa-store/src/repo/ext_t08m2.rs` (already registered) for any missing repo methods. **Don't run `cargo sqlx prepare`**.

## Work
1. **Process config:** `ntfy_url: Option<Url>` (`FA_NTFY_URL`) and `ntfy_token: Option<SecretString>` (`FA_NTFY_TOKEN`, redacted in Debug), following the existing keys. Empty strings mean unset.
2. **TOML sections** exactly as the plan lists: `[household]` additions; `[[task_lists]]`, `[[task_definitions]]`, `[[routines]]` (with `steps` as an array of inline tables), per-member `[members.notify]`, and `task_mode` + `task_definition` on `[[weather_rules]]`. Omitted list fields take `fa_domain::tasks::list_defaults(kind)`; omitted `on_missed` takes `default_on_missed`.
3. **Validation** (pure, in `validate()`): list names unique; definitions reference existing lists and member keys; exactly one of `every_days` and `rrule`; `fixed` has one candidate, others at least one; seasons as `MM-DD` in pairs; routines reference member keys and an optional `"source/calendar"`; quiet times parse; `copy_to` is an adult. RRULE syntax and DSL conditions are **not** validated here (fa-server does that through fa-tasks and fa-weather before calling import); expose the strings so fa-server can (`HouseholdConfig::rrules()`, `::conditions()` iterators, or similar; report what you chose).
4. **Import** (existing modes): upsert lists by name, definitions by (list, title) with candidates in order, routines by name with members and steps, notify prefs per member (never topics; existing topics are kept), weather rule `task_mode`/`task_definition` by title. Imported definitions are written with `next_due_at = NULL` when new, or when their schedule fields changed: `tasks.materialize` initialises NULL ones (plan *Materialisation*).
5. **Export:** the new sections, without topics, without absences, without runs or task occurrences.
6. **Tests:** parse the plan's examples; validation errors; import then export round-trips (`#[sqlx::test]`); import is idempotent; the example household TOML still parses and has **no** task data (D-047).

## Acceptance
- [ ] `cargo test -p fa-config` passes (dev DB online; fresh Postgres on port 55408)
- [ ] `cargo clippy -p fa-config --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] Report the TOML snippet you used in tests (for the README's configuration reference)

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
