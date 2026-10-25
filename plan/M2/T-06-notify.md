---
id: T-06
title: "fa-notify: outbox drain, prefs, quiet hours, batching, copies, test"
milestone: M2
wave: 3
depends_on: [T-02, T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-notify/**", "crates/fa-store/src/repo/ext_t06m2.rs"]
---

# T-06: fa-notify: outbox drain, prefs, quiet hours, batching, copies, test

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Notifications* (all of it), *Service APIs* (`NotifyService`), *Jobs*, *HTTP API v1 additions* (the notify-prefs rows)
- `arch/Tasks.md` §5; decision `decisions/D-044-task-notifications.md`
- `crates/fa-connectors/src/ntfy.rs` (T-02), `crates/fa-domain` (`notify` helpers, `NotifyPrefView`), `crates/fa-store` (T-04's `NotifyPrefRepo`, `NotificationRepo`, `CalendarSourceRepo::needing_reauth`, `MemberRepo`), `crates/fa-weather/src/jobs.rs` (job pattern)

## You own
- `crates/fa-notify/**` (add the workspace dependencies you need: `fa-store`, `fa-jobs`, `fa-connectors`, `chrono`, `chrono-tz`, `rand`, `serde_json`, `thiserror`, `tracing`, `uuid`, `sqlx`; `tokio` and `wiremock` for tests)
- `crates/fa-store/src/repo/ext_t06m2.rs` (already registered in `repo/mod.rs`) for repo methods you need that T-04 didn't provide. **Don't run `cargo sqlx prepare`**; the main session does after the wave.

## Work
1. `NotifyConfig { ntfy_url: Option<String>, ntfy_token: Option<String>, public_base_url: String }`. Build a dedicated `fa_connectors::Http::new(true)` for ntfy (plain `http` on the tailnet or localhost is normal for ntfy; this is independent of `FA_ALLOW_HTTP_FEEDS`), as T-02's doc comment on `publish` says. `NotifyService::new(store, config)` has:
   - `get_prefs(household, member) -> NotifyPrefView`: creates the pref with defaults (`{assigned, overdue, digest}`, 120, 10, no overrides) and a topic (24 chars `[a-z0-9]` from `rand`) if missing. `subscribe_url` = `{ntfy_url}/{topic}` when configured. `server_configured = ntfy_url.is_some()`.
   - `put_prefs(household, member, NotifyPrefPatch, expected_version)`: validates minutes (due_soon 5..=1440, nudge 1..=120), times, and that `copy_to_member` is an adult of the household.
   - `send_test(household, member)`: publishes "Test notification" directly (bypasses the outbox and quiet hours). Errors `NotConfigured` and `Publish(String)`.
2. `notify.send` job, exactly the plan's steps 1–6, plus the `source_reauth` producer. Use a `Notifier` trait (`async fn publish(&self, topic, &NtfyMessage) -> Result<(), NotifyError>`) with the ntfy implementation, so tests can inject a fake. Batch per member; `Click` = `public_base_url` + `click_path`; tags by trigger (`assigned` → `memo`, `overdue` → `warning`, `digest` → `sunrise`, `due_soon` → `hourglass`, `routine_nudge` → `alarm_clock`, `source_reauth` → `key`).
3. `handlers(service)` and `default_schedules()`.
4. **Tests:** with a fake notifier: disabled trigger → dropped; quiet hours (household and member override, across midnight) defer with the right `not_before`; the 60-second batch window merges three rows into one "3 updates" message with max priority; failures back off and give up after 5; child copy rows go to the adult with the prefix and follow the adult's quiet hours; not configured → dropped. With wiremock: the real ntfy client path sends the right headers and token. `get_prefs` creates a topic once and returns the same one after.

## Acceptance
- [ ] `cargo test -p fa-notify` passes (dev DB online; fresh Postgres on port 55406 as in the rules)
- [ ] `cargo clippy -p fa-notify --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] Report the exact public API of `NotifyService` for T-09, and the env vars to document (`FA_NTFY_URL`, `FA_NTFY_TOKEN`, and whether plain `http` ntfy URLs are allowed by default)

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
