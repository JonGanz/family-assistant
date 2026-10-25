---
id: T-04
title: "fa-store: M2 migration, repos, .sqlx"
milestone: M2
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-store/**", ".sqlx/**"]
---

# T-04: fa-store: M2 migration, repos, .sqlx

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Migration* (exact tables and columns), *Time semantics*, *Materialisation*, *Strategies*, *Completion, skip, undo*, *Routines*, *Notifications*
- `arch/Tasks.md` §3; `arch/Domain.md` *Persistence conventions*
- `crates/fa-store`: migrations `20261024000002_identity.up.sql` (RLS, grants, triggers pattern) and `20261025000001_m1_feeds.up.sql`; `store.rs` (`tx_for`/`tx_system`); `repo/*` (patterns, `Versioned`, `conflict_or_missing`); `repo/notify.rs`; `change.rs`
- `crates/fa-domain` as finished by T-01 (the M2 entities, enums, DTOs)

## You own
- `crates/fa-store/**`
- `.sqlx/**`

## Work
1. **Migration** `migrations/20261026000001_m2_tasks.{up,down}.sql` with exactly the plan's table. Enums are `text` + CHECK with the T-01 `as_str()` values. RLS (`ENABLE` + `FORCE`), the `tenant` policy, `set_updated_at` triggers, and grants to `fa_app` on every new table, as in 0002. The down migration drops everything in reverse (and the added columns). **Never edit earlier migrations.** Apply it to the dev DB by running the migration (allowed: schema only; don't read rows).
2. **Repositories** (associated functions on `&mut PgConnection`; versioned entities return `Versioned<T>`; `expected_version` mismatches → `StoreError::Conflict`). These are the methods Wave 3 relies on; add more if useful, but don't rename these:
   - **HouseholdRepo** (extend): the M2 fields in `get`/`list` and in `update_settings`.
   - **TaskListRepo:** `insert`, `get`, `list(include_archived)`, `update(&patch, expected_version)`, `archive`, `get_by_name`.
   - **TaskDefinitionRepo:** `insert(&new, candidates: &[MemberId])`, `get`, `list(list: Option<TaskListId>, include_archived)`, `update(&def, candidates: Option<&[MemberId]>, expected_version)`, `archive`, `candidates(id) -> Vec<(MemberId, i32)>`, `due_for_materialize(conn, now) -> Vec<(HouseholdId, TaskDefinitionId)>` (system tx: active, unpaused, and either `next_due_at IS NULL` or `next_due_at - (lead_days + 1) days <= now`; fa-tasks checks precisely with the tz), `lock(conn, id) -> Option<TaskDefinition>` (`FOR UPDATE SKIP LOCKED`), `set_schedule_state(conn, id, rotation_cursor, last_done_at, next_due_at)` (no version bump: this is system state, but do bump `updated_at`), `with_condition(conn) -> Vec<(TaskDefinitionId, String)>`, `set_condition_result(conn, id, ok, checked_at)`, `get_by_title(list, title)`.
   - **TaskRepo:** `insert(&NewTaskRow, assignees, steps: &[String]) -> Option<Versioned<Task>>` (`None` when the `(definition_id, origin, due_at)` unique key already exists: use `ON CONFLICT DO NOTHING`), `get`, `get_view_rows(&TaskFilter) -> Vec<TaskRow>` (a `TaskRow` carries the task, its list name and list `completion_policy`, assignee ids, steps, so services build `TaskView` without N+1; the filter covers ids, status set, list, member (assignee), due range, `visible_before`, `calendar_display_not_hidden`), `open_for_definition(def) -> Vec<Task>`, `update(&task, expected_version)` (title, notes, due, priority, calendar_display, assignees, steps replacement keeping ticks for unchanged texts), `set_status(conn, id, status, by, device, completion_undo: Option<Value>)` (sets/clears `completed_*`, `skipped_by`), `reopen(conn, id)`, `delete_created_after(def, after) -> u64` (only open tasks with no ticked steps), `set_claim(conn, id, member, until)`, `clear_expired_claims(now) -> u64`, `tick_step(conn, task, step, done, by)`, `archive`, `overdue_open(now) -> Vec<(TaskId, Vec<MemberId>)>`, `due_soon_candidates(now, horizon) -> Vec<(TaskId, DateTime<Utc>, Vec<MemberId>, String)>`.
   - **RoutineRepo:** `insert(&new, members, steps)`, `get`, `list(include_archived)`, `update(.., members: Option, steps: Option, expected_version)` (steps replacement keeps ids for unchanged `ord`+text), `archive`, `get_by_name`.
   - **RoutineRunRepo:** `ensure(conn, routine, member, date, steps) -> bool` (idempotent; creates run steps), `for_date(conn, date, member: Option) -> Vec<RunRow>` (with routine name, window, steps and ticks), `tick(conn, run, step, done, by, device) -> RunStatus` (updates the run status: done when all steps ticked, open otherwise), `skip_before(conn, date) -> u64`, `open_with_steps_left(conn, date) -> Vec<RunRow>`.
   - **MemberAwayRepo:** `insert`, `list(member: Option)`, `delete`, `away_on(conn, date) -> Vec<MemberId>`.
   - **NotifyPrefRepo** (extend): the new columns; `get_for_member(member) -> Option<Versioned<NotifyPref>>`, `upsert_for_member(..)`, `list_all(conn)`.
   - **NotificationRepo:** `insert_dedupe(conn, &NewNotification) -> bool` (`ON CONFLICT (household_id, dedupe_key) DO NOTHING`), `pending(conn, now, limit) -> Vec<Notification>` (system tx across households: `status = 'pending' AND not_before <= now`, oldest first), `mark_sent(ids, at)`, `mark_dropped(id, reason)`, `defer(id, not_before)`, `record_failure(id, error, next_try, give_up: bool)`.
   - **EventRepo** (extend): `all_day_titles(conn, calendar, date) -> Vec<String>` (all-day events covering `date`, not cancelled).
   - **WeatherRuleRepo** (extend): `task_mode` in reads and writes.
   - **CalendarSourceRepo** (extend): `needing_reauth(conn) -> Vec<(CalendarSourceId, Option<MemberId>, Option<DateTime<Utc>>)>`.
   - **MemberRepo** (extend, if missing): `adults(conn)`.
3. **Tests** (`#[sqlx::test]`): migration up and down; RLS isolates two households on every new table; the `(definition_id, origin, due_at)` dedupe; `lock` skips a row locked by another transaction; step replacement keeps ticks; run `tick` sets done/open; `insert_dedupe` idempotent; `pending` ordering and `not_before`; `all_day_titles` covers multi-day all-day events; version conflicts.
4. Run `cargo sqlx prepare --workspace` against the migrated dev DB and keep the offline data current.

## Acceptance
- [ ] `cargo test -p fa-store` passes on the dev DB **and** on a fresh throwaway Postgres (see the rules for the container recipe; port 55404)
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-store --all-targets -- -D warnings` and `cargo fmt --check` are clean

## Wave 1 outcomes (read before starting)
- T-01 is done. Entities carry `household_id`, `created_at`, `updated_at` but **no `version`** (that lives in `Versioned<T>`). Candidates, assignees, routine members and steps are separate tables and not on the entity structs; return them alongside (tuples or row structs) as your repo signatures say.
- Enums use a `db_enum!` macro (`as_str`, `FromStr`, `ALL`). `ListCompletionPolicy` gained `sqlx::Type`, `as_str`, `FromStr`.
- `Household` has `now_list_size`, `now_weights: fa_domain::tasks::NowWeights`, `quiet_from`, `quiet_until` (`NaiveTime`), and `WeatherRule` has `task_mode`. **The main session put interim defaults** in `crates/fa-store/src/repo/household.rs` (row mapping) and `repo/weather_rule.rs`, and in test fixtures across crates. Replace the fa-store ones with the real columns (reads, `insert`, `update_settings`, and the weather rule insert/update); fixtures in other crates may stay.
- `HouseholdPatch` gained `now_list_size`, `now_weights`, `quiet_from`, `quiet_until` (all `Option`).
- Additional DTOs exist beyond the plan (`ValidateDefinitionRequest/Response`, `RoutineStepInput`, `RoutineStepView`); they don't affect the store.
