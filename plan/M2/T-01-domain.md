---
id: T-01
title: "fa-domain: M2 IDs, enums, DTOs, pure helpers, authz"
milestone: M2
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-domain/**"]
---

# T-01: fa-domain: M2 IDs, enums, DTOs, pure helpers, authz

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Domain types*, *Time semantics*, *Strategies*, *Now score*, *Notifications*, *HTTP API v1 additions*
- `arch/Tasks.md` §3, §4, §5, §7
- `crates/fa-domain/src` (ids.rs, entities.rs, feeds.rs, api.rs, authz.rs, dashboard.rs) for the existing patterns (ID newtypes, enum derives, `ts(export_to = …)`, ToSchema)

## You own
- `crates/fa-domain/**`

## Work
1. **IDs and enums** exactly as listed in the plan, following the existing derive sets. Every enum has `as_str()` and `FromStr` matching the DB text values in the plan's migration table.
2. **Entities** (plain structs used by fa-store and services): `TaskList`, `TaskDefinition` (with `Schedule` and an optional season as `(i32, i32)` MMDD), `Task`, `TaskStep`, `Routine`, `RoutineStep`, `RoutineRun`, `RoutineRunStep`, `MemberAway`, `Notification`, and the M2 fields on `Household` (`now_list_size`, `now_weights`, `quiet_from`, `quiet_until`) and `WeatherRule` (`task_mode`). Adding fields to `Household` and `WeatherRule` breaks constructors in other crates: list every compile error outside fa-domain in your report (the main session fixes them before Wave 2), and keep defaults easy (`NowWeights::default()`).
3. **DTOs** from the plan's list, with serde + `utoipa::ToSchema` + `ts_rs::TS` (`export_to` like the M1 DTOs). Times of day serialise as `"HH:MM"`; seasons as `"MM-DD"`. `HouseholdView`/`HouseholdPatch`, `WeatherRuleView` and its inputs, and `DashboardView` gain the M2 fields.
4. **Pure helpers** (no I/O), each with unit tests:
   - `tasks::list_defaults(kind)` (arch/Tasks.md §3.2);
   - `tasks::due_instant(date, due_time: Option<NaiveTime>, tz) -> DateTime<Utc>` (23:59 when None; on a DST gap use the first valid instant after; on an overlap use the earlier);
   - `tasks::visible_from(date, lead_days, tz)`;
   - `tasks::in_season(mmdd, from, until)` (wrapping) and `tasks::next_in_season(date, from, until) -> NaiveDate`;
   - `tasks::default_on_missed(ScheduleKind)` (after_done → roll_over, rrule → auto_skip);
   - `tasks::now_score(&ScoreInput{visible_from, due_at, priority_flag, origin}, &NowWeights, now) -> f64` exactly as the plan's formula;
   - `tasks::parse_mmdd("MM-DD") -> Result<i32>` / `format_mmdd`;
   - `routines::day_qualifies(filter, date, school_titles: Option<&[String]>, keywords: &[String]) -> bool`;
   - `notify::effective_quiet`, `notify::in_quiet` (wraps midnight; equal from/until means no quiet hours), `notify::quiet_end_after(now, from, until, tz)`.

   Tests include DST days in Europe/London (2026-03-29) and America/New_York (2026-03-08), a season wrapping the year end (11-01 to 02-28), and quiet hours across midnight.
5. **Authz:** change `Resource::Task` to `Task{assignees, created_by, definition}`, add `Resource::TaskList{child_can_add, dashboard_add}`, and add the actions `ManageTasks`, `AddTask`, `EditTask`, `ClaimTask`, `TickRoutineStep`, `ManageNotifyPrefs` with the semantics in the plan and arch/Tasks.md §7. Update `Action::ALL` and extend the exhaustive matrix tests for every new action × actor (adult, child, device, device + elevation, system). Devices: `AddTask` only on `dashboard_add` lists; `CompleteTask` and `TickRoutineStep` while the override is on; nothing else.
6. Run `cargo test -p fa-domain` so ts-rs writes the TS files (they land in `apps/web/src/gen/types/`, which is fine; the main session regenerates later).

## Acceptance
- [ ] `cargo test -p fa-domain` passes
- [ ] `cargo clippy -p fa-domain --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` is clean
- [ ] Report lists every compile error in other crates caused by the `Household`/`WeatherRule`/`Resource::Task` changes (`CARGO_INCREMENTAL=0 cargo check --workspace --all-targets 2>&1 | grep -E '^error' -A5`)
