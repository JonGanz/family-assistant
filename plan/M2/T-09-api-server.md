---
id: T-09
title: "fa-api + fa-server: M2 routes, dashboard, SSE, wiring, CLI"
milestone: M2
wave: 4
depends_on: [T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-api/**", "bins/fa-server/**"]
---

# T-09: fa-api + fa-server: M2 routes, dashboard, SSE, wiring, CLI

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: **HTTP API v1 additions (the whole table)**, *Service APIs*, *Jobs*, *Process config*, *Household TOML additions*, *Domain types* (authz)
- `arch/Tasks.md` §6, §7; `arch/Frontend.md` §4.2 (M2 dashboard data)
- `crates/fa-api` (lib.rs routing, routes.rs and feeds.rs handler patterns, extract.rs actors, error.rs, openapi.rs), `bins/fa-server` (wiring.rs, serve.rs, ops.rs, cli.rs)
- The public APIs of `fa-tasks` (`TaskService`), `fa-notify` (`NotifyService`), `fa-weather` (`validate_condition`), `fa-config` (the M2 sections and ntfy keys). The Wave 3 reports are summarised in *Wave 3 outcomes* below.

## You own
- `crates/fa-api/**`
- `bins/fa-server/**`

## Work
1. **Routes:** every row of the plan's *HTTP API v1 additions* table, with exactly those paths, actors, DTOs, and status codes. Put them in a new `tasks.rs` (and `notify.rs`) module. Handlers stay thin: build the actor, load what the policy needs (the task's assignees and `created_by`, the list flags, the household override), call `Policy::authorize` with the M2 actions, call the service, map errors to problem+json (`409` conflicts and wrong states, `422` validation with `{field, position?, message}`, `503`/`502` for the notify test).
   - Devices must send `member` on complete, step ticks, and adds; reject a member who isn't in the household.
   - Children: `GET /tasks` and `/routines/today` are scoped to them by the service's `Viewer`; never return another child's tasks.
   - `POST /task-definitions` and `PATCH` validate RRULEs (`fa_tasks::validate_rrule`) and `season_condition` (`fa_weather::validate_condition`); `/task-definitions/validate` returns `{ok, errors[], next_dates[]}`.
   - `GET/PUT /members/{id}/notify-prefs` hide `topic` and `subscribe_url` from anyone but that member and adults.
2. **Household:** `GET/PATCH /household` carry the M2 fields; validate `now_list_size` (1..=50), weights (0..=10), and times.
3. **Weather rules:** the M1 rule routes carry `task_mode` and `task_definition`.
4. **Dashboard:** `GET /dashboard` adds `tasks` (`TaskService::dashboard_tasks`) and `routines` (`dashboard_routines`), filtered by the device's `members` setting like events.
5. **SSE:** `tasks.updated` and `routines.updated` already come through the ChangeBus bridge; make sure the OpenAPI/stream docs list them.
6. **OpenAPI:** every new route and DTO in `openapi.rs`, so `fa-server gen-openapi` produces the web client types.
7. **fa-server:**
   - wiring: build `TaskService` and `NotifyService` (`NotifyConfig` from `FA_NTFY_URL`, `FA_NTFY_TOKEN`, `FA_PUBLIC_BASE_URL`), add them to `Services`, register their handlers and default schedules.
   - `config import`: before importing, validate every RRULE and `season_condition` from the TOML (report all errors with their location), like the M1 DSL check.
   - CLI: `fa-server notify test --member <key>` (prints the subscribe URL on success, a clear error when ntfy is not configured) and `fa-server tasks run-now` (runs one pass of `tasks.materialize`, `tasks.tick`, and `routines.materialize` for the household; useful for demos and e2e).
8. **Tests** (`crates/fa-api/tests/m2.rs`, using the existing test harness): the authz matrix through HTTP for complete, add, edit, skip, claim, step ticks, routine ticks, notify prefs (adult, child, device with override on and off); child scoping; a device complete without `member` → 422; the Now view; the calendar range; definitions with a bad RRULE and a bad condition → 422; dashboard includes tasks and routines; notify test → 503 when not configured.

## Acceptance
- [ ] `cargo test -p fa-api` and `cargo test -p fa-server` pass (dev DB online; fresh Postgres on port 55409 with `SQLX_OFFLINE=true`)
- [ ] `SQLX_OFFLINE=true cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo fmt --check` is clean
- [ ] `cargo run -q -p fa-server -- gen-openapi --out /tmp/claude-openapi-check.json` works (don't write into `apps/web/src/gen`; the main session regenerates it)
- [ ] Report: new CLI commands and env vars (for the README)

## Wave 3 outcomes (read before starting)
- **T-07 (fa-weather):** task rules enqueue `tasks.from_weather{household, rule, date}` after commit; season conditions write `condition_ok` (true/false/NULL). `fa_weather::validate_condition(&str)` exists. Rule CRUD lives in fa-api (`feeds.rs`), so **you** carry `task_mode` and `task_definition` there.
- **Contract (main session):** `weather_rule.task_definition` is jsonb `{"definition": "<TaskDefinitionId>"}`. In `WeatherRuleView` and the rule inputs it's `Option<serde_json::Value>` (from T-01). Validate on create/patch: when `action = task`, `task_definition` must be `{"definition": <id>}` naming an existing, non-archived definition of the household (422 otherwise); when `action = alert`, it must be null.
- **T-06 (fa-notify) public API:**
  ```rust
  NotifyConfig { ntfy_url: Option<String>, ntfy_token: Option<String>, public_base_url: String }
  NotifyService::new(store, config) -> NotifyService   // infallible, Clone
  get_prefs(&self, HouseholdId, MemberId) -> Result<NotifyPrefView, NotifyError>
  put_prefs(&self, HouseholdId, MemberId, NotifyPrefPatch, expected_version: i32) -> Result<NotifyPrefView, NotifyError>
  send_test(&self, HouseholdId, MemberId) -> Result<(), NotifyError>
  fa_notify::handlers(NotifyService) -> Vec<Arc<dyn JobHandler>>   // not Result
  fa_notify::default_schedules()
  enum NotifyError { NotConfigured, Publish(String), Invalid(String), NotFound, Store(StoreError) }
  ```
  Map `NotConfigured` → 503, `Publish` → 502, `Invalid` → 422, `NotFound` → 404, `Store(Conflict)` → 409. `NotifyPrefView.topic` and `subscribe_url` are always filled: **fa-api must blank them** for anyone who is neither that member nor an adult. `public_base_url` comes from `FA_PUBLIC_BASE_URL`. Plain `http://` ntfy URLs are allowed.
- **T-08 (fa-config):** `ProcessConfig.ntfy_url: Option<Url>`, `ntfy_token: Option<SecretString>`. For `config import`, validate first with `cfg.rrules()` (yields `(definition title, rrule)`) through `fa_tasks::validate_rrule`, and `cfg.conditions()` (yields `(label, condition)`, covering weather-rule conditions and `season_condition`) through `fa_weather::validate_condition`; report all errors, then import. `ImportReport` gained `task_lists_*`, `task_definitions_*`, `routines_*`, `notify_*` lists: print them in the CLI summary. Weather-rule `task_definition` is resolved by title on import (`import_rule_tasks`).
- **T-05 (fa-tasks) public API** (`fa_tasks::Result<T>`, error `TaskError`):
  ```rust
  TaskService::new(store) -> Self      // with_clock(store, Clock) for tests
  // Viewer { member: Option<MemberId>, role: Option<Role>, device: bool }
  // TaskQuery { view: TaskViewMode(Now|Mine|All), member, list, status: Option<StatusFilter(Open|Done|Skipped|Suggested|Any)>, from, to }
  list_tasks(household, &TaskQuery, Viewer) -> Vec<TaskView>      // status None = open; from/to applies the calendar rule
  get_task(household, TaskId) -> TaskView
  create_task(household, NewTask, by: Option<MemberId>, device: Option<DeviceId>) -> TaskView
  update_task(household, TaskId, TaskPatch, expected_version: i32) -> TaskView
  archive_task(household, TaskId)
  complete(household, TaskId, member: Option<MemberId>, device: Option<DeviceId>) -> TaskView
  reopen(household, TaskId) / skip(household, TaskId, by: Option<MemberId>) / accept(household, TaskId)
  claim(household, TaskId, member) / release(household, TaskId, member)
  tick_step(household, TaskId, TaskStepId, done: bool, member: Option<MemberId>, device: Option<DeviceId>) -> TaskView
  dashboard_tasks(household, from, to) -> Vec<TaskView>; dashboard_routines(household) -> Vec<RoutineRunView>
  list_task_lists / get_task_list / create_task_list(TaskListInput) / update_task_list(id, TaskListPatch, v) / archive_task_list
  list_definitions(household, Option<TaskListId>) / get_definition / create_definition(TaskDefinitionInput) / update_definition(id, TaskDefinitionPatch, v) / archive_definition
  list_routines / get_routine / create_routine(RoutineInput) / update_routine(id, RoutinePatch, v) / archive_routine
  routines_today(household, member: Option<MemberId>) -> Vec<RoutineRunView>
  tick_routine_step(household, RoutineRunId, RoutineStepId, done, member: Option<MemberId>, device: Option<DeviceId>) -> RoutineRunView
  list_away(household, member: Option) / add_away(household, member, NewAway) -> MemberAwayView / delete_away(household, MemberAwayId)
  materialize_all(now) / tick_all(now) / digest_all(now) / routines_materialize_all(now)   // use for `tasks run-now`
  fa_tasks::{validate_rrule(&str) -> Result<(), String>, preview_dates(&Schedule, start_date, due_time, tz, from, n), handlers(Arc<TaskService>), default_schedules()}
  ```
  Error mapping: `NotFound` → 404; `Conflict`, `WrongStatus{..}`, `ClaimedByOther` → 409; `Invalid{field, position, message}`, `NotCandidate`, `NotPool` → 422. fa-tasks does **not** validate `season_condition`: you do (`fa_weather::validate_condition`) on definition create/patch and in `/task-definitions/validate`. Weather tasks have `definition_id = NULL` and `weather_rule_id` set.
- **Main session after Wave 3:** `.sqlx` regenerated; `scripts/ci-local.sh` is green (fresh Postgres): Rust 362 passed, web 134 passed. The main session also added `task_mode`/`task_definition` to the `WeatherRuleConfig` literal in `ops.rs` tests and `WeatherError::Job` mapping in `error.rs`, and `HouseholdView`/`WeatherRuleView`/`DashboardView` placeholders in `feeds.rs` (`tasks: Vec::new()`, `routines: Vec::new()`): replace those with the real data.
